// Iced has built-in support for some keyboard shortcuts. This module provides
// support for implementing other shortcuts until Iced provides its own support.

use std::collections::VecDeque;

use iced::widget::text_input;

use crate::{
    gui::{
        common::{EditAction, Message, UndoSubject},
        modal, style,
        widget::{Element, TextInput, Undoable},
    },
    prelude::StrictPath,
    resource::config::Config,
};

fn path_appears_valid(path: &str) -> bool {
    !path.contains("://")
}

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
    pub sources: Vec<TextHistory>,
}

impl TextHistories {
    pub fn new(_config: &Config, sources: &[StrictPath]) -> Self {
        Self {
            sources: sources.iter().map(TextHistory::path).collect(),
        }
    }

    pub fn input<'a>(&self, subject: UndoSubject) -> Element<'a> {
        let current = match &subject {
            UndoSubject::Source { index } => self.sources[*index].current(),
            // UndoSubject::Source(i) => self.sources.get(*i).map(|x| x.current()).unwrap_or_default(),
        };

        let event: Box<dyn Fn(String) -> Message> = match subject.clone() {
            UndoSubject::Source { index } => Box::new(move |value| Message::Modal {
                event: modal::Event::EditedSource {
                    action: EditAction::Change(index, value),
                },
            }),
            // UndoSubject::Source(i) => Box::new(move |value| Message::EditedSource(EditAction::Change(i, value))),
        };

        let placeholder = match &subject {
            UndoSubject::Source { .. } => "".to_string(),
        };

        let icon = match &subject {
            UndoSubject::Source { .. } => (!path_appears_valid(&current)).then_some(text_input::Icon {
                font: crate::gui::font::ICONS,
                code_point: crate::gui::icon::Icon::Error.as_char(),
                size: None,
                spacing: 5.0,
                side: text_input::Side::Right,
            }),
        };

        Undoable::new(
            {
                let mut input = TextInput::new(&placeholder, &current)
                    .on_input(event)
                    .class(style::TextInput)
                    // TODO: Would like to fill up to a max width, but fill always overrides parent's max width.
                    .width(400)
                    .padding(5);

                if let Some(icon) = icon {
                    input = input.icon(icon);
                }

                input
            },
            move |action| Message::UndoRedo(action, subject.clone()),
        )
        .into()
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
