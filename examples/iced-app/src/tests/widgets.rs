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
            assert_eq!(app.__update(message).units(), 2);
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
            assert_eq!(app.__update(message).units(), 2);
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
            assert_eq!(app.__update(message).units(), 2);
        }
    }
}

#[cfg(test)]
mod component_state {
    ui_lang::include_app!("src/ui/component_state.ice");

    #[test]
    fn keeps_component_instances_isolated() {
        let (mut app, _) = ComponentState::__boot();
        let _ = app.__update(__ComponentStateMessage::__CounterHandleIncrement(
            "first".into(),
        ));
        let _ = app.__update(__ComponentStateMessage::__CounterBindDraft(
            "second".into(),
            "local".into(),
        ));
        let _ = app.__update(__ComponentStateMessage::__CounterHandleChanged(
            "first".into(),
            true,
        ));
        let _ = app.__update(__ComponentStateMessage::__FlagHandleChanged(
            "first/flag".into(),
            true,
        ));

        assert_eq!(app.__ice_component_counter["first"].count, 1);
        assert!(app.__ice_component_counter["first"].enabled);
        assert_eq!(app.__ice_component_counter["second"].count, 0);
        assert!(!app.__ice_component_counter["second"].enabled);
        assert_eq!(app.__ice_component_counter["second"].draft, "local");
        assert!(app.__ice_component_flag["first/flag"].checked);
        assert!(!app.__ice_component_flag.contains_key("second/flag"));
    }
}
