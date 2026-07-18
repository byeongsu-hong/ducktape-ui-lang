ui_lang::include_app!("src/ui/tasks.ice");

#[cfg(test)]
mod alignment;
#[cfg(test)]
mod background_gradient;
#[cfg(test)]
mod border_radius;
#[cfg(test)]
mod color;
#[cfg(test)]
mod content_fit;
#[cfg(test)]
mod font_values;
#[cfg(test)]
mod length;
#[cfg(test)]
mod mouse_interaction;
#[cfg(test)]
mod rotation;
#[cfg(test)]
mod shadow;

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
    pub type AppRenderer = iced::Renderer;

    #[cfg(test)]
    pub fn daemon_title(_: iced::window::Id) -> String {
        "Background agent".into()
    }

    #[cfg(test)]
    pub fn daemon_theme(_: iced::window::Id) -> iced::Theme {
        iced::Theme::Dark
    }

    #[cfg(test)]
    pub fn daemon_scale(_: iced::window::Id) -> f64 {
        1.0
    }

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
    pub use crate::alignment::{alignment_round_trip, horizontal_round_trip, vertical_round_trip};

    #[cfg(test)]
    pub use crate::background_gradient::{
        background_round_trip, color_stop_round_trip, gradient_round_trip, linear_round_trip,
    };

    #[cfg(test)]
    pub use crate::border_radius::{border_round_trip, radius_round_trip};

    #[cfg(test)]
    pub use crate::color::color_round_trip;

    #[cfg(test)]
    pub use crate::content_fit::content_fit_round_trip;

    #[cfg(test)]
    pub use crate::font_values::{
        family_round_trip, font_round_trip, stretch_round_trip, style_round_trip, weight_round_trip,
    };

    #[cfg(test)]
    pub use crate::length::length_round_trip;

    #[cfg(test)]
    pub use crate::mouse_interaction::interaction_round_trip;

    #[cfg(test)]
    pub use crate::rotation::rotation_round_trip;

    #[cfg(test)]
    pub use crate::shadow::shadow_round_trip;

    #[cfg(test)]
    pub fn elastic(value: f64) -> f64 {
        value
    }

    #[cfg(test)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Motion {
        pub value: f64,
    }

    #[cfg(test)]
    impl iced::animation::Float for Motion {
        fn float_value(&self) -> f32 {
            self.value as f32
        }
    }

    #[cfg(test)]
    pub fn motion(value: f64) -> Motion {
        Motion { value }
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
    pub fn borrowed_help<'a>(
        label: &'a str,
        active: &'a bool,
    ) -> iced::Element<'a, bool, iced::Theme, AppRenderer> {
        iced::widget::button(iced::widget::text(if label.is_empty() {
            "Borrowed extern component"
        } else {
            label
        }))
        .on_press(!*active)
        .into()
    }

    #[cfg(test)]
    pub struct IndexedOverlayHost {
        index: f32,
    }

    #[cfg(test)]
    pub struct IndexedOverlay {
        pub index: f32,
    }

    #[cfg(test)]
    impl iced::advanced::Widget<(), iced::Theme, iced::Renderer> for IndexedOverlayHost {
        fn size(&self) -> iced::Size<iced::Length> {
            iced::Size::new(iced::Length::Shrink, iced::Length::Shrink)
        }

        fn layout(
            &mut self,
            _tree: &mut iced::advanced::widget::Tree,
            _renderer: &iced::Renderer,
            limits: &iced::advanced::layout::Limits,
        ) -> iced::advanced::layout::Node {
            iced::advanced::layout::atomic(limits, iced::Length::Shrink, iced::Length::Shrink)
        }

        fn draw(
            &self,
            _tree: &iced::advanced::widget::Tree,
            _renderer: &mut iced::Renderer,
            _theme: &iced::Theme,
            _style: &iced::advanced::renderer::Style,
            _layout: iced::advanced::Layout<'_>,
            _cursor: iced::mouse::Cursor,
            _viewport: &iced::Rectangle,
        ) {
        }

        fn overlay<'a>(
            &'a mut self,
            _tree: &'a mut iced::advanced::widget::Tree,
            _layout: iced::advanced::Layout<'a>,
            _renderer: &iced::Renderer,
            _viewport: &iced::Rectangle,
            _translation: iced::Vector,
        ) -> Option<iced::advanced::overlay::Element<'a, (), iced::Theme, iced::Renderer>> {
            Some(iced::advanced::overlay::Element::new(Box::new(
                IndexedOverlay { index: self.index },
            )))
        }
    }

    #[cfg(test)]
    impl iced::advanced::Overlay<(), iced::Theme, iced::Renderer> for IndexedOverlay {
        fn layout(
            &mut self,
            _renderer: &iced::Renderer,
            _bounds: iced::Size,
        ) -> iced::advanced::layout::Node {
            iced::advanced::layout::Node::new(iced::Size::new(1.0, 1.0))
        }

        fn draw(
            &self,
            _renderer: &mut iced::Renderer,
            _theme: &iced::Theme,
            _style: &iced::advanced::renderer::Style,
            _layout: iced::advanced::Layout<'_>,
            _cursor: iced::mouse::Cursor,
        ) {
        }

        fn index(&self) -> f32 {
            self.index
        }
    }

    #[cfg(test)]
    pub fn native_overlay(index: f64) -> iced::Element<'static, ()> {
        iced::Element::new(IndexedOverlayHost {
            index: index as f32,
        })
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
    pub fn workspace_panes(theme: &iced::Theme, active: bool) -> iced::widget::pane_grid::Style {
        let mut style = iced::widget::pane_grid::default(theme);
        style.hovered_split.width = if active { 5.0 } else { 2.0 };
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
mod application {
    use super::{__TasksMessage, Tasks};

    #[test]
    fn resolves_application_callbacks_from_state() {
        let (mut app, _) = Tasks::__boot();
        assert_eq!(Tasks::__title(&app), "Ice Tasks");
        assert_eq!(Tasks::__theme(&app), Tasks::__app_theme());

        app.window_title = "Renamed".into();
        app.app_theme = "dark".into();
        app.app_background = "#123456".into();
        app.app_text = "#abcdef".into();
        app.ui_scale = 1.5;
        let style = Tasks::__style(&app, &iced::Theme::Dark);
        assert_eq!(Tasks::__title(&app), "Renamed");
        assert_eq!(Tasks::__theme(&app), iced::Theme::Dark);
        assert_eq!(style.background_color, "#123456".parse().unwrap());
        assert_eq!(style.text_color, "#abcdef".parse().unwrap());
        assert_eq!(Tasks::__scale_factor(&app), 1.5);

        app.app_theme = "unknown".into();
        app.app_background = "invalid".into();
        let base = <iced::Theme as iced::theme::Base>::base(&iced::Theme::Dark);
        assert_eq!(Tasks::__theme(&app), Tasks::__app_theme());
        assert_eq!(
            Tasks::__style(&app, &iced::Theme::Dark).background_color,
            base.background_color
        );
        app.ui_scale = 0.0;
        assert_eq!(Tasks::__scale_factor(&app), f32::EPSILON);
    }

    #[test]
    fn constructs_structured_boot_preset() {
        let (app, task) = Tasks::__preset_0();
        assert!(!app.loading);
        assert_eq!(task.units(), 0);

        let (app, task) = Tasks::__preset_1();
        assert_eq!(app.draft, "Preset task");
        assert!(app.loading);
        assert_eq!(task.units(), 1);
    }

    #[test]
    fn opens_and_targets_a_named_window() {
        let (mut app, _) = Tasks::__boot();
        assert_eq!(app.__update(__TasksMessage::OpenChild).units(), 1);

        let id = iced::window::Id::unique();
        assert_eq!(app.__update(__TasksMessage::ChildOpened(id)).units(), 1);
        assert_eq!(app.child_window, Some(id));

        assert_eq!(
            app.__update(__TasksMessage::ChildSized(640.0, 480.0))
                .units(),
            0
        );
        assert_eq!((app.child_width, app.child_height), (640.0, 480.0));
    }

    #[test]
    fn constructs_window_capture_queries() {
        let (mut app, _) = Tasks::__boot();
        assert_eq!(app.__update(__TasksMessage::ReadRawWindowId).units(), 1);
        assert_eq!(app.__update(__TasksMessage::CaptureWindow).units(), 1);
        assert_eq!(app.__update(__TasksMessage::SetWindowIcon).units(), 1);
        assert_eq!(app.__update(__TasksMessage::InspectWindowHandle).units(), 1);

        let pixels = vec![255, 0, 0, 255, 0, 255, 0, 255];
        let _ = app.__update(__TasksMessage::WindowCaptured(pixels, 2, 1, 1.5));
        let _ = app.__update(__TasksMessage::RawWindowIdRead("42".into()));
        assert!(app.snapshot_ready);
        assert_eq!((app.snapshot_width, app.snapshot_height), (2, 1));
        assert_eq!(app.snapshot_scale, 1.5);
        assert_eq!(app.raw_window_id, "42");
    }
}

#[cfg(test)]
mod showcase {
    ui_lang::include_app!("src/ui/showcase.ice");

    #[test]
    fn qr_data_initializes() {
        let _ = Showcase::__boot();
    }

    #[test]
    fn appends_markdown_and_tracks_image_uris() {
        let (mut app, _) = Showcase::__boot();
        assert!(app.help_images.is_empty());

        assert_eq!(app.__update(__ShowcaseMessage::ExtendMarkdown).units(), 0);
        assert_eq!(app.help_images, ["asset://ice"]);
    }

    #[test]
    fn resizes_a_named_nested_pane_split() {
        let (mut app, _) = Showcase::__boot();
        let split = app.__pane_nested_workspace_splits["editor_stack"];

        let _ = app.__update(__ShowcaseMessage::ResizeNestedEditor);

        let regions = app.__pane_nested_workspace.layout().split_regions(
            0.0,
            0.0,
            iced::Size::new(100.0, 100.0),
        );
        assert_eq!(regions[&split].2, 0.45);
    }

    #[test]
    fn constructs_a_native_pane_grid_style() {
        let style = crate::backend::workspace_panes(&iced::Theme::Dark, true);
        assert_eq!(style.hovered_split.width, 5.0);

        let (app, _) = Showcase::__boot();
        let _ = app.__view();
    }

    #[test]
    fn opens_and_renders_a_runtime_pane_template() {
        let (mut app, _) = Showcase::__boot();
        app.tasks = vec![crate::backend::Task {
            id: 1,
            title: "Dynamic pane".into(),
            done: false,
        }];

        let _ = app.__update(__ShowcaseMessage::OpenTaskPane);
        assert!(
            app.__pane_nested_workspace
                .iter()
                .any(|(_, pane)| { matches!(pane, __IcePaneNestedWorkspace::PaneTask(1)) })
        );
        let _ = app.__update(__ShowcaseMessage::MaximizeTaskPane);
        assert!(app.__pane_nested_workspace.maximized().is_some());
        let _ = app.__view();
        app.tasks.clear();
        let _ = app.__view();

        let _ = app.__update(__ShowcaseMessage::CloseTaskPane(1));
        assert!(
            !app.__pane_nested_workspace
                .iter()
                .any(|(_, pane)| { matches!(pane, __IcePaneNestedWorkspace::PaneTask(1)) })
        );

        let _ = app.__update(__ShowcaseMessage::OpenModePane);
        assert!(app.__pane_nested_workspace.iter().any(|(_, pane)| {
            matches!(pane, __IcePaneNestedWorkspace::ModePane(name) if name == "List")
        }));
        let _ = app.__view();
        let _ = app.__update(__ShowcaseMessage::CloseModePane("List".into()));
    }
}

#[cfg(test)]
mod window_events {
    ui_lang::include_app!("src/ui/window_events.ice");

    #[test]
    fn stores_the_originating_window() {
        let (mut app, _) = WindowEvents::__boot();
        let id = iced::window::Id::unique();
        let _ = app.__update(__WindowEventsMessage::Focused(id));
        assert_eq!(app.last_window, Some(id));
    }
}

#[cfg(test)]
mod mouse_events {
    ui_lang::include_app!("src/ui/mouse_events.ice");
}

#[cfg(test)]
mod touch_events {
    ui_lang::include_app!("src/ui/touch_events.ice");
}

#[cfg(test)]
mod input_method_events {
    ui_lang::include_app!("src/ui/input_method_events.ice");
}

#[cfg(test)]
mod generic_events {
    ui_lang::include_app!("src/ui/generic_events.ice");

    #[test]
    fn constructs_native_event_listeners() {
        let (app, _) = GenericEvents::__boot();
        assert_eq!(app.__subscription().units(), 5);
    }
}

#[cfg(test)]
mod keyboard_values {
    ui_lang::include_app!("src/ui/keyboard_values.ice");

    #[test]
    fn preserves_native_keyboard_values() {
        let (mut app, _) = KeyboardValues::__boot();
        assert_eq!(
            app.dynamic_native,
            Some(iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Xkb(42)
            ))
        );
        assert_eq!(app.platform_command, iced::keyboard::Modifiers::COMMAND);
        let event = __IceKeyPress {
            key: iced::keyboard::Key::Character("с".into()),
            modified_key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter),
            physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::KeyC),
            location: iced::keyboard::Location::Numpad,
            modifiers: iced::keyboard::Modifiers::CTRL,
            text: Some("с".into()),
            repeat: false,
        };
        let _ = app.__update(__KeyboardValuesMessage::Pressed(event));

        assert_eq!(app.latin.as_deref(), Some("c"));
        assert_eq!(app.kind, "character");
        assert_eq!(app.named.as_deref(), Some("Enter"));
        assert_eq!(app.character.as_deref(), Some("с"));
        assert_eq!(app.code.as_deref(), Some("KeyC"));
        assert_eq!(app.location_name, "numpad");
        assert!(app.modifiers.control());
    }
}

