use crate::Error;
use crate::ast::*;
use crate::check::{controlled_state_bindings, expr_type};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

mod canvas;
mod expr;
mod statement;
mod style;
mod view;

use canvas::*;
use expr::*;
use statement::*;
use style::*;
use view::*;

pub fn generate(document: &Document, source_path: &str) -> Result<String, Error> {
    let message = format!("__{}Message", document.app);
    let mut out = String::new();
    writeln!(
        out,
        "const _: &str = include_str!({});",
        rust_string(source_path)
    )
    .unwrap();
    generate_keyboard_types(&mut out, document);
    generate_system_types(&mut out, document);
    generate_widget_selector_types(&mut out, document);
    generate_canvas_types(&mut out, document);

    writeln!(out, "#[derive(Debug)]\npub struct {} {{", document.app).unwrap();
    for qr in &document.qr_codes {
        writeln!(
            out,
            "pub(crate) {}: ::iced::widget::qr_code::Data,",
            qr.name
        )
        .unwrap();
    }
    for node in pane_grids(&document.view) {
        let ViewNode::PaneGrid { name, .. } = node else {
            unreachable!()
        };
        writeln!(
            out,
            "pub(crate) {}: ::iced::widget::pane_grid::State<&'static str>,",
            pane_field(name)
        )
        .unwrap();
    }
    for state in &document.states {
        writeln!(
            out,
            "pub(crate) {}: {},",
            state.name,
            state.ty.rust(&document.structs)
        )
        .unwrap();
    }
    writeln!(out, "}}").unwrap();

    writeln!(out, "#[derive(Debug, Clone)]\nenum {message} {{").unwrap();
    for handler in &document.handlers {
        if handler.name == "mount" {
            continue;
        }
        let variant = pascal(&handler.name);
        if handler.params.is_empty() {
            writeln!(out, "{variant},").unwrap();
        } else {
            let fields = handler
                .params
                .iter()
                .map(|param| param.ty.rust(&document.structs))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(out, "{variant}({fields}),").unwrap();
        }
    }
    for binding in controlled_state_bindings(document, false)
        .expect("checker validates controlled input bindings")
    {
        writeln!(out, "{}(::std::string::String),", binding_variant(&binding)).unwrap();
    }
    for binding in controlled_state_bindings(document, true)
        .expect("checker validates controlled editor bindings")
    {
        writeln!(
            out,
            "{}(::iced::widget::text_editor::Action),",
            editor_variant(&binding)
        )
        .unwrap();
    }
    if needs_extern_noop(document) {
        writeln!(out, "__ExternNoop,").unwrap();
    }
    for node in pane_grids(&document.view) {
        let ViewNode::PaneGrid { name, options, .. } = node else {
            unreachable!()
        };
        if options.resize_leeway.is_some() {
            writeln!(
                out,
                "{}(::iced::widget::pane_grid::ResizeEvent),",
                pane_resize_variant(name)
            )
            .unwrap();
        }
        if options.draggable {
            writeln!(
                out,
                "{}(::iced::widget::pane_grid::DragEvent),",
                pane_drag_variant(name)
            )
            .unwrap();
        }
    }
    writeln!(out, "}}").unwrap();

    generate_extern_probes(&mut out, document);
    generate_editor_binding_mapper(&mut out, document);
    writeln!(out, "impl {} {{", document.app).unwrap();
    generate_named_windows(&mut out, document, source_path);
    writeln!(out, "pub fn run() -> ::iced::Result {{").unwrap();
    let subscription = if document.subscriptions.is_empty() {
        ""
    } else {
        ".subscription(Self::__subscription)"
    };
    let default_font = document
        .fonts
        .iter()
        .find(|font| font.default)
        .map_or_else(String::new, |font| {
            format!(".default_font({})", font_decl_code(font))
        });
    let title = document
        .settings
        .title
        .as_ref()
        .map_or("", |_| ".title(Self::__title)");
    let settings = app_settings_code(&document.settings);
    let fonts = font_assets_code(&document.settings, source_path);
    let window = window_settings_code(document.settings.window.as_ref(), source_path);
    let executor = document
        .settings
        .executor
        .as_ref()
        .map_or_else(String::new, |executor| format!(".executor::<{executor}>()"));
    let presets = if document.presets.is_empty() {
        String::new()
    } else {
        format!(
            ".presets([{}])",
            document
                .presets
                .iter()
                .enumerate()
                .map(|(index, preset)| format!(
                    "::iced::Preset::new({}, Self::__preset_{index})",
                    rust_string(&preset.name)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let scale_factor = document
        .settings
        .scale_factor
        .as_ref()
        .map_or("", |_| ".scale_factor(Self::__scale_factor)");
    let style = if document.settings.background.is_some() || document.settings.text_color.is_some()
    {
        ".style(Self::__style)"
    } else {
        ""
    };
    writeln!(out, "::iced::application(Self::__boot, Self::__update, Self::__view){title}{subscription}.theme(Self::__theme){style}{settings}{default_font}{fonts}{window}{scale_factor}{executor}{presets}.run()").unwrap();
    writeln!(out, "}}").unwrap();

    generate_theme(&mut out, document)?;
    generate_boot(&mut out, document, &message)?;
    generate_presets(&mut out, document, &message)?;
    generate_update(&mut out, document, &message)?;
    generate_subscription(&mut out, document, &message)?;
    generate_view(&mut out, document, &message)?;
    writeln!(out, "}}").unwrap();
    Ok(out)
}

fn font_assets_code(settings: &AppSettings, source_path: &str) -> String {
    let parent = Path::new(source_path)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    settings
        .fonts
        .iter()
        .map(|font| {
            format!(
                ".font(include_bytes!({}).as_slice())",
                rust_string(&parent.join(&font.path).display().to_string())
            )
        })
        .collect()
}

fn app_settings_code(settings: &AppSettings) -> String {
    let mut fields = String::new();
    if let Some(id) = &settings.id {
        write!(
            fields,
            "id: ::std::option::Option::Some({}.to_owned()),",
            rust_string(id)
        )
        .unwrap();
    }
    if let Some(size) = settings.default_text_size {
        write!(fields, "default_text_size: ::iced::Pixels({size} as f32),").unwrap();
    }
    if let Some(value) = settings.antialiasing {
        write!(fields, "antialiasing: {value},").unwrap();
    }
    if let Some(value) = settings.vsync {
        write!(fields, "vsync: {value},").unwrap();
    }
    if fields.is_empty() {
        String::new()
    } else {
        format!(".settings(::iced::Settings {{ {fields} ..::std::default::Default::default() }})")
    }
}

fn window_settings_code(settings: Option<&WindowSettings>, source_path: &str) -> String {
    let Some(settings) = settings else {
        return String::new();
    };
    format!(
        ".window({})",
        window_settings_value_code(settings, source_path)
    )
}

fn generate_named_windows(out: &mut String, document: &Document, source_path: &str) {
    for (index, window) in document.settings.windows.iter().enumerate() {
        writeln!(
            out,
            "fn __window_{index}() -> ::iced::window::Settings {{ {} }}",
            window_settings_value_code(&window.settings, source_path)
        )
        .unwrap();
    }
}

fn window_settings_value_code(settings: &WindowSettings, source_path: &str) -> String {
    let mut fields = String::new();
    let size =
        |(width, height): (f64, f64)| format!("::iced::Size::new({width} as f32, {height} as f32)");
    if let Some(value) = settings.size {
        write!(fields, "size: {},", size(value)).unwrap();
    }
    for (name, value) in [
        ("maximized", settings.maximized),
        ("fullscreen", settings.fullscreen),
        ("visible", settings.visible),
        ("resizable", settings.resizable),
        ("closeable", settings.closeable),
        ("minimizable", settings.minimizable),
        ("decorations", settings.decorations),
        ("transparent", settings.transparent),
        ("blur", settings.blur),
        ("exit_on_close_request", settings.exit_on_close_request),
    ] {
        if let Some(value) = value {
            write!(fields, "{name}: {value},").unwrap();
        }
    }
    if let Some(position) = settings.position {
        let position = match position {
            WindowPosition::Default => "::iced::window::Position::Default".into(),
            WindowPosition::Centered => "::iced::window::Position::Centered".into(),
            WindowPosition::Specific(x, y) => format!(
                "::iced::window::Position::Specific(::iced::Point::new({x} as f32, {y} as f32))"
            ),
        };
        write!(fields, "position: {position},").unwrap();
    }
    if let Some(value) = settings.min_size {
        write!(
            fields,
            "min_size: ::std::option::Option::Some({}),",
            size(value)
        )
        .unwrap();
    }
    if let Some(value) = settings.max_size {
        write!(
            fields,
            "max_size: ::std::option::Option::Some({}),",
            size(value)
        )
        .unwrap();
    }
    if let Some(level) = settings.level {
        let level = match level {
            WindowLevel::Normal => "Normal",
            WindowLevel::AlwaysOnBottom => "AlwaysOnBottom",
            WindowLevel::AlwaysOnTop => "AlwaysOnTop",
        };
        write!(fields, "level: ::iced::window::Level::{level},").unwrap();
    }
    if let Some(icon) = &settings.icon {
        let parent = Path::new(source_path)
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let path = parent.join(&icon.path).display().to_string();
        write!(
            fields,
            "icon: ::std::option::Option::Some({{ const __ICE_RGBA: &[u8] = include_bytes!({}); const _: () = ::std::assert!(__ICE_RGBA.len() == {}, \"window icon RGBA byte length does not match width × height × 4\"); ::iced::window::icon::from_rgba(__ICE_RGBA.to_vec(), {}, {}).expect(\"statically checked RGBA window icon\") }}),",
            rust_string(&path),
            icon.byte_len,
            icon.width,
            icon.height
        )
        .unwrap();
    }
    if settings.linux.is_some()
        || settings.windows.is_some()
        || settings.macos.is_some()
        || settings.wasm.is_some()
    {
        write!(
            fields,
            "platform_specific: {},",
            window_platform_code(settings)
        )
        .unwrap();
    }
    format!("::iced::window::Settings {{ {fields} ..::std::default::Default::default() }}")
}

fn window_platform_code(settings: &WindowSettings) -> String {
    let mut linux = String::new();
    if let Some(settings) = &settings.linux {
        if let Some(value) = &settings.application_id {
            write!(
                linux,
                "__platform.application_id = {}.to_owned();",
                rust_string(value)
            )
            .unwrap();
        }
        if let Some(value) = settings.override_redirect {
            write!(linux, "__platform.override_redirect = {value};").unwrap();
        }
    }

    let mut windows = String::new();
    if let Some(settings) = &settings.windows {
        for (name, value) in [
            ("drag_and_drop", settings.drag_and_drop),
            ("skip_taskbar", settings.skip_taskbar),
            ("undecorated_shadow", settings.undecorated_shadow),
        ] {
            if let Some(value) = value {
                write!(windows, "__platform.{name} = {value};").unwrap();
            }
        }
        if let Some(value) = settings.corner {
            let value = match value {
                WindowCorner::Default => "Default",
                WindowCorner::DoNotRound => "DoNotRound",
                WindowCorner::Round => "Round",
                WindowCorner::RoundSmall => "RoundSmall",
            };
            write!(
                windows,
                "__platform.corner_preference = ::iced::window::settings::platform::CornerPreference::{value};"
            )
            .unwrap();
        }
    }

    let mut macos = String::new();
    if let Some(settings) = &settings.macos {
        for (name, value) in [
            ("title_hidden", settings.title_hidden),
            ("titlebar_transparent", settings.titlebar_transparent),
            ("fullsize_content_view", settings.fullsize_content_view),
        ] {
            if let Some(value) = value {
                write!(macos, "__platform.{name} = {value};").unwrap();
            }
        }
    }

    let mut wasm = String::new();
    if let Some(Some(target)) = settings
        .wasm
        .as_ref()
        .and_then(|settings| settings.target.as_ref())
    {
        write!(
            wasm,
            "__platform.target = ::std::option::Option::Some({}.to_owned());",
            rust_string(target)
        )
        .unwrap();
    } else if settings
        .wasm
        .as_ref()
        .is_some_and(|settings| settings.target == Some(None))
    {
        wasm.push_str("__platform.target = ::std::option::Option::None;");
    }

    format!(
        "{{ #[cfg(target_os = \"linux\")] {{ #[allow(unused_mut)] let mut __platform: ::iced::window::settings::PlatformSpecific = ::std::default::Default::default(); {linux} __platform }} #[cfg(target_os = \"windows\")] {{ #[allow(unused_mut)] let mut __platform: ::iced::window::settings::PlatformSpecific = ::std::default::Default::default(); {windows} __platform }} #[cfg(target_os = \"macos\")] {{ #[allow(unused_mut)] let mut __platform: ::iced::window::settings::PlatformSpecific = ::std::default::Default::default(); {macos} __platform }} #[cfg(target_arch = \"wasm32\")] {{ #[allow(unused_mut)] let mut __platform: ::iced::window::settings::PlatformSpecific = ::std::default::Default::default(); {wasm} __platform }} #[cfg(not(any(target_os = \"linux\", target_os = \"windows\", target_os = \"macos\", target_arch = \"wasm32\")))] {{ ::std::default::Default::default() }} }}"
    )
}

fn generate_keyboard_types(out: &mut String, document: &Document) {
    if !document
        .subscriptions
        .iter()
        .any(|subscription| matches!(&subscription.source, SubscriptionSource::Keyboard(_)))
        && !canvas_events(document)
            .iter()
            .any(|event| matches!(event.source, SubscriptionSource::Keyboard(_)))
    {
        return;
    }
    out.push_str(
        r#"#[derive(Debug, Clone)]
struct __IceKeyPress {
    key: ::iced::keyboard::Key,
    modified_key: ::iced::keyboard::Key,
    physical_key: ::iced::keyboard::key::Physical,
    location: ::iced::keyboard::Location,
    modifiers: ::iced::keyboard::Modifiers,
    text: ::std::option::Option<::std::string::String>,
    repeat: bool,
}
#[derive(Debug, Clone)]
struct __IceKeyRelease {
    key: ::iced::keyboard::Key,
    modified_key: ::iced::keyboard::Key,
    physical_key: ::iced::keyboard::key::Physical,
    location: ::iced::keyboard::Location,
    modifiers: ::iced::keyboard::Modifiers,
}
"#,
    );
}

fn generate_system_types(out: &mut String, document: &Document) {
    let information = uses_system_task(document, "__ice_system_info");
    let theme = uses_system_task(document, "__ice_system_theme")
        || document
            .subscriptions
            .iter()
            .any(|subscription| matches!(&subscription.source, SubscriptionSource::SystemTheme));
    if information {
        out.push_str(
            r#"#[derive(Debug, Clone)]
struct __IceSystemInfo {
    system_name: ::std::option::Option<::std::string::String>,
    system_kernel: ::std::option::Option<::std::string::String>,
    system_version: ::std::option::Option<::std::string::String>,
    system_short_version: ::std::option::Option<::std::string::String>,
    cpu_brand: ::std::string::String,
    cpu_cores: ::std::option::Option<i64>,
    memory_total: i64,
    memory_used: ::std::option::Option<i64>,
    graphics_backend: ::std::string::String,
    graphics_adapter: ::std::string::String,
}

fn __ice_system_info(value: ::iced::system::Information) -> __IceSystemInfo {
    __IceSystemInfo {
        system_name: value.system_name,
        system_kernel: value.system_kernel,
        system_version: value.system_version,
        system_short_version: value.system_short_version,
        cpu_brand: value.cpu_brand,
        cpu_cores: value.cpu_cores.map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
        memory_total: i64::try_from(value.memory_total).unwrap_or(i64::MAX),
        memory_used: value.memory_used.map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
        graphics_backend: value.graphics_backend,
        graphics_adapter: value.graphics_adapter,
    }
}
"#,
        );
    }
    if theme {
        out.push_str(
            r#"fn __ice_system_theme(value: ::iced::theme::Mode) -> ::std::string::String {
    match value {
        ::iced::theme::Mode::None => "none",
        ::iced::theme::Mode::Light => "light",
        ::iced::theme::Mode::Dark => "dark",
    }.to_owned()
}
"#,
        );
    }
}

fn generate_widget_selector_types(out: &mut String, document: &Document) {
    let uses_builtin = |statements: &[Statement]| {
        statements_use_widget_selector(statements, |selector| {
            !matches!(selector, WidgetSelector::Extern { .. })
        })
    };
    if !document
        .handlers
        .iter()
        .any(|handler| uses_builtin(&handler.statements))
        && !document
            .presets
            .iter()
            .any(|preset| uses_builtin(&preset.statements))
    {
        return;
    }
    out.push_str(
        r#"#[derive(Debug, Clone)]
struct __IceWidgetTarget {
    kind: ::std::string::String,
    id: ::std::option::Option<::iced::widget::Id>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    visible_x: ::std::option::Option<f64>,
    visible_y: ::std::option::Option<f64>,
    visible_width: ::std::option::Option<f64>,
    visible_height: ::std::option::Option<f64>,
    content: ::std::option::Option<::std::string::String>,
    content_x: ::std::option::Option<f64>,
    content_y: ::std::option::Option<f64>,
    content_width: ::std::option::Option<f64>,
    content_height: ::std::option::Option<f64>,
    translation_x: ::std::option::Option<f64>,
    translation_y: ::std::option::Option<f64>,
}
fn __ice_widget_target(
    kind: &str,
    id: ::std::option::Option<::iced::widget::Id>,
    bounds: ::iced::Rectangle,
    visible: ::std::option::Option<::iced::Rectangle>,
    content: ::std::option::Option<::std::string::String>,
    content_bounds: ::std::option::Option<::iced::Rectangle>,
    translation: ::std::option::Option<::iced::Vector>,
) -> __IceWidgetTarget {
    __IceWidgetTarget {
        kind: kind.to_owned(),
        id,
        x: bounds.x as f64,
        y: bounds.y as f64,
        width: bounds.width as f64,
        height: bounds.height as f64,
        visible_x: visible.map(|bounds| bounds.x as f64),
        visible_y: visible.map(|bounds| bounds.y as f64),
        visible_width: visible.map(|bounds| bounds.width as f64),
        visible_height: visible.map(|bounds| bounds.height as f64),
        content,
        content_x: content_bounds.map(|bounds| bounds.x as f64),
        content_y: content_bounds.map(|bounds| bounds.y as f64),
        content_width: content_bounds.map(|bounds| bounds.width as f64),
        content_height: content_bounds.map(|bounds| bounds.height as f64),
        translation_x: translation.map(|translation| translation.x as f64),
        translation_y: translation.map(|translation| translation.y as f64),
    }
}
fn __ice_widget_target_from_target(value: ::iced::widget::selector::Target) -> __IceWidgetTarget {
    use ::iced::widget::selector::Target;
    match value {
        Target::Container { id, bounds, visible_bounds } => __ice_widget_target("container", id, bounds, visible_bounds, None, None, None),
        Target::Focusable { id, bounds, visible_bounds } => __ice_widget_target("focusable", id, bounds, visible_bounds, None, None, None),
        Target::Scrollable { id, bounds, visible_bounds, content_bounds, translation } => __ice_widget_target("scrollable", id, bounds, visible_bounds, None, Some(content_bounds), Some(translation)),
        Target::TextInput { id, bounds, visible_bounds, content } => __ice_widget_target("text-input", id, bounds, visible_bounds, Some(content), None, None),
        Target::Text { id, bounds, visible_bounds, content } => __ice_widget_target("text", id, bounds, visible_bounds, Some(content), None, None),
        Target::Custom { id, bounds, visible_bounds } => __ice_widget_target("custom", id, bounds, visible_bounds, None, None, None),
    }
}
fn __ice_widget_target_from_text(value: ::iced::widget::selector::Text) -> __IceWidgetTarget {
    use ::iced::widget::selector::Text;
    match value {
        Text::Raw { id, bounds, visible_bounds } => __ice_widget_target("text", id, bounds, visible_bounds, None, None, None),
        Text::Input { id, bounds, visible_bounds } => __ice_widget_target("text-input", id, bounds, visible_bounds, None, None, None),
    }
}
"#,
    );
}

fn statements_use_widget_selector(
    statements: &[Statement],
    predicate: impl Copy + Fn(&WidgetSelector) -> bool,
) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::WidgetOperation {
            operation: WidgetOperation::Find { selector, .. },
            ..
        } => predicate(selector),
        Statement::TaskGroup { statements, .. } => {
            statements_use_widget_selector(statements, predicate)
        }
        Statement::Abortable { task, .. } => {
            statements_use_widget_selector(::std::slice::from_ref(task), predicate)
        }
        _ => false,
    })
}

