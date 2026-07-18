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
    #[derive(Clone, Debug, PartialEq)]
    pub struct NetworkError {
        pub message: String,
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
    use super::Tasks;

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
}

#[cfg(test)]
mod showcase {
    ui_lang::include_app!("src/ui/showcase.ice");

    #[test]
    fn qr_data_initializes() {
        let _ = Showcase::__boot();
    }
}

#[cfg(test)]
mod window_events {
    ui_lang::include_app!("src/ui/window_events.ice");
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
mod canvas_events {
    ui_lang::include_app!("src/ui/canvas_events.ice");

    #[test]
    fn initializes() {
        let _ = CanvasEvents::__boot();
    }
}
