pub fn native_help(active: bool) -> iced::Element<'static, bool> {
    let hint = if active {
        "Pointer entered the external component"
    } else {
        "This tooltip and mouse area are built in Rust"
    };
    iced::widget::mouse_area(iced::widget::tooltip(
        iced::widget::text("Extern component"),
        iced::widget::text(hint),
        iced::widget::tooltip::Position::Bottom,
    ))
    .on_enter(true)
    .on_exit(false)
    .into()
}

pub struct DocsViewer {
    prefix: String,
}

pub fn docs_viewer(prefix: String) -> DocsViewer {
    DocsViewer { prefix }
}

pub fn summary_text(theme: &iced::Theme, busy: bool) -> iced::widget::text::Style {
    if busy {
        iced::widget::text::warning(theme)
    } else {
        iced::widget::text::primary(theme)
    }
}

pub fn volume_slider(
    theme: &iced::Theme,
    status: iced::widget::slider::Status,
    busy: bool,
) -> iced::widget::slider::Style {
    let mut style = iced::widget::slider::default(theme, status);
    if busy {
        style.handle.border_color = theme.palette().danger;
    }
    style
}

pub fn loading_progress(theme: &iced::Theme, active: bool) -> iced::widget::progress_bar::Style {
    if active {
        iced::widget::progress_bar::warning(theme)
    } else {
        iced::widget::progress_bar::success(theme)
    }
}

pub fn action_button(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
    busy: bool,
) -> iced::widget::button::Style {
    if busy {
        iced::widget::button::secondary(theme, status)
    } else {
        iced::widget::button::primary(theme, status)
    }
}

pub fn task_checkbox(
    theme: &iced::Theme,
    status: iced::widget::checkbox::Status,
    busy: bool,
) -> iced::widget::checkbox::Style {
    if busy {
        iced::widget::checkbox::secondary(theme, status)
    } else {
        iced::widget::checkbox::primary(theme, status)
    }
}

pub fn notification_toggler(
    theme: &iced::Theme,
    status: iced::widget::toggler::Status,
    busy: bool,
) -> iced::widget::toggler::Style {
    let mut style = iced::widget::toggler::default(theme, status);
    if busy {
        style.text_color = Some(theme.palette().text);
    }
    style
}

pub fn view_radio(
    theme: &iced::Theme,
    status: iced::widget::radio::Status,
    busy: bool,
) -> iced::widget::radio::Style {
    let mut style = iced::widget::radio::default(theme, status);
    if busy {
        style.text_color = Some(theme.palette().text);
    }
    style
}

pub fn summary_container(theme: &iced::Theme, busy: bool) -> iced::widget::container::Style {
    if busy {
        iced::widget::container::bordered_box(theme)
    } else {
        iced::widget::container::rounded_box(theme)
    }
}

pub fn status_svg(
    theme: &iced::Theme,
    status: iced::widget::svg::Status,
    active: bool,
) -> iced::widget::svg::Style {
    let color = active.then(|| match status {
        iced::widget::svg::Status::Idle => theme.palette().text,
        iced::widget::svg::Status::Hovered => theme.palette().primary,
    });
    iced::widget::svg::Style { color }
}

pub fn form_input(
    theme: &iced::Theme,
    status: iced::widget::text_input::Status,
    disabled: bool,
) -> iced::widget::text_input::Style {
    let mut style = iced::widget::text_input::default(theme, status);
    if disabled {
        style.value = theme.palette().text;
    }
    style
}

pub fn task_scroll(
    theme: &iced::Theme,
    status: iced::widget::scrollable::Status,
    active: bool,
) -> iced::widget::scrollable::Style {
    let mut style = iced::widget::scrollable::default(theme, status);
    if active {
        style.container.text_color = Some(theme.palette().text);
    }
    style
}

pub fn view_picker(
    theme: &iced::Theme,
    status: iced::widget::pick_list::Status,
    active: bool,
) -> iced::widget::pick_list::Style {
    let mut style = iced::widget::pick_list::default(theme, status);
    if active {
        style.handle_color = theme.palette().primary;
    }
    style
}

pub fn view_menu(theme: &iced::Theme, active: bool) -> iced::overlay::menu::Style {
    let mut style = iced::overlay::menu::default(theme);
    if active {
        style.selected_text_color = theme.palette().text;
    }
    style
}

impl<'a> iced::widget::markdown::Viewer<'a, String> for DocsViewer {
    fn on_link_click(url: iced::widget::markdown::Uri) -> String {
        url
    }

    fn image(
        &self,
        _settings: iced::widget::markdown::Settings,
        url: &'a iced::widget::markdown::Uri,
        _title: &'a str,
        _alt: &iced::widget::markdown::Text,
    ) -> iced::Element<'a, String> {
        iced::widget::text(format!("{} image: {url}", self.prefix)).into()
    }
}
