use std::cmp::Reverse;

use chrono::{DateTime, Local};

use crate::config::Config;
use crate::github::{Contributions, Pr};

#[derive(Debug, Clone)]
pub enum Status {
    Loading,
    Ready,
    Error(String),
}

/// The full dataset produced by one refresh.
pub struct DashboardData {
    pub prs: Vec<Pr>,
    pub contributions: Vec<Contributions>,
}

/// Messages produced by background tasks and applied to the [`App`].
pub enum AppEvent {
    FetchDone(Result<DashboardData, String>),
    SummaryDone {
        /// Identifies the PR the summary is for, so stale results are ignored.
        key: String,
        result: Result<String, String>,
    },
}

pub struct App {
    pub users: Vec<String>,
    pub base_url: String,
    pub timeline_days: u32,

    /// Raw fetched PRs; `prs` is derived from this via [`App::recompute`].
    all_prs: Vec<Pr>,
    /// PRs under the current user filter, ordered open-first then closed.
    pub prs: Vec<Pr>,
    pub contributions: Vec<Contributions>,
    /// Cursor into `prs`.
    pub selected: usize,
    /// Index into `users` to filter by, or `None` for all users.
    pub user_filter: Option<usize>,
    /// When set, show only open, ready PRs awaiting a review from a configured
    /// user (the "waiting on you" review queue).
    pub waiting_only: bool,
    pub status: Status,
    pub last_refresh: Option<DateTime<Local>>,
    /// When the in-flight fetch started, for the elapsed-time indicator.
    pub loading_since: Option<DateTime<Local>>,
    /// Free-running counter driving the loading spinner animation.
    pub tick: u64,

    /// Summary text for the currently selected PR (cleared on selection change).
    pub summary: Option<String>,
    pub summarizing: bool,
}

impl App {
    pub fn new(config: &Config) -> Self {
        Self {
            users: config.users.clone(),
            base_url: config.base_url.clone(),
            timeline_days: config.timeline_days,
            all_prs: Vec::new(),
            prs: Vec::new(),
            contributions: Vec::new(),
            selected: 0,
            user_filter: None,
            waiting_only: false,
            status: Status::Loading,
            last_refresh: None,
            loading_since: Some(Local::now()),
            tick: 0,
            summary: None,
            summarizing: false,
        }
    }

    pub fn selected_pr(&self) -> Option<&Pr> {
        self.prs.get(self.selected)
    }

    /// Configured user the current filter points at, if any.
    pub fn filter_user(&self) -> Option<&str> {
        self.user_filter
            .and_then(|i| self.users.get(i))
            .map(String::as_str)
    }

    pub fn filter_label(&self) -> &str {
        self.filter_user().unwrap_or("all")
    }

    /// Contributions visible under the current user filter.
    pub fn visible_contributions(&self) -> Vec<&Contributions> {
        match self.filter_user() {
            Some(user) => self
                .contributions
                .iter()
                .filter(|c| c.user == user)
                .collect(),
            None => self.contributions.iter().collect(),
        }
    }

    pub fn open_count(&self) -> usize {
        self.prs.iter().filter(|p| p.is_open()).count()
    }

    pub fn closed_count(&self) -> usize {
        self.prs.len() - self.open_count()
    }

    /// Users the "waiting on you" filter checks against: the single filtered
    /// user when one is selected, otherwise every configured user.
    fn waiting_users(&self) -> Vec<String> {
        match self.filter_user() {
            Some(user) => vec![user.to_string()],
            None => self.users.clone(),
        }
    }

    /// How many PRs (across all users, ignoring the waiting filter itself) are
    /// awaiting a review from a configured user. Drives the header badge.
    pub fn waiting_count(&self) -> usize {
        let users = self.waiting_users();
        self.all_prs.iter().filter(|p| p.waiting_on(&users)).count()
    }

    /// Rebuild `prs` from `all_prs`, applying the user filter, the optional
    /// "waiting on you" filter, and the open-first ordering (most recent PR
    /// number first within each group).
    fn recompute(&mut self) {
        let filter = self.filter_user().map(str::to_string);
        let waiting_users = self.waiting_users();
        let mut filtered: Vec<Pr> = self
            .all_prs
            .iter()
            .filter(|pr| match &filter {
                Some(user) => pr.involved_users.iter().any(|u| u == user),
                None => true,
            })
            .filter(|pr| !self.waiting_only || pr.waiting_on(&waiting_users))
            .cloned()
            .collect();

        // Open PRs (is_open == true) come before closed; `!is_open` maps open
        // to false (0) so it sorts first. Newer PR numbers lead within a group.
        filtered.sort_by_key(|pr| (!pr.is_open(), Reverse(pr.number)));
        self.prs = filtered;

        if self.selected >= self.prs.len() {
            self.selected = 0;
        }
    }

    /// Toggle the "waiting on you" review-queue filter.
    pub fn toggle_waiting(&mut self) {
        self.waiting_only = !self.waiting_only;
        self.recompute();
        self.on_selection_change();
    }

    pub fn cycle_user(&mut self) {
        let n = self.users.len();
        self.user_filter = match self.user_filter {
            _ if n == 0 => None,
            None => Some(0),
            Some(i) if i + 1 < n => Some(i + 1),
            Some(_) => None,
        };
        self.recompute();
        self.on_selection_change();
    }

    /// Browser URL for the PR under the cursor, if any.
    pub fn selected_pr_url(&self) -> Option<String> {
        let pr = self.selected_pr()?;
        Some(crate::github::pr_web_url(&self.base_url, &pr.repo, pr.number))
    }

    pub fn select_next(&mut self) {
        if self.prs.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.prs.len();
        self.on_selection_change();
    }

    pub fn select_prev(&mut self) {
        if self.prs.is_empty() {
            return;
        }
        self.selected = (self.selected + self.prs.len() - 1) % self.prs.len();
        self.on_selection_change();
    }

    fn on_selection_change(&mut self) {
        self.summary = None;
        self.summarizing = false;
    }

    pub fn begin_loading(&mut self) {
        self.status = Status::Loading;
        self.loading_since = Some(Local::now());
    }

    pub fn apply(&mut self, event: AppEvent) {
        match event {
            AppEvent::FetchDone(Ok(data)) => {
                self.all_prs = data.prs;
                self.contributions = data.contributions;
                self.selected = 0;
                self.recompute();
                self.summary = None;
                self.summarizing = false;
                self.status = Status::Ready;
                self.last_refresh = Some(Local::now());
                self.loading_since = None;
            }
            AppEvent::FetchDone(Err(e)) => {
                self.status = Status::Error(e);
                self.loading_since = None;
            }
            AppEvent::SummaryDone { key, result } => {
                // Ignore results for a PR the user has navigated away from.
                if self.selected_pr().map(Pr::key) != Some(key) {
                    return;
                }
                self.summarizing = false;
                match result {
                    Ok(text) => self.summary = Some(text),
                    Err(e) => self.summary = Some(format!("Error: {e}")),
                }
            }
        }
    }
}