#[cfg(test)]
mod pointer_values {
    ui_lang::include_app!("src/ui/pointer_values.ice");

    #[test]
    fn preserves_native_pointer_values() {
        let (mut app, _) = PointerValues::__boot();
        let _ = app.__update(__PointerValuesMessage::Inspect);

        assert_eq!(app.cursor_position, Some(iced::Point::new(12.0, 24.0)));
        assert_eq!(app.cursor_in, Some(iced::Point::new(2.0, 4.0)));
        assert!(app.cursor_levitating);
        assert!(app.over);
        assert_eq!(app.click_kind, "single");
        assert_eq!(app.width, 40.0);

        let _ = app.__update(__PointerValuesMessage::Pressed(iced::mouse::Button::Other(
            9,
        )));
        assert_eq!(app.button, iced::mouse::Button::Other(9));
        assert_eq!(app.button_kind, "other");
        assert_eq!(app.button_number, Some(9));

        let _ = app.__update(__PointerValuesMessage::Touched(
            iced::touch::Finger(u64::MAX),
            7.0,
            8.0,
        ));
        assert_eq!(app.finger, iced::touch::Finger(u64::MAX));
        assert_eq!(app.finger_id, u64::MAX.to_string());
    }
}

#[cfg(test)]
mod transformation_values {
    ui_lang::include_app!("src/ui/transformation_values.ice");

