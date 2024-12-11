// Iced has built-in support for some keyboard shortcuts. This module provides
// support for implementing other shortcuts until Iced provides its own support.

use std::collections::VecDeque;

use crate::{prelude::StrictPath, resource::config::Config};

#[derive(Debug, Clone, Copy)]
pub enum Shortcut {
    Undo,
    Redo,
}

impl From<crate::gui::undoable::Action> for Shortcut {
    fn from(source: crate::gui::undoable::Action) -> Self {
        match source {
            crate::gui::undoable::Action::Undo => Self::Undo,
            crate::gui::undoable::Action::Redo => Self::Redo,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextHistory {
    history: VecDeque<String>,
    limit: usize,
    position: usize,
}

impl Default for TextHistory {
    fn default() -> Self {
        Self::new("", 100)
    }
}

impl TextHistory {
    pub fn new(initial: &str, limit: usize) -> Self {
        let mut history = VecDeque::<String>::new();
        history.push_back(initial.to_string());
        Self {
            history,
            limit,
            position: 0,
        }
    }

    pub fn raw(initial: &str) -> Self {
        Self::new(initial, 100)
    }

    pub fn path(initial: &StrictPath) -> Self {
        Self::raw(&initial.raw())
    }

    pub fn push(&mut self, text: &str) {
        if self.current() == text {
            return;
        }
        if self.position + 1 < self.history.len() {
            self.history.truncate(self.position + 1);
        }
        if self.position + 1 >= self.limit {
            self.history.pop_front();
        }
        self.history.push_back(text.to_string());
        self.position = self.history.len() - 1;
    }

    pub fn current(&self) -> String {
        match self.history.get(self.position) {
            Some(x) => x.to_string(),
            None => "".to_string(),
        }
    }

    pub fn undo(&mut self) -> String {
        self.position = if self.position == 0 { 0 } else { self.position - 1 };
        self.current()
    }

    pub fn redo(&mut self) -> String {
        self.position = std::cmp::min(self.position + 1, self.history.len() - 1);
        self.current()
    }

    pub fn apply(&mut self, shortcut: Shortcut) -> String {
        match shortcut {
            Shortcut::Undo => self.undo(),
            Shortcut::Redo => self.redo(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TextHistories {
    pub image_duration: TextHistory,
}

impl TextHistories {
    pub fn new(config: &Config) -> Self {
        Self {
            image_duration: TextHistory::raw(&config.playback.image_duration.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_history() {
        let mut ht = TextHistory::new("initial", 3);

        assert_eq!(ht.current(), "initial");
        assert_eq!(ht.undo(), "initial");
        assert_eq!(ht.redo(), "initial");

        ht.push("a");
        assert_eq!(ht.current(), "a");
        assert_eq!(ht.undo(), "initial");
        assert_eq!(ht.undo(), "initial");
        assert_eq!(ht.redo(), "a");
        assert_eq!(ht.redo(), "a");

        // Duplicates are ignored:
        ht.push("a");
        ht.push("a");
        ht.push("a");
        assert_eq!(ht.undo(), "initial");

        // History is clipped at the limit:
        ht.push("b");
        ht.push("c");
        ht.push("d");
        assert_eq!(ht.undo(), "c");
        assert_eq!(ht.undo(), "b");
        assert_eq!(ht.undo(), "b");

        // Redos are lost on push:
        ht.push("e");
        assert_eq!(ht.current(), "e");
        assert_eq!(ht.redo(), "e");
        assert_eq!(ht.undo(), "b");
        assert_eq!(ht.undo(), "b");
    }
}