fn generate_canvas_types(out: &mut String, document: &Document) {
    if !uses_canvas(document) {
        return;
    }
    out.push_str(
        r#"struct __IceCanvasProgram<State, Message, Draw, Update, Interaction> {
    draw: Draw,
    update: Update,
    interaction: Interaction,
    message: ::std::marker::PhantomData<fn() -> (State, Message)>,
}
impl<State, Message, Draw, Update, Interaction> ::iced::widget::canvas::Program<Message>
    for __IceCanvasProgram<State, Message, Draw, Update, Interaction>
where
    State: ::std::default::Default + 'static,
    Draw: Fn(
        &State,
        &::iced::Renderer,
        &::iced::Theme,
        ::iced::Rectangle,
        ::iced::mouse::Cursor,
    ) -> ::std::vec::Vec<::iced::widget::canvas::Geometry>,
    Update: Fn(
        &mut State,
        &::iced::widget::canvas::Event,
        ::iced::Rectangle,
        ::iced::mouse::Cursor,
    ) -> ::std::option::Option<::iced::widget::canvas::Action<Message>>,
    Interaction: Fn(
        &State,
        ::iced::Rectangle,
        ::iced::mouse::Cursor,
    ) -> ::iced::mouse::Interaction,
{
    type State = State;

    fn update(
        &self,
        state: &mut Self::State,
        event: &::iced::widget::canvas::Event,
        bounds: ::iced::Rectangle,
        cursor: ::iced::mouse::Cursor,
    ) -> ::std::option::Option<::iced::widget::canvas::Action<Message>> {
        (self.update)(state, event, bounds, cursor)
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &::iced::Renderer,
        theme: &::iced::Theme,
        bounds: ::iced::Rectangle,
        cursor: ::iced::mouse::Cursor,
    ) -> ::std::vec::Vec<::iced::widget::canvas::Geometry> {
        (self.draw)(state, renderer, theme, bounds, cursor)
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: ::iced::Rectangle,
        cursor: ::iced::mouse::Cursor,
    ) -> ::iced::mouse::Interaction {
        (self.interaction)(state, bounds, cursor)
    }
}
fn __ice_canvas_interaction(value: &str) -> ::iced::mouse::Interaction {
    match value {
        "hidden" => ::iced::mouse::Interaction::Hidden,
        "idle" => ::iced::mouse::Interaction::Idle,
        "context-menu" => ::iced::mouse::Interaction::ContextMenu,
        "help" => ::iced::mouse::Interaction::Help,
        "pointer" => ::iced::mouse::Interaction::Pointer,
        "progress" => ::iced::mouse::Interaction::Progress,
        "wait" => ::iced::mouse::Interaction::Wait,
        "cell" => ::iced::mouse::Interaction::Cell,
        "crosshair" => ::iced::mouse::Interaction::Crosshair,
        "text" => ::iced::mouse::Interaction::Text,
        "alias" => ::iced::mouse::Interaction::Alias,
        "copy" => ::iced::mouse::Interaction::Copy,
        "move" => ::iced::mouse::Interaction::Move,
        "no-drop" => ::iced::mouse::Interaction::NoDrop,
        "not-allowed" => ::iced::mouse::Interaction::NotAllowed,
        "grab" => ::iced::mouse::Interaction::Grab,
        "grabbing" => ::iced::mouse::Interaction::Grabbing,
        "resize-horizontal" => ::iced::mouse::Interaction::ResizingHorizontally,
        "resize-vertical" => ::iced::mouse::Interaction::ResizingVertically,
        "resize-diagonal-up" => ::iced::mouse::Interaction::ResizingDiagonallyUp,
        "resize-diagonal-down" => ::iced::mouse::Interaction::ResizingDiagonallyDown,
        "resize-column" => ::iced::mouse::Interaction::ResizingColumn,
        "resize-row" => ::iced::mouse::Interaction::ResizingRow,
        "all-scroll" => ::iced::mouse::Interaction::AllScroll,
        "zoom-in" => ::iced::mouse::Interaction::ZoomIn,
        "zoom-out" => ::iced::mouse::Interaction::ZoomOut,
        _ => ::iced::mouse::Interaction::default(),
    }
}
"#,
    );
    for group in canvas_cache_groups(document) {
        writeln!(
            out,
            "static {}: ::std::sync::OnceLock<::iced::widget::canvas::Group> = ::std::sync::OnceLock::new();",
            canvas_group_symbol(group)
        )
        .unwrap();
    }
}