    #[test]
    fn preserves_and_applies_native_transformations() {
        let (mut app, _) = TransformationValues::__boot();
        let _ = app.__update(__TransformationValuesMessage::Inspect);

        assert_eq!(app.translation, iced::Vector::new(10.0, 20.0));
        assert_eq!(app.scale_factor, 2.0);
        assert_eq!(app.matrix.len(), 16);
        assert_eq!(app.point_value, iced::Point::new(12.0, 24.0));
        assert_eq!(app.vector_value, iced::Vector::new(2.0, 4.0));
        assert_eq!(app.size_value, iced::Size::new(6.0, 8.0));
        assert_eq!(
            app.bounds,
            iced::Rectangle {
                x: 12.0,
                y: 24.0,
                width: 6.0,
                height: 8.0,
            }
        );
        assert_eq!(app.cursor.position(), Some(iced::Point::new(12.0, 24.0)));
        assert_eq!(app.click.position(), iced::Point::new(12.0, 24.0));
        assert_eq!(app.recovered, iced::Point::new(1.0, 2.0));
        assert!(app.identity_equal);
        assert!(app.maybe_projection.is_some());
        assert!(app.invalid_projection.is_none());
    }
}

#[cfg(test)]
mod geometry_values {
    ui_lang::include_app!("src/ui/geometry_values.ice");

