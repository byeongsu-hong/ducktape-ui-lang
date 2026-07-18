use crate::backend::AppError;

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkError {
    pub message: String,
}

pub fn copy_text(text: String) -> iced::Task<Result<(), AppError>> {
    iced::Task::batch([iced::clipboard::write::<()>(text), iced::Task::done(())]).map(Ok)
}

pub fn count_stream(limit: i64) -> impl iced::futures::Stream<Item = i64> + Send + 'static {
    iced::futures::stream::iter(0..limit.max(0))
}

pub fn range_stream(
    start: i64,
    limit: i64,
) -> impl iced::futures::Stream<Item = i64> + Send + 'static {
    iced::futures::stream::iter(start..start.saturating_add(limit.max(0)))
}

pub fn fallible_stream() -> impl iced::futures::Stream<Item = Result<i64, AppError>> + Send + 'static
{
    iced::futures::stream::iter([
        Ok(1),
        Err(AppError {
            message: "stream failed".into(),
        }),
    ])
}

pub struct CounterRecipe {
    id: i64,
}

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

pub fn counter_recipe(id: i64) -> CounterRecipe {
    CounterRecipe { id }
}

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

pub fn count_sip(limit: i64) -> impl iced::task::Sipper<i64, i64> + Send + 'static {
    iced::task::sipper(move |mut sender| async move {
        let limit = limit.max(0);
        for value in 1..=limit {
            sender.send(value).await;
        }
        limit
    })
}

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

pub fn double_task(value: i64) -> iced::Task<i64> {
    iced::Task::done(value * 2)
}

pub fn optional_task(value: i64) -> iced::Task<Option<i64>> {
    iced::Task::done((value > 0).then_some(value))
}

pub fn fallible_task(value: i64) -> iced::Task<Result<i64, AppError>> {
    iced::Task::done(if value >= 0 {
        Ok(value)
    } else {
        Err(AppError {
            message: "task failed".into(),
        })
    })
}

pub async fn refresh_time() -> i64 {
    1
}

pub fn even_refresh(value: i64) -> Option<i64> {
    (value % 2 == 0).then_some(value)
}

pub fn visible_pointer(x: f64, y: f64) -> Option<String> {
    (x >= 0.0 && y >= 0.0).then(|| format!("{x},{y}"))
}

pub fn allow_frame() -> Option<bool> {
    Some(true)
}

pub fn network_task(value: i64) -> iced::Task<Result<i64, NetworkError>> {
    iced::Task::done(if value >= 0 {
        Ok(value)
    } else {
        Err(NetworkError {
            message: "network failed".into(),
        })
    })
}

pub fn normalize_error(error: NetworkError) -> AppError {
    AppError {
        message: error.message,
    }
}

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

pub fn event_label(event: iced::Event) -> Option<String> {
    Some(event_name(event))
}

pub fn app_events() -> iced::Subscription<bool> {
    iced::event::listen_with(|event, _status, _window| focus_event(event))
}

fn focus_event(event: iced::Event) -> Option<bool> {
    matches!(event, iced::Event::Window(iced::window::Event::Focused)).then_some(true)
}

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
