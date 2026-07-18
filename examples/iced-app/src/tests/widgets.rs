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
