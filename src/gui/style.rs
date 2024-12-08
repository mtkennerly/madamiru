use iced::{
    border::Radius,
    widget::{button, checkbox, container, pane_grid, pick_list, scrollable, slider, svg, text_input},
    Background, Border, Color, Shadow, Vector,
};

use crate::resource::config;

macro_rules! rgb8 {
    ($r:expr, $g:expr, $b:expr) => {
        Color::from_rgb($r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0)
    };
}

trait ColorExt {
    fn alpha(self, alpha: f32) -> Color;
}

impl ColorExt for Color {
    fn alpha(mut self, alpha: f32) -> Self {
        self.a = alpha;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    background: Color,
    field: Color,
    text: Color,
    text_button: Color,
    text_selection: Color,
    positive: Color,
    negative: Color,
    disabled: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from(config::Theme::Light)
    }
}

impl From<config::Theme> for Theme {
    fn from(source: config::Theme) -> Self {
        match source {
            config::Theme::Light => Self {
                background: Color::WHITE,
                field: rgb8!(230, 230, 230),
                text: Color::BLACK,
                text_button: Color::WHITE,
                text_selection: Color::from_rgb(0.8, 0.8, 1.0),
                positive: rgb8!(28, 107, 223),
                negative: rgb8!(255, 0, 0),
                disabled: rgb8!(169, 169, 169),
            },
            config::Theme::Dark => Self {
                background: rgb8!(41, 41, 41),
                field: rgb8!(74, 74, 74),
                text: Color::WHITE,
                ..Self::from(config::Theme::Light)
            },
        }
    }
}

