ui_lang::include_app!("src/ui/tasks.ice");

mod backend {
    use std::sync::{LazyLock, Mutex, MutexGuard};

    #[derive(Clone, Debug, Hash, PartialEq)]
    pub struct Task {
        pub id: i64,
        pub title: String,
        pub done: bool,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct AppError {
        pub message: String,
    }

    #[cfg(test)]
    pub type SliderNumber = f32;

    #[cfg(test)]
    pub fn keyboard_value(
        key: iced::keyboard::Key,
        _: iced::keyboard::key::Physical,
        _: iced::keyboard::Location,
        _: iced::keyboard::Modifiers,
    ) -> iced::keyboard::Key {
        key
    }

    #[cfg(test)]
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

    #[cfg(test)]
    pub fn transformation_round_trip(
        value: iced::Transformation,
        _: iced::Vector,
        _: iced::Size,
    ) -> iced::Transformation {
        value
    }

    #[cfg(test)]
    pub fn exact_rectangle() -> iced::Rectangle<u32> {
        iced::Rectangle {
            x: 1,
            y: 2,
            width: 3,
            height: 4,
        }
    }

    #[cfg(test)]
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

    #[cfg(test)]
    pub fn unit_round_trip(
        _: iced::Pixels,
        padding: iced::Padding,
        _: iced::Degrees,
        _: iced::Radians,
    ) -> iced::Padding {
        padding
    }

    #[cfg(test)]
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

    #[cfg(test)]
    #[derive(Clone)]
    pub struct AlternateTheme {
        active: bool,
    }

    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
    #[derive(Clone, Debug, PartialEq)]
    pub struct NetworkError {
        pub message: String,
    }

    #[cfg(test)]
    #[derive(Clone, Debug, PartialEq)]
    pub struct EditorCommand {
        pub save: bool,
    }

    #[cfg(test)]
    #[derive(Debug)]
    pub struct DemoHighlighter {
        token: String,
        line: usize,
    }

    #[cfg(test)]
    impl iced::advanced::text::Highlighter for DemoHighlighter {
        type Settings = String;
        type Highlight = ();
        type Iterator<'a> = std::option::IntoIter<(std::ops::Range<usize>, ())>;

        fn new(settings: &Self::Settings) -> Self {
            Self {
                token: settings.clone(),
                line: 0,
            }
        }

        fn update(&mut self, settings: &Self::Settings) {
            self.token.clone_from(settings);
            self.line = 0;
        }

        fn change_line(&mut self, line: usize) {
            self.line = line;
        }

        fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
            self.line += 1;
            line.find(&self.token)
                .map(|start| (start..start + self.token.len(), ()))
                .into_iter()
        }

        fn current_line(&self) -> usize {
            self.line
        }
    }

    // ponytail: a process-wide lock is enough for the sample; replace it when
    // persistence or concurrent write throughput becomes a real requirement.
    static TASKS: LazyLock<Mutex<Vec<Task>>> = LazyLock::new(|| {
        Mutex::new(vec![Task {
            id: 1,
            title: "Build the smallest useful compiler".into(),
            done: false,
        }])
    });