    #[test]
    fn preserves_and_applies_native_geometry_values() {
        let (mut app, _) = GeometryValues::__boot();
        let _ = app.__update(__GeometryValuesMessage::Inspect);

        assert_eq!(app.point_value, iced::Point::new(3.25, 4.75));
        assert_eq!(app.point_difference, iced::Vector::new(3.25, 4.75));
        assert_eq!(app.point_distance, 5.0);
        assert_eq!(app.snapped_point, iced::Point::new(3, 5));
        assert_eq!((app.snapped_x, app.snapped_y), (3, 5));
        assert_eq!(
            (app.exact_x, app.exact_y, app.exact_width, app.exact_height),
            (1, 2, 3, 4)
        );
        assert_eq!(app.point_values, [3.25, 4.75]);
        assert_eq!(app.point_display, "Point { x: 3.25, y: 4.75 }");
        assert_eq!(app.vector_value, iced::Vector::new(3.0, 3.0));
        assert_eq!(app.size_min, iced::Size::new(3.0, 2.0));
        assert_eq!(app.size_max, iced::Size::new(10.0, 8.0));
        assert_eq!(app.size_expanded, iced::Size::new(13.0, 10.0));
        assert_eq!(
            app.size_rotated,
            iced::Size::new(2.0, 4.0).rotate(iced::Radians(0.5))
        );
        assert_eq!(app.size_ratio, iced::Size::new(50.0, 50.0));
        assert_eq!(app.size_value, iced::Size::new(14.0, 27.0));
        assert_eq!(app.size_from_u32, iced::Size::new(640.0, 480.0));
        assert_eq!(app.maybe_size, Some(iced::Size::new(640.0, 480.0)));
        assert_eq!(app.invalid_size, None);
        assert_eq!(app.size_vector, iced::Vector::new(14.0, 27.0));
        assert_eq!(
            app.sized_bounds,
            iced::Rectangle::with_size(iced::Size::new(5.0, 6.0))
        );
        assert_eq!(app.radius_bounds, iced::Rectangle::with_radius(3.0));
        assert!((app.vertex_rotation - std::f64::consts::FRAC_PI_2).abs() < 0.0001);
        assert!(app.contains_point);
        assert_eq!(app.point_to_bounds, 5.0);
        assert_eq!(app.bounds_offset, iced::Vector::new(2.0, 2.0));
        assert!(app.within_bounds);
        assert_eq!(
            app.intersection,
            Some(iced::Rectangle {
                x: 5.0,
                y: 5.0,
                width: 5.0,
                height: 5.0
            })
        );
        assert!(app.intersects_bounds);
        assert_eq!(
            app.union_bounds,
            iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 15.0,
                height: 15.0
            }
        );
        assert_eq!(
            app.snapped_bounds,
            Some(iced::Rectangle {
                x: 1,
                y: 3,
                width: 4,
                height: 4
            })
        );
        assert_eq!(
            app.expanded_bounds,
            iced::Rectangle {
                x: 6.0,
                y: 19.0,
                width: 46.0,
                height: 64.0
            }
        );
        assert_eq!(
            app.shrunk_bounds,
            iced::Rectangle {
                x: 14.0,
                y: 21.0,
                width: 34.0,
                height: 56.0
            }
        );
        assert_eq!(
            app.rotated_bounds,
            iced::Rectangle {
                x: 10.0,
                y: 20.0,
                width: 40.0,
                height: 60.0
            }
            .rotate(iced::Radians(0.5))
        );
        assert_eq!(
            app.zoomed_bounds,
            iced::Rectangle {
                x: -10.0,
                y: -10.0,
                width: 80.0,
                height: 120.0
            }
        );
        assert_eq!(app.anchor, iced::Point::new(40.0, 60.0));
        assert_eq!(
            app.converted_bounds,
            iced::Rectangle {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 4.0
            }
        );
        assert_eq!(
            app.moved_bounds,
            iced::Rectangle {
                x: 11.0,
                y: 22.0,
                width: 40.0,
                height: 60.0
            }
        );
        assert_eq!(
            app.scaled_bounds,
            iced::Rectangle {
                x: 20.0,
                y: 40.0,
                width: 80.0,
                height: 120.0
            }
        );
        assert_eq!(app.center, iced::Point::new(30.0, 50.0));
        assert_eq!(app.bounds_size, iced::Size::new(40.0, 60.0));
        assert_eq!(app.area, 2400.0);
    }
}

