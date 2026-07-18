pub type SliderNumber = f32;

pub fn keyboard_value(
    key: iced::keyboard::Key,
    _: iced::keyboard::key::Physical,
    _: iced::keyboard::Location,
    _: iced::keyboard::Modifiers,
) -> iced::keyboard::Key {
    key
}

pub fn pointer_click(
    click: iced::advanced::mouse::Click,
    _: iced::mouse::Cursor,
    _: iced::mouse::Button,
    _: iced::touch::Finger,
    _: iced::Point,
    _: iced::Rectangle,
) -> iced::advanced::mouse::Click {
    click
}

pub fn transformation_round_trip(
    value: iced::Transformation,
    _: iced::Vector,
    _: iced::Size,
) -> iced::Transformation {
    value
}

pub fn exact_rectangle() -> iced::Rectangle<u32> {
    iced::Rectangle {
        x: 1,
        y: 2,
        width: 3,
        height: 4,
    }
}

pub fn geometry_round_trip(
    _: iced::Point,
    _: iced::Point<u32>,
    _: iced::Vector,
    _: iced::Size,
    bounds: iced::Rectangle,
    _: Option<iced::Rectangle<u32>>,
) -> iced::Rectangle {
    bounds
}

pub fn unit_round_trip(
    _: iced::Pixels,
    padding: iced::Padding,
    _: iced::Degrees,
    _: iced::Radians,
) -> iced::Padding {
    padding
}

pub fn native_theme(dark: bool) -> iced::Theme {
    let palette = if dark {
        iced::theme::Palette::DARK
    } else {
        iced::theme::Palette::LIGHT
    };
    iced::Theme::custom_with_fn(
        if dark { "Native dark" } else { "Native light" },
        palette,
        move |palette| {
            let mut extended = iced::theme::palette::Extended::generate(palette);
            extended.primary.base.color = if dark {
                iced::Color::from_rgb8(0x7c, 0x3a, 0xed)
            } else {
                iced::Color::from_rgb8(0x25, 0x63, 0xeb)
            };
            extended
        },
    )
}

#[derive(Clone)]
pub struct AlternateTheme {
    active: bool,
}

impl iced::theme::Base for AlternateTheme {
    fn default(preference: iced::theme::Mode) -> Self {
        Self {
            active: preference == iced::theme::Mode::Dark,
        }
    }

    fn mode(&self) -> iced::theme::Mode {
        if self.active {
            iced::theme::Mode::Dark
        } else {
            iced::theme::Mode::Light
        }
    }

    fn base(&self) -> iced::theme::Style {
        iced::theme::Style {
            background_color: if self.active {
                iced::Color::BLACK
            } else {
                iced::Color::WHITE
            },
            text_color: if self.active {
                iced::Color::WHITE
            } else {
                iced::Color::BLACK
            },
        }
    }

    fn palette(&self) -> Option<iced::theme::Palette> {
        None
    }

    fn name(&self) -> &str {
        if self.active {
            "Alternate dark"
        } else {
            "Alternate light"
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn alternate_panel(
    active: bool,
) -> (
    Option<AlternateTheme>,
    iced::Element<'static, (), AlternateTheme>,
    Option<fn(&AlternateTheme) -> iced::Color>,
    Option<fn(&AlternateTheme) -> iced::Background>,
) {
    let content = iced::widget::Space::new().width(24).height(24).into();
    (
        active.then_some(AlternateTheme { active }),
        content,
        active.then_some(
            (|theme| iced::theme::Base::base(theme).text_color)
                as fn(&AlternateTheme) -> iced::Color,
        ),
        active.then_some(
            (|theme| iced::theme::Base::base(theme).background_color.into())
                as fn(&AlternateTheme) -> iced::Background,
        ),
    )
}

pub fn slider_number(value: f64) -> SliderNumber {
    value as SliderNumber
}