    fn tasks() -> Result<MutexGuard<'static, Vec<Task>>, AppError> {
        TASKS.lock().map_err(|_| AppError {
            message: "Task storage is unavailable.".into(),
        })
    }

    pub async fn list_tasks() -> Result<Vec<Task>, AppError> {
        Ok(tasks()?.clone())
    }

    pub async fn create_task(title: String) -> Result<Vec<Task>, AppError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(AppError {
                message: "A task needs a title.".into(),
            });
        }

        let mut tasks = tasks()?;
        let id = tasks.iter().map(|task| task.id).max().unwrap_or(0) + 1;
        tasks.push(Task {
            id,
            title: title.into(),
            done: false,
        });
        Ok(tasks.clone())
    }

    pub async fn set_task_done(id: i64, done: bool) -> Result<Vec<Task>, AppError> {
        let mut tasks = tasks()?;
        let Some(task) = tasks.iter_mut().find(|task| task.id == id) else {
            return Err(AppError {
                message: "That task no longer exists.".into(),
            });
        };

        task.done = done;
        Ok(tasks.clone())
    }

    #[cfg(test)]
    pub fn slider_number(value: f64) -> SliderNumber {
        value as SliderNumber
    }

    #[cfg(test)]
    pub fn editor_keys(
        event: iced::widget::text_editor::KeyPress,
        readonly: bool,
    ) -> Option<iced::widget::text_editor::Binding<EditorCommand>> {
        if event.key.to_latin(event.physical_key) == Some('s') && event.modifiers.command() {
            Some(iced::widget::text_editor::Binding::Custom(EditorCommand {
                save: true,
            }))
        } else if readonly {
            None
        } else {
            iced::widget::text_editor::Binding::from_key_press(event)
        }
    }

    #[cfg(test)]
    pub fn editor_highlight<'a, Message: 'a>(
        editor: iced::widget::text_editor::TextEditor<
            'a,
            iced::advanced::text::highlighter::PlainText,
            Message,
        >,
        token: String,
    ) -> iced::Element<'a, Message> {
        editor
            .highlight_with::<DemoHighlighter>(token, |_, theme| {
                iced::advanced::text::highlighter::Format {
                    color: Some(theme.palette().primary),
                    font: Some(iced::Font::MONOSPACE),
                }
            })
            .into()
    }

    #[cfg(test)]
    pub fn editor_surface(
        theme: &iced::Theme,
        status: iced::widget::text_editor::Status,
        readonly: bool,
    ) -> iced::widget::text_editor::Style {
        let mut style = iced::widget::text_editor::default(theme, status);
        if readonly {
            style.value = theme.palette().text;
        }
        style
    }

    pub fn describe_window(window: &dyn iced::window::Window, prefix: String) -> String {
        format!("{prefix}: raw-handle={}", window.window_handle().is_ok())
    }

    pub fn status_shader(speed: f64) -> StatusShader {
        StatusShader { speed }
    }

    pub struct StatusShader {
        speed: f64,
    }

    #[derive(Debug)]
    pub struct StatusPrimitive {
        phase: f32,
    }

    pub struct StatusPipeline;

    impl iced::widget::shader::Pipeline for StatusPipeline {
        fn new(
            _device: &iced::wgpu::Device,
            _queue: &iced::wgpu::Queue,
            _format: iced::wgpu::TextureFormat,
        ) -> Self {
            Self
        }
    }

    impl iced::widget::shader::Primitive for StatusPrimitive {
        type Pipeline = StatusPipeline;

        fn prepare(
            &self,
            _pipeline: &mut Self::Pipeline,
            _device: &iced::wgpu::Device,
            _queue: &iced::wgpu::Queue,
            _bounds: &iced::Rectangle,
            _viewport: &iced::widget::shader::Viewport,
        ) {
            let _ = self.phase;
        }
    }

    impl iced::widget::shader::Program<bool> for StatusShader {
        type State = bool;
        type Primitive = StatusPrimitive;

        fn update(
            &self,
            state: &mut Self::State,
            event: &iced::Event,
            _bounds: iced::Rectangle,
            _cursor: iced::mouse::Cursor,
        ) -> Option<iced::widget::shader::Action<bool>> {
            let hovered = match event {
                iced::Event::Mouse(iced::mouse::Event::CursorEntered) => true,
                iced::Event::Mouse(iced::mouse::Event::CursorLeft) => false,
                _ => return None,
            };
            *state = hovered;
            Some(iced::widget::shader::Action::publish(hovered))
        }

        fn draw(
            &self,
            state: &Self::State,
            _cursor: iced::mouse::Cursor,
            _bounds: iced::Rectangle,
        ) -> Self::Primitive {
            StatusPrimitive {
                phase: (self.speed as f32) + f32::from(*state),
            }
        }

        fn mouse_interaction(
            &self,
            _state: &Self::State,
            _bounds: iced::Rectangle,
            _cursor: iced::mouse::Cursor,
        ) -> iced::mouse::Interaction {
            iced::mouse::Interaction::Pointer
        }
    }

    #[cfg(test)]
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

    #[cfg(test)]
    pub struct DocsViewer {
        prefix: String,
    }

    #[cfg(test)]
    pub fn docs_viewer(prefix: String) -> DocsViewer {
        DocsViewer { prefix }
    }

    #[cfg(test)]
    pub fn summary_text(theme: &iced::Theme, busy: bool) -> iced::widget::text::Style {
        if busy {
            iced::widget::text::warning(theme)
        } else {
            iced::widget::text::primary(theme)
        }
    }

    #[cfg(test)]
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

    #[cfg(test)]
    pub fn loading_progress(
        theme: &iced::Theme,
        active: bool,
    ) -> iced::widget::progress_bar::Style {
        if active {
            iced::widget::progress_bar::warning(theme)
        } else {
            iced::widget::progress_bar::success(theme)
        }
    }

    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
    pub fn summary_container(theme: &iced::Theme, busy: bool) -> iced::widget::container::Style {
        if busy {
            iced::widget::container::bordered_box(theme)
        } else {
            iced::widget::container::rounded_box(theme)
        }
    }

    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
    pub fn view_menu(theme: &iced::Theme, active: bool) -> iced::overlay::menu::Style {
        let mut style = iced::overlay::menu::default(theme);
        if active {
            style.selected_text_color = theme.palette().text;
        }
        style
    }

    #[cfg(test)]
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

    #[cfg(test)]
    pub fn copy_text(text: String) -> iced::Task<Result<(), AppError>> {
        iced::Task::batch([iced::clipboard::write::<()>(text), iced::Task::done(())]).map(Ok)
    }

    #[cfg(test)]
    pub fn count_stream(limit: i64) -> impl iced::futures::Stream<Item = i64> + Send + 'static {
        iced::futures::stream::iter(0..limit.max(0))
    }

    #[cfg(test)]
    pub fn range_stream(
        start: i64,
        limit: i64,
    ) -> impl iced::futures::Stream<Item = i64> + Send + 'static {
        iced::futures::stream::iter(start..start.saturating_add(limit.max(0)))
    }

    #[cfg(test)]
    pub fn fallible_stream()
    -> impl iced::futures::Stream<Item = Result<i64, AppError>> + Send + 'static {
        iced::futures::stream::iter([
            Ok(1),
            Err(AppError {
                message: "stream failed".into(),
            }),
        ])
    }

    #[cfg(test)]
    pub struct CounterRecipe {
        id: i64,
    }

    #[cfg(test)]
    impl iced::advanced::subscription::Recipe for CounterRecipe {
        type Output = i64;

        fn hash(&self, state: &mut iced::advanced::subscription::Hasher) {
            std::hash::Hash::hash(&self.id, state);
        }

        fn stream(
            self: Box<Self>,
            _input: iced::advanced::subscription::EventStream,
        ) -> iced::futures::stream::BoxStream<'static, Self::Output> {
            Box::pin(iced::futures::stream::iter([self.id]))
        }
    }

    #[cfg(test)]
    pub fn counter_recipe(id: i64) -> CounterRecipe {
        CounterRecipe { id }
    }

    #[cfg(test)]
    pub fn raw_event(event: iced::advanced::subscription::Event) -> Option<String> {
        Some(match event {
            iced::advanced::subscription::Event::Interaction { status, .. } => {
                format!("{status:?}")
            }
            iced::advanced::subscription::Event::SystemThemeChanged(mode) => {
                format!("{mode:?}")
            }
        })
    }

    #[cfg(test)]
    pub fn by_kind(kind: String) -> impl iced::widget::selector::Selector<Output = String> {
        move |candidate: iced::widget::selector::Candidate<'_>| {
            let candidate_kind = match candidate {
                iced::widget::selector::Candidate::Container { .. } => "container",
                iced::widget::selector::Candidate::Focusable { .. } => "focusable",
                iced::widget::selector::Candidate::Scrollable { .. } => "scrollable",
                iced::widget::selector::Candidate::TextInput { .. } => "text-input",
                iced::widget::selector::Candidate::Text { .. } => "text",
                iced::widget::selector::Candidate::Custom { .. } => "custom",
            };
            (candidate_kind == kind).then(|| kind.clone())
        }
    }

    #[cfg(test)]
    pub fn count_sip(limit: i64) -> impl iced::task::Sipper<i64, i64> + Send + 'static {
        iced::task::sipper(move |mut sender| async move {
            let limit = limit.max(0);
            for value in 1..=limit {
                sender.send(value).await;
            }
            limit
        })
    }

    #[cfg(test)]
    pub fn fallible_sip(limit: i64) -> impl iced::task::Straw<i64, i64, AppError> + Send + 'static {
        iced::task::sipper(move |mut sender| async move {
            sender.send(1).await;
            if limit < 0 {
                Err(AppError {
                    message: "sip failed".into(),
                })
            } else {
                Ok(limit)
            }
        })
    }

    #[cfg(test)]
    pub fn double_task(value: i64) -> iced::Task<i64> {
        iced::Task::done(value * 2)
    }

    #[cfg(test)]
    pub fn optional_task(value: i64) -> iced::Task<Option<i64>> {
        iced::Task::done((value > 0).then_some(value))
    }

    #[cfg(test)]
    pub fn fallible_task(value: i64) -> iced::Task<Result<i64, AppError>> {
        iced::Task::done(if value >= 0 {
            Ok(value)
        } else {
            Err(AppError {
                message: "task failed".into(),
            })
        })
    }

    #[cfg(test)]
    pub async fn refresh_time() -> i64 {
        1
    }

    #[cfg(test)]
    pub fn even_refresh(value: i64) -> Option<i64> {
        (value % 2 == 0).then_some(value)
    }

    #[cfg(test)]
    pub fn visible_pointer(x: f64, y: f64) -> Option<String> {
        (x >= 0.0 && y >= 0.0).then(|| format!("{x},{y}"))
    }

    #[cfg(test)]
    pub fn allow_frame() -> Option<bool> {
        Some(true)
    }

    #[cfg(test)]
    pub fn network_task(value: i64) -> iced::Task<Result<i64, NetworkError>> {
        iced::Task::done(if value >= 0 {
            Ok(value)
        } else {
            Err(NetworkError {
                message: "network failed".into(),
            })
        })
    }

    #[cfg(test)]
    pub fn normalize_error(error: NetworkError) -> AppError {
        AppError {
            message: error.message,
        }
    }

    #[cfg(test)]
    pub fn event_name(event: iced::Event) -> String {
        match event {
            iced::Event::Keyboard(_) => "keyboard",
            iced::Event::Mouse(_) => "mouse",
            iced::Event::Window(_) => "window",
            iced::Event::Touch(_) => "touch",
            iced::Event::InputMethod(_) => "input-method",
        }
        .into()
    }

    #[cfg(test)]
    pub fn event_label(event: iced::Event) -> Option<String> {
        Some(event_name(event))
    }

    #[cfg(test)]
    pub fn app_events() -> iced::Subscription<bool> {
        iced::event::listen_with(|event, _status, _window| focus_event(event))
    }

    #[cfg(test)]
    fn focus_event(event: iced::Event) -> Option<bool> {
        matches!(event, iced::Event::Window(iced::window::Event::Focused)).then_some(true)
    }

    #[cfg(test)]
    mod tests {
        use super::focus_event;

        #[test]
        fn subscription_ignores_high_frequency_pointer_events() {
            assert_eq!(
                focus_event(iced::Event::Window(iced::window::Event::Focused)),
                Some(true)
            );
            assert_eq!(
                focus_event(iced::Event::Mouse(iced::mouse::Event::CursorLeft)),
                None
            );
        }
    }
}

fn main() -> iced::Result {
    Tasks::run()
}

#[cfg(test)]
mod tests;