#[cfg(test)]
mod padding_angles {
    ui_lang::include_app!("src/ui/padding_angles.ice");

    #[test]
    fn preserves_native_padding_and_angle_values() {
        let (mut app, _) = PaddingAngles::__boot();
        let _ = app.__update(__PaddingAnglesMessage::Inspect);

        assert_eq!(app.pixel_value, iced::Pixels(8.0));
        assert_eq!(app.u32_pixels, iced::Pixels(u32::MAX as f32));
        assert_eq!(app.maybe_pixels, Some(iced::Pixels(42.0)));
        assert!(app.invalid_pixels.is_none());
        assert!(app.pixel_ordered);
        assert_eq!(app.all_padding, iced::Padding::new(5.0));
        assert_eq!(app.pixel_padding, iced::Padding::new(6.0));
        assert_eq!(app.top_padding, iced::Padding::ZERO.top(1.0));
        assert_eq!(app.right_padding, iced::Padding::ZERO.right(2.0));
        assert_eq!(app.bottom_padding, iced::Padding::ZERO.bottom(3.0));
        assert_eq!(app.left_padding, iced::Padding::ZERO.left(4.0));
        assert_eq!(app.horizontal_padding, iced::Padding::ZERO.horizontal(5.0));
        assert_eq!(app.vertical_padding, iced::Padding::ZERO.vertical(6.0));
        assert_eq!(app.axes_padding, iced::Padding::from([7.0, 8.0]));
        assert_eq!(
            app.changed_padding,
            iced::Padding {
                top: 6.0,
                right: 5.0,
                bottom: 6.0,
                left: 5.0
            }
        );
        assert_eq!(
            app.fitted_padding,
            iced::Padding {
                top: 3.0,
                right: 0.0,
                bottom: 0.0,
                left: 2.0
            }
        );
        assert_eq!(app.padding_size, iced::Size::new(6.0, 4.0));
        assert_eq!(
            app.expanded_bounds,
            iced::Rectangle {
                x: 6.0,
                y: 19.0,
                width: 36.0,
                height: 44.0
            }
        );
        assert_eq!(
            app.shrunk_bounds,
            iced::Rectangle {
                x: 14.0,
                y: 21.0,
                width: 24.0,
                height: 36.0
            }
        );
        assert_eq!((app.padding_x, app.padding_y), (6.0, 4.0));
        assert!(app.padding_equal);
        assert_eq!(app.degree_value, iced::Degrees(90.0));
        assert_eq!(app.degree_start, *iced::Degrees::RANGE.start());
        assert_eq!(app.degree_end, *iced::Degrees::RANGE.end());
        assert!(app.degree_in_range);
        assert!(!app.degree_out_of_range);
        assert!(app.degree_ordered);
        assert_eq!(app.radians_start, *iced::Radians::RANGE.start());
        assert_eq!(app.radians_end, *iced::Radians::RANGE.end());
        assert_eq!(app.radians_pi, iced::Radians::PI);
        assert_eq!(
            app.radians_from_degrees,
            iced::Radians::from(iced::Degrees(180.0))
        );
        assert!((app.radians_math.0 - (1.0 + std::f32::consts::PI)).abs() < 0.0001);
        assert_eq!(app.radians_reverse, iced::Radians(3.0));
        assert!(app.radians_in_range);
        assert!(app.radians_equal_scalar);
        assert_eq!(app.radians_display, "1 rad");
        assert!((app.distance_start.x - 50.0).abs() < 0.0001);
        assert!((app.distance_start.y - 50.0).abs() < 0.0001);
        assert!((app.distance_end.x - 50.0).abs() < 0.0001);
        assert!((app.distance_end.y - 0.0).abs() < 0.0001);
        assert_eq!(
            app.rotated_size,
            iced::Size::new(10.0, 20.0).rotate(iced::Radians(1.0))
        );
        assert_eq!(
            app.rotated_bounds,
            iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 20.0
            }
            .rotate(iced::Radians(1.0))
        );
        assert!((app.vertices_angle.0 - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
    }
}

#[cfg(test)]
mod dynamic_widget_operations {
    ui_lang::include_app!("src/ui/dynamic_widget_operations.ice");

