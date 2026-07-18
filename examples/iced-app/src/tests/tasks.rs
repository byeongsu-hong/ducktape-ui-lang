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

mod task_stream {
    ui_lang::include_app!("src/ui/task_stream.ice");

    #[test]
    fn constructs_both_native_stream_units() {
        let (mut app, _) = TaskStream::__boot();
        assert_eq!(app.__update(__TaskStreamMessage::Start).units(), 2);
        assert_eq!(app.__subscription().units(), 5);
    }
}

mod task_sip {
    ui_lang::include_app!("src/ui/task_sip.ice");

    #[test]
    fn constructs_both_native_sipper_units() {
        let (mut app, _) = TaskSip::__boot();
        assert_eq!(app.__update(__TaskSipMessage::Start).units(), 2);
    }
}

mod task_flow {
    ui_lang::include_app!("src/ui/task_flow.ice");

    #[test]
    fn constructs_native_task_combinators() {
        let (mut app, _) = TaskFlow::__boot();
        assert_eq!(app.__update(__TaskFlowMessage::Start).units(), 8);
    }
}

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

mod timer {
    ui_lang::include_app!("src/ui/timer.ice");

    #[test]
    fn constructs_all_native_time_operations() {
        let (mut app, _) = TimerEvents::__boot();
        assert_eq!(app.__subscription().units(), 4);
        assert_eq!(app.__update(__TimerEventsMessage::Start).units(), 1);
    }
}

mod canvas_events {
    ui_lang::include_app!("src/ui/canvas_events.ice");

    #[test]
    fn initializes() {
        let _ = CanvasEvents::__boot();
    }
}
