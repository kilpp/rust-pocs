use chrono::{DateTime, Local};

use crate::config::Config;
use crate::github::Contributions;
use crate::lang::LangStat;

#[derive(Debug, Clone)]
pub enum Status {
    Loading,
    Ready,
    Error(String),
}

/// The full dataset produced by one refresh.
pub struct DashboardData {
    pub stats: Vec<LangStat>,
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

pub struct App {
    pub users: Vec<String>,
    pub base_url: String,
    pub timeline_days: u32,

    pub stats: Vec<LangStat>,
    pub contributions: Vec<Contributions>,
    pub selected: usize,
    pub status: Status,
    pub last_refresh: Option<DateTime<Local>>,

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
            stats: Vec::new(),
            contributions: Vec::new(),
            selected: 0,
            status: Status::Loading,
            last_refresh: None,
            summary: None,
            summarizing: false,
        }
    }

    pub fn selected_lang(&self) -> Option<&LangStat> {
        self.stats.get(self.selected)
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
        self.summary = None;
        self.summarizing = false;
    }

    pub fn begin_loading(&mut self) {
        self.status = Status::Loading;
    }

    pub fn apply(&mut self, event: AppEvent) {
        match event {
            AppEvent::FetchDone(Ok(data)) => {
                self.stats = data.stats;
                self.contributions = data.contributions;
                self.selected = 0;
                self.summary = None;
                self.summarizing = false;
                self.status = Status::Ready;
                self.last_refresh = Some(Local::now());
            }
            AppEvent::FetchDone(Err(e)) => {
                self.status = Status::Error(e);
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
