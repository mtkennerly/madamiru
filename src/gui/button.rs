use iced::{alignment, keyboard, widget::tooltip, Padding};

use crate::{
    gui::{
        common::{BrowseFileSubject, BrowseSubject, EditAction, Message},
        icon::Icon,
        style,
        widget::{text, Button, Container, Element, Tooltip},
    },
    lang,
    path::StrictPath,
};

pub struct CustomButton<'a> {
    content: Element<'a>,
    on_press: Option<Message>,
    class: style::Button,
    padding: Option<Padding>,
    tooltip: Option<String>,
    tooltip_position: tooltip::Position,
}

impl CustomButton<'_> {
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    pub fn tooltip(mut self, tooltip: String) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    pub fn tooltip_below(mut self, tooltip: String) -> Self {
        self.tooltip = Some(tooltip);
        self.tooltip_position = tooltip::Position::Bottom;
        self
    }
}

impl<'a> From<CustomButton<'a>> for Element<'a> {
    fn from(value: CustomButton<'a>) -> Self {
        let mut button = Button::new(value.content)
            .class(value.class)
            .on_press_maybe(value.on_press);

        if let Some(padding) = value.padding {
            button = button.padding(padding);
        }

        match value.tooltip {
            Some(tooltip) => Tooltip::new(
                button,
                Container::new(text(tooltip).size(14)).padding([2, 4]),
                value.tooltip_position,
            )
            .gap(5)
            .class(style::Container::Tooltip)
            .into(),
            None => button.into(),
        }
    }
}

pub fn primary<'a>(content: String) -> CustomButton<'a> {
    CustomButton {
        content: text(content).align_x(alignment::Horizontal::Center).into(),
        on_press: None,
        class: style::Button::Primary,
        padding: Some([5, 40].into()),
        tooltip: None,
        tooltip_position: tooltip::Position::Top,
    }
}

pub fn negative<'a>(content: String) -> CustomButton<'a> {
    CustomButton {
        content: text(content).align_x(alignment::Horizontal::Center).into(),
        on_press: None,
        class: style::Button::Negative,
        padding: Some([5, 40].into()),
        tooltip: None,
        tooltip_position: tooltip::Position::Top,
    }
}

pub fn icon<'a>(icon: Icon) -> CustomButton<'a> {
    CustomButton {
        content: icon.small_control().into(),
        on_press: None,
        class: style::Button::Icon,
        padding: None,
        tooltip: None,
        tooltip_position: tooltip::Position::Top,
    }
}

pub fn big_icon<'a>(icon: Icon) -> CustomButton<'a> {
    CustomButton {
        content: icon.big_control().into(),
        on_press: None,
        class: style::Button::Icon,
        padding: None,
        tooltip: None,
        tooltip_position: tooltip::Position::Top,
    }
}

pub fn choose_folder<'a>(subject: BrowseSubject, raw: StrictPath, modifiers: &keyboard::Modifiers) -> CustomButton<'a> {
    if modifiers.shift() {
        icon(Icon::OpenInNew)
            .on_press(Message::OpenDir { path: raw })
            .tooltip(lang::action::open_folder())
    } else {
        icon(Icon::FolderOpen)
            .on_press(Message::BrowseDir(subject))
            .tooltip(lang::action::select_folder())
    }
}

pub fn choose_file<'a>(
    subject: BrowseFileSubject,
    raw: StrictPath,
    modifiers: &keyboard::Modifiers,
) -> CustomButton<'a> {
    if modifiers.shift() {
        icon(Icon::FileOpen)
            .on_press(Message::OpenFile { path: raw })
            .tooltip(lang::action::open_folder_of_file())
    } else {
        icon(Icon::File)
            .on_press(Message::BrowseFile(subject))
            .tooltip(lang::action::select_file())
    }
}

pub fn move_up<'a>(action: fn(EditAction) -> Message, index: usize) -> CustomButton<'a> {
    icon(Icon::ArrowUpward).on_press_maybe((index > 0).then(|| action(EditAction::move_up(index))))
}

pub fn move_down<'a>(action: fn(EditAction) -> Message, index: usize, max: usize) -> CustomButton<'a> {
    icon(Icon::ArrowDownward).on_press_maybe((index < max - 1).then(|| action(EditAction::move_down(index))))
}