    #[test]
    fn constructs_dynamic_widget_tasks() {
        let (mut app, _) = DynamicOperations::__boot();
        for message in [
            __DynamicOperationsMessage::Focus,
            __DynamicOperationsMessage::FocusNamed,
            __DynamicOperationsMessage::Check,
            __DynamicOperationsMessage::Front,
            __DynamicOperationsMessage::End,
            __DynamicOperationsMessage::Cursor,
            __DynamicOperationsMessage::All,
            __DynamicOperationsMessage::Range,
            __DynamicOperationsMessage::Snap,
            __DynamicOperationsMessage::SnapEnd,
            __DynamicOperationsMessage::ScrollTo,
            __DynamicOperationsMessage::ScrollBy,
        ] {
            assert_eq!(app.__update(message).units(), 1);
        }
    }
}

#[cfg(test)]
mod scoped_widget_operations {
    ui_lang::include_app!("src/ui/scoped_widget_operations.ice");

    #[test]
    fn constructs_scoped_widget_tasks() {
        let (mut app, _) = ScopedOperations::__boot();
        for message in [
            __ScopedOperationsMessage::FocusComponent,
            __ScopedOperationsMessage::FocusDefault,
            __ScopedOperationsMessage::FocusSlot,
            __ScopedOperationsMessage::FocusKeyed,
            __ScopedOperationsMessage::FocusHeader,
            __ScopedOperationsMessage::FocusCell,
            __ScopedOperationsMessage::SnapPane,
        ] {
            assert_eq!(app.__update(message).units(), 1);
        }
    }
}

#[cfg(test)]
mod widget_selectors {
    ui_lang::include_app!("src/ui/widget_selectors.ice");

