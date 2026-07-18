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

mod generic_events {
    ui_lang::include_app!("src/ui/generic_events.ice");

    #[test]
    fn constructs_native_event_listeners() {
        let (app, _) = GenericEvents::__boot();
        assert_eq!(app.__subscription().units(), 5);
    }
}
