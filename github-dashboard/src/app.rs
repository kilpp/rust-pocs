use chrono::{DateTime, Local};

use crate::config::Config;
use crate::github::{Contributions, Pr};
use crate::lang::{self, LangStat};

#[derive(Debug, Clone)]
pub enum Status {
    Loading,
    Ready,
    Error(String),
}

/// How the Languages table is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Total,
    Open,
    Closed,
    Name,
}

impl SortMode {
    pub fn label(self) -> &'static str {
        match self {
            SortMode::Total => "total",
            SortMode::Open => "open",
            SortMode::Closed => "closed",
            SortMode::Name => "name",
        }
    }

    pub fn next(self) -> Self {
        match self {
            SortMode::Total => SortMode::Open,
            SortMode::Open => SortMode::Closed,
            SortMode::Closed => SortMode::Name,
            SortMode::Name => SortMode::Total,
        }
    }

    fn apply(self, stats: &mut [LangStat]) {
        match self {
            SortMode::Total => stats.sort_by(|a, b| {
                b.total().cmp(&a.total()).then_with(|| a.language.cmp(&b.language))
            }),
            SortMode::Open => stats.sort_by(|a, b| {
                b.prs_open.cmp(&a.prs_open).then_with(|| a.language.cmp(&b.language))
            }),
            SortMode::Closed => stats.sort_by(|a, b| {
                b.prs_closed.cmp(&a.prs_closed).then_with(|| a.language.cmp(&b.language))
            }),
            SortMode::Name => stats.sort_by(|a, b| a.language.cmp(&b.language)),
        }
    }
}

/// The full dataset produced by one refresh.
pub struct DashboardData {
    /// Raw PRs paired with their changed filenames; stats are derived on demand
    /// so the user filter and sort mode can be applied without re-fetching.
    pub prs: Vec<(Pr, Vec<String>)>,
    pub contributions: Vec<Contributions>,
}

/// Messages produced by background tasks and applied to the [`App`].
pub enum AppEvent {
    FetchDone(Result<DashboardData, String>),
    SummaryDone {
        language: String,
        result: Result<String, String>,
    },
}

/// Number of PRs the summary panel lists (and the PR cursor can reach).
pub const PR_LIST_LIMIT: usize = 15;

pub struct App {
    pub users: Vec<String>,
    pub base_url: String,
    pub timeline_days: u32,

    /// Raw fetched PRs; `stats` is derived from this via [`App::recompute`].
    all_prs: Vec<(Pr, Vec<String>)>,
    pub stats: Vec<LangStat>,
    pub contributions: Vec<Contributions>,
    pub selected: usize,
    /// Cursor into the selected language's PR list (summary panel).
    pub selected_pr: usize,
    /// Index into `users` to filter by, or `None` for all users.
    pub user_filter: Option<usize>,
    pub sort_mode: SortMode,
    pub status: Status,
    pub last_refresh: Option<DateTime<Local>>,
    /// When the in-flight fetch started, for the elapsed-time indicator.
    pub loading_since: Option<DateTime<Local>>,
    /// Free-running counter driving the loading spinner animation.
    pub tick: u64,

    /// Summary text for the currently selected language (cleared on selection change).
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
            stats: Vec::new(),
            contributions: Vec::new(),
            selected: 0,
            selected_pr: 0,
            user_filter: None,
            sort_mode: SortMode::Total,
            status: Status::Loading,
            last_refresh: None,
            loading_since: Some(Local::now()),
            tick: 0,
            summary: None,
            summarizing: false,
        }
    }

    pub fn selected_lang(&self) -> Option<&LangStat> {
        self.stats.get(self.selected)
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

    /// Rebuild `stats` from `all_prs`, applying the user filter and sort mode.
    fn recompute(&mut self) {
        let filter = self.filter_user().map(str::to_string);
        let filtered: Vec<(Pr, Vec<String>)> = self
            .all_prs
            .iter()
            .filter(|(pr, _)| match &filter {
                Some(user) => pr.involved_users.iter().any(|u| u == user),
                None => true,
            })
            .cloned()
            .collect();

        let mut stats = lang::aggregate(filtered);
        self.sort_mode.apply(&mut stats);
        self.stats = stats;

        if self.selected >= self.stats.len() {
            self.selected = 0;
        }
        self.selected_pr = 0;
    }

    /// How many PRs the cursor can move through for the selected language.
    fn pr_count(&self) -> usize {
        self.selected_lang()
            .map(|l| l.prs.len().min(PR_LIST_LIMIT))
            .unwrap_or(0)
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

    pub fn cycle_sort(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.recompute();
        self.on_selection_change();
    }

    pub fn select_pr_next(&mut self) {
        let n = self.pr_count();
        if n == 0 {
            return;
        }
        self.selected_pr = (self.selected_pr + 1) % n;
    }

    pub fn select_pr_prev(&mut self) {
        let n = self.pr_count();
        if n == 0 {
            return;
        }
        self.selected_pr = (self.selected_pr + n - 1) % n;
    }

    /// Browser URL for the PR under the cursor, if any.
    pub fn selected_pr_url(&self) -> Option<String> {
        let pr = self.selected_lang()?.prs.get(self.selected_pr)?;
        Some(crate::github::pr_web_url(&self.base_url, &pr.repo, pr.number))
    }

    pub fn total_prs(&self) -> usize {
        // PRs may appear under multiple languages; this is a sum of per-language
        // counts, i.e. total language-attributions rather than unique PRs.
        self.stats.iter().map(LangStat::total).sum()
    }

    pub fn select_next(&mut self) {
        if self.stats.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.stats.len();
        self.on_selection_change();
    }

    pub fn select_prev(&mut self) {
        if self.stats.is_empty() {
            return;
        }
        self.selected = (self.selected + self.stats.len() - 1) % self.stats.len();
        self.on_selection_change();
    }

    fn on_selection_change(&mut self) {
        self.selected_pr = 0;
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
            AppEvent::SummaryDone { language, result } => {
                // Ignore results for a language the user has navigated away from.
                if self.selected_lang().map(|l| l.language.as_str()) != Some(language.as_str()) {
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