    #[test]
    fn constructs_native_selector_tasks() {
        let (mut app, _) = WidgetSelectors::__boot();
        for message in [
            __WidgetSelectorsMessage::FindId,
            __WidgetSelectorsMessage::FindText,
            __WidgetSelectorsMessage::FindPoint,
            __WidgetSelectorsMessage::FindFocused,
            __WidgetSelectorsMessage::FindAllText,
            __WidgetSelectorsMessage::FindCustom,
        ] {
            assert_eq!(app.__update(message).units(), 1);
        }
    }
}

#[cfg(test)]
mod font_events {
    ui_lang::include_app!("src/ui/font_events.ice");
}

#[cfg(test)]
mod task_groups {
    ui_lang::include_app!("src/ui/task_groups.ice");
}

#[cfg(test)]
mod task_cancel {
    ui_lang::include_app!("src/ui/task_cancel.ice");

    #[test]
    fn aborts_native_task_handle() {
        let (mut app, _) = TaskCancel::__boot();
        let task = app.__update(__TaskCancelMessage::Start);
        assert!(!app.request.as_ref().unwrap().is_aborted());

        let _ = app.__update(__TaskCancelMessage::Cancel);
        assert!(app.request.as_ref().unwrap().is_aborted());
        drop(task);
    }
}

#[cfg(test)]
mod task_stream {
    ui_lang::include_app!("src/ui/task_stream.ice");

    #[test]
    fn constructs_both_native_stream_units() {
        let (mut app, _) = TaskStream::__boot();
        assert_eq!(app.__update(__TaskStreamMessage::Start).units(), 2);
        assert_eq!(app.__subscription().units(), 5);
    }
}

#[cfg(test)]
mod task_sip {
    ui_lang::include_app!("src/ui/task_sip.ice");

    #[test]
    fn constructs_both_native_sipper_units() {
        let (mut app, _) = TaskSip::__boot();
        assert_eq!(app.__update(__TaskSipMessage::Start).units(), 2);
    }
}

#[cfg(test)]
mod task_flow {
    ui_lang::include_app!("src/ui/task_flow.ice");

    #[test]
    fn constructs_native_task_combinators() {
        let (mut app, _) = TaskFlow::__boot();
        assert_eq!(app.__update(__TaskFlowMessage::Start).units(), 8);
    }
}

#[cfg(test)]
mod task_map {
    ui_lang::include_app!("src/ui/task_map.ice");

    #[test]
    fn maps_success_values_and_preserves_errors() {
        use iced::futures::StreamExt;

        let (mut app, _) = TaskMap::__boot();
        let task = app.__update(__TaskMapMessage::Start);
        let mut stream = iced_runtime::task::into_stream(task).unwrap();
        let messages = iced::futures::executor::block_on(async move {
            let mut messages = Vec::new();
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(message) = action {
                    messages.push(message);
                }
            }
            messages
        });
        for message in messages {
            let _ = app.__update(message);
        }

        assert_eq!(app.mapped, 5);
        assert_eq!(app.mapped_optional, Some(2));
        assert_eq!(app.mapped_result, 8);
        assert_eq!(app.error, "task failed");
    }
}

#[cfg(test)]
mod theme_factory {
    ui_lang::include_app!("src/ui/theme_factory.ice");

    #[test]
    fn constructs_app_and_nested_native_themes() {
        let (mut app, _) = NativeTheme::__boot();
        let theme = app.__theme();
        assert_eq!(theme.to_string(), "Native dark");
        assert!(theme.extended_palette().is_dark);
        assert_eq!(
            theme.extended_palette().primary.base.color,
            iced::Color::from_rgb8(0x7c, 0x3a, 0xed)
        );

        app.dark = false;
        assert_eq!(app.__theme().to_string(), "Native light");
        let _ = app.__view();
    }
}

#[cfg(test)]
mod alternate_theme {
    ui_lang::include_app!("src/ui/alternate_theme.ice");

    #[test]
    fn constructs_an_alternate_theme_subtree() {
        let (mut app, _) = AlternateThemeApp::__boot();
        let (theme, _, text_color, background) = crate::backend::alternate_panel(true);
        let theme = theme.unwrap();
        assert_eq!(iced::theme::Base::name(&theme), "Alternate dark");
        assert_eq!(text_color.unwrap()(&theme), iced::Color::WHITE);
        assert_eq!(background.unwrap()(&theme), iced::Color::BLACK.into());
        let _ = app.__view();

        app.active = false;
        let (theme, _, text_color, background) = crate::backend::alternate_panel(false);
        assert!(theme.is_none() && text_color.is_none() && background.is_none());
        let _ = app.__view();
    }
}

#[cfg(test)]
mod native_overlay {
    ui_lang::include_app!("src/ui/native_overlay.ice");

