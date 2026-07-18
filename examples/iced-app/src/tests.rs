use super::*;

macro_rules! compile_fixtures {
    ($($name:ident => $path:literal),+ $(,)?) => {
        $(mod $name { ui_lang::include_app!($path); })+
    };
}

compile_fixtures! {
    mouse_events => "src/ui/mouse_events.ice",
    touch_events => "src/ui/touch_events.ice",
    input_method_events => "src/ui/input_method_events.ice",
    font_events => "src/ui/font_events.ice",
    task_groups => "src/ui/task_groups.ice",
}

mod application;
mod events;
mod showcase;
mod tasks;
mod values;
mod widgets;