fn uses_system_task(document: &Document, name: &str) -> bool {
    document
        .handlers
        .iter()
        .any(|handler| statements_use_system_task(&handler.statements, name))
}

fn statements_use_system_task(statements: &[Statement], name: &str) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::Run {
            kind: EffectKind::Task,
            function,
            ..
        } => function == name,
        Statement::TaskGroup { statements, .. } => statements_use_system_task(statements, name),
        Statement::Abortable { task, .. } => {
            statements_use_system_task(::std::slice::from_ref(task), name)
        }
        Statement::TaskFlow {
            source, transforms, ..
        } => {
            task_source_uses_system(source, name)
                || transforms.iter().any(|transform| match transform {
                    TaskTransform::Then { source, .. } | TaskTransform::AndThen { source, .. } => {
                        task_source_uses_system(source, name)
                    }
                    TaskTransform::MapError { .. }
                    | TaskTransform::Collect { .. }
                    | TaskTransform::Discard { .. } => false,
                })
        }
        _ => false,
    })
}

fn task_source_uses_system(source: &TaskSource, name: &str) -> bool {
    matches!(source, TaskSource::Effect { function, .. } if function == name)
}

fn generate_extern_probes(out: &mut String, document: &Document) {
    if document
        .functions
        .iter()
        .any(|item| item.kind == ExternKind::EventFilter)
    {
        writeln!(out, "#[cfg(not(target_arch = \"wasm32\"))] type __IceEventStream<T> = ::iced::futures::stream::BoxStream<'static, T>; #[cfg(target_arch = \"wasm32\")] type __IceEventStream<T> = ::iced::futures::stream::LocalBoxStream<'static, T>;").unwrap();
    }
    for item in &document.structs {
        writeln!(
            out,
            "#[allow(dead_code)] fn __ui_lang_check_{}(value: &{}) {{",
            item.name.to_ascii_lowercase(),
            item.rust_path
        )
        .unwrap();
        for (field, ty) in &item.fields {
            writeln!(
                out,
                "let _: &{} = &value.{field};",
                ty.rust(&document.structs)
            )
            .unwrap();
        }
        writeln!(out, "}}").unwrap();
    }
    for item in &document.functions {
        let params = item
            .params
            .iter()
            .enumerate()
            .map(|(index, (_, ty))| format!("arg{index}: {}", ty.rust(&document.structs)))
            .collect::<Vec<_>>()
            .join(", ");
        let args = (0..item.params.len())
            .map(|index| format!("arg{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let output = item.error.as_ref().map_or_else(
            || item.output.rust(&document.structs),
            |error| {
                format!(
                    "::std::result::Result<{}, {}>",
                    item.output.rust(&document.structs),
                    error.rust(&document.structs)
                )
            },
        );
        match item.kind {
            ExternKind::Future => writeln!(
                out,
                "#[allow(dead_code)] async fn __ui_lang_check_{}({params}) {{ let _: {output} = {}({args}).await; }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Component => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_component_{}({params}) {{ let _: ::iced::Element<'static, {output}> = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Shader => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_shader_{}({params}) {{ let __program = {}({args}); fn __accept<P: ::iced::widget::shader::Program<{output}>>(_: &P) {{}} __accept(&__program); let _: ::iced::Element<'static, {output}> = ::iced::widget::Shader::new(__program).into(); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Task => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_task_{}({params}) {{ let _: ::iced::Task<{output}> = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Stream => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_stream_{}({params}) {{ let _: ::iced::Task<{output}> = ::iced::Task::run({}({args}), |value| value); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Sip => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_sip_{}({params}) {{ let _: ::iced::Task<()> = ::iced::Task::sip({}({args}), |value| {{ let _: {} = value; }}, |value| {{ let _: {output} = value; }}); }}",
                item.name,
                item.rust_path,
                item.progress
                    .as_ref()
                    .expect("sip extern has a progress type")
                    .rust(&document.structs)
            )
            .unwrap(),
            ExternKind::Recipe => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_recipe_{}({params}) {{ let __recipe = {}({args}); fn __accept<R: ::iced::advanced::subscription::Recipe<Output = {output}>>(_: &R) {{}} __accept(&__recipe); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Selector => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_selector_{}({params}) {{ let _: ::iced::Task<::std::option::Option<{output}>> = ::iced::widget::selector::find({}({args})); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::EventFilter => {
                let recipe = format!("__IceEventFilter{}", pascal(&item.name));
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_event_filter_{}() {{ let _: fn(::iced::advanced::subscription::Event) -> ::std::option::Option<{output}> = {}; }}",
                    item.name, item.rust_path
                )
                .unwrap();
                writeln!(
                    out,
                    "struct {recipe}<I> {{ id: I }} impl<I: ::std::hash::Hash + 'static> ::iced::advanced::subscription::Recipe for {recipe}<I> {{ type Output = {output}; fn hash(&self, state: &mut ::iced::advanced::subscription::Hasher) {{ ::std::hash::Hash::hash(&::std::any::TypeId::of::<Self>(), state); ::std::hash::Hash::hash(&self.id, state); }} fn stream(self: ::std::boxed::Box<Self>, input: ::iced::advanced::subscription::EventStream) -> __IceEventStream<Self::Output> {{ ::std::boxed::Box::pin(::iced::futures::StreamExt::filter_map(input, |event| ::iced::futures::future::ready({}(event)))) }} }}",
                    item.rust_path
                )
                .unwrap();
            }
            ExternKind::Sync => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_sync_{}({params}) {{ let _: {output} = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Subscription => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_subscription_{}({params}) {{ let _: ::iced::Subscription<{output}> = {}({args}); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::Window => {
                let params = if params.is_empty() {
                    "window: &dyn ::iced::window::Window".into()
                } else {
                    format!("window: &dyn ::iced::window::Window, {params}")
                };
                let args = if args.is_empty() {
                    "window".into()
                } else {
                    format!("window, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_window_{}({params}) {{ let _: {output} = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::MarkdownViewer => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_markdown_viewer_{}({params}) {{ let __viewer = {}({args}); fn __accept<V>(_: &V) where for<'a> V: ::iced::widget::markdown::Viewer<'a, {output}, ::iced::Theme, ::iced::Renderer> {{}} __accept(&__viewer); }}",
                item.name, item.rust_path
            )
            .unwrap(),
            ExternKind::EditorBinding => {
                let callback_params = std::iter::once(
                    "::iced::widget::text_editor::KeyPress".to_owned(),
                )
                .chain(
                    item.params
                        .iter()
                        .map(|(_, ty)| ty.rust(&document.structs)),
                )
                .collect::<Vec<_>>()
                .join(", ");
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_editor_binding_{}() {{ let _: fn({callback_params}) -> ::std::option::Option<::iced::widget::text_editor::Binding<{output}>> = {}; }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::EditorHighlighter => writeln!(
                out,
                "#[allow(dead_code)] fn __ui_lang_check_editor_highlighter_{}({params}) {{ let __content = ::iced::widget::text_editor::Content::new(); let __editor = ::iced::widget::text_editor(&__content).on_action(|_| ()); let _: ::iced::Element<'_, ()> = {}(__editor{}).into(); }}",
                item.name,
                item.rust_path,
                if args.is_empty() {
                    String::new()
                } else {
                    format!(", {args}")
                }
            )
            .unwrap(),
            ExternKind::EditorStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::text_editor::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::text_editor::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_editor_style_{}({params}) {{ let _: ::iced::widget::text_editor::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::TextStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme".into()
                } else {
                    format!("theme: &::iced::Theme, {params}")
                };
                let args = if args.is_empty() {
                    "theme".into()
                } else {
                    format!("theme, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_text_style_{}({params}) {{ let _: ::iced::widget::text::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::SliderStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::slider::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::slider::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_slider_style_{}({params}) {{ let _: ::iced::widget::slider::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::ProgressStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme".into()
                } else {
                    format!("theme: &::iced::Theme, {params}")
                };
                let args = if args.is_empty() {
                    "theme".into()
                } else {
                    format!("theme, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_progress_style_{}({params}) {{ let _: ::iced::widget::progress_bar::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::ButtonStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::button::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::button::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_button_style_{}({params}) {{ let _: ::iced::widget::button::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::CheckboxStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::checkbox::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::checkbox::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_checkbox_style_{}({params}) {{ let _: ::iced::widget::checkbox::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::TogglerStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::toggler::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::toggler::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_toggler_style_{}({params}) {{ let _: ::iced::widget::toggler::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::RadioStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::radio::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::radio::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_radio_style_{}({params}) {{ let _: ::iced::widget::radio::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::ContainerStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme".into()
                } else {
                    format!("theme: &::iced::Theme, {params}")
                };
                let args = if args.is_empty() {
                    "theme".into()
                } else {
                    format!("theme, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_container_style_{}({params}) {{ let _: ::iced::widget::container::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::SvgStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::svg::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::svg::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_svg_style_{}({params}) {{ let _: ::iced::widget::svg::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::InputStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::text_input::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::text_input::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_input_style_{}({params}) {{ let _: ::iced::widget::text_input::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::ScrollStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::scrollable::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::scrollable::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_scroll_style_{}({params}) {{ let _: ::iced::widget::scrollable::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::PickListStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme, status: ::iced::widget::pick_list::Status".into()
                } else {
                    format!(
                        "theme: &::iced::Theme, status: ::iced::widget::pick_list::Status, {params}"
                    )
                };
                let args = if args.is_empty() {
                    "theme, status".into()
                } else {
                    format!("theme, status, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_pick_list_style_{}({params}) {{ let _: ::iced::widget::pick_list::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
            ExternKind::MenuStyle => {
                let params = if params.is_empty() {
                    "theme: &::iced::Theme".into()
                } else {
                    format!("theme: &::iced::Theme, {params}")
                };
                let args = if args.is_empty() {
                    "theme".into()
                } else {
                    format!("theme, {args}")
                };
                writeln!(
                    out,
                    "#[allow(dead_code)] fn __ui_lang_check_menu_style_{}({params}) {{ let _: ::iced::overlay::menu::Style = {}({args}); }}",
                    item.name, item.rust_path
                )
                .unwrap();
            }
        }
    }
}

fn generate_editor_binding_mapper(out: &mut String, document: &Document) {
    if !document
        .functions
        .iter()
        .any(|item| item.kind == ExternKind::EditorBinding)
    {
        return;
    }
    writeln!(
        out,
        "fn __ice_map_editor_binding<T, M>(binding: ::iced::widget::text_editor::Binding<T>, custom: &impl Fn(T) -> M) -> ::iced::widget::text_editor::Binding<M> {{ use ::iced::widget::text_editor::Binding; match binding {{ Binding::Unfocus => Binding::Unfocus, Binding::Copy => Binding::Copy, Binding::Cut => Binding::Cut, Binding::Paste => Binding::Paste, Binding::Move(value) => Binding::Move(value), Binding::Select(value) => Binding::Select(value), Binding::SelectWord => Binding::SelectWord, Binding::SelectLine => Binding::SelectLine, Binding::SelectAll => Binding::SelectAll, Binding::Insert(value) => Binding::Insert(value), Binding::Enter => Binding::Enter, Binding::Backspace => Binding::Backspace, Binding::Delete => Binding::Delete, Binding::Sequence(values) => Binding::Sequence(values.into_iter().map(|value| __ice_map_editor_binding(value, custom)).collect()), Binding::Custom(value) => Binding::Custom(custom(value)), }} }}"
    )
    .unwrap();
}

fn generate_theme(out: &mut String, document: &Document) -> Result<(), Error> {
    let env = state_env(document, "self");
    let color = |name: &str, fallback: &str| {
        color_code(
            document
                .theme
                .get(name)
                .map(String::as_str)
                .unwrap_or(fallback),
            None,
        )
    };
    writeln!(out, "fn __app_theme() -> ::iced::Theme {{").unwrap();
    writeln!(
        out,
        "::iced::Theme::custom(\"{}\", ::iced::theme::Palette {{",
        document.app
    )
    .unwrap();
    writeln!(out, "background: {},", color("background", "#000000")).unwrap();
    writeln!(out, "text: {},", color("foreground", "#ffffff")).unwrap();
    writeln!(out, "primary: {},", color("primary", "#5865f2")).unwrap();
    writeln!(out, "success: {},", color("primary", "#5865f2")).unwrap();
    writeln!(out, "warning: {},", color("danger", "#c3423f")).unwrap();
    writeln!(out, "danger: {},", color("danger", "#c3423f")).unwrap();
    writeln!(out, "}})\n}}").unwrap();
    writeln!(out, "fn __theme(&self) -> ::iced::Theme {{").unwrap();
    if let Some(setting) = &document.settings.theme {
        let value = expr_code(&setting.value, &env, document, ValueMode::Owned)?;
        writeln!(out, "match ({value}).as_str() {{").unwrap();
        writeln!(out, "\"app\" => Self::__app_theme(),").unwrap();
        writeln!(out, "\"default\" => <::iced::Theme as ::iced::theme::Base>::default(::iced::theme::Mode::None),").unwrap();
        for name in BUILT_IN_THEMES {
            writeln!(out, "\"{name}\" => ::iced::Theme::{},", pascal(name)).unwrap();
        }
        writeln!(out, "_ => Self::__app_theme(),\n}}").unwrap();
    } else {
        writeln!(out, "Self::__app_theme()").unwrap();
    }
    writeln!(out, "}}").unwrap();
    if let Some(setting) = &document.settings.title {
        let value = expr_code(&setting.value, &env, document, ValueMode::Owned)?;
        writeln!(
            out,
            "fn __title(&self) -> ::std::string::String {{ {value} }}"
        )
        .unwrap();
    }
    if document.settings.background.is_some() || document.settings.text_color.is_some() {
        writeln!(out, "fn __style(&self, __theme: &::iced::Theme) -> ::iced::theme::Style {{ let mut __style = ::iced::theme::Base::base(__theme);").unwrap();
        for (setting, field) in [
            (&document.settings.background, "background_color"),
            (&document.settings.text_color, "text_color"),
        ] {
            if let Some(setting) = setting {
                let value = expr_code(&setting.value, &env, document, ValueMode::Owned)?;
                writeln!(out, "__style.{field} = ({value}).parse::<::iced::Color>().unwrap_or(__style.{field});").unwrap();
            }
        }
        writeln!(out, "__style }}").unwrap();
    }
    if let Some(setting) = &document.settings.scale_factor {
        let value = expr_code(&setting.value, &env, document, ValueMode::Owned)?;
        writeln!(
            out,
            "fn __scale_factor(&self) -> f32 {{ (({value}) as f32).max(f32::EPSILON) }}"
        )
        .unwrap();
    }
    Ok(())
}

fn generate_boot(out: &mut String, document: &Document, message: &str) -> Result<(), Error> {
    writeln!(out, "fn __state() -> Self {{\nSelf {{").unwrap();
    for qr in &document.qr_codes {
        writeln!(out, "{}: {},", qr.name, qr_data_code(qr)).unwrap();
    }
    for state in &document.states {
        writeln!(
            out,
            "{}: {},",
            state.name,
            initial_code(&state.initial, &state.ty, document)
        )
        .unwrap();
    }
    for node in pane_grids(&document.view) {
        let ViewNode::PaneGrid {
            name,
            configuration,
            ..
        } = node
        else {
            unreachable!()
        };
        writeln!(
            out,
            "{}: ::iced::widget::pane_grid::State::with_configuration({}),",
            pane_field(name),
            pane_configuration_code(configuration)
        )
        .unwrap();
    }
    writeln!(
        out,
        "}}\n}}\nfn __boot() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self::__state();"
    )
    .unwrap();
    if let Some(mount) = document
        .handlers
        .iter()
        .find(|handler| handler.name == "mount")
    {
        let env = state_env(document, "state");
        writeln!(out, "let task = (|| {{").unwrap();
        let has_task = generate_statements(
            out,
            &mount.statements,
            document,
            message,
            &env,
            "state",
            false,
        )?;
        if !has_task {
            writeln!(out, "::iced::Task::none()").unwrap();
        }
        writeln!(out, "}})();").unwrap();
    } else {
        writeln!(out, "let task = ::iced::Task::none();").unwrap();
    }
    writeln!(out, "(state, task)\n}}").unwrap();
    Ok(())
}

fn generate_presets(out: &mut String, document: &Document, message: &str) -> Result<(), Error> {
    for (index, preset) in document.presets.iter().enumerate() {
        writeln!(
            out,
            "fn __preset_{index}() -> (Self, ::iced::Task<{message}>) {{\nlet mut state = Self::__state();\nlet task = (|| {{"
        )
        .unwrap();
        let env = state_env(document, "state");
        let has_task = generate_statements(
            out,
            &preset.statements,
            document,
            message,
            &env,
            "state",
            false,
        )?;
        if !has_task {
            writeln!(out, "::iced::Task::none()").unwrap();
        }
        writeln!(out, "}})();\n(state, task)\n}}").unwrap();
    }
    Ok(())
}

fn generate_update(out: &mut String, document: &Document, message: &str) -> Result<(), Error> {
    writeln!(
        out,
        "fn __update(&mut self, message: {message}) -> ::iced::Task<{message}> {{\nmatch message {{"
    )
    .unwrap();
    for handler in &document.handlers {
        if handler.name == "mount" {
            continue;
        }
        let variant = pascal(&handler.name);
        let pattern = if handler.params.is_empty() {
            format!("{message}::{variant}")
        } else {
            format!(
                "{message}::{variant}({})",
                handler
                    .params
                    .iter()
                    .map(|param| param.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        writeln!(out, "{pattern} => {{").unwrap();
        let mut env = state_env(document, "self");
        for param in &handler.params {
            env.insert(
                param.name.clone(),
                Binding {
                    code: param.name.clone(),
                    ty: param.ty.clone(),
                    local: true,
                },
            );
        }
        let has_task = generate_statements(
            out,
            &handler.statements,
            document,
            message,
            &env,
            "self",
            true,
        )?;
        if !has_task {
            writeln!(out, "::iced::Task::none()").unwrap();
        }
        writeln!(out, "}}").unwrap();
    }
    for node in pane_grids(&document.view) {
        let ViewNode::PaneGrid { name, options, .. } = node else {
            unreachable!()
        };
        if options.resize_leeway.is_some() {
            writeln!(
                out,
                "{message}::{}(__event) => {{ self.{}.resize(__event.split, __event.ratio); ::iced::Task::none() }},",
                pane_resize_variant(name),
                pane_field(name)
            )
            .unwrap();
        }
        if options.draggable {
            writeln!(
                out,
                "{message}::{}(__event) => {{ if let ::iced::widget::pane_grid::DragEvent::Dropped {{ pane, target }} = __event {{ self.{}.drop(pane, target); }} ::iced::Task::none() }},",
                pane_drag_variant(name),
                pane_field(name)
            )
            .unwrap();
        }
    }
    for binding in controlled_state_bindings(document, false)
        .expect("checker validates controlled input bindings")
    {
        let variant = binding_variant(&binding);
        writeln!(
            out,
            "{message}::{variant}(value) => {{ self.{binding} = value; ::iced::Task::none() }}"
        )
        .unwrap();
    }
    for binding in controlled_state_bindings(document, true)
        .expect("checker validates controlled editor bindings")
    {
        let variant = editor_variant(&binding);
        writeln!(
            out,
            "{message}::{variant}(action) => {{ self.{binding}.perform(action); ::iced::Task::none() }}"
        )
        .unwrap();
    }
    if needs_extern_noop(document) {
        writeln!(out, "{message}::__ExternNoop => ::iced::Task::none(),").unwrap();
    }
    writeln!(out, "}}\n}}").unwrap();
    Ok(())
}

fn subscription_payload_arity(source: &SubscriptionSource, window_id: bool) -> usize {
    let arity = match source {
        SubscriptionSource::Every { .. }
        | SubscriptionSource::Repeat { .. }
        | SubscriptionSource::Run { .. }
        | SubscriptionSource::Recipe { .. }
        | SubscriptionSource::Events { .. }
        | SubscriptionSource::Extern { .. }
        | SubscriptionSource::Event { .. }
        | SubscriptionSource::Keyboard(_)
        | SubscriptionSource::SystemTheme => 1,
        SubscriptionSource::InputMethod(InputMethodEvent::Opened | InputMethodEvent::Closed)
        | SubscriptionSource::Mouse(MouseEvent::Entered | MouseEvent::Left)
        | SubscriptionSource::Window(
            WindowEvent::Frame
            | WindowEvent::Closed
            | WindowEvent::CloseRequested
            | WindowEvent::Focused
            | WindowEvent::Unfocused
            | WindowEvent::FilesHoveredLeft,
        ) => 0,
        SubscriptionSource::InputMethod(InputMethodEvent::Commit)
        | SubscriptionSource::Mouse(MouseEvent::Pressed | MouseEvent::Released)
        | SubscriptionSource::Window(
            WindowEvent::Rescaled | WindowEvent::FileHovered | WindowEvent::FileDropped,
        ) => 1,
        SubscriptionSource::Mouse(MouseEvent::Moved)
        | SubscriptionSource::Window(WindowEvent::Moved | WindowEvent::Resized) => 2,
        SubscriptionSource::InputMethod(InputMethodEvent::Preedit)
        | SubscriptionSource::Mouse(MouseEvent::Wheel)
        | SubscriptionSource::Touch(_) => 3,
        SubscriptionSource::Window(WindowEvent::Opened) => 4,
    };
    arity + usize::from(window_id)
}

fn identified_window_filter(filter: &str, arity: usize) -> String {
    match arity {
        0 => format!("({filter}).map(|_| __id)"),
        1 => format!("({filter}).map(|__value| (__id, __value))"),
        count => format!(
            "({filter}).map(|__value| (__id, {}))",
            (0..count)
                .map(|index| format!("__value.{index}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn generate_subscription(
    out: &mut String,
    document: &Document,
    message: &str,
) -> Result<(), Error> {
    if document.subscriptions.is_empty() {
        return Ok(());
    }
    let env = state_env(document, "self");
    writeln!(
        out,
        "fn __subscription(&self) -> ::iced::Subscription<{message}> {{"
    )
    .unwrap();
    writeln!(out, "::iced::Subscription::batch([").unwrap();
    for subscription in &document.subscriptions {
        let source_arity = subscription_payload_arity(&subscription.source, subscription.window_id);
        let filter = subscription
            .filter
            .as_ref()
            .map(|filter| {
                let function = document
                    .functions
                    .iter()
                    .find(|item| item.name == *filter && item.kind == ExternKind::Sync)
                    .ok_or_else(|| {
                        Error::new(
                            "E130",
                            &subscription.span,
                            format!("unknown subscription filter `{filter}`"),
                        )
                    })?;
                let args = match source_arity {
                    0 => String::new(),
                    1 => "__value".into(),
                    count => (0..count)
                        .map(|index| format!("__value.{index}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                };
                Ok(format!(
                    ".filter_map(|{}| {}({args}))",
                    if source_arity == 0 { "_" } else { "__value" },
                    function.rust_path
                ))
            })
            .transpose()?
            .unwrap_or_default();
        let context = subscription
            .context
            .as_ref()
            .map(|context| expr_code(context, &env, document, ValueMode::Owned))
            .transpose()?
            .map(|context| format!(".with({context})"))
            .unwrap_or_default();
        let output_arity = if subscription.filter.is_some() {
            1
        } else {
            source_arity
        };
        let mut payloads = Vec::new();
        if subscription.context.is_some() {
            payloads.push("__value.0".to_owned());
        }
        match output_arity {
            0 => {}
            1 => payloads.push(if subscription.context.is_some() {
                "__value.1".into()
            } else {
                "__value".into()
            }),
            count => payloads.extend((0..count).map(|index| {
                if subscription.context.is_some() {
                    format!("__value.1.{index}")
                } else {
                    format!("__value.{index}")
                }
            })),
        }
        let payloads = payloads.iter().map(String::as_str).collect::<Vec<_>>();
        let route = ordered_route_code(&subscription.route, &payloads, &env, document, message)?;
        let transforms = format!("{filter}{context}");
        let condition = subscription
            .condition
            .as_ref()
            .map(|condition| expr_code(condition, &env, document, ValueMode::Owned))
            .transpose()?;
        if let Some(condition) = &condition {
            write!(out, "if {condition} {{ ::iced::Subscription::batch([").unwrap();
        }
        match &subscription.source {
            SubscriptionSource::Every { milliseconds } => {
                writeln!(out, "::iced::time::every(::std::time::Duration::from_millis({milliseconds})){transforms}.map(move |__value| {route}),").unwrap();
            }
            SubscriptionSource::Repeat {
                function,
                milliseconds,
            } => {
                let source = document
                    .functions
                    .iter()
                    .find(|item| item.name == *function && item.kind == ExternKind::Future)
                    .ok_or_else(|| {
                        Error::new(
                            "E130",
                            &subscription.span,
                            format!("unknown repeated async function `{function}`"),
                        )
                    })?;
                writeln!(out, "::iced::time::repeat({}, ::std::time::Duration::from_millis({milliseconds})){transforms}.map(move |__value| {route}),", source.rust_path).unwrap();
            }
            SubscriptionSource::Run { function, args } => {
                let source = document
                    .functions
                    .iter()
                    .find(|item| item.name == *function && item.kind == ExternKind::Stream)
                    .ok_or_else(|| {
                        Error::new(
                            "E130",
                            &subscription.span,
                            format!("unknown subscription stream `{function}`"),
                        )
                    })?;
                if args.is_empty() {
                    writeln!(
                        out,
                        "::iced::Subscription::run({}){transforms}.map(move |__value| {route}),",
                        source.rust_path
                    )
                    .unwrap();
                } else {
                    let data = args
                        .iter()
                        .map(|arg| expr_code(arg, &env, document, ValueMode::Owned))
                        .collect::<Result<Vec<_>, _>>()?;
                    let types = source
                        .params
                        .iter()
                        .map(|(_, ty)| ty.rust(&document.structs))
                        .collect::<Vec<_>>();
                    let (data, data_type, builder_args) = if args.len() == 1 {
                        (data[0].clone(), types[0].clone(), "__data.clone()".into())
                    } else {
                        (
                            format!("({},)", data.join(", ")),
                            format!("({},)", types.join(", ")),
                            (0..args.len())
                                .map(|index| format!("__data.{index}.clone()"))
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                    };
                    writeln!(out, "::iced::Subscription::run_with({data}, |__data: &{data_type}| {}({builder_args})){transforms}.map(move |__value| {route}),", source.rust_path).unwrap();
                }
            }
            SubscriptionSource::Recipe { function, args } => {
                let source = document
                    .functions
                    .iter()
                    .find(|item| item.name == *function && item.kind == ExternKind::Recipe)
                    .ok_or_else(|| {
                        Error::new(
                            "E130",
                            &subscription.span,
                            format!("unknown subscription recipe `{function}`"),
                        )
                    })?;
                let args = args
                    .iter()
                    .map(|arg| expr_code(arg, &env, document, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                writeln!(out, "::iced::advanced::subscription::from_recipe({}({args})){transforms}.map(move |__value| {route}),", source.rust_path).unwrap();
            }
            SubscriptionSource::Events { id, filter } => {
                let _source = document
                    .functions
                    .iter()
                    .find(|item| item.name == *filter && item.kind == ExternKind::EventFilter)
                    .ok_or_else(|| {
                        Error::new(
                            "E130",
                            &subscription.span,
                            format!("unknown event filter `{filter}`"),
                        )
                    })?;
                let id = expr_code(id, &env, document, ValueMode::Owned)?;
                let recipe = format!("__IceEventFilter{}", pascal(filter));
                writeln!(out, "::iced::advanced::subscription::from_recipe({recipe} {{ id: {id} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            SubscriptionSource::Event { raw } => {
                if !*raw && subscription.status.is_none() && !subscription.window_id {
                    writeln!(
                        out,
                        "::iced::event::listen(){transforms}.map(move |__value| {route}),"
                    )
                    .unwrap();
                } else {
                    let value = if subscription.window_id {
                        "::std::option::Option::Some((__id, __event))"
                    } else {
                        "::std::option::Option::Some(__event)"
                    };
                    let status = if *raw || subscription.status.is_some() {
                        subscription.status
                    } else {
                        Some(EventStatus::Ignored)
                    };
                    let (filter, status) = event_status_filter(value, status);
                    let listen = if *raw { "listen_raw" } else { "listen_with" };
                    writeln!(out, "::iced::event::{listen}(|__event, {status}, __id| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
                }
            }
            SubscriptionSource::Extern { function, args } => {
                let source = document
                    .functions
                    .iter()
                    .find(|item| item.name == *function && item.kind == ExternKind::Subscription)
                    .ok_or_else(|| {
                        Error::new(
                            "E130",
                            &subscription.span,
                            format!("unknown extern subscription `{function}`"),
                        )
                    })?;
                let args = args
                    .iter()
                    .map(|arg| expr_code(arg, &env, document, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                writeln!(
                    out,
                    "{}({args}){transforms}.map(move |__value| {route}),",
                    source.rust_path
                )
                .unwrap();
            }
            SubscriptionSource::InputMethod(event) => {
                let filter = match event {
                    InputMethodEvent::Opened => {
                        "matches!(__event, ::iced::Event::InputMethod(::iced::advanced::input_method::Event::Opened)).then_some(())"
                    }
                    InputMethodEvent::Preedit => {
                        "match __event { ::iced::Event::InputMethod(::iced::advanced::input_method::Event::Preedit(content, range)) => { let (start, end) = range.map_or((::std::option::Option::None, ::std::option::Option::None), |range| (::std::option::Option::Some(i64::try_from(range.start).unwrap_or(i64::MAX)), ::std::option::Option::Some(i64::try_from(range.end).unwrap_or(i64::MAX)))); ::std::option::Option::Some((content, start, end)) }, _ => ::std::option::Option::None }"
                    }
                    InputMethodEvent::Commit => {
                        "match __event { ::iced::Event::InputMethod(::iced::advanced::input_method::Event::Commit(content)) => ::std::option::Option::Some(content), _ => ::std::option::Option::None }"
                    }
                    InputMethodEvent::Closed => {
                        "matches!(__event, ::iced::Event::InputMethod(::iced::advanced::input_method::Event::Closed)).then_some(())"
                    }
                };
                let (filter, status) = event_status_filter(filter, subscription.status);
                writeln!(out, "::iced::event::listen_with(|__event, {status}, _| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            SubscriptionSource::Keyboard(event) => {
                let filter = match event {
                    KeyboardEvent::Press => {
                        "match __event { ::iced::keyboard::Event::KeyPressed { key, modified_key, physical_key, location, modifiers, text, repeat } => ::std::option::Option::Some(__IceKeyPress { key, modified_key, physical_key, location, modifiers, text: text.map(|value| value.to_string()), repeat }), _ => ::std::option::Option::None }"
                    }
                    KeyboardEvent::Release => {
                        "match __event { ::iced::keyboard::Event::KeyReleased { key, modified_key, physical_key, location, modifiers } => ::std::option::Option::Some(__IceKeyRelease { key, modified_key, physical_key, location, modifiers }), _ => ::std::option::Option::None }"
                    }
                    KeyboardEvent::Modifiers => {
                        "match __event { ::iced::keyboard::Event::ModifiersChanged(modifiers) => ::std::option::Option::Some(modifiers), _ => ::std::option::Option::None }"
                    }
                };
                if subscription.status.is_some() {
                    let filter = format!(
                        "match __event {{ ::iced::Event::Keyboard(__event) => {{ {filter} }}, _ => ::std::option::Option::None }}"
                    );
                    let (filter, status) = event_status_filter(&filter, subscription.status);
                    writeln!(out, "::iced::event::listen_with(|__event, {status}, _| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
                } else {
                    writeln!(out, "::iced::keyboard::listen().filter_map(|__event| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
                }
            }
            SubscriptionSource::Mouse(event) => {
                let filter = match event {
                    MouseEvent::Entered => {
                        "matches!(__event, ::iced::Event::Mouse(::iced::mouse::Event::CursorEntered)).then_some(())"
                    }
                    MouseEvent::Left => {
                        "matches!(__event, ::iced::Event::Mouse(::iced::mouse::Event::CursorLeft)).then_some(())"
                    }
                    MouseEvent::Moved => {
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::CursorMoved { position }) => ::std::option::Option::Some((position.x as f64, position.y as f64)), _ => ::std::option::Option::None }"
                    }
                    MouseEvent::Pressed => {
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::ButtonPressed(button)) => ::std::option::Option::Some(button), _ => ::std::option::Option::None }"
                    }
                    MouseEvent::Released => {
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::ButtonReleased(button)) => ::std::option::Option::Some(button), _ => ::std::option::Option::None }"
                    }
                    MouseEvent::Wheel => {
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::WheelScrolled { delta }) => { let (x, y, pixels) = match delta { ::iced::mouse::ScrollDelta::Lines { x, y } => (x as f64, y as f64, false), ::iced::mouse::ScrollDelta::Pixels { x, y } => (x as f64, y as f64, true) }; ::std::option::Option::Some((x, y, pixels)) }, _ => ::std::option::Option::None }"
                    }
                };
                let (filter, status) = event_status_filter(filter, subscription.status);
                writeln!(out, "::iced::event::listen_with(|__event, {status}, _| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            SubscriptionSource::SystemTheme => {
                writeln!(out, "::iced::system::theme_changes().map(__ice_system_theme){transforms}.map(move |__value| {route}),").unwrap();
            }
            SubscriptionSource::Touch(event) => {
                let variant = match event {
                    TouchEvent::Pressed => "FingerPressed",
                    TouchEvent::Moved => "FingerMoved",
                    TouchEvent::Lifted => "FingerLifted",
                    TouchEvent::Lost => "FingerLost",
                };
                let filter = format!(
                    "match __event {{ ::iced::Event::Touch(::iced::touch::Event::{variant} {{ id, position }}) => ::std::option::Option::Some((id, position.x as f64, position.y as f64)), _ => ::std::option::Option::None }}"
                );
                let (filter, status) = event_status_filter(&filter, subscription.status);
                writeln!(out, "::iced::event::listen_with(|__event, {status}, _| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
            }
            SubscriptionSource::Window(event) => {
                if *event == WindowEvent::Frame {
                    writeln!(
                        out,
                        "::iced::window::frames(){transforms}.map(move |__value| {route}),"
                    )
                    .unwrap();
                    if condition.is_some() {
                        writeln!(out, "]) }} else {{ ::iced::Subscription::none() }},").unwrap();
                    }
                    continue;
                }
                let filter = match event {
                    WindowEvent::Opened => {
                        "match __event { ::iced::window::Event::Opened { position, size } => { let (x, y) = position.map_or((::std::option::Option::None, ::std::option::Option::None), |position| (::std::option::Option::Some(position.x as f64), ::std::option::Option::Some(position.y as f64))); ::std::option::Option::Some((x, y, size.width as f64, size.height as f64)) }, _ => ::std::option::Option::None }"
                    }
                    WindowEvent::Closed => {
                        "matches!(__event, ::iced::window::Event::Closed).then_some(())"
                    }
                    WindowEvent::Moved => {
                        "match __event { ::iced::window::Event::Moved(position) => ::std::option::Option::Some((position.x as f64, position.y as f64)), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::Resized => {
                        "match __event { ::iced::window::Event::Resized(size) => ::std::option::Option::Some((size.width as f64, size.height as f64)), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::Rescaled => {
                        "match __event { ::iced::window::Event::Rescaled(scale) => ::std::option::Option::Some(scale as f64), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::CloseRequested => {
                        "matches!(__event, ::iced::window::Event::CloseRequested).then_some(())"
                    }
                    WindowEvent::Focused => {
                        "matches!(__event, ::iced::window::Event::Focused).then_some(())"
                    }
                    WindowEvent::Unfocused => {
                        "matches!(__event, ::iced::window::Event::Unfocused).then_some(())"
                    }
                    WindowEvent::FileHovered => {
                        "match __event { ::iced::window::Event::FileHovered(path) => ::std::option::Option::Some(path.to_string_lossy().into_owned()), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::FileDropped => {
                        "match __event { ::iced::window::Event::FileDropped(path) => ::std::option::Option::Some(path.to_string_lossy().into_owned()), _ => ::std::option::Option::None }"
                    }
                    WindowEvent::FilesHoveredLeft => {
                        "matches!(__event, ::iced::window::Event::FilesHoveredLeft).then_some(())"
                    }
                    WindowEvent::Frame => unreachable!("handled above"),
                };
                let filter = if subscription.window_id {
                    identified_window_filter(
                        filter,
                        subscription_payload_arity(&subscription.source, false),
                    )
                } else {
                    filter.to_owned()
                };
                if subscription.status.is_some() {
                    let filter = format!(
                        "match __event {{ ::iced::Event::Window(__event) => {{ {filter} }}, _ => ::std::option::Option::None }}"
                    );
                    let (filter, status) = event_status_filter(&filter, subscription.status);
                    writeln!(out, "::iced::event::listen_with(|__event, {status}, __id| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
                } else {
                    writeln!(out, "::iced::window::events().filter_map(|(__id, __event)| {{ {filter} }}){transforms}.map(move |__value| {route}),").unwrap();
                }
            }
        }
        if condition.is_some() {
            writeln!(out, "]) }} else {{ ::iced::Subscription::none() }},").unwrap();
        }
    }
    writeln!(out, "])\n}}").unwrap();
    Ok(())
}

fn event_status_filter(filter: &str, status: Option<EventStatus>) -> (String, &'static str) {
    match status {
        None | Some(EventStatus::Any) => (filter.to_owned(), "_"),
        Some(EventStatus::Captured) => (
            format!(
                "if matches!(__status, ::iced::event::Status::Captured) {{ {filter} }} else {{ ::std::option::Option::None }}"
            ),
            "__status",
        ),
        Some(EventStatus::Ignored) => (
            format!(
                "if matches!(__status, ::iced::event::Status::Ignored) {{ {filter} }} else {{ ::std::option::Option::None }}"
            ),
            "__status",
        ),
    }
}

#[cfg(test)]
#[path = "codegen/tests.rs"]
mod tests;