    #[test]
    fn constructs_a_custom_indexed_overlay() {
        let (app, _) = NativeOverlay::__boot();
        let overlay = crate::backend::IndexedOverlay { index: 42.0 };
        assert_eq!(
            iced::advanced::Overlay::<(), iced::Theme, iced::Renderer>::index(&overlay),
            42.0
        );
        let _ = app.__view();
    }
}

#[cfg(test)]
mod timer {
    ui_lang::include_app!("src/ui/timer.ice");

    #[test]
    fn constructs_all_native_time_operations() {
        let (mut app, _) = TimerEvents::__boot();
        assert_eq!(app.__subscription().units(), 4);
        assert_eq!(app.__update(__TimerEventsMessage::Start).units(), 1);
    }
}

#[cfg(test)]
mod animation {
    ui_lang::include_app!("src/ui/animation.ice");

    #[test]
    fn drives_native_animations_only_while_active() {
        let (mut app, _) = NativeAnimation::__boot();
        assert_eq!(app.__subscription().units(), 0);

        let _ = app.__update(__NativeAnimationMessage::Start);
        assert!(app.expanded.value());
        assert_eq!(app.progress.value(), 1.0);
        assert_eq!(app.custom_motion.value().value, 1.0);
        assert_eq!(app.__subscription().units(), 1);
        let _ = app.__view();

        let _ = app.__update(__NativeAnimationMessage::Sample);
        assert!(app.maybe_progress.is_some());
        assert!(app.maybe_visibility.is_none());
        let _ = app.__update(__NativeAnimationMessage::Rewind(iced::time::Instant::now()));
        assert_eq!(app.progress.value(), 0.0);
        assert_eq!(
            app.__update(__NativeAnimationMessage::__AnimationFrame)
                .units(),
            0
        );
    }
}

#[cfg(test)]
mod image_allocation {
    ui_lang::include_app!("src/ui/image_allocation.ice");

    #[test]
    fn constructs_native_allocation_and_preserves_exact_errors() {
        use iced::futures::StreamExt;

        let (mut app, _) = ImageAllocation::__boot();
        let task = app.__update(__ImageAllocationMessage::Allocate);
        assert_eq!(task.units(), 1);
        let mut stream = iced_runtime::task::into_stream(task).unwrap();
        let message = iced::futures::executor::block_on(async move {
            let iced_runtime::Action::Image(iced_runtime::image::Action::Allocate(_, sender)) =
                stream.next().await.unwrap()
            else {
                panic!("expected native image allocation action")
            };
            sender
                .send(Err(iced::widget::image::Error::Unsupported))
                .unwrap();
            let iced_runtime::Action::Output(message) = stream.next().await.unwrap() else {
                panic!("expected routed allocation error")
            };
            message
        });
        assert_eq!(
            app.__update(__ImageAllocationMessage::AllocateFlow).units(),
            1
        );
        let _ = app.__update(message);
        assert_eq!(app.error_kind, "unsupported");
        assert_eq!(app.error_message, "loading images is unsupported");
        assert!(matches!(
            app.failure,
            Some(iced::widget::image::Error::Unsupported)
        ));
        let _ = app.__view();
    }
}

#[cfg(test)]
mod debug_timing {
    ui_lang::include_app!("src/ui/debug_timing.ice");

    #[test]
    fn owns_and_finishes_native_debug_spans() {
        let (mut app, _) = DebugTiming::__boot();
        assert!(app.timer.is_none());

        let _ = app.__update(__DebugTimingMessage::Begin);
        assert!(app.timer.is_some());
        let _ = app.__update(__DebugTimingMessage::Begin);
        assert!(app.timer.is_some());

        let _ = app.__update(__DebugTimingMessage::Finish);
        assert!(app.timer.is_none());
        let _ = app.__update(__DebugTimingMessage::Compute);
        assert_eq!(app.measured, 42);
        let _ = app.__view();
    }
}

#[cfg(test)]
mod canvas_events {
    ui_lang::include_app!("src/ui/canvas_events.ice");

    #[test]
    fn initializes() {
        let _ = CanvasEvents::__boot();
    }
}

#[cfg(test)]
mod daemon {
    ui_lang::include_app!("src/ui/daemon.ice");

    #[test]
    fn constructs_window_open_and_exit_tasks() {
        let (mut app, open) = BackgroundAgent::__boot();
        let window = iced::window::Id::unique();
        assert_eq!(open.units(), 1);
        assert_eq!(app.__title(window), "Background agent");
        assert_eq!(app.__theme(window), iced::Theme::Dark);
        assert_eq!(app.__scale_factor(window), 1.0);
        let _ = app.__view(window);
        assert_eq!(app.__update(__BackgroundAgentMessage::Quit).units(), 1);
    }
}
