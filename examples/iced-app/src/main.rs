ui_lang::include_app!("src/ui/tasks.ice");

mod backend {
    use std::sync::{LazyLock, Mutex, MutexGuard};

    #[derive(Clone, Debug, PartialEq)]
    pub struct Task {
        pub id: i64,
        pub title: String,
        pub done: bool,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct AppError {
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

    pub fn copy_text(text: String) -> iced::Task<Result<(), AppError>> {
        iced::Task::batch([iced::clipboard::write::<()>(text), iced::Task::done(())]).map(Ok)
    }

    pub fn app_events() -> iced::Subscription<bool> {
        iced::event::listen().map(|_| true)
    }
}

fn main() -> iced::Result {
    Tasks::run()
}
