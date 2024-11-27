use iced::widget as w;

use crate::gui::{common::Message, style::Theme};

pub type Renderer = iced::Renderer;

pub type Element<'a> = iced::Element<'a, Message, Theme, Renderer>;

pub type Button<'a> = w::Button<'a, Message, Theme, Renderer>;
pub type Checkbox<'a> = w::Checkbox<'a, Message, Theme, Renderer>;
pub type Column<'a> = w::Column<'a, Message, Theme, Renderer>;
pub type Container<'a> = w::Container<'a, Message, Theme, Renderer>;
pub type PickList<'a, T, L, V> = w::PickList<'a, T, L, V, Message, Theme, Renderer>;
pub type Responsive<'a> = w::Responsive<'a, Message, Theme, Renderer>;
pub type Row<'a> = w::Row<'a, Message, Theme, Renderer>;
pub type Scrollable<'a> = w::Scrollable<'a, Message, Theme, Renderer>;
pub type Stack<'a> = w::Stack<'a, Message, Theme, Renderer>;
pub type Text<'a> = w::Text<'a, Theme, Renderer>;
pub type TextInput<'a> = w::TextInput<'a, Message, Theme, Renderer>;
pub type Tooltip<'a> = w::Tooltip<'a, Message, Theme, Renderer>;
pub type Undoable<'a, F> = crate::gui::undoable::Undoable<'a, Message, Theme, Renderer, F>;

pub use w::Space;

pub fn checkbox<'a>(label: impl Into<String>, is_checked: bool, f: impl Fn(bool) -> Message + 'a) -> Checkbox<'a> {
    Checkbox::new(label, is_checked)
        .on_toggle(f)
        .size(20)
        .text_shaping(w::text::Shaping::Advanced)
}

pub fn pick_list<'a, T, L, V>(
    options: L,
    selected: Option<V>,
    on_selected: impl Fn(T) -> Message + 'a,
) -> PickList<'a, T, L, V>
where
    T: ToString + PartialEq + Clone,
    L: std::borrow::Borrow<[T]> + 'a,
    V: std::borrow::Borrow<T> + 'a,
    Message: Clone,
    Renderer: iced::advanced::text::Renderer,
{
    PickList::new(options, selected, on_selected)
        .text_shaping(w::text::Shaping::Advanced)
        .padding(5)
}

pub fn text<'a>(content: impl iced::widget::text::IntoFragment<'a>) -> Text<'a> {
    Text::new(content).shaping(w::text::Shaping::Advanced)
}
