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
mod timer {
    ui_lang::include_app!("src/ui/timer.ice");
}

#[cfg(test)]
mod canvas_events {
    ui_lang::include_app!("src/ui/canvas_events.ice");

    #[test]
    fn initializes() {
        let _ = CanvasEvents::__boot();
    }
}
