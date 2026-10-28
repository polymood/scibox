//! The command palette of the TUI: the query and the list of the matches.

use ratatui::widgets::ListState;
use scibox_core::search::{self, Entry};

pub struct Palette {
    pub query: String,
    pub list: ListState,
    /// The indices of the entries that agree with the query, best first.
    pub matches: Vec<usize>,
}

impl Palette {
    pub fn new(entries: &[Entry]) -> Self {
        let mut p = Palette {
            query: String::new(),
            list: ListState::default(),
            matches: Vec::new(),
        };
        p.update(entries);
        p
    }

    /// Find the entries that agree with the query again.
    pub fn update(&mut self, entries: &[Entry]) {
        self.matches = search::find(entries, &self.query);
        self.list.select(Some(0));
    }
}