impl iced::application::DefaultStyle for Theme {
    fn default_style(&self) -> iced::daemon::Appearance {
        iced::application::Appearance {
            background_color: self.background,
            text_color: self.text,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Text;
impl iced::widget::text::Catalog for Theme {
    type Class<'a> = Text;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, _item: &Self::Class<'_>) -> iced::widget::text::Style {
        iced::widget::text::Style { color: None }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Menu;
impl iced::widget::overlay::menu::Catalog for Theme {
    type Class<'a> = Menu;

    fn default<'a>() -> <Self as iced::overlay::menu::Catalog>::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &<Self as iced::overlay::menu::Catalog>::Class<'_>) -> iced::overlay::menu::Style {
        iced::overlay::menu::Style {
            background: self.field.into(),
            border: Border {
                color: self.text.alpha(0.5),
                width: 1.0,
                radius: 5.0.into(),
            },
            text_color: self.text,
            selected_background: self.positive.into(),
            selected_text_color: Color::WHITE,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Button {
    #[default]
    Primary,
    Negative,
    Icon,
}
impl button::Catalog for Theme {
    type Class<'a> = Button;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, class: &Self::Class<'_>, status: button::Status) -> button::Style {
        let active = button::Style {
            background: match class {
                Button::Primary => Some(self.positive.into()),
                Button::Negative => Some(self.negative.into()),
                Button::Icon => None,
            },
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 10.0.into(),
            },
            text_color: match class {
                Button::Icon => self.text,
                _ => self.text_button,
            },
            shadow: Shadow {
                offset: Vector::new(1.0, 1.0),
                ..Default::default()
            },
        };

        match status {
            button::Status::Active => active,
            button::Status::Hovered => button::Style {
                background: match class {
                    Button::Primary => Some(self.positive.alpha(0.8).into()),
                    Button::Negative => Some(self.negative.alpha(0.8).into()),
                    Button::Icon => Some(self.text.alpha(0.2).into()),
                },
                border: active.border,
                text_color: match class {
                    Button::Icon => self.text.alpha(0.9),
                    _ => self.text_button.alpha(0.9),
                },
                shadow: Shadow {
                    offset: Vector::new(1.0, 2.0),
                    ..Default::default()
                },
            },
            button::Status::Pressed => button::Style {
                shadow: Shadow {
                    offset: Vector::default(),
                    ..active.shadow
                },
                ..active
            },
            button::Status::Disabled => button::Style {
                shadow: Shadow {
                    offset: Vector::default(),
                    ..active.shadow
                },
                background: active.background.map(|background| match background {
                    Background::Color(color) => Background::Color(Color {
                        a: color.a * 0.5,
                        ..color
                    }),
                    Background::Gradient(gradient) => Background::Gradient(gradient.scale_alpha(0.5)),
                }),
                text_color: Color {
                    a: active.text_color.a * 0.5,
                    ..active.text_color
                },
                ..active
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Container {
    #[default]
    Wrapper,
    Primary,
    ModalForeground,
    ModalBackground,
    Player,
    PlayerGroup,
    PlayerGroupControls,
    PlayerGroupTitle,
    Tooltip,
}
impl container::Catalog for Theme {
    type Class<'a> = Container;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, class: &Self::Class<'_>) -> container::Style {
        container::Style {
            background: Some(match class {
                Container::Wrapper => Color::TRANSPARENT.into(),
                Container::Player => self.field.alpha(0.15).into(),
                Container::PlayerGroup => self.field.alpha(0.3).into(),
                Container::PlayerGroupControls => self.field.into(),
                Container::PlayerGroupTitle => self.field.alpha(0.45).into(),
                Container::ModalBackground => self.field.alpha(0.5).into(),
                Container::Tooltip => self.field.into(),
                _ => self.background.into(),
            }),
            border: Border {
                color: match class {
                    Container::Wrapper => Color::TRANSPARENT,
                    Container::Player => self.field.alpha(0.8),
                    Container::PlayerGroup | Container::PlayerGroupTitle => self.field,
                    Container::PlayerGroupControls => self.disabled,
                    Container::ModalForeground => self.disabled,
                    _ => self.text,
                },
                width: match class {
                    Container::Player
                    | Container::PlayerGroup
                    | Container::PlayerGroupControls
                    | Container::PlayerGroupTitle
                    | Container::ModalForeground => 1.0,
                    _ => 0.0,
                },
                radius: match class {
                    Container::ModalForeground | Container::Player | Container::PlayerGroupControls => 10.0.into(),
                    Container::PlayerGroup => Radius::new(10.0).top(0.0),
                    Container::PlayerGroupTitle => Radius::new(10.0).bottom(0.0),
                    Container::ModalBackground => 5.0.into(),
                    Container::Tooltip => 20.0.into(),
                    _ => 0.0.into(),
                },
            },
            text_color: match class {
                Container::Wrapper => None,
                _ => Some(self.text),
            },
            shadow: Shadow {
                color: Color::TRANSPARENT,
                offset: Vector::ZERO,
                blur_radius: 0.0,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Scrollable;
impl scrollable::Catalog for Theme {
    type Class<'a> = Scrollable;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &Self::Class<'_>, status: scrollable::Status) -> scrollable::Style {
        let active = scrollable::Style {
            container: container::Style::default(),
            vertical_rail: scrollable::Rail {
                background: Some(Color::TRANSPARENT.into()),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 5.0.into(),
                },
                scroller: scrollable::Scroller {
                    color: self.text.alpha(0.7),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: 5.0.into(),
                    },
                },
            },
            horizontal_rail: scrollable::Rail {
                background: Some(Color::TRANSPARENT.into()),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 5.0.into(),
                },
                scroller: scrollable::Scroller {
                    color: self.text.alpha(0.7),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: 5.0.into(),
                    },
                },
            },
            gap: None,
        };

        match status {
            scrollable::Status::Active => active,
            scrollable::Status::Hovered {
                is_horizontal_scrollbar_hovered,
                is_vertical_scrollbar_hovered,
            } => {
                if !is_horizontal_scrollbar_hovered && !is_vertical_scrollbar_hovered {
                    return active;
                }

                scrollable::Style {
                    vertical_rail: scrollable::Rail {
                        background: Some(self.text.alpha(0.4).into()),
                        border: Border {
                            color: self.text.alpha(0.8),
                            ..active.vertical_rail.border
                        },
                        ..active.vertical_rail
                    },
                    horizontal_rail: scrollable::Rail {
                        background: Some(self.text.alpha(0.4).into()),
                        border: Border {
                            color: self.text.alpha(0.8),
                            ..active.horizontal_rail.border
                        },
                        ..active.horizontal_rail
                    },
                    ..active
                }
            }
            scrollable::Status::Dragged { .. } => self.style(
                _class,
                scrollable::Status::Hovered {
                    is_horizontal_scrollbar_hovered: true,
                    is_vertical_scrollbar_hovered: true,
                },
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PickList;
impl pick_list::Catalog for Theme {
    type Class<'a> = PickList;

    fn default<'a>() -> <Self as pick_list::Catalog>::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &<Self as pick_list::Catalog>::Class<'_>, status: pick_list::Status) -> pick_list::Style {
        let active = pick_list::Style {
            border: Border {
                color: self.text.alpha(0.7),
                width: 1.0,
                radius: 5.0.into(),
            },
            background: self.field.alpha(0.6).into(),
            text_color: self.text,
            placeholder_color: iced::Color::BLACK,
            handle_color: self.text,
        };

        match status {
            pick_list::Status::Active => active,
            pick_list::Status::Hovered => pick_list::Style {
                background: self.field.into(),
                ..active
            },
            pick_list::Status::Opened => active,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Checkbox;
impl checkbox::Catalog for Theme {
    type Class<'a> = Checkbox;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &Self::Class<'_>, status: checkbox::Status) -> checkbox::Style {
        let active = checkbox::Style {
            background: self.field.alpha(0.6).into(),
            icon_color: self.text,
            border: Border {
                color: self.text.alpha(0.6),
                width: 1.0,
                radius: 5.0.into(),
            },
            text_color: Some(self.text),
        };

        match status {
            checkbox::Status::Active { .. } => active,
            checkbox::Status::Hovered { .. } => checkbox::Style {
                background: self.field.into(),
                ..active
            },
            checkbox::Status::Disabled { .. } => checkbox::Style {
                background: match active.background {
                    Background::Color(color) => Background::Color(Color {
                        a: color.a * 0.5,
                        ..color
                    }),
                    Background::Gradient(gradient) => Background::Gradient(gradient.scale_alpha(0.5)),
                },
                ..active
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TextInput;
impl text_input::Catalog for Theme {
    type Class<'a> = TextInput;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &Self::Class<'_>, status: text_input::Status) -> text_input::Style {
        let active = text_input::Style {
            background: Color::TRANSPARENT.into(),
            border: Border {
                color: self.text.alpha(0.8),
                width: 1.0,
                radius: 5.0.into(),
            },
            icon: self.negative,
            placeholder: self.text.alpha(0.5),
            value: self.text,
            selection: self.text_selection,
        };

        match status {
            text_input::Status::Active => active,
            text_input::Status::Hovered | text_input::Status::Focused => text_input::Style {
                border: Border {
                    color: self.text,
                    ..active.border
                },
                ..active
            },
            text_input::Status::Disabled => text_input::Style {
                background: self.disabled.into(),
                value: self.text.alpha(0.5),
                ..active
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Slider;
impl iced::widget::slider::Catalog for Theme {
    type Class<'a> = Slider;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &Self::Class<'_>, status: slider::Status) -> slider::Style {
        let fade = 0.75;

        let active = slider::Style {
            rail: slider::Rail {
                backgrounds: (self.positive.alpha(fade).into(), self.field.alpha(fade).into()),
                width: 5.0,
                border: Border {
                    color: self.field.alpha(fade),
                    width: 1.0,
                    radius: 5.0.into(),
                },
            },
            handle: slider::Handle {
                shape: slider::HandleShape::Circle { radius: 5.0 },
                background: self.positive.alpha(fade).into(),
                border_width: 1.0,
                border_color: self.field.alpha(fade),
            },
        };

        match status {
            slider::Status::Active => active,
            slider::Status::Hovered | slider::Status::Dragged => slider::Style {
                rail: slider::Rail {
                    backgrounds: (self.positive.into(), self.field.into()),
                    ..active.rail
                },
                handle: slider::Handle {
                    background: self.positive.into(),
                    border_color: self.field,
                    ..active.handle
                },
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Svg;
impl svg::Catalog for Theme {
    type Class<'a> = Svg;

    fn default<'a>() -> Self::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &Self::Class<'_>, _status: svg::Status) -> svg::Style {
        svg::Style { color: None }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PaneGrid;
impl pane_grid::Catalog for Theme {
    type Class<'a> = PaneGrid;

    fn default<'a>() -> <Self as pane_grid::Catalog>::Class<'a> {
        Default::default()
    }

    fn style(&self, _class: &<Self as pane_grid::Catalog>::Class<'_>) -> pane_grid::Style {
        pane_grid::Style {
            hovered_region: pane_grid::Highlight {
                background: self.positive.alpha(0.5).into(),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 5.0.into(),
                },
            },
            hovered_split: pane_grid::Line {
                color: self.disabled,
                width: 2.0,
            },
            picked_split: pane_grid::Line {
                color: self.disabled.alpha(0.8),
                width: 2.0,
            },
        }
    }
}
