use crate::Error;
use crate::ast::*;
use crate::check::{controlled_state_bindings, expr_type};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

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
    generate_mouse_types(&mut out, document);
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
        r#"#[derive(Debug, Clone, Copy)]
struct __IceKeyModifiers {
    shift: bool,
    control: bool,
    alt: bool,
    logo: bool,
    command: bool,
    jump: bool,
    macos_command: bool,
}
#[derive(Debug, Clone)]
struct __IceKeyPress {
    key: ::std::string::String,
    modified_key: ::std::string::String,
    physical_key: ::std::string::String,
    location: ::std::string::String,
    modifiers: __IceKeyModifiers,
    text: ::std::option::Option<::std::string::String>,
    repeat: bool,
}
#[derive(Debug, Clone)]
struct __IceKeyRelease {
    key: ::std::string::String,
    modified_key: ::std::string::String,
    physical_key: ::std::string::String,
    location: ::std::string::String,
    modifiers: __IceKeyModifiers,
}
fn __ice_key(value: ::iced::keyboard::Key) -> ::std::string::String {
    match value {
        ::iced::keyboard::Key::Named(value) => ::std::format!("{value:?}"),
        ::iced::keyboard::Key::Character(value) => value.to_string(),
        ::iced::keyboard::Key::Unidentified => "Unidentified".to_owned(),
    }
}
fn __ice_key_modifiers(value: ::iced::keyboard::Modifiers) -> __IceKeyModifiers {
    __IceKeyModifiers {
        shift: value.shift(),
        control: value.control(),
        alt: value.alt(),
        logo: value.logo(),
        command: value.command(),
        jump: value.jump(),
        macos_command: value.macos_command(),
    }
}
fn __ice_key_location(value: ::iced::keyboard::Location) -> ::std::string::String {
    match value {
        ::iced::keyboard::Location::Standard => "standard",
        ::iced::keyboard::Location::Left => "left",
        ::iced::keyboard::Location::Right => "right",
        ::iced::keyboard::Location::Numpad => "numpad",
    }.to_owned()
}
"#,
    );
}

fn generate_mouse_types(out: &mut String, document: &Document) {
    if !document
        .subscriptions
        .iter()
        .any(|subscription| matches!(&subscription.source, SubscriptionSource::Mouse(_)))
        && !canvas_events(document)
            .iter()
            .any(|event| matches!(event.source, SubscriptionSource::Mouse(_)))
    {
        return;
    }
    out.push_str(
        r#"fn __ice_mouse_button(value: ::iced::mouse::Button) -> ::std::string::String {
    match value {
        ::iced::mouse::Button::Left => "left".to_owned(),
        ::iced::mouse::Button::Right => "right".to_owned(),
        ::iced::mouse::Button::Middle => "middle".to_owned(),
        ::iced::mouse::Button::Back => "back".to_owned(),
        ::iced::mouse::Button::Forward => "forward".to_owned(),
        ::iced::mouse::Button::Other(number) => ::std::format!("other-{number}"),
    }
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
                        "match __event { ::iced::keyboard::Event::KeyPressed { key, modified_key, physical_key, location, modifiers, text, repeat } => ::std::option::Option::Some(__IceKeyPress { key: __ice_key(key), modified_key: __ice_key(modified_key), physical_key: ::std::format!(\"{physical_key:?}\"), location: __ice_key_location(location), modifiers: __ice_key_modifiers(modifiers), text: text.map(|value| value.to_string()), repeat }), _ => ::std::option::Option::None }"
                    }
                    KeyboardEvent::Release => {
                        "match __event { ::iced::keyboard::Event::KeyReleased { key, modified_key, physical_key, location, modifiers } => ::std::option::Option::Some(__IceKeyRelease { key: __ice_key(key), modified_key: __ice_key(modified_key), physical_key: ::std::format!(\"{physical_key:?}\"), location: __ice_key_location(location), modifiers: __ice_key_modifiers(modifiers) }), _ => ::std::option::Option::None }"
                    }
                    KeyboardEvent::Modifiers => {
                        "match __event { ::iced::keyboard::Event::ModifiersChanged(modifiers) => ::std::option::Option::Some(__ice_key_modifiers(modifiers)), _ => ::std::option::Option::None }"
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
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::ButtonPressed(button)) => ::std::option::Option::Some(__ice_mouse_button(button)), _ => ::std::option::Option::None }"
                    }
                    MouseEvent::Released => {
                        "match __event { ::iced::Event::Mouse(::iced::mouse::Event::ButtonReleased(button)) => ::std::option::Option::Some(__ice_mouse_button(button)), _ => ::std::option::Option::None }"
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
                    "match __event {{ ::iced::Event::Touch(::iced::touch::Event::{variant} {{ id, position }}) => ::std::option::Option::Some((id.0.to_string(), position.x as f64, position.y as f64)), _ => ::std::option::Option::None }}"
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

fn generate_view(out: &mut String, document: &Document, message: &str) -> Result<(), Error> {
    let env = state_env(document, "self");
    let root = render_node(
        &document.view,
        document,
        message,
        &env,
        &rust_string(&document.app),
        None,
    )?;
    writeln!(
        out,
        "fn __view(&self) -> ::iced::Element<'_, {message}> {{ {root} }}"
    )
    .unwrap();
    Ok(())
}

fn task_source_code(
    source: &TaskSource,
    document: &Document,
    env: &HashMap<String, Binding>,
) -> Result<String, Error> {
    match source {
        TaskSource::Done { value, .. } => Ok(format!(
            "::iced::Task::done({})",
            expr_code(value, env, document, ValueMode::Owned)?
        )),
        TaskSource::None { output, .. } => Ok(format!(
            "::iced::Task::<{}>::none()",
            output.rust(&document.structs)
        )),
        TaskSource::Effect {
            kind,
            function,
            args,
            span,
        } => {
            if *kind == EffectKind::Task {
                match function.as_str() {
                    "__ice_system_info" => {
                        return Ok("::iced::system::information().map(__ice_system_info)".into());
                    }
                    "__ice_system_theme" => {
                        return Ok("::iced::system::theme().map(__ice_system_theme)".into());
                    }
                    "__ice_time_now" => return Ok("::iced::time::now()".into()),
                    "__ice_clipboard_read" => return Ok("::iced::clipboard::read()".into()),
                    "__ice_clipboard_read_primary" => {
                        return Ok("::iced::clipboard::read_primary()".into());
                    }
                    "__ice_font_load" => {
                        let bytes = expr_code(&args[0], env, document, ValueMode::Owned)?;
                        return Ok(format!(
                            "::iced::font::load({bytes}).map(|result| match result {{ ::std::result::Result::Ok(value) => value, ::std::result::Result::Err(error) => match error {{}} }})"
                        ));
                    }
                    _ => {}
                }
            }
            let action = document
                .functions
                .iter()
                .find(|item| item.name == *function && item.kind == (*kind).into())
                .ok_or_else(|| {
                    Error::new(
                        "E130",
                        span,
                        format!("unknown extern task source `{function}`"),
                    )
                })?;
            let args = args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            Ok(match kind {
                EffectKind::Future => format!(
                    "::iced::Task::perform({}({args}), |value| value)",
                    action.rust_path
                ),
                EffectKind::Task => format!("{}({args})", action.rust_path),
                EffectKind::Stream => format!(
                    "::iced::Task::run({}({args}), |value| value)",
                    action.rust_path
                ),
            })
        }
    }
}

fn task_flow_code(
    root: &TaskSource,
    transforms: &[TaskTransform],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
) -> Result<String, Error> {
    let mut task = task_source_code(root, document, env)?;
    let type_env = env
        .iter()
        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
        .collect::<HashMap<_, _>>();
    for (index, transform) in transforms.iter().enumerate() {
        match transform {
            TaskTransform::Then {
                binding, source, ..
            }
            | TaskTransform::AndThen {
                binding, source, ..
            } => {
                let (output, error) =
                    crate::check::task_flow_type(root, &transforms[..index], document, &type_env)?;
                let output = output.expect("discard is the final transform");
                let binding_ty =
                    if matches!(transform, TaskTransform::AndThen { .. }) && error.is_none() {
                        let Type::Option(inner) = output else {
                            unreachable!("checked optional and-then")
                        };
                        *inner
                    } else {
                        output
                    };
                let next_env = HashMap::from([(
                    binding.clone(),
                    Binding {
                        code: binding.clone(),
                        ty: binding_ty,
                        local: false,
                    },
                )]);
                let next = task_source_code(source, document, &next_env)?;
                let method = if matches!(transform, TaskTransform::Then { .. }) {
                    "then"
                } else {
                    "and_then"
                };
                task = format!("({task}).{method}(move |{binding}| {next})");
            }
            TaskTransform::MapError { binding, value, .. } => {
                let (_, error) =
                    crate::check::task_flow_type(root, &transforms[..index], document, &type_env)?;
                let error = error.expect("checked map-error input");
                let map_env = HashMap::from([(
                    binding.clone(),
                    Binding {
                        code: binding.clone(),
                        ty: error,
                        local: false,
                    },
                )]);
                let value = expr_code(value, &map_env, document, ValueMode::Owned)?;
                task = format!("({task}).map_err(move |{binding}| {value})");
            }
            TaskTransform::Collect { .. } => task = format!("({task}).collect()"),
            TaskTransform::Discard { .. } => task = format!("({task}).discard::<{message}>()"),
        }
    }
    Ok(task)
}

fn generate_statements(
    out: &mut String,
    statements: &[Statement],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    state: &str,
    return_task: bool,
) -> Result<bool, Error> {
    let mut has_task = false;
    for statement in statements {
        match statement {
            Statement::Assign { target, value, .. } => {
                let code = expr_code(value, env, document, ValueMode::Owned)?;
                if document
                    .states
                    .iter()
                    .any(|item| item.name == *target && matches!(item.ty, Type::Combo(_)))
                {
                    writeln!(
                        out,
                        "{state}.{target} = ::iced::widget::combo_box::State::new({code});"
                    )
                    .unwrap();
                } else {
                    writeln!(out, "{state}.{target} = {code};").unwrap();
                }
            }
            Statement::MarkdownAppend { target, value, .. } => {
                let code = expr_code(value, env, document, ValueMode::Owned)?;
                writeln!(out, "{state}.{target}.push_str(&{code});").unwrap();
            }
            Statement::ComboPush { target, value, .. } => {
                let code = expr_code(value, env, document, ValueMode::Owned)?;
                writeln!(out, "{state}.{target}.push({code});").unwrap();
            }
            Statement::ReturnIf { condition, .. } => {
                let code = expr_code(condition, env, document, ValueMode::Owned)?;
                writeln!(out, "if {code} {{ return ::iced::Task::none(); }}").unwrap();
            }
            Statement::Run {
                kind,
                function,
                args,
                success,
                error,
                span,
            } => {
                has_task = true;
                if *kind == EffectKind::Task
                    && matches!(
                        function.as_str(),
                        "__ice_system_info"
                            | "__ice_system_theme"
                            | "__ice_time_now"
                            | "__ice_clipboard_read"
                            | "__ice_clipboard_read_primary"
                            | "__ice_font_load"
                    )
                {
                    if function == "__ice_font_load" {
                        let bytes = expr_code(&args[0], env, document, ValueMode::Owned)?;
                        let success_message = route_code(success, "value", env, document, message)?;
                        writeln!(
                            out,
                            "{}::iced::font::load({bytes}).map(move |result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => match error {{}} }}){}",
                            if return_task { "return " } else { "" },
                            if return_task { ";" } else { "" }
                        )
                        .unwrap();
                        continue;
                    }
                    let task = match function.as_str() {
                        "__ice_system_info" => {
                            "::iced::system::information().map(__ice_system_info)"
                        }
                        "__ice_system_theme" => "::iced::system::theme().map(__ice_system_theme)",
                        "__ice_time_now" => "::iced::time::now()",
                        "__ice_clipboard_read" => "::iced::clipboard::read()",
                        "__ice_clipboard_read_primary" => "::iced::clipboard::read_primary()",
                        _ => unreachable!(),
                    };
                    let success_message = route_code(success, "value", env, document, message)?;
                    writeln!(
                        out,
                        "{}{task}.map(move |value| {success_message}){}",
                        if return_task { "return " } else { "" },
                        if return_task { ";" } else { "" }
                    )
                    .unwrap();
                    continue;
                }
                let extern_kind = match kind {
                    EffectKind::Future => ExternKind::Future,
                    EffectKind::Task => ExternKind::Task,
                    EffectKind::Stream => ExternKind::Stream,
                };
                let action = document
                    .functions
                    .iter()
                    .find(|item| item.name == *function && item.kind == extern_kind)
                    .ok_or_else(|| {
                        Error::new("E130", span, format!("unknown extern fn `{function}`"))
                    })?;
                let args = args
                    .iter()
                    .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                let success_message = route_code(success, "value", env, document, message)?;
                if let (Some(error_route), Some(_)) = (error, &action.error) {
                    let error_message = route_code(error_route, "error", env, document, message)?;
                    match kind {
                        EffectKind::Future => writeln!(out, "{}::iced::Task::perform({}({args}), |result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }}){}", if return_task { "return " } else { "" }, action.rust_path, if return_task { ";" } else { "" }).unwrap(),
                        EffectKind::Task => writeln!(out, "{}{}({args}).map(|result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }}){}", if return_task { "return " } else { "" }, action.rust_path, if return_task { ";" } else { "" }).unwrap(),
                        EffectKind::Stream => writeln!(out, "{}::iced::Task::run({}({args}), |result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }}){}", if return_task { "return " } else { "" }, action.rust_path, if return_task { ";" } else { "" }).unwrap(),
                    }
                } else {
                    match kind {
                        EffectKind::Future => writeln!(
                            out,
                            "{}::iced::Task::perform({}({args}), |value| {success_message}){}",
                            if return_task { "return " } else { "" },
                            action.rust_path,
                            if return_task { ";" } else { "" }
                        )
                        .unwrap(),
                        EffectKind::Task => writeln!(
                            out,
                            "{}{}({args}).map(|value| {success_message}){}",
                            if return_task { "return " } else { "" },
                            action.rust_path,
                            if return_task { ";" } else { "" }
                        )
                        .unwrap(),
                        EffectKind::Stream => writeln!(
                            out,
                            "{}::iced::Task::run({}({args}), |value| {success_message}){}",
                            if return_task { "return " } else { "" },
                            action.rust_path,
                            if return_task { ";" } else { "" }
                        )
                        .unwrap(),
                    }
                }
            }
            Statement::Sip {
                function,
                args,
                progress,
                success,
                error,
                span,
            } => {
                has_task = true;
                let action = document
                    .functions
                    .iter()
                    .find(|item| item.name == *function && item.kind == ExternKind::Sip)
                    .ok_or_else(|| {
                        Error::new("E130", span, format!("unknown extern sip `{function}`"))
                    })?;
                let args = args
                    .iter()
                    .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                let progress_message = route_code(progress, "value", env, document, message)?;
                let success_message = route_code(success, "value", env, document, message)?;
                let prefix = if return_task { "return " } else { "" };
                let suffix = if return_task { ";" } else { "" };
                if let (Some(error_route), Some(_)) = (error, &action.error) {
                    let error_message = route_code(error_route, "error", env, document, message)?;
                    writeln!(out, "{prefix}::iced::Task::sip({}({args}), |value| {progress_message}, |result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }}){suffix}", action.rust_path).unwrap();
                } else {
                    writeln!(out, "{prefix}::iced::Task::sip({}({args}), |value| {progress_message}, |value| {success_message}){suffix}", action.rust_path).unwrap();
                }
            }
            Statement::TaskFlow {
                source,
                transforms,
                success,
                error,
                units,
                ..
            } => {
                has_task = true;
                let type_env = env
                    .iter()
                    .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                    .collect::<HashMap<_, _>>();
                let (output, error_ty) =
                    crate::check::task_flow_type(source, transforms, document, &type_env)?;
                let task = task_flow_code(source, transforms, document, message, env)?;
                let mapped = if output.is_none() {
                    task
                } else {
                    let success = success.as_ref().expect("checked flow done route");
                    let success_message = route_code(success, "value", env, document, message)?;
                    if error_ty.is_some() {
                        let error = error.as_ref().expect("checked flow error route");
                        let error_message = route_code(error, "error", env, document, message)?;
                        format!(
                            "({task}).map(|result| match result {{ ::std::result::Result::Ok(value) => {success_message}, ::std::result::Result::Err(error) => {error_message} }})"
                        )
                    } else {
                        format!("({task}).map(|value| {success_message})")
                    }
                };
                let task = if let Some(units) = units {
                    let units_message = route_code(units, "__units", env, document, message)?;
                    format!(
                        "{{ let __task = {mapped}; let __units = i64::try_from(__task.units()).unwrap_or(i64::MAX); ::iced::Task::batch([__task, ::iced::Task::done({units_message})]) }}"
                    )
                } else {
                    mapped
                };
                writeln!(
                    out,
                    "{}{task}{}",
                    if return_task { "return " } else { "" },
                    if return_task { ";" } else { "" }
                )
                .unwrap();
            }
            Statement::TaskGroup {
                kind, statements, ..
            } => {
                has_task = true;
                let mut task_env = env.clone();
                for binding in task_env.values_mut() {
                    binding.local = false;
                }
                if return_task {
                    write!(out, "return ").unwrap();
                }
                match kind {
                    TaskGroupKind::Parallel => {
                        writeln!(out, "::iced::Task::batch([").unwrap();
                        for statement in statements {
                            write!(out, "{{ ").unwrap();
                            generate_statements(
                                out,
                                ::std::slice::from_ref(statement),
                                document,
                                message,
                                &task_env,
                                state,
                                false,
                            )?;
                            writeln!(out, "}},").unwrap();
                        }
                        write!(out, "])").unwrap();
                    }
                    TaskGroupKind::Sequential => {
                        write!(out, "::iced::Task::none()").unwrap();
                        for statement in statements {
                            write!(out, ".chain({{ ").unwrap();
                            generate_statements(
                                out,
                                ::std::slice::from_ref(statement),
                                document,
                                message,
                                &task_env,
                                state,
                                false,
                            )?;
                            write!(out, "}})").unwrap();
                        }
                    }
                }
                writeln!(out, "{}", if return_task { ";" } else { "" }).unwrap();
            }
            Statement::Abortable {
                handle,
                abort_on_drop,
                task,
                ..
            } => {
                has_task = true;
                let mut task_env = env.clone();
                for binding in task_env.values_mut() {
                    binding.local = false;
                }
                if return_task {
                    write!(out, "return ").unwrap();
                }
                writeln!(out, "{{ let (__task, __handle) = ({{").unwrap();
                generate_statements(
                    out,
                    ::std::slice::from_ref(task),
                    document,
                    message,
                    &task_env,
                    state,
                    false,
                )?;
                writeln!(out, "}}).abortable();").unwrap();
                writeln!(
                    out,
                    "{state}.{handle} = ::std::option::Option::Some(__handle{}); __task }}{}",
                    if *abort_on_drop {
                        ".abort_on_drop()"
                    } else {
                        ""
                    },
                    if return_task { ";" } else { "" }
                )
                .unwrap();
            }
            Statement::Abort { handle, .. } => {
                writeln!(out, "if let ::std::option::Option::Some(__handle) = &{state}.{handle} {{ __handle.abort(); }}").unwrap();
            }
            Statement::ClipboardWrite { primary, value, .. } => {
                has_task = true;
                let value = expr_code(value, env, document, ValueMode::Owned)?;
                let function = if *primary { "write_primary" } else { "write" };
                writeln!(
                    out,
                    "{}::iced::clipboard::{function}::<{message}>({value}){}",
                    if return_task { "return " } else { "" },
                    if return_task { ";" } else { "" }
                )
                .unwrap();
            }
            Statement::WidgetOperation {
                operation, route, ..
            } => {
                has_task = true;
                let id = |target: &WidgetTarget| widget_target_code(target, env, document);
                let value = |value: &Expr, cast: &str| {
                    Ok::<_, Error>(format!(
                        "({}) as {cast}",
                        expr_code(value, env, document, ValueMode::Owned)?
                    ))
                };
                let task = match operation {
                    WidgetOperation::FocusPrevious => {
                        format!("::iced::widget::operation::focus_previous::<{message}>()")
                    }
                    WidgetOperation::FocusNext => {
                        format!("::iced::widget::operation::focus_next::<{message}>()")
                    }
                    WidgetOperation::Focus { target } => format!(
                        "::iced::widget::operation::focus::<{message}>({})",
                        id(target)?
                    ),
                    WidgetOperation::Focused { target } => {
                        let route = route.as_ref().expect("checker requires focused route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::widget::operation::is_focused({}).map(move |value| {message_code})",
                            id(target)?
                        )
                    }
                    WidgetOperation::CursorFront { target } => format!(
                        "::iced::widget::operation::move_cursor_to_front::<{message}>({})",
                        id(target)?
                    ),
                    WidgetOperation::CursorEnd { target } => format!(
                        "::iced::widget::operation::move_cursor_to_end::<{message}>({})",
                        id(target)?
                    ),
                    WidgetOperation::Cursor { target, position } => format!(
                        "::iced::widget::operation::move_cursor_to::<{message}>({}, {})",
                        id(target)?,
                        value(position, "usize")?
                    ),
                    WidgetOperation::SelectAll { target } => format!(
                        "::iced::widget::operation::select_all::<{message}>({})",
                        id(target)?
                    ),
                    WidgetOperation::Select { target, start, end } => format!(
                        "::iced::widget::operation::select_range::<{message}>({}, {}, {})",
                        id(target)?,
                        value(start, "usize")?,
                        value(end, "usize")?
                    ),
                    WidgetOperation::Snap { target, x, y } => format!(
                        "::iced::widget::operation::snap_to::<{message}>({}, ::iced::widget::operation::RelativeOffset {{ x: {}, y: {} }})",
                        id(target)?,
                        value(x, "f32")?,
                        value(y, "f32")?
                    ),
                    WidgetOperation::SnapEnd { target } => format!(
                        "::iced::widget::operation::snap_to_end::<{message}>({})",
                        id(target)?
                    ),
                    WidgetOperation::ScrollTo { target, x, y } => format!(
                        "::iced::widget::operation::scroll_to::<{message}>({}, ::iced::widget::operation::AbsoluteOffset {{ x: {}, y: {} }})",
                        id(target)?,
                        value(x, "f32")?,
                        value(y, "f32")?
                    ),
                    WidgetOperation::ScrollBy { target, x, y } => format!(
                        "::iced::widget::operation::scroll_by::<{message}>({}, ::iced::widget::operation::AbsoluteOffset {{ x: {}, y: {} }})",
                        id(target)?,
                        value(x, "f32")?,
                        value(y, "f32")?
                    ),
                    WidgetOperation::Find { selector, all } => {
                        let route = route.as_ref().expect("checker requires selector route");
                        let (selector, conversion) = widget_selector_code(selector, env, document)?;
                        let function = if *all { "find_all" } else { "find" };
                        let mut task = format!("::iced::widget::selector::{function}({selector})");
                        if let Some(conversion) = conversion {
                            if *all {
                                write!(task, ".map(|values| values.into_iter().map({conversion}).collect::<::std::vec::Vec<_>>())").unwrap();
                            } else {
                                write!(task, ".map(|value| value.map({conversion}))").unwrap();
                            }
                        }
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!("{task}.map(move |value| {message_code})")
                    }
                };
                writeln!(
                    out,
                    "{}{task}{}",
                    if return_task { "return " } else { "" },
                    if return_task { ";" } else { "" }
                )
                .unwrap();
            }
            Statement::PaneOperation {
                grid,
                operation,
                route,
                ..
            } => {
                let field = pane_field(grid);
                let pane = |name: &str| {
                    format!(
                        "{state}.{field}.iter().find_map(|(__pane, __name)| (*__name == {}).then_some(*__pane))",
                        rust_string(name)
                    )
                };
                let edge = |edge: &PaneEdge| match edge {
                    PaneEdge::Top => "Top",
                    PaneEdge::Left => "Left",
                    PaneEdge::Right => "Right",
                    PaneEdge::Bottom => "Bottom",
                };
                let axis = |axis: &PaneAxis| match axis {
                    PaneAxis::Horizontal => "Horizontal",
                    PaneAxis::Vertical => "Vertical",
                };
                match operation {
                    PaneOperation::Maximize { pane: name } => writeln!(
                        out,
                        "{{ let __pane = {}; if let ::std::option::Option::Some(__pane) = __pane {{ {state}.{field}.maximize(__pane); }} }}",
                        pane(name)
                    )
                    .unwrap(),
                    PaneOperation::Restore => {
                        writeln!(out, "{state}.{field}.restore();").unwrap()
                    }
                    PaneOperation::Swap { first, second } => writeln!(
                        out,
                        "{{ let __first = {}; let __second = {}; if let (::std::option::Option::Some(__first), ::std::option::Option::Some(__second)) = (__first, __second) {{ {state}.{field}.swap(__first, __second); }} }}",
                        pane(first),
                        pane(second)
                    )
                    .unwrap(),
                    PaneOperation::Close { pane: name } => writeln!(
                        out,
                        "{{ let __pane = {}; if let ::std::option::Option::Some(__pane) = __pane {{ let _ = {state}.{field}.close(__pane); }} }}",
                        pane(name)
                    )
                    .unwrap(),
                    PaneOperation::Move { pane: name, edge: side } => writeln!(
                        out,
                        "{{ let __pane = {}; if let ::std::option::Option::Some(__pane) = __pane {{ {state}.{field}.move_to_edge(__pane, ::iced::widget::pane_grid::Edge::{}); }} }}",
                        pane(name),
                        edge(side)
                    )
                    .unwrap(),
                    PaneOperation::Resize { ratio } => writeln!(
                        out,
                        "{{ let __split = {state}.{field}.layout().splits().next().copied(); if let ::std::option::Option::Some(__split) = __split {{ {state}.{field}.resize(__split, ({}) as f32); }} }}",
                        expr_code(ratio, env, document, ValueMode::Owned)?
                    )
                    .unwrap(),
                    PaneOperation::Drop {
                        pane: name,
                        target,
                        edge: side,
                    } => {
                        let region = side.as_ref().map_or_else(
                            || "::iced::widget::pane_grid::Region::Center".into(),
                            |side| {
                                format!(
                                    "::iced::widget::pane_grid::Region::Edge(::iced::widget::pane_grid::Edge::{})",
                                    edge(side)
                                )
                            },
                        );
                        writeln!(
                            out,
                            "{{ let __pane = {}; let __target = {}; if let (::std::option::Option::Some(__pane), ::std::option::Option::Some(__target)) = (__pane, __target) {{ {state}.{field}.drop(__pane, ::iced::widget::pane_grid::Target::Pane(__target, {region})); }} }}",
                            pane(name),
                            pane(target)
                        )
                        .unwrap();
                    }
                    PaneOperation::Split {
                        target,
                        pane: name,
                        axis: direction,
                        ratio,
                    } => writeln!(
                        out,
                        "{{ let __target = {}; let __pane = {}; if let (::std::option::Option::Some(__target), ::std::option::Option::None) = (__target, __pane) {{ if let ::std::option::Option::Some((_, __split)) = {state}.{field}.split(::iced::widget::pane_grid::Axis::{}, __target, {}) {{ {state}.{field}.resize(__split, ({}) as f32); }} }} }}",
                        pane(target),
                        pane(name),
                        axis(direction),
                        rust_string(name),
                        expr_code(ratio, env, document, ValueMode::Owned)?
                    )
                    .unwrap(),
                    PaneOperation::Maximized | PaneOperation::Adjacent { .. } => {
                        has_task = true;
                        let value = match operation {
                            PaneOperation::Maximized => format!(
                                "{state}.{field}.maximized().and_then(|__pane| {state}.{field}.get(__pane)).map(|__name| (*__name).to_owned())"
                            ),
                            PaneOperation::Adjacent { pane: name, edge: side } => {
                                let direction = match side {
                                    PaneEdge::Top => "Up",
                                    PaneEdge::Left => "Left",
                                    PaneEdge::Right => "Right",
                                    PaneEdge::Bottom => "Down",
                                };
                                format!(
                                    "{}.and_then(|__pane| {state}.{field}.adjacent(__pane, ::iced::widget::pane_grid::Direction::{direction})).and_then(|__pane| {state}.{field}.get(__pane)).map(|__name| (*__name).to_owned())",
                                    pane(name)
                                )
                            }
                            _ => unreachable!(),
                        };
                        let route = route.as_ref().expect("checker requires pane query route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        let task = format!(
                            "{{ let value = {value}; ::iced::Task::done({message_code}) }}"
                        );
                        writeln!(
                            out,
                            "{}{task}{}",
                            if return_task { "return " } else { "" },
                            if return_task { ";" } else { "" }
                        )
                        .unwrap();
                    }
                }
            }
            Statement::WindowOperation {
                operation,
                target,
                route,
                ..
            } => {
                has_task = true;
                let target = target
                    .as_ref()
                    .map(|target| expr_code(target, env, document, ValueMode::Owned))
                    .transpose()?;
                let id = target.as_deref().unwrap_or("__window");
                let value = |value: &Expr, cast: &str| {
                    Ok::<_, Error>(format!(
                        "({}) as {cast}",
                        expr_code(value, env, document, ValueMode::Owned)?
                    ))
                };
                let size = |width: &Expr, height: &Expr| {
                    Ok::<_, Error>(format!(
                        "::iced::Size::new({}, {})",
                        value(width, "f32")?,
                        value(height, "f32")?
                    ))
                };
                let optional_size = |size_value: &Option<(Expr, Expr)>| {
                    Ok::<_, Error>(match size_value {
                        Some((width, height)) => {
                            format!("::std::option::Option::Some({})", size(width, height)?)
                        }
                        None => "::std::option::Option::None".into(),
                    })
                };
                let bool_value = |value: &Expr| expr_code(value, env, document, ValueMode::Owned);
                let task = match operation {
                    WindowOperation::Open(name) => {
                        let settings = name.as_ref().map_or_else(
                            || "::std::default::Default::default()".into(),
                            |name| {
                                let index = document
                                    .settings
                                    .windows
                                    .iter()
                                    .position(|window| window.name == *name)
                                    .expect("checker validates named windows");
                                format!("Self::__window_{index}()")
                            },
                        );
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "{{ let (_, __task) = ::iced::window::open({settings}); __task.map(move |value| {message_code}) }}"
                        )
                    }
                    WindowOperation::Oldest | WindowOperation::Latest => {
                        let function = if matches!(operation, WindowOperation::Oldest) {
                            "oldest"
                        } else {
                            "latest"
                        };
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!("::iced::window::{function}().map(move |value| {message_code})")
                    }
                    WindowOperation::Close => {
                        format!("::iced::window::close::<{message}>({id})")
                    }
                    WindowOperation::Drag => {
                        format!("::iced::window::drag::<{message}>({id})")
                    }
                    WindowOperation::DragResize(direction) => {
                        let direction = match direction {
                            WindowDirection::North => "North",
                            WindowDirection::South => "South",
                            WindowDirection::East => "East",
                            WindowDirection::West => "West",
                            WindowDirection::NorthEast => "NorthEast",
                            WindowDirection::NorthWest => "NorthWest",
                            WindowDirection::SouthEast => "SouthEast",
                            WindowDirection::SouthWest => "SouthWest",
                        };
                        format!(
                            "::iced::window::drag_resize::<{message}>({id}, ::iced::window::Direction::{direction})"
                        )
                    }
                    WindowOperation::Resize(width, height) => format!(
                        "::iced::window::resize::<{message}>({id}, {})",
                        size(width, height)?
                    ),
                    WindowOperation::Resizable(enabled) => format!(
                        "::iced::window::set_resizable::<{message}>({id}, {})",
                        bool_value(enabled)?
                    ),
                    WindowOperation::MinSize(size) => format!(
                        "::iced::window::set_min_size::<{message}>({id}, {})",
                        optional_size(size)?
                    ),
                    WindowOperation::MaxSize(size) => format!(
                        "::iced::window::set_max_size::<{message}>({id}, {})",
                        optional_size(size)?
                    ),
                    WindowOperation::ResizeIncrements(size) => format!(
                        "::iced::window::set_resize_increments::<{message}>({id}, {})",
                        optional_size(size)?
                    ),
                    WindowOperation::Size => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = ordered_route_code(
                            route,
                            &["value.width as f64", "value.height as f64"],
                            env,
                            document,
                            message,
                        )?;
                        format!("::iced::window::size({id}).map(move |value| {message_code})")
                    }
                    WindowOperation::IsMaximized => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::window::is_maximized({id}).map(move |value| {message_code})"
                        )
                    }
                    WindowOperation::Maximize(enabled) => format!(
                        "::iced::window::maximize::<{message}>({id}, {})",
                        bool_value(enabled)?
                    ),
                    WindowOperation::IsMinimized => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::window::is_minimized({id}).map(move |value| {message_code})"
                        )
                    }
                    WindowOperation::Minimize(enabled) => format!(
                        "::iced::window::minimize::<{message}>({id}, {})",
                        bool_value(enabled)?
                    ),
                    WindowOperation::Position => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            ordered_route_code(route, &["x", "y"], env, document, message)?;
                        format!(
                            "::iced::window::position({id}).map(move |value| {{ let (x, y) = value.map_or((::std::option::Option::None, ::std::option::Option::None), |value| (::std::option::Option::Some(value.x as f64), ::std::option::Option::Some(value.y as f64))); {message_code} }})"
                        )
                    }
                    WindowOperation::ScaleFactor => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            route_code(route, "value as f64", env, document, message)?;
                        format!(
                            "::iced::window::scale_factor({id}).map(move |value| {message_code})"
                        )
                    }
                    WindowOperation::Move(x, y) => format!(
                        "::iced::window::move_to::<{message}>({id}, ::iced::Point::new({}, {}))",
                        value(x, "f32")?,
                        value(y, "f32")?
                    ),
                    WindowOperation::Mode => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::window::mode({id}).map(move |value| {{ let value = match value {{ ::iced::window::Mode::Windowed => \"windowed\", ::iced::window::Mode::Fullscreen => \"fullscreen\", ::iced::window::Mode::Hidden => \"hidden\" }}.to_owned(); {message_code} }})"
                        )
                    }
                    WindowOperation::SetMode(mode) => {
                        let mode = match mode {
                            WindowMode::Windowed => "Windowed",
                            WindowMode::Fullscreen => "Fullscreen",
                            WindowMode::Hidden => "Hidden",
                        };
                        format!(
                            "::iced::window::set_mode::<{message}>({id}, ::iced::window::Mode::{mode})"
                        )
                    }
                    WindowOperation::ToggleMaximize => {
                        format!("::iced::window::toggle_maximize::<{message}>({id})")
                    }
                    WindowOperation::ToggleDecorations => {
                        format!("::iced::window::toggle_decorations::<{message}>({id})")
                    }
                    WindowOperation::Attention(attention) => {
                        let attention: String = match attention {
                            None => "::std::option::Option::None".into(),
                            Some(WindowAttention::Critical) => "::std::option::Option::Some(::iced::window::UserAttention::Critical)".into(),
                            Some(WindowAttention::Informational) => "::std::option::Option::Some(::iced::window::UserAttention::Informational)".into(),
                        };
                        format!(
                            "::iced::window::request_user_attention::<{message}>({id}, {attention})"
                        )
                    }
                    WindowOperation::Focus => {
                        format!("::iced::window::gain_focus::<{message}>({id})")
                    }
                    WindowOperation::SetLevel(level) => {
                        let level = match level {
                            WindowLevel::Normal => "Normal",
                            WindowLevel::AlwaysOnBottom => "AlwaysOnBottom",
                            WindowLevel::AlwaysOnTop => "AlwaysOnTop",
                        };
                        format!(
                            "::iced::window::set_level::<{message}>({id}, ::iced::window::Level::{level})"
                        )
                    }
                    WindowOperation::SystemMenu => {
                        format!("::iced::window::show_system_menu::<{message}>({id})")
                    }
                    WindowOperation::RawId => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code =
                            route_code(route, "value.to_string()", env, document, message)?;
                        format!(
                            "::iced::window::raw_id::<{message}>({id}).map(move |value| {message_code})"
                        )
                    }
                    WindowOperation::Screenshot => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = ordered_route_code(
                            route,
                            &[
                                "value.rgba.to_vec()",
                                "value.size.width as i64",
                                "value.size.height as i64",
                                "value.scale_factor as f64",
                            ],
                            env,
                            document,
                            message,
                        )?;
                        format!("::iced::window::screenshot({id}).map(move |value| {message_code})")
                    }
                    WindowOperation::MousePassthrough(enabled) => {
                        let enabled = bool_value(enabled)?;
                        format!(
                            "if {enabled} {{ ::iced::window::enable_mouse_passthrough::<{message}>({id}) }} else {{ ::iced::window::disable_mouse_passthrough::<{message}>({id}) }}"
                        )
                    }
                    WindowOperation::MonitorSize => {
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = ordered_route_code(
                            route,
                            &["width", "height"],
                            env,
                            document,
                            message,
                        )?;
                        format!(
                            "::iced::window::monitor_size({id}).map(move |value| {{ let (width, height) = value.map_or((::std::option::Option::None, ::std::option::Option::None), |value| (::std::option::Option::Some(value.width as f64), ::std::option::Option::Some(value.height as f64))); {message_code} }})"
                        )
                    }
                    WindowOperation::AutomaticTabbing(enabled) => format!(
                        "::iced::window::allow_automatic_tabbing::<{message}>({})",
                        bool_value(enabled)?
                    ),
                    WindowOperation::Icon {
                        pixels,
                        width,
                        height,
                    } => {
                        let pixels = expr_code(pixels, env, document, ValueMode::Owned)?;
                        let width = expr_code(width, env, document, ValueMode::Owned)?;
                        let height = expr_code(height, env, document, ValueMode::Owned)?;
                        format!(
                            "{{ let __pixels = {pixels}; let __width = {width}; let __height = {height}; match (::std::primitive::u32::try_from(__width), ::std::primitive::u32::try_from(__height)) {{ (::std::result::Result::Ok(__width), ::std::result::Result::Ok(__height)) if __width > 0 && __height > 0 && __width.checked_mul(__height).is_some() => ::iced::window::icon::from_rgba(__pixels, __width, __height).map_or_else(|_| ::iced::Task::none(), |__icon| ::iced::window::set_icon::<{message}>({id}, __icon)), _ => ::iced::Task::none(), }} }}"
                        )
                    }
                    WindowOperation::Callback { function, args } => {
                        let callback = document
                            .functions
                            .iter()
                            .find(|item| item.name == *function && item.kind == ExternKind::Window)
                            .expect("checker validates window callback");
                        let args = args
                            .iter()
                            .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                            .collect::<Result<Vec<_>, _>>()?
                            .join(", ");
                        let args = if args.is_empty() {
                            String::new()
                        } else {
                            format!(", {args}")
                        };
                        let route = route.as_ref().expect("checker requires window route");
                        let message_code = route_code(route, "value", env, document, message)?;
                        format!(
                            "::iced::window::run({id}, move |__window| {}(__window{args})).map(move |value| {message_code})",
                            callback.rust_path
                        )
                    }
                };
                let task = if target.is_some()
                    || matches!(
                        operation,
                        WindowOperation::Open(_)
                            | WindowOperation::Oldest
                            | WindowOperation::Latest
                            | WindowOperation::AutomaticTabbing(_)
                    ) {
                    task
                } else {
                    format!("::iced::window::oldest().and_then(move |__window| {task})")
                };
                writeln!(
                    out,
                    "{}{task}{}",
                    if return_task { "return " } else { "" },
                    if return_task { ";" } else { "" }
                )
                .unwrap();
            }
        }
    }
    Ok(has_task)
}

fn render_node(
    node: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    match node {
        ViewNode::Layout {
            kind,
            options,
            id,
            styles,
            children,
            ..
        } => render_layout(
            *kind, options, id, styles, children, document, message, env, scope, slot,
        ),
        ViewNode::Container {
            options,
            id,
            styles,
            content,
            ..
        } => render_container(
            options, id, styles, content, document, message, env, scope, slot,
        ),
        ViewNode::Overlay {
            options,
            content,
            layer,
            ..
        } => render_overlay(options, content, layer, document, message, env, scope, slot),
        ViewNode::PaneGrid {
            name,
            options,
            panes,
            ..
        } => render_pane_grid(name, options, panes, document, message, env, scope, slot),
        ViewNode::Text {
            value,
            options,
            styles,
            ..
        } => {
            let style = Style::parse(styles, document);
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            let mut code = format!("::iced::widget::text({value})");
            append_text_options(&mut code, options, &style, env, document)?;
            if let Some(color) = style.text_color {
                write!(code, ".color({})", theme_color(document, &color)).unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::RichText {
            options,
            color,
            spans,
            styles,
            route,
            ..
        } => render_rich_text(options, color, spans, styles, route, document, message, env),
        ViewNode::Input {
            label,
            id,
            binding,
            hint,
            disabled,
            options,
            styles,
            span,
        } => {
            let style = Style::parse(styles, document);
            let state = env.get(binding).ok_or_else(|| {
                Error::new("E150", span, format!("unknown input state `{binding}`"))
            })?;
            let state_name = controlled_state_name(&state.code, "input", span)?;
            let variant = binding_variant(&state_name);
            let mut input = format!(
                "::iced::widget::text_input({}, &{})",
                rust_string(hint),
                state.code
            );
            if let Some(id) = id {
                write!(
                    input,
                    ".id(::iced::widget::Id::from({}))",
                    id_code(id, scope, env, document)?
                )
                .unwrap();
            }
            if let Some(padding) = style.padding_code() {
                write!(input, ".padding({padding})").unwrap();
            }
            if style.width_fill {
                input.push_str(".width(::iced::Fill)");
            }
            if let Some(secure) = &options.secure {
                write!(
                    input,
                    ".secure({})",
                    expr_code(secure, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(width) = &options.width {
                write!(input, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(padding) = &options.padding {
                write!(
                    input,
                    ".padding({} as f32)",
                    expr_code(padding, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(size) = &options.text_size {
                write!(
                    input,
                    ".size({} as f32)",
                    expr_code(size, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(height) = &options.line_height {
                write!(
                    input,
                    ".line_height(::iced::widget::text::LineHeight::Relative({} as f32))",
                    expr_code(height, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(align) = options.align {
                let align = match align {
                    InputAlignment::Left => "Left",
                    InputAlignment::Center => "Center",
                    InputAlignment::Right => "Right",
                };
                write!(input, ".align_x(::iced::alignment::Horizontal::{align})").unwrap();
            }
            if let Some(font) = &options.font {
                write!(input, ".font({})", font_preset_code(font, document)?).unwrap();
            }
            if let Some(icon) = &options.icon {
                write!(
                    input,
                    ".icon({})",
                    text_input_icon_code(icon, env, document)?
                )
                .unwrap();
            }
            let constructor =
                format!("{message}::{variant} as fn(::std::string::String) -> {message}");
            if let Some(disabled) = disabled {
                let disabled = expr_code(disabled, env, document, ValueMode::Owned)?;
                write!(
                    input,
                    ".on_input_maybe(if {disabled} {{ None }} else {{ Some({constructor}) }})"
                )
                .unwrap();
            } else {
                write!(input, ".on_input({constructor})").unwrap();
            }
            if let Some(route) = &options.submit {
                let submit = route_code(route, "", env, document, message)?;
                if let Some(disabled) = disabled {
                    write!(
                        input,
                        ".on_submit_maybe(if {} {{ None }} else {{ Some({submit}) }})",
                        expr_code(disabled, env, document, ValueMode::Owned)?
                    )
                    .unwrap();
                } else {
                    write!(input, ".on_submit({submit})").unwrap();
                }
            }
            if let Some(route) = &options.paste {
                let paste = route_code(route, "__value", env, document, message)?;
                if let Some(disabled) = disabled {
                    write!(
                        input,
                        ".on_paste_maybe(if {} {{ None }} else {{ Some(move |__value| {paste}) }})",
                        expr_code(disabled, env, document, ValueMode::Owned)?
                    )
                    .unwrap();
                } else {
                    write!(input, ".on_paste(move |__value| {paste})").unwrap();
                }
            }
            input.push_str(&text_input_style_code(
                &options.style,
                options.custom_style.as_ref(),
                Some(&style),
                env,
                document,
                "style",
                "text_input",
            )?);
            Ok(format!(
                "::iced::widget::column![::iced::widget::text({}), {input}].spacing(6).into()",
                rust_string(label)
            ))
        }
        ViewNode::Button {
            label,
            content,
            id,
            disabled,
            options,
            styles,
            route,
            ..
        } => {
            let style = Style::parse(styles, document);
            let message_code = route_code(route, "", env, document, message)?;
            let content = if let Some(content) = content {
                let child_scope = id.as_ref().map_or_else(
                    || Ok(scope.to_owned()),
                    |id| id_code(id, scope, env, document),
                )?;
                render_node(content, document, message, env, &child_scope, slot)?
            } else {
                format!(
                    "::iced::widget::text({}).into()",
                    rust_string(label.as_ref().expect("button label"))
                )
            };
            let mut code = format!(
                "{{ let __button_content: ::iced::Element<'_, {message}> = {content}; ::iced::widget::button(__button_content)"
            );
            if let Some(padding) = style.padding_code() {
                write!(code, ".padding({padding})").unwrap();
            }
            if let Some(width) = &options.width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = &options.height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            if let Some(padding) = &options.padding {
                write!(
                    code,
                    ".padding({} as f32)",
                    expr_code(padding, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(clip) = &options.clip {
                write!(
                    code,
                    ".clip({})",
                    expr_code(clip, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(disabled) = disabled {
                let disabled = expr_code(disabled, env, document, ValueMode::Owned)?;
                write!(
                    code,
                    ".on_press_maybe(if {disabled} {{ None }} else {{ Some({message_code}) }})"
                )
                .unwrap();
            } else {
                write!(code, ".on_press({message_code})").unwrap();
            }
            code.push_str(&button_style_code(&style, &options.style, env, document)?);
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Checkbox {
            label,
            checked,
            disabled,
            options,
            style,
            route,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let checked = expr_code(checked, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let mut code = format!("::iced::widget::checkbox({checked}).label({label})");
            append_bool_control_options(&mut code, options, env, document, false)?;
            if let Some(disabled) = disabled {
                let disabled = expr_code(disabled, env, document, ValueMode::Owned)?;
                write!(
                    code,
                    ".on_toggle_maybe(if {disabled} {{ None }} else {{ Some(move |__value| {message_code}) }})"
                )
                .unwrap();
            } else {
                write!(code, ".on_toggle(move |__value| {message_code})").unwrap();
            }
            code.push_str(&checkbox_style_code(style, env, document)?);
            Ok(format!("{code}.into()"))
        }
        ViewNode::Toggler {
            label,
            checked,
            disabled,
            options,
            style,
            route,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let checked = expr_code(checked, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let mut code = format!("::iced::widget::toggler({checked}).label({label})");
            append_bool_control_options(&mut code, options, env, document, true)?;
            if let Some(disabled) = disabled {
                let disabled = expr_code(disabled, env, document, ValueMode::Owned)?;
                write!(code, ".on_toggle_maybe(if {disabled} {{ None }} else {{ Some(move |__value| {message_code}) }})").unwrap();
            } else {
                write!(code, ".on_toggle(move |__value| {message_code})").unwrap();
            }
            code.push_str(&toggler_style_code(style, env, document)?);
            Ok(format!("{code}.into()"))
        }
        ViewNode::Slider {
            value,
            min,
            max,
            step,
            options,
            vertical,
            route,
            release,
            ..
        } => {
            let value = expr_code(value, env, document, ValueMode::Borrowed)?;
            let min = expr_code(min, env, document, ValueMode::Borrowed)?;
            let max = expr_code(max, env, document, ValueMode::Borrowed)?;
            let step = expr_code(step, env, document, ValueMode::Borrowed)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let helper = if *vertical {
                "vertical_slider"
            } else {
                "slider"
            };
            let mut code = format!(
                "::iced::widget::{helper}(({min})..=({max}), {value}, move |__value| {message_code}).step({step})"
            );
            if let Some(default) = &options.default {
                write!(
                    code,
                    ".default({})",
                    expr_code(default, env, document, ValueMode::Borrowed)?
                )
                .unwrap();
            }
            if let Some(shift_step) = &options.shift_step {
                write!(
                    code,
                    ".shift_step({})",
                    expr_code(shift_step, env, document, ValueMode::Borrowed)?
                )
                .unwrap();
            }
            for (length, method) in [(&options.width, "width"), (&options.height, "height")] {
                if let Some(length) = length {
                    write!(code, ".{method}({})", length_code(length, env, document)?).unwrap();
                }
            }
            append_slider_styles(&mut code, &options.style, env, document)?;
            if let Some(release) = release {
                write!(
                    code,
                    ".on_release({})",
                    route_code(release, "", env, document, message)?
                )
                .unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Progress {
            value,
            min,
            max,
            options,
            vertical,
            ..
        } => {
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            let min = expr_code(min, env, document, ValueMode::Owned)?;
            let max = expr_code(max, env, document, ValueMode::Owned)?;
            let mut code = format!(
                "::iced::widget::progress_bar(({min} as f32)..=({max} as f32), {value} as f32)"
            );
            if let Some(length) = &options.length {
                write!(code, ".length({})", length_code(length, env, document)?).unwrap();
            }
            if let Some(girth) = &options.girth {
                write!(code, ".girth({})", length_code(girth, env, document)?).unwrap();
            }
            if *vertical {
                code.push_str(".vertical()");
            }
            append_progress_options(&mut code, options, env, document)?;
            Ok(format!("{code}.into()"))
        }
        ViewNode::Radio {
            label,
            value,
            selected,
            options,
            style,
            route,
            ..
        } => {
            let label = expr_code(label, env, document, ValueMode::Owned)?;
            let value = expr_code(value, env, document, ValueMode::Owned)?;
            let selected = expr_code(selected, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, &value, env, document, message)?;
            let mut code = format!(
                "::iced::widget::radio({label}, true, if {selected} {{ Some(true) }} else {{ None }}, move |_| {message_code})"
            );
            append_bool_control_options(&mut code, options, env, document, false)?;
            code.push_str(&radio_style_code(style, env, document)?);
            Ok(format!("{code}.into()"))
        }
        ViewNode::PickList {
            options,
            selected,
            options_config,
            route,
            ..
        } => {
            let options = expr_code(options, env, document, ValueMode::Owned)?;
            let selected = expr_code(selected, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let mut code = format!(
                "::iced::widget::pick_list({options}, {selected}, move |__value| {message_code})"
            );
            if let Some(placeholder) = &options_config.placeholder {
                write!(
                    code,
                    ".placeholder({})",
                    expr_code(placeholder, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(width) = &options_config.width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = &options_config.menu_height {
                write!(
                    code,
                    ".menu_height({})",
                    length_code(height, env, document)?
                )
                .unwrap();
            }
            if let Some(padding) = &options_config.padding {
                write!(
                    code,
                    ".padding({} as f32)",
                    expr_code(padding, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(size) = &options_config.text_size {
                write!(
                    code,
                    ".text_size({} as f32)",
                    expr_code(size, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(height) = &options_config.line_height {
                write!(
                    code,
                    ".text_line_height(::iced::widget::text::LineHeight::Relative({} as f32))",
                    expr_code(height, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(shaping) = options_config.shaping {
                write!(
                    code,
                    ".text_shaping(::iced::widget::text::Shaping::{})",
                    text_shaping_code(shaping)
                )
                .unwrap();
            }
            if let Some(font) = &options_config.font {
                write!(code, ".font({})", font_preset_code(font, document)?).unwrap();
            }
            if let Some(handle) = &options_config.handle {
                write!(
                    code,
                    ".handle({})",
                    pick_list_handle_code(handle, env, document)?
                )
                .unwrap();
            }
            if let Some(route) = &options_config.open {
                write!(
                    code,
                    ".on_open({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options_config.close {
                write!(
                    code,
                    ".on_close({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            code.push_str(&pick_list_style_code(options_config, env, document)?);
            Ok(format!("{code}.into()"))
        }
        ViewNode::ComboBox {
            state,
            selected,
            placeholder,
            options,
            route,
            span,
        } => {
            let state = env.get(state).ok_or_else(|| {
                Error::new("E150", span, format!("unknown combo state `{state}`"))
            })?;
            let selected = expr_code(selected, env, document, ValueMode::Owned)?;
            let message_code = route_code(route, "__value", env, document, message)?;
            let mut code = format!(
                "{{ let __combo_selection = {selected}; ::iced::widget::combo_box(&{}, {}, __combo_selection.as_ref(), move |__value| {message_code})",
                state.code,
                rust_string(placeholder)
            );
            if let Some(width) = &options.width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = &options.menu_height {
                write!(
                    code,
                    ".menu_height({})",
                    length_code(height, env, document)?
                )
                .unwrap();
            }
            if let Some(padding) = &options.padding {
                write!(
                    code,
                    ".padding({} as f32)",
                    expr_code(padding, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(size) = &options.text_size {
                write!(
                    code,
                    ".size({} as f32)",
                    expr_code(size, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(height) = &options.line_height {
                write!(
                    code,
                    ".line_height(::iced::widget::text::LineHeight::Relative({} as f32))",
                    expr_code(height, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(shaping) = options.shaping {
                write!(
                    code,
                    ".text_shaping(::iced::widget::text::Shaping::{})",
                    text_shaping_code(shaping)
                )
                .unwrap();
            }
            if let Some(font) = &options.font {
                write!(code, ".font({})", font_preset_code(font, document)?).unwrap();
            }
            if let Some(icon) = &options.icon {
                write!(
                    code,
                    ".icon({})",
                    text_input_icon_code(icon, env, document)?
                )
                .unwrap();
            }
            if let Some(route) = &options.input {
                write!(
                    code,
                    ".on_input(move |__value| {})",
                    route_code(route, "__value", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options.hover {
                write!(
                    code,
                    ".on_option_hovered(move |__value| {})",
                    route_code(route, "__value", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options.open {
                write!(
                    code,
                    ".on_open({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options.close {
                write!(
                    code,
                    ".on_close({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            code.push_str(&text_input_style_code(
                &options.style,
                options.custom_style.as_ref(),
                None,
                env,
                document,
                "input_style",
                "text_input",
            )?);
            code.push_str(&menu_style_code(
                options.menu_style.as_deref(),
                options.custom_menu_style.as_ref(),
                env,
                document,
            )?);
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Rule {
            axis,
            thickness,
            options,
            ..
        } => {
            let thickness = expr_code(thickness, env, document, ValueMode::Owned)?;
            let axis = match axis {
                Axis::Horizontal => "horizontal",
                Axis::Vertical => "vertical",
            };
            let mut code = format!("::iced::widget::rule::{axis}({thickness} as f32)");
            append_rule_options(&mut code, options, env, document)?;
            Ok(format!("{code}.into()"))
        }
        ViewNode::QrCode {
            data,
            cell_size,
            total_size,
            cell,
            background,
            ..
        } => {
            let mut code = format!("::iced::widget::qr_code(&self.{data})");
            if let Some(value) = cell_size {
                write!(
                    code,
                    ".cell_size({} as f32)",
                    expr_code(value, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(value) = total_size {
                write!(
                    code,
                    ".total_size({} as f32)",
                    expr_code(value, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if cell.is_some() || background.is_some() {
                let cell = cell.as_deref().map(|value| theme_color(document, value));
                let background = background
                    .as_deref()
                    .map(|value| theme_color(document, value));
                write!(
                    code,
                    ".style(|theme| {{ let default = ::iced::widget::qr_code::default(theme); ::iced::widget::qr_code::Style {{ cell: {}, background: {} }} }})",
                    cell.unwrap_or_else(|| "default.cell".into()),
                    background.unwrap_or_else(|| "default.background".into())
                )
                .unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Space { width, height, .. } => {
            let mut code = String::from("::iced::widget::space()");
            if let Some(width) = width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Component {
            name,
            args,
            id,
            slots,
            span,
        } => {
            let component = document
                .components
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| Error::new("E122", span, format!("unknown component `{name}`")))?;
            let mut component_env = HashMap::new();
            for (index, (param, ty)) in component.params.iter().enumerate() {
                let arg = if args.iter().any(|arg| arg.name.is_some()) {
                    args.iter()
                        .find(|arg| arg.name.as_ref() == Some(param))
                        .expect("checker requires every named component prop")
                } else {
                    &args[index]
                };
                component_env.insert(
                    param.clone(),
                    Binding {
                        code: expr_code(&arg.value, env, document, ValueMode::Borrowed)?,
                        ty: ty.clone(),
                        local: false,
                    },
                );
            }
            let component_scope = id.as_ref().map_or_else(
                || format!("format!(\"{{}}/{}\", {scope})", name),
                |id| id_code(id, scope, env, document).unwrap_or_else(|_| scope.into()),
            );
            let component_slots = (!slots.is_empty()).then(|| SlotContext {
                entries: slots
                    .iter()
                    .map(|component_slot| SlotContent {
                        name: component_slot.name.clone(),
                        node: (*component_slot.content).clone(),
                        env: env.clone(),
                    })
                    .collect(),
                parent: slot.cloned().map(Box::new),
            });
            render_node(
                &component.root,
                document,
                message,
                &component_env,
                &component_scope,
                component_slots.as_ref(),
            )
        }
        ViewNode::Slot { name, span } => {
            let slot = slot.ok_or_else(|| {
                Error::new(
                    "E170",
                    span,
                    "slot reached codegen without component content",
                )
            })?;
            let content = slot
                .entries
                .iter()
                .find(|entry| entry.name == *name)
                .ok_or_else(|| {
                    Error::new(
                        "E170",
                        span,
                        format!("slot `{name}` reached codegen without component content"),
                    )
                })?;
            render_node(
                &content.node,
                document,
                message,
                &content.env,
                scope,
                slot.parent.as_deref(),
            )
        }
        ViewNode::ExternComponent {
            function,
            args,
            route,
            span,
        } => {
            let component = document
                .functions
                .iter()
                .find(|item| item.name == *function && item.kind == ExternKind::Component)
                .ok_or_else(|| {
                    Error::new(
                        "E130",
                        span,
                        format!("unknown extern component `{function}`"),
                    )
                })?;
            let args = args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            let mapped = if let Some(route) = route {
                route_code(route, "__value", env, document, message)?
            } else {
                format!("{message}::__ExternNoop")
            };
            Ok(format!(
                "{}({args}).map(move |__value| {mapped}).into()",
                component.rust_path
            ))
        }
        ViewNode::Shader {
            function,
            args,
            width,
            height,
            route,
            span,
        } => {
            let shader = document
                .functions
                .iter()
                .find(|item| item.name == *function && item.kind == ExternKind::Shader)
                .ok_or_else(|| Error::new("E191", span, format!("unknown shader `{function}`")))?;
            let args = args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            let mut code = format!("::iced::widget::Shader::new({}({args}))", shader.rust_path);
            if let Some(width) = width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            let output = shader.output.rust(&document.structs);
            let mapped = if let Some(route) = route {
                route_code(route, "__value", env, document, message)?
            } else {
                format!("{message}::__ExternNoop")
            };
            Ok(format!(
                "{{ let __shader: ::iced::Element<'_, {output}> = {code}.into(); __shader.map(move |__value| {mapped}).into() }}"
            ))
        }
        ViewNode::Media {
            kind,
            source,
            options,
            span,
        } => {
            let source_type = expr_type(
                source,
                &env.iter()
                    .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                    .collect(),
                document,
                span,
            )?;
            let source = expr_code(source, env, document, ValueMode::Owned)?;
            let mut code = match kind {
                MediaKind::Image => format!("::iced::widget::image({source})"),
                MediaKind::Viewer if source_type == Type::Str => format!(
                    "::iced::widget::image::viewer(::iced::widget::image::Handle::from_path({source}))"
                ),
                MediaKind::Viewer => format!("::iced::widget::image::viewer({source})"),
                MediaKind::Svg if options.svg_memory && source_type == Type::Bytes => format!(
                    "::iced::widget::svg(::iced::widget::svg::Handle::from_memory({source}))"
                ),
                MediaKind::Svg if options.svg_memory => format!(
                    "::iced::widget::svg(::iced::widget::svg::Handle::from_memory(({source}).into_bytes()))"
                ),
                MediaKind::Svg => format!("::iced::widget::svg({source})"),
            };
            if let Some(width) = &options.width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = &options.height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            if let Some(fit) = options.fit {
                let fit = match fit {
                    ContentFit::Contain => "Contain",
                    ContentFit::Cover => "Cover",
                    ContentFit::Fill => "Fill",
                    ContentFit::None => "None",
                    ContentFit::ScaleDown => "ScaleDown",
                };
                write!(code, ".content_fit(::iced::ContentFit::{fit})").unwrap();
            }
            if let Some(rotation) = &options.rotation {
                let rotation = expr_code(rotation, env, document, ValueMode::Owned)?;
                write!(
                    code,
                    ".rotation({})",
                    if options.rotation_solid {
                        format!("::iced::Rotation::Solid(::iced::Radians({rotation} as f32))")
                    } else {
                        format!("{rotation} as f32")
                    }
                )
                .unwrap();
            }
            if let Some(opacity) = &options.opacity {
                write!(
                    code,
                    ".opacity({} as f32)",
                    expr_code(opacity, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if *kind == MediaKind::Svg {
                let custom = options
                    .svg_style
                    .as_ref()
                    .map(|style| {
                        let function = document
                            .functions
                            .iter()
                            .find(|item| {
                                item.name == style.function && item.kind == ExternKind::SvgStyle
                            })
                            .expect("checker validates svg style");
                        let args = style
                            .args
                            .iter()
                            .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                            .collect::<Result<Vec<_>, _>>()?;
                        Ok::<_, Error>(format!(
                            "{}(__theme, __status{})",
                            function.rust_path,
                            args.iter()
                                .map(|arg| format!(", {arg}"))
                                .collect::<String>()
                        ))
                    })
                    .transpose()?;
                let has_colors = options.svg_color.is_some() || options.svg_hover_color.is_some();
                if !has_colors {
                    if let Some(custom) = custom {
                        write!(code, ".style(move |__theme, __status| {custom})").unwrap();
                    }
                } else {
                    let base = custom
                        .unwrap_or_else(|| "::iced::widget::svg::Style::default()".to_owned());
                    let idle = options
                        .svg_color
                        .as_ref()
                        .map(|color| format!("Some({})", theme_color(document, color)));
                    let hovered = match &options.svg_hover_color {
                        Some(Some(color)) => {
                            Some(format!("Some({})", theme_color(document, color)))
                        }
                        Some(None) => Some("None".to_owned()),
                        None => idle.clone(),
                    };
                    write!(
                        code,
                        ".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{"
                    )
                    .unwrap();
                    if let Some(idle) = idle {
                        write!(
                            code,
                            " ::iced::widget::svg::Status::Idle => __style.color = {idle},"
                        )
                        .unwrap();
                    }
                    if let Some(hovered) = hovered {
                        write!(
                            code,
                            " ::iced::widget::svg::Status::Hovered => __style.color = {hovered},"
                        )
                        .unwrap();
                    }
                    code.push_str(" _ => {} } __style })");
                }
            }
            if let Some(filter) = options.filter {
                let filter = match filter {
                    ImageFilter::Linear => "Linear",
                    ImageFilter::Nearest => "Nearest",
                };
                write!(
                    code,
                    ".filter_method(::iced::widget::image::FilterMethod::{filter})"
                )
                .unwrap();
            }
            for (value, method) in [
                (&options.padding, "padding"),
                (&options.min_scale, "min_scale"),
                (&options.max_scale, "max_scale"),
                (&options.scale_step, "scale_step"),
            ] {
                if let Some(value) = value {
                    write!(
                        code,
                        ".{method}({} as f32)",
                        expr_code(value, env, document, ValueMode::Owned)?
                    )
                    .unwrap();
                }
            }
            if let Some(scale) = &options.scale {
                write!(
                    code,
                    ".scale({} as f32)",
                    expr_code(scale, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(expand) = &options.expand {
                write!(
                    code,
                    ".expand({})",
                    expr_code(expand, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(radius) = radius_code(
                options.radius.as_ref(),
                [
                    options.radius_top_left.as_ref(),
                    options.radius_top_right.as_ref(),
                    options.radius_bottom_right.as_ref(),
                    options.radius_bottom_left.as_ref(),
                ],
                env,
                document,
            )? {
                write!(code, ".border_radius({radius})").unwrap();
            }
            if let Some([x, y, width, height]) = &options.crop {
                write!(
                    code,
                    ".crop(::iced::Rectangle {{ x: {}, y: {}, width: {}, height: {} }})",
                    u32_code(x, env, document)?,
                    u32_code(y, env, document)?,
                    u32_code(width, env, document)?,
                    u32_code(height, env, document)?,
                )
                .unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::Tooltip {
            options,
            content,
            tip,
            ..
        } => {
            let content = render_node(content, document, message, env, scope, slot)?;
            let tip = render_node(tip, document, message, env, scope, slot)?;
            let position = match options.position {
                TooltipPosition::Top => "Top",
                TooltipPosition::Bottom => "Bottom",
                TooltipPosition::Left => "Left",
                TooltipPosition::Right => "Right",
                TooltipPosition::FollowCursor => "FollowCursor",
            };
            let gap = expr_code(&options.gap, env, document, ValueMode::Owned)?;
            let padding = expr_code(&options.padding, env, document, ValueMode::Owned)?;
            let delay = expr_code(&options.delay_ms, env, document, ValueMode::Owned)?;
            let snap = expr_code(&options.snap, env, document, ValueMode::Owned)?;
            let mut code = format!(
                "{{ let __tooltip_content: ::iced::Element<'_, {message}> = {content}; let __tooltip_tip: ::iced::Element<'_, {message}> = {tip}; ::iced::widget::tooltip(__tooltip_content, __tooltip_tip, ::iced::widget::tooltip::Position::{position}).gap({gap} as f32).padding({padding} as f32).delay(::std::time::Duration::from_millis({delay} as u64)).snap_within_viewport({snap})"
            );
            append_tooltip_style(&mut code, options, env, document)?;
            code.push_str(".into() }");
            Ok(code)
        }
        ViewNode::MouseArea {
            options, content, ..
        } => {
            let content = render_node(content, document, message, env, scope, slot)?;
            let mut code = format!(
                "{{ let __mouse_content: ::iced::Element<'_, {message}> = {content}; ::iced::widget::mouse_area(__mouse_content)"
            );
            for (route, method) in [
                (&options.press, "on_press"),
                (&options.release, "on_release"),
                (&options.double_click, "on_double_click"),
                (&options.right_press, "on_right_press"),
                (&options.right_release, "on_right_release"),
                (&options.middle_press, "on_middle_press"),
                (&options.middle_release, "on_middle_release"),
                (&options.enter, "on_enter"),
                (&options.exit, "on_exit"),
            ] {
                if let Some(route) = route {
                    write!(
                        code,
                        ".{method}({})",
                        route_code(route, "", env, document, message)?
                    )
                    .unwrap();
                }
            }
            if let Some(route) = &options.move_route {
                write!(
                    code,
                    ".on_move(move |__point| {})",
                    ordered_route_code(
                        route,
                        &["__point.x as f64", "__point.y as f64"],
                        env,
                        document,
                        message,
                    )?
                )
                .unwrap();
            }
            if let Some(route) = &options.scroll {
                let lines = ordered_route_code(
                    route,
                    &["__x as f64", "__y as f64", "false"],
                    env,
                    document,
                    message,
                )?;
                let pixels = ordered_route_code(
                    route,
                    &["__x as f64", "__y as f64", "true"],
                    env,
                    document,
                    message,
                )?;
                write!(
                    code,
                    ".on_scroll(move |__delta| match __delta {{ ::iced::mouse::ScrollDelta::Lines {{ x: __x, y: __y }} => {lines}, ::iced::mouse::ScrollDelta::Pixels {{ x: __x, y: __y }} => {pixels} }})"
                )
                .unwrap();
            }
            if let Some(interaction) = options.interaction {
                write!(
                    code,
                    ".interaction(::iced::mouse::Interaction::{})",
                    mouse_interaction_code(interaction)
                )
                .unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Canvas {
            options,
            locals,
            commands,
            events,
            ..
        } => render_canvas(options, locals, commands, events, document, message, env),
        ViewNode::Theme {
            preset,
            text,
            background,
            content,
            ..
        } => {
            let content = render_node(content, document, message, env, scope, slot)?;
            let mut code = format!(
                "{{ let __theme_content: ::iced::Element<'_, {message}> = {content}; ::iced::widget::themer({}, __theme_content)",
                theme_preset_code(preset)
            );
            if let Some(color) = text {
                write!(code, ".text_color(|_| {})", theme_color(document, color)).unwrap();
            }
            if let Some(background) = background {
                write!(
                    code,
                    ".background(|_| {})",
                    background_code(background, env, document)?
                )
                .unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Float {
            scale,
            x,
            y,
            style,
            content,
            ..
        } => {
            let content = render_node(content, document, message, env, scope, slot)?;
            let scale = expr_code(scale, env, document, ValueMode::Owned)?;
            let mut translate_env = env.clone();
            for (name, code) in [
                ("original_x", "(__original.x as f64)"),
                ("original_y", "(__original.y as f64)"),
                ("original_width", "(__original.width as f64)"),
                ("original_height", "(__original.height as f64)"),
                ("viewport_x", "(__viewport.x as f64)"),
                ("viewport_y", "(__viewport.y as f64)"),
                ("viewport_width", "(__viewport.width as f64)"),
                ("viewport_height", "(__viewport.height as f64)"),
            ] {
                translate_env.insert(
                    name.to_owned(),
                    Binding {
                        code: code.to_owned(),
                        ty: Type::F64,
                        local: true,
                    },
                );
            }
            let x = expr_code(x, &translate_env, document, ValueMode::Owned)?;
            let y = expr_code(y, &translate_env, document, ValueMode::Owned)?;
            let mut code = format!(
                "{{ let __float_content: ::iced::Element<'_, {message}> = {content}; let __float = ::iced::widget::float(__float_content).scale({scale} as f32).translate(move |__original, __viewport| ::iced::Vector::new({x} as f32, {y} as f32))"
            );
            append_float_style(&mut code, style, env, document)?;
            Ok(format!("{code}; __float.into() }}"))
        }
        ViewNode::Pin {
            width,
            height,
            x,
            y,
            content,
            ..
        } => {
            let content = render_node(content, document, message, env, scope, slot)?;
            let x = expr_code(x, env, document, ValueMode::Owned)?;
            let y = expr_code(y, env, document, ValueMode::Owned)?;
            let mut code = format!(
                "{{ let __pin_content: ::iced::Element<'_, {message}> = {content}; ::iced::widget::pin(__pin_content).x({x} as f32).y({y} as f32)"
            );
            if let Some(width) = width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Sensor {
            options, content, ..
        } => {
            let content = render_node(content, document, message, env, scope, slot)?;
            let mut code = format!(
                "{{ let __sensor_content: ::iced::Element<'_, {message}> = {content}; ::iced::widget::sensor(__sensor_content)"
            );
            if let Some(route) = &options.show {
                write!(
                    code,
                    ".on_show(move |__size| {})",
                    size_route_code(route, "__size", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options.resize {
                write!(
                    code,
                    ".on_resize(move |__size| {})",
                    size_route_code(route, "__size", env, document, message)?
                )
                .unwrap();
            }
            if let Some(route) = &options.hide {
                write!(
                    code,
                    ".on_hide({})",
                    route_code(route, "", env, document, message)?
                )
                .unwrap();
            }
            if let Some(key) = &options.key {
                write!(
                    code,
                    ".key({})",
                    expr_code(key, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(distance) = &options.anticipate {
                write!(
                    code,
                    ".anticipate({} as f32)",
                    expr_code(distance, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(delay) = &options.delay_ms {
                write!(
                    code,
                    ".delay(::std::time::Duration::from_millis({} as u64))",
                    expr_code(delay, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            Ok(format!("{code}.into() }}"))
        }
        ViewNode::Responsive {
            content,
            width,
            height,
            ..
        } => {
            let builder = match content {
                ResponsiveContent::Breakpoint {
                    breakpoint,
                    narrow,
                    wide,
                } => {
                    let breakpoint = expr_code(breakpoint, env, document, ValueMode::Owned)?;
                    let narrow = render_node(narrow, document, message, env, scope, slot)?;
                    let wide = render_node(wide, document, message, env, scope, slot)?;
                    format!(
                        "move |__size| {{ let __responsive: ::iced::Element<'_, {message}> = if __size.width < {breakpoint} as f32 {{ {narrow} }} else {{ {wide} }}; __responsive }}"
                    )
                }
                ResponsiveContent::Size {
                    width,
                    height,
                    content,
                } => {
                    let mut child_env = env.clone();
                    child_env.insert(
                        width.clone(),
                        Binding {
                            code: "(__size.width as f64)".into(),
                            ty: Type::F64,
                            local: true,
                        },
                    );
                    child_env.insert(
                        height.clone(),
                        Binding {
                            code: "(__size.height as f64)".into(),
                            ty: Type::F64,
                            local: true,
                        },
                    );
                    let content = render_node(content, document, message, &child_env, scope, slot)?;
                    format!(
                        "move |__size| {{ let __responsive: ::iced::Element<'_, {message}> = {content}; __responsive }}"
                    )
                }
            };
            let mut code = format!("::iced::widget::responsive({builder})");
            if let Some(width) = width {
                write!(code, ".width({})", length_code(width, env, document)?).unwrap();
            }
            if let Some(height) = height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            Ok(format!("{code}.into()"))
        }
        ViewNode::KeyedColumn {
            item,
            items,
            key,
            options,
            child,
            span,
        } => render_keyed_column(
            item, items, key, options, child, span, document, message, env, scope, slot,
        ),
        ViewNode::Lazy {
            dependency,
            binding,
            child,
            span,
        } => {
            let dependency_type = expr_type(
                dependency,
                &env.iter()
                    .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                    .collect(),
                document,
                span,
            )?;
            let dependency = expr_code(dependency, env, document, ValueMode::Owned)?;
            let mut child_env = HashMap::new();
            child_env.insert(
                binding.clone(),
                Binding {
                    code: binding.clone(),
                    ty: dependency_type.clone(),
                    local: false,
                },
            );
            let child = render_node(
                child,
                document,
                message,
                &child_env,
                "__lazy_scope.clone()",
                None,
            )?;
            let dependency_rust = dependency_type.rust(&document.structs);
            Ok(format!(
                "::iced::widget::lazy(({dependency}, ({scope}).to_owned()), move |__dependency| {{ let {binding}: {dependency_rust} = __dependency.0.clone(); let __lazy_scope = __dependency.1.clone(); let __lazy_content: ::iced::Element<'static, {message}> = {child}; __lazy_content }}).into()"
            ))
        }
        ViewNode::Markdown {
            content,
            options,
            route,
            ..
        } => {
            let mut settings = String::from(
                "let mut __markdown_settings = ::iced::widget::markdown::Settings::from(self.__theme());",
            );
            for (value, field) in [
                (&options.text_size, "text_size"),
                (&options.h1_size, "h1_size"),
                (&options.h2_size, "h2_size"),
                (&options.h3_size, "h3_size"),
                (&options.h4_size, "h4_size"),
                (&options.h5_size, "h5_size"),
                (&options.h6_size, "h6_size"),
                (&options.code_size, "code_size"),
                (&options.spacing, "spacing"),
            ] {
                if let Some(value) = value {
                    write!(
                        settings,
                        " __markdown_settings.{field} = ({} as f32).into();",
                        expr_code(value, env, document, ValueMode::Owned)?
                    )
                    .unwrap();
                }
            }
            let style = &options.style;
            if let Some(font) = &style.font {
                write!(
                    settings,
                    " __markdown_settings.style.font = {};",
                    font_preset_code(font, document)?
                )
                .unwrap();
            }
            if let Some(background) = &style.inline_code_background {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_highlight.background = {};",
                    background_code(background, env, document)?
                )
                .unwrap();
            }
            if let Some(color) = &style.inline_code_color {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_color = {};",
                    theme_color(document, color)
                )
                .unwrap();
            }
            if let Some(font) = &style.inline_code_font {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_font = {};",
                    font_preset_code(font, document)?
                )
                .unwrap();
            }
            if let Some(font) = &style.code_block_font {
                write!(
                    settings,
                    " __markdown_settings.style.code_block_font = {};",
                    font_preset_code(font, document)?
                )
                .unwrap();
            }
            if let Some(color) = &style.link_color {
                write!(
                    settings,
                    " __markdown_settings.style.link_color = {};",
                    theme_color(document, color)
                )
                .unwrap();
            }
            if let Some(padding) = typed_padding_code(&style.inline_code_padding, env, document)? {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_padding = {padding};"
                )
                .unwrap();
            }
            if let Some(color) = &style.inline_code_border_color {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_highlight.border.color = {};",
                    theme_color(document, color)
                )
                .unwrap();
            }
            if let Some(width) = &style.inline_code_border_width {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_highlight.border.width = {} as f32;",
                    expr_code(width, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(radius) = radius_code(
                style.inline_code_radius.as_ref(),
                [
                    style.inline_code_radius_top_left.as_ref(),
                    style.inline_code_radius_top_right.as_ref(),
                    style.inline_code_radius_bottom_right.as_ref(),
                    style.inline_code_radius_bottom_left.as_ref(),
                ],
                env,
                document,
            )? {
                write!(
                    settings,
                    " __markdown_settings.style.inline_code_highlight.border.radius = {radius};"
                )
                .unwrap();
            }
            let route = route_code(route, "__event", env, document, message)?;
            let view = if let Some(viewer) = &options.viewer {
                let function = document
                    .functions
                    .iter()
                    .find(|item| {
                        item.name == viewer.function && item.kind == ExternKind::MarkdownViewer
                    })
                    .expect("checker validates markdown viewer");
                let args = viewer
                    .args
                    .iter()
                    .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                format!(
                    "let __markdown_viewer = {}({args}); ::iced::widget::markdown::view_with(self.{content}.items(), __markdown_settings, &__markdown_viewer)",
                    function.rust_path
                )
            } else {
                format!(
                    "::iced::widget::markdown::view(self.{content}.items(), __markdown_settings)"
                )
            };
            Ok(format!(
                "{{ {settings} {view}.map(move |__event| {route}) }}"
            ))
        }
        ViewNode::TextEditor {
            binding,
            id,
            disabled,
            options,
            span,
        } => {
            let state = env.get(binding).ok_or_else(|| {
                Error::new("E150", span, format!("unknown editor state `{binding}`"))
            })?;
            let state_name = controlled_state_name(&state.code, "editor", span)?;
            let mut code = format!("::iced::widget::text_editor(&{})", state.code);
            if let Some(id) = id {
                write!(
                    code,
                    ".id(::iced::widget::Id::from({}))",
                    id_code(id, scope, env, document)?
                )
                .unwrap();
            }
            if let Some(placeholder) = &options.placeholder {
                write!(code, ".placeholder({})", rust_string(placeholder)).unwrap();
            }
            if let Some(width) = &options.width {
                write!(
                    code,
                    ".width({} as f32)",
                    expr_code(width, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(height) = &options.height {
                write!(code, ".height({})", length_code(height, env, document)?).unwrap();
            }
            for (value, method) in [
                (&options.min_height, "min_height"),
                (&options.max_height, "max_height"),
                (&options.size, "size"),
                (&options.padding, "padding"),
            ] {
                if let Some(value) = value {
                    write!(
                        code,
                        ".{method}({} as f32)",
                        expr_code(value, env, document, ValueMode::Owned)?
                    )
                    .unwrap();
                }
            }
            if let Some(line_height) = &options.line_height {
                match line_height {
                    TextLineHeight::Relative(value) => write!(
                        code,
                        ".line_height(::iced::widget::text::LineHeight::Relative({} as f32))",
                        expr_code(value, env, document, ValueMode::Owned)?
                    )
                    .unwrap(),
                    TextLineHeight::Absolute(value) => write!(
                        code,
                        ".line_height(::iced::widget::text::LineHeight::Absolute(({} as f32).into()))",
                        expr_code(value, env, document, ValueMode::Owned)?
                    )
                    .unwrap(),
                }
            }
            if let Some(wrapping) = options.wrapping {
                write!(
                    code,
                    ".wrapping(::iced::widget::text::Wrapping::{})",
                    text_wrapping_code(wrapping)
                )
                .unwrap();
            }
            if let Some(font) = &options.font {
                write!(code, ".font({})", font_preset_code(font, document)?).unwrap();
            }
            if let Some(syntax) = &options.highlight {
                let theme = match options
                    .highlight_theme
                    .unwrap_or(HighlightTheme::Base16Ocean)
                {
                    HighlightTheme::SolarizedDark => "SolarizedDark",
                    HighlightTheme::Base16Mocha => "Base16Mocha",
                    HighlightTheme::Base16Ocean => "Base16Ocean",
                    HighlightTheme::Base16Eighties => "Base16Eighties",
                    HighlightTheme::InspiredGithub => "InspiredGitHub",
                };
                write!(
                    code,
                    ".highlight({}, ::iced::highlighter::Theme::{theme})",
                    rust_string(syntax)
                )
                .unwrap();
            }
            if let Some(binding) = &options.key_binding {
                let function = document
                    .functions
                    .iter()
                    .find(|item| {
                        item.name == binding.function && item.kind == ExternKind::EditorBinding
                    })
                    .expect("checker validates editor binding");
                let args = binding
                    .args
                    .iter()
                    .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?;
                let route = route_code(
                    options
                        .key_binding_route
                        .as_ref()
                        .expect("parser requires a key-binding route"),
                    "__value",
                    env,
                    document,
                    message,
                )?;
                write!(
                    code,
                    ".key_binding(move |__key_press| {}(__key_press{}).map(|__binding| __ice_map_editor_binding(__binding, &|__value| {route})))",
                    function.rust_path,
                    args.iter().map(|arg| format!(", {arg}")).collect::<String>()
                )
                .unwrap();
            }
            code.push_str(&text_input_style_code(
                &options.style,
                options.custom_style.as_ref(),
                None,
                env,
                document,
                "style",
                "text_editor",
            )?);
            let finish = |editor: String| -> Result<String, Error> {
                if let Some(highlighter) = &options.highlighter {
                    let function = document
                        .functions
                        .iter()
                        .find(|item| {
                            item.name == highlighter.function
                                && item.kind == ExternKind::EditorHighlighter
                        })
                        .expect("checker validates editor highlighter");
                    let args = highlighter
                        .args
                        .iter()
                        .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(format!(
                        "{}({editor}{})",
                        function.rust_path,
                        args.iter()
                            .map(|arg| format!(", {arg}"))
                            .collect::<String>()
                    ))
                } else {
                    Ok(editor)
                }
            };
            let variant = editor_variant(&state_name);
            let enabled = format!(
                "{code}.on_action({message}::{variant} as fn(::iced::widget::text_editor::Action) -> {message})"
            );
            if let Some(disabled) = disabled {
                let disabled = expr_code(disabled, env, document, ValueMode::Owned)?;
                let disabled_editor = finish(code)?;
                let enabled_editor = finish(enabled)?;
                Ok(format!(
                    "if {disabled} {{ {disabled_editor}.into() }} else {{ {enabled_editor}.into() }}"
                ))
            } else {
                Ok(format!("{}.into()", finish(enabled)?))
            }
        }
        ViewNode::Table {
            item,
            rows,
            options,
            columns,
            span,
        } => render_table(
            item, rows, options, columns, span, document, message, env, scope, slot,
        ),
        ViewNode::If { span, .. } | ViewNode::For { span, .. } => Err(Error::new(
            "E170",
            span,
            "if and for must be children of a layout node",
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_container(
    options: &ContainerOptions,
    id: &Option<Id>,
    styles: &[String],
    content: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let child_scope = id.as_ref().map_or_else(
        || Ok(scope.to_owned()),
        |id| id_code(id, scope, env, document),
    )?;
    let content = render_node(content, document, message, env, &child_scope, slot)?;
    let style = Style::parse(styles, document);
    let mut code = String::from("::iced::widget::container(__container_content)");
    if let Some(id) = id {
        write!(
            code,
            ".id(::iced::widget::Id::from({}))",
            id_code(id, scope, env, document)?
        )
        .unwrap();
    }
    if let Some(padding) = style.padding_code() {
        write!(code, ".padding({padding})").unwrap();
    }
    append_size(&mut code, &style);
    if let Some(max_width) = style.max_width {
        write!(code, ".max_width({max_width})").unwrap();
    }
    if let Some(padding) = typed_padding_code(&options.padding, env, document)? {
        write!(code, ".padding({padding})").unwrap();
    }
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    if let Some(height) = &options.height {
        write!(code, ".height({})", length_code(height, env, document)?).unwrap();
    }
    for (method, value) in [
        ("max_width", &options.max_width),
        ("max_height", &options.max_height),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(align) = options.align_x {
        let align = match align {
            FlexAlignment::Start => "Left",
            FlexAlignment::Center => "Center",
            FlexAlignment::End => "Right",
        };
        write!(code, ".align_x(::iced::alignment::Horizontal::{align})").unwrap();
    }
    if let Some(align) = options.align_y {
        let align = match align {
            FlexAlignment::Start => "Top",
            FlexAlignment::Center => "Center",
            FlexAlignment::End => "Bottom",
        };
        write!(code, ".align_y(::iced::alignment::Vertical::{align})").unwrap();
    }
    if let Some(clip) = &options.clip {
        write!(
            code,
            ".clip({})",
            expr_code(clip, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(surface) = container_surface_style_value(
        &style,
        &options.style,
        options.custom_style.as_ref(),
        env,
        document,
    )? {
        write!(code, ".style(move |__theme| {surface})").unwrap();
    }
    let code = if style.self_center {
        format!("::iced::widget::container({code}).width(::iced::Fill).center_x(::iced::Fill)")
    } else {
        code
    };
    Ok(format!(
        "{{ let __container_content: ::iced::Element<'_, {message}> = {content}; {code}.into() }}"
    ))
}

#[allow(clippy::too_many_arguments)]
fn render_overlay(
    options: &OverlayOptions,
    content: &ViewNode,
    layer: &ViewNode,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let content = render_node(content, document, message, env, scope, slot)?;
    let layer = render_node(layer, document, message, env, scope, slot)?;
    let visible = expr_code(&options.visible, env, document, ValueMode::Owned)?;
    let padding = expr_code(&options.padding, env, document, ValueMode::Owned)?;
    let backdrop = theme_color(document, &options.backdrop);
    let dismiss = options.dismiss.as_ref().map_or_else(
        || Ok(format!("{message}::__ExternNoop")),
        |route| route_code(route, "", env, document, message),
    )?;
    let align_x = match options.align_x {
        FlexAlignment::Start => "Left",
        FlexAlignment::Center => "Center",
        FlexAlignment::End => "Right",
    };
    let align_y = match options.align_y {
        FlexAlignment::Start => "Top",
        FlexAlignment::Center => "Center",
        FlexAlignment::End => "Bottom",
    };
    let noop = format!("{message}::__ExternNoop");
    Ok(format!(
        "{{ let __overlay_base: ::iced::Element<'_, {message}> = {content}; if {visible} {{ let __overlay_layer: ::iced::Element<'_, {message}> = {layer}; let __overlay_backdrop = ::iced::widget::container(::iced::widget::space()).width(::iced::Fill).height(::iced::Fill).style(|_| ::iced::widget::container::Style {{ background: ::std::option::Option::Some(::iced::Background::Color({backdrop})), ..::iced::widget::container::Style::default() }}); let __overlay_backdrop: ::iced::Element<'_, {message}> = ::iced::widget::mouse_area(__overlay_backdrop).on_press({dismiss}).on_release({noop}).on_right_press({noop}).on_right_release({noop}).on_middle_press({noop}).on_middle_release({noop}).on_scroll(|_| {noop}).into(); let __overlay_panel = ::iced::widget::mouse_area(__overlay_layer).on_press({noop}).on_release({noop}).on_right_press({noop}).on_right_release({noop}).on_middle_press({noop}).on_middle_release({noop}).on_scroll(|_| {noop}); let __overlay_panel: ::iced::Element<'_, {message}> = ::iced::widget::container(__overlay_panel).width(::iced::Fill).height(::iced::Fill).padding({padding} as f32).align_x(::iced::alignment::Horizontal::{align_x}).align_y(::iced::alignment::Vertical::{align_y}).into(); let __overlay_surface: ::iced::Element<'_, {message}> = ::iced::widget::Stack::new().width(::iced::Fill).height(::iced::Fill).push(__overlay_backdrop).push(__overlay_panel).into(); ::iced::widget::Stack::new().width(::iced::Fill).height(::iced::Fill).push(__overlay_base).push(::iced::widget::float(__overlay_surface).translate(|_, _| ::iced::Vector::new(::core::f32::EPSILON, 0.0))).into() }} else {{ __overlay_base }} }}"
    ))
}

#[allow(clippy::too_many_arguments)]
fn render_rich_text(
    options: &TextOptions,
    color: &Option<String>,
    spans: &[RichSpan],
    styles: &[String],
    route: &Option<Route>,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
) -> Result<String, Error> {
    let spans = spans
        .iter()
        .map(|item| render_rich_span(item, document, env))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    let style = Style::parse(styles, document);
    let mut code = String::from("::iced::widget::rich_text(__rich_spans)");
    append_text_options(&mut code, options, &style, env, document)?;
    if let Some(color) = color.as_ref().or(style.text_color.as_ref()) {
        write!(code, ".color({})", theme_color(document, color)).unwrap();
    }
    if let Some(route) = route {
        write!(
            code,
            ".on_link_click(move |__link| {})",
            route_code(route, "__link", env, document, message)?
        )
        .unwrap();
    }
    Ok(format!(
        "{{ let __rich_spans: ::std::vec::Vec<::iced::widget::text::Span<'_, ::std::string::String>> = ::std::vec![{spans}]; {code}.into() }}"
    ))
}

#[allow(clippy::too_many_arguments)]
fn render_pane_grid(
    name: &str,
    options: &PaneGridOptions,
    panes: &[PaneView],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let arms = panes
        .iter()
        .map(|pane| {
            let pane_scope = format!("format!(\"{{}}/{}\", {scope})", pane.name);
            Ok(format!(
                "{} => {}",
                rust_string(&pane.name),
                render_pane_content(pane, document, message, env, &pane_scope, slot)?
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    let field = pane_field(name);
    let mut code = format!(
        "::iced::widget::pane_grid(&self.{field}, move |_, __pane_name, _| match *__pane_name {{ {arms}, _ => ::core::unreachable!() }})"
    );
    for (length, method) in [(&options.width, "width"), (&options.height, "height")] {
        if let Some(length) = length {
            write!(code, ".{method}({})", length_code(length, env, document)?).unwrap();
        }
    }
    for (value, method) in [
        (&options.spacing, "spacing"),
        (&options.min_size, "min_size"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(leeway) = &options.resize_leeway {
        write!(
            code,
            ".on_resize({} as f32, {message}::{})",
            expr_code(leeway, env, document, ValueMode::Owned)?,
            pane_resize_variant(name)
        )
        .unwrap();
    }
    if options.draggable {
        write!(code, ".on_drag({message}::{})", pane_drag_variant(name)).unwrap();
    }
    if let Some(route) = &options.click {
        let route = route_code(route, "__pane_name.to_owned()", env, document, message)?;
        write!(
            code,
            ".on_click(move |__pane| {{ let __pane_name = self.{field}.get(__pane).copied().unwrap_or(\"\"); {route} }})"
        )
        .unwrap();
    }
    append_pane_grid_style(&mut code, &options.style, env, document)?;
    Ok(format!("{code}.into()"))
}

fn append_pane_grid_style(
    code: &mut String,
    style: &PaneGridStyle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let has_radius = style.region_radius.is_some()
        || style.region_radius_top_left.is_some()
        || style.region_radius_top_right.is_some()
        || style.region_radius_bottom_right.is_some()
        || style.region_radius_bottom_left.is_some();
    if style.region_background.is_none()
        && style.region_border.is_none()
        && style.region_border_width.is_none()
        && !has_radius
        && style.hovered_split.is_none()
        && style.hovered_split_width.is_none()
        && style.picked_split.is_none()
        && style.picked_split_width.is_none()
    {
        return Ok(());
    }
    code.push_str(
        ".style(move |__theme| { let mut __style = ::iced::widget::pane_grid::default(__theme);",
    );
    if let Some(background) = &style.region_background {
        write!(
            code,
            " __style.hovered_region.background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(border) = &style.region_border {
        write!(
            code,
            " __style.hovered_region.border.color = {};",
            theme_color(document, border)
        )
        .unwrap();
    }
    if let Some(width) = &style.region_border_width {
        write!(
            code,
            " __style.hovered_region.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if has_radius {
        let radius = radius_code(
            style.region_radius.as_ref(),
            [
                style.region_radius_top_left.as_ref(),
                style.region_radius_top_right.as_ref(),
                style.region_radius_bottom_right.as_ref(),
                style.region_radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .expect("pane-grid region radius options were present");
        write!(code, " __style.hovered_region.border.radius = {radius};").unwrap();
    }
    for (color, width, field) in [
        (
            &style.hovered_split,
            &style.hovered_split_width,
            "hovered_split",
        ),
        (
            &style.picked_split,
            &style.picked_split_width,
            "picked_split",
        ),
    ] {
        if let Some(color) = color {
            write!(
                code,
                " __style.{field}.color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(width) = width {
            write!(
                code,
                " __style.{field}.width = {} as f32;",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    code.push_str(" __style })");
    Ok(())
}

fn render_pane_content(
    pane: &PaneView,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let body = render_node(&pane.content, document, message, env, scope, slot)?;
    let mut declarations = format!("let __pane_content: ::iced::Element<'_, {message}> = {body};");
    let mut content = String::from("::iced::widget::pane_grid::Content::new(__pane_content)");
    if let Some(style) = container_surface_style_value(
        &Style::parse(&pane.styles, document),
        &pane.style,
        None,
        env,
        document,
    )? {
        write!(content, ".style(move |_| {style})").unwrap();
    }
    if let Some(title) = &pane.title {
        let title_content = render_node(&title.content, document, message, env, scope, slot)?;
        write!(
            declarations,
            " let __pane_title: ::iced::Element<'_, {message}> = {title_content};"
        )
        .unwrap();
        let mut title_bar = String::from("::iced::widget::pane_grid::TitleBar::new(__pane_title)");
        if let Some(padding) = typed_padding_code(&title.padding, env, document)? {
            write!(title_bar, ".padding({padding})").unwrap();
        }
        if let Some(controls) = &title.controls {
            let controls = render_node(controls, document, message, env, scope, slot)?;
            write!(
                declarations,
                " let __pane_controls: ::iced::Element<'_, {message}> = {controls};"
            )
            .unwrap();
            if let Some(compact) = &title.compact_controls {
                let compact = render_node(compact, document, message, env, scope, slot)?;
                write!(
                    declarations,
                    " let __pane_compact_controls: ::iced::Element<'_, {message}> = {compact};"
                )
                .unwrap();
                title_bar.push_str(".controls(::iced::widget::pane_grid::Controls::dynamic(__pane_controls, __pane_compact_controls))");
            } else {
                title_bar.push_str(
                    ".controls(::iced::widget::pane_grid::Controls::new(__pane_controls))",
                );
            }
        }
        if title.always_show_controls {
            title_bar.push_str(".always_show_controls()");
        }
        if let Some(style) = container_surface_style_value(
            &Style::parse(&title.styles, document),
            &title.style,
            None,
            env,
            document,
        )? {
            write!(title_bar, ".style(move |_| {style})").unwrap();
        }
        write!(content, ".title_bar({title_bar})").unwrap();
    }
    Ok(format!("{{ {declarations} {content} }}"))
}

fn render_rich_span(
    item: &RichSpan,
    document: &Document,
    env: &HashMap<String, Binding>,
) -> Result<String, Error> {
    let style = Style::parse(&item.styles, document);
    let value = expr_code(&item.value, env, document, ValueMode::Owned)?;
    let mut code = format!("::iced::widget::span({value})");
    if let Some(size) = &item.options.size {
        write!(
            code,
            ".size({} as f32)",
            expr_code(size, env, document, ValueMode::Owned)?
        )
        .unwrap();
    } else if let Some(size) = style.text_size {
        write!(code, ".size({size})").unwrap();
    }
    if let Some(line_height) = &item.options.line_height {
        let line_height = match line_height {
            TextLineHeight::Relative(value) => format!(
                "::iced::widget::text::LineHeight::Relative({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            ),
            TextLineHeight::Absolute(value) => format!(
                "::iced::widget::text::LineHeight::Absolute(({} as f32).into())",
                expr_code(value, env, document, ValueMode::Owned)?
            ),
        };
        write!(code, ".line_height({line_height})").unwrap();
    }
    if let Some(font) = &item.options.font {
        let font = font_preset_code(font, document)?;
        if style.bold {
            write!(
                code,
                ".font(::iced::Font {{ weight: ::iced::font::Weight::Bold, ..{font} }})"
            )
            .unwrap();
        } else {
            write!(code, ".font({font})").unwrap();
        }
    } else if style.bold {
        code.push_str(
            ".font(::iced::Font { weight: ::iced::font::Weight::Bold, ..::iced::Font::DEFAULT })",
        );
    }
    if let Some(color) = item.options.color.as_ref().or(style.text_color.as_ref()) {
        write!(code, ".color({})", theme_color(document, color)).unwrap();
    }
    if let Some(link) = &item.options.link {
        write!(
            code,
            ".link({})",
            expr_code(link, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(background) = &item.options.background {
        write!(
            code,
            ".background({})",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    let has_border = item.options.border.is_some()
        || item.options.border_width.is_some()
        || item.options.radius.is_some()
        || item.options.radius_top_left.is_some()
        || item.options.radius_top_right.is_some()
        || item.options.radius_bottom_right.is_some()
        || item.options.radius_bottom_left.is_some();
    if has_border {
        let color = item
            .options
            .border
            .as_ref()
            .map(|color| theme_color(document, color))
            .unwrap_or_else(|| "::iced::Color::TRANSPARENT".into());
        let width = item.options.border_width.as_ref().map_or_else(
            || Ok("0.0".to_owned()),
            |width| expr_code(width, env, document, ValueMode::Owned),
        )?;
        let radius = radius_code(
            item.options.radius.as_ref(),
            [
                item.options.radius_top_left.as_ref(),
                item.options.radius_top_right.as_ref(),
                item.options.radius_bottom_right.as_ref(),
                item.options.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .unwrap_or_else(|| "::iced::border::Radius::default()".into());
        write!(
            code,
            ".border(::iced::Border {{ color: {color}, width: {width} as f32, radius: {radius} }})"
        )
        .unwrap();
    }
    if let Some(padding) = typed_padding_code(&item.options.padding, env, document)? {
        write!(code, ".padding({padding})").unwrap();
    }
    if let Some(underline) = &item.options.underline {
        write!(
            code,
            ".underline({})",
            expr_code(underline, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(strikethrough) = &item.options.strikethrough {
        write!(
            code,
            ".strikethrough({})",
            expr_code(strikethrough, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    Ok(code)
}

#[allow(clippy::too_many_arguments)]
fn render_table(
    item: &str,
    rows: &Expr,
    options: &TableOptions,
    columns: &[TableColumn],
    span: &Span,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let Type::List(inner) = expr_type(
        rows,
        &env.iter()
            .map(|(name, binding)| (name.clone(), binding.ty.clone()))
            .collect(),
        document,
        span,
    )?
    else {
        unreachable!("checker validates table rows")
    };
    let rows = expr_code(rows, env, document, ValueMode::Owned)?;
    let row_type = *inner;
    let row_rust = row_type.rust(&document.structs);
    let mut cell_env = env.clone();
    cell_env.insert(
        item.into(),
        Binding {
            code: item.into(),
            ty: row_type,
            local: true,
        },
    );
    let mut column_codes = Vec::with_capacity(columns.len());
    for (index, column) in columns.iter().enumerate() {
        let header_scope = format!("format!(\"{{}}/header({index})\", {scope})");
        let cell_scope = format!("format!(\"{{}}/row({{}})/column({index})\", {scope}, __row)");
        let header = render_node(&column.header, document, message, env, &header_scope, slot)?;
        let cell = render_node(
            &column.cell,
            document,
            message,
            &cell_env,
            &cell_scope,
            slot,
        )?;
        let mut code = format!(
            "{{ let __table_header: ::iced::Element<'_, {message}> = {header}; ::iced::widget::table::column(__table_header, move |(__row, {item}): (usize, {row_rust})| -> ::iced::Element<'_, {message}> {{ {cell} }})"
        );
        if let Some(width) = &column.width {
            write!(code, ".width({})", length_code(width, env, document)?).unwrap();
        }
        if let Some(align) = column.align_x {
            let align = match align {
                InputAlignment::Left => "Left",
                InputAlignment::Center => "Center",
                InputAlignment::Right => "Right",
            };
            write!(code, ".align_x(::iced::alignment::Horizontal::{align})").unwrap();
        }
        if let Some(align) = column.align_y {
            let align = match align {
                VerticalAlignment::Top => "Top",
                VerticalAlignment::Center => "Center",
                VerticalAlignment::Bottom => "Bottom",
            };
            write!(code, ".align_y(::iced::alignment::Vertical::{align})").unwrap();
        }
        code.push_str(" }");
        column_codes.push(code);
    }
    let mut code = format!(
        "::iced::widget::table::table(::std::vec![{}], {rows}.into_iter().enumerate())",
        column_codes.join(", ")
    );
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    for (value, method) in [
        (&options.padding, "padding"),
        (&options.padding_x, "padding_x"),
        (&options.padding_y, "padding_y"),
        (&options.separator, "separator"),
        (&options.separator_x, "separator_x"),
        (&options.separator_y, "separator_y"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    Ok(format!("{code}.into()"))
}

#[allow(clippy::too_many_arguments)]
fn render_keyed_column(
    item: &str,
    items: &Expr,
    key: &Expr,
    options: &LayoutOptions,
    child: &ViewNode,
    span: &Span,
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let Type::List(inner) = expr_type(
        items,
        &env.iter()
            .map(|(name, binding)| (name.clone(), binding.ty.clone()))
            .collect(),
        document,
        span,
    )?
    else {
        unreachable!("checker validates keyed lists")
    };
    let items = expr_code(items, env, document, ValueMode::Borrowed)?;
    let mut child_env = env.clone();
    child_env.insert(
        item.into(),
        Binding {
            code: item.into(),
            ty: *inner,
            local: false,
        },
    );
    let key = expr_code(key, &child_env, document, ValueMode::Owned)?;
    let child_scope = format!("format!(\"{{}}/key({{}})\", {scope}, __key)");
    let child = render_node(child, document, message, &child_env, &child_scope, slot)?;
    let mut code = format!(
        "{{ let mut __children: ::std::vec::Vec<_> = ::std::vec::Vec::new(); for {item} in {items}.iter() {{ let __key = {key}; let __child: ::iced::Element<'_, {message}> = {child}; __children.push((__key, __child)); }} let __layout = ::iced::widget::keyed_column(__children)"
    );
    if let Some(spacing) = &options.spacing {
        write!(
            code,
            ".spacing({} as f32)",
            expr_code(spacing, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(padding) = typed_padding_code(&options.padding, env, document)? {
        write!(code, ".padding({padding})").unwrap();
    }
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    if let Some(height) = &options.height {
        write!(code, ".height({})", length_code(height, env, document)?).unwrap();
    }
    if let Some(max_width) = &options.max_width {
        write!(
            code,
            ".max_width({} as f32)",
            expr_code(max_width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(align) = options.align {
        let align = match align {
            FlexAlignment::Start => "Start",
            FlexAlignment::Center => "Center",
            FlexAlignment::End => "End",
        };
        write!(code, ".align_items(::iced::Alignment::{align})").unwrap();
    }
    Ok(format!("{code}; __layout.into() }}"))
}

#[allow(clippy::too_many_arguments)]
fn render_layout(
    kind: Layout,
    options: &LayoutOptions,
    id: &Option<Id>,
    styles: &[String],
    children: &[ViewNode],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<String, Error> {
    let style = Style::parse(styles, document);
    if kind == Layout::Scroll {
        let child_scope = id.as_ref().map_or_else(
            || Ok(scope.to_owned()),
            |id| id_code(id, scope, env, document),
        )?;
        let child = render_node(&children[0], document, message, env, &child_scope, slot)?;
        let mut code = String::from("::iced::widget::scrollable(__scroll_content)");
        let scroll = options.scroll.as_ref().expect("scroll options");
        let bar = scroll_bar_code(scroll, env, document)?;
        let direction = match scroll.direction {
            ScrollDirection::Vertical => {
                format!("::iced::widget::scrollable::Direction::Vertical({bar})")
            }
            ScrollDirection::Horizontal => {
                format!("::iced::widget::scrollable::Direction::Horizontal({bar})")
            }
            ScrollDirection::Both => format!(
                "::iced::widget::scrollable::Direction::Both {{ vertical: {bar}, horizontal: {bar} }}"
            ),
        };
        write!(code, ".direction({direction})").unwrap();
        if let Some(id) = id {
            write!(
                code,
                ".id(::iced::widget::Id::from({}))",
                id_code(id, scope, env, document)?
            )
            .unwrap();
        }
        let anchor = |anchor| match anchor {
            ScrollAnchor::Start => "Start",
            ScrollAnchor::End => "End",
        };
        write!(
            code,
            ".anchor_x(::iced::widget::scrollable::Anchor::{})",
            anchor(scroll.anchor_x)
        )
        .unwrap();
        write!(
            code,
            ".anchor_y(::iced::widget::scrollable::Anchor::{})",
            anchor(scroll.anchor_y)
        )
        .unwrap();
        if let Some(auto_scroll) = &scroll.auto_scroll {
            write!(
                code,
                ".auto_scroll({})",
                expr_code(auto_scroll, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(route) = &scroll.route {
            let message_code = ordered_route_code(
                route,
                &[
                    "__absolute.x as f64",
                    "__absolute.y as f64",
                    "__relative.x as f64",
                    "__relative.y as f64",
                ],
                env,
                document,
                message,
            )?;
            write!(
                code,
                ".on_scroll(move |__viewport| {{ let __absolute = __viewport.absolute_offset(); let __relative = __viewport.relative_offset(); {message_code} }})"
            )
            .unwrap();
        } else if let Some(route) = &scroll.viewport_route {
            let message_code = ordered_route_code(
                route,
                &[
                    "__absolute.x as f64",
                    "__absolute.y as f64",
                    "__reversed.x as f64",
                    "__reversed.y as f64",
                    "__relative.x as f64",
                    "__relative.y as f64",
                    "__bounds.x as f64",
                    "__bounds.y as f64",
                    "__bounds.width as f64",
                    "__bounds.height as f64",
                    "__content_bounds.x as f64",
                    "__content_bounds.y as f64",
                    "__content_bounds.width as f64",
                    "__content_bounds.height as f64",
                ],
                env,
                document,
                message,
            )?;
            write!(
                code,
                ".on_scroll(move |__viewport| {{ let __absolute = __viewport.absolute_offset(); let __reversed = __viewport.absolute_offset_reversed(); let __relative = __viewport.relative_offset(); let __bounds = __viewport.bounds(); let __content_bounds = __viewport.content_bounds(); {message_code} }})"
            )
            .unwrap();
        }
        code.push_str(&scroll_style_code(
            &scroll.styles,
            scroll.custom_style.as_ref(),
            env,
            document,
        )?);
        append_size(&mut code, &style);
        if let Some(width) = &scroll.width {
            write!(code, ".width({})", length_code(width, env, document)?).unwrap();
        }
        if let Some(height) = &scroll.height {
            write!(code, ".height({})", length_code(height, env, document)?).unwrap();
        }
        return Ok(format!(
            "{{ let __scroll_content: ::iced::Element<'_, {message}> = {child}; {code}.into() }}"
        ));
    }

    let mut body = String::from("{ let mut __children: ::std::vec::Vec<::iced::Element<'_, ");
    write!(body, "{message}>> = ::std::vec::Vec::new();").unwrap();
    let child_scope = id.as_ref().map_or_else(
        || Ok(scope.to_owned()),
        |id| id_code(id, scope, env, document),
    )?;
    render_children(
        &mut body,
        children,
        document,
        message,
        env,
        &child_scope,
        slot,
    )?;
    let constructor = match kind {
        Layout::Column => "column",
        Layout::Row => "row",
        Layout::Grid => "grid",
        Layout::Stack => "stack",
        Layout::Scroll => unreachable!("scroll returned above"),
    };
    if kind == Layout::Stack && options.under > 0 {
        write!(
            body,
            " let __under = ({} as usize).min(__children.len()); let __above = __children.split_off(__under); let __layout = __above.into_iter().fold(::iced::widget::Stack::new(), |__stack, __child| __stack.push(__child)); let __layout = __children.into_iter().rev().fold(__layout, |__stack, __child| __stack.push_under(__child))",
            options.under
        )
        .unwrap();
    } else {
        write!(
            body,
            " let __layout = ::iced::widget::{constructor}(__children)"
        )
        .unwrap();
    }
    if let Some(gap) = style.gap {
        write!(body, ".spacing({gap})").unwrap();
    }
    if matches!(kind, Layout::Column | Layout::Row)
        && let Some(padding) = style.padding_code()
    {
        write!(body, ".padding({padding})").unwrap();
    }
    if style.items_center {
        if kind == Layout::Column {
            body.push_str(".align_x(::iced::Center)");
        } else {
            body.push_str(".align_y(::iced::Center)");
        }
    }
    if kind == Layout::Grid {
        if let Some(spacing) = &options.spacing {
            write!(
                body,
                ".spacing({} as f32)",
                expr_code(spacing, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(width) = &options.width {
            let LengthValue::Fixed(width) = width else {
                unreachable!("grid widths are always fixed")
            };
            write!(
                body,
                ".width({} as f32)",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(height) = &options.grid_height {
            match height {
                GridSizing::AspectRatio { width, height } => write!(
                    body,
                    ".height(::iced::widget::grid::aspect_ratio({} as f32, {} as f32))",
                    expr_code(width, env, document, ValueMode::Owned)?,
                    expr_code(height, env, document, ValueMode::Owned)?
                )
                .unwrap(),
                GridSizing::EvenlyDistribute(length) => {
                    write!(body, ".height({})", length_code(length, env, document)?).unwrap();
                }
            }
        }
        if let Some(fluid) = &options.fluid {
            write!(
                body,
                ".fluid({} as f32)",
                expr_code(fluid, env, document, ValueMode::Owned)?
            )
            .unwrap();
        } else if let Some(columns) = &options.columns {
            write!(
                body,
                ".columns({} as usize)",
                expr_code(columns, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if matches!(kind, Layout::Column | Layout::Row) {
        if let Some(spacing) = &options.spacing {
            write!(
                body,
                ".spacing({} as f32)",
                expr_code(spacing, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(padding) = typed_padding_code(&options.padding, env, document)? {
            write!(body, ".padding({padding})").unwrap();
        }
        if let Some(width) = &options.width {
            write!(body, ".width({})", length_code(width, env, document)?).unwrap();
        }
        if let Some(height) = &options.height {
            write!(body, ".height({})", length_code(height, env, document)?).unwrap();
        }
        if let Some(max_width) = &options.max_width {
            write!(
                body,
                ".max_width({} as f32)",
                expr_code(max_width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(align) = options.align {
            let alignment = match (kind, align) {
                (Layout::Column, FlexAlignment::Start) => "::iced::alignment::Horizontal::Left",
                (Layout::Column, FlexAlignment::Center) => "::iced::alignment::Horizontal::Center",
                (Layout::Column, FlexAlignment::End) => "::iced::alignment::Horizontal::Right",
                (Layout::Row, FlexAlignment::Start) => "::iced::alignment::Vertical::Top",
                (Layout::Row, FlexAlignment::Center) => "::iced::alignment::Vertical::Center",
                (Layout::Row, FlexAlignment::End) => "::iced::alignment::Vertical::Bottom",
                _ => unreachable!("only row and column reach flex alignment"),
            };
            let method = if kind == Layout::Column {
                "align_x"
            } else {
                "align_y"
            };
            write!(body, ".{method}({alignment})").unwrap();
        }
        if let Some(clip) = &options.clip {
            write!(
                body,
                ".clip({})",
                expr_code(clip, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if options.wrap {
            body.push_str(".wrap()");
            if let Some(spacing) = &options.wrap_spacing {
                let method = if kind == Layout::Column {
                    "horizontal_spacing"
                } else {
                    "vertical_spacing"
                };
                write!(
                    body,
                    ".{method}({} as f32)",
                    expr_code(spacing, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
            if let Some(align) = options.wrap_align {
                let alignment = match (kind, align) {
                    (Layout::Column, FlexAlignment::Start) => "::iced::alignment::Vertical::Top",
                    (Layout::Column, FlexAlignment::Center) => {
                        "::iced::alignment::Vertical::Center"
                    }
                    (Layout::Column, FlexAlignment::End) => "::iced::alignment::Vertical::Bottom",
                    (Layout::Row, FlexAlignment::Start) => "::iced::alignment::Horizontal::Left",
                    (Layout::Row, FlexAlignment::Center) => "::iced::alignment::Horizontal::Center",
                    (Layout::Row, FlexAlignment::End) => "::iced::alignment::Horizontal::Right",
                    _ => unreachable!("only row and column can wrap"),
                };
                write!(body, ".align_x({alignment})").unwrap();
            }
        }
    }
    if kind == Layout::Stack {
        if let Some(clip) = &options.clip {
            write!(
                body,
                ".clip({})",
                expr_code(clip, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(width) = &options.width {
            write!(body, ".width({})", length_code(width, env, document)?).unwrap();
        }
        if let Some(height) = &options.height {
            write!(body, ".height({})", length_code(height, env, document)?).unwrap();
        }
        append_size(&mut body, &style);
    }
    body.push(';');
    body.push_str(" let __content = ::iced::widget::container(__layout)");
    if matches!(kind, Layout::Grid | Layout::Stack)
        && let Some(padding) = style.padding_code()
    {
        write!(body, ".padding({padding})").unwrap();
    }
    append_size(&mut body, &style);
    if let Some(max_width) = style.max_width {
        write!(body, ".max_width({max_width})").unwrap();
    }
    body.push_str(&container_style_code(&style, document));
    body.push(';');
    if style.self_center {
        body.push_str(" ::iced::widget::container(__content).width(::iced::Fill).center_x(::iced::Fill).into() }");
    } else {
        body.push_str(" __content.into() }");
    }
    Ok(body)
}

fn scroll_bar_code(
    scroll: &ScrollOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let constructor = if scroll.hidden_bar { "hidden" } else { "new" };
    let mut code = format!("::iced::widget::scrollable::Scrollbar::{constructor}()");
    for (value, method) in [
        (&scroll.bar_width, "width"),
        (&scroll.bar_margin, "margin"),
        (&scroll.scroller_width, "scroller_width"),
        (&scroll.bar_spacing, "spacing"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    Ok(code)
}

fn scroll_style_code(
    styles: &[ScrollStatusStyle],
    custom: Option<&ExternCall>,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = custom
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::ScrollStyle)
                .expect("checker validates scroll style");
            let args = style
                .args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(format!(
                "{}(__theme, __status{})",
                function.rust_path,
                args.iter()
                    .map(|arg| format!(", {arg}"))
                    .collect::<String>()
            ))
        })
        .transpose()?;
    if styles.is_empty() {
        return Ok(custom
            .map(|custom| format!(".style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }
    let base = custom
        .unwrap_or_else(|| "::iced::widget::scrollable::default(__theme, __status)".to_owned());
    let mut code =
        format!(".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{");
    for (status, pattern) in [
        (
            ScrollStatus::Active,
            "Active { is_horizontal_scrollbar_disabled: __horizontal_disabled, is_vertical_scrollbar_disabled: __vertical_disabled }",
        ),
        (
            ScrollStatus::Hovered,
            "Hovered { is_horizontal_scrollbar_hovered: __horizontal_interaction, is_vertical_scrollbar_hovered: __vertical_interaction, is_horizontal_scrollbar_disabled: __horizontal_disabled, is_vertical_scrollbar_disabled: __vertical_disabled }",
        ),
        (
            ScrollStatus::Dragged,
            "Dragged { is_horizontal_scrollbar_dragged: __horizontal_interaction, is_vertical_scrollbar_dragged: __vertical_interaction, is_horizontal_scrollbar_disabled: __horizontal_disabled, is_vertical_scrollbar_disabled: __vertical_disabled }",
        ),
    ] {
        write!(code, " ::iced::widget::scrollable::Status::{pattern} => {{").unwrap();
        for style in styles.iter().filter(|style| style.status == status) {
            write!(code, " if {} {{", scroll_selector_code(style)).unwrap();
            append_scroll_status_style(&mut code, style, env, document)?;
            code.push_str(" }");
        }
        code.push_str(" }");
    }
    code.push_str(" } __style })");
    Ok(code)
}

fn scroll_selector_code(style: &ScrollStatusStyle) -> String {
    let mut conditions = Vec::new();
    for (value, binding) in [
        (style.horizontal_disabled, "__horizontal_disabled"),
        (style.vertical_disabled, "__vertical_disabled"),
        (style.horizontal_interaction, "__horizontal_interaction"),
        (style.vertical_interaction, "__vertical_interaction"),
    ] {
        if let Some(value) = value {
            conditions.push(format!("{binding} == {value}"));
        }
    }
    if conditions.is_empty() {
        "true".into()
    } else {
        conditions.join(" && ")
    }
}

fn append_scroll_status_style(
    code: &mut String,
    style: &ScrollStatusStyle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    append_scroll_surface_style(
        code,
        &style.container,
        "__style.container",
        true,
        true,
        env,
        document,
    )?;
    for (rail, target) in [
        (&style.horizontal_rail, "__style.horizontal_rail"),
        (&style.vertical_rail, "__style.vertical_rail"),
    ] {
        append_scroll_surface_style(code, &rail.rail, target, true, false, env, document)?;
        append_scroll_surface_style(
            code,
            &rail.scroller,
            &format!("{target}.scroller"),
            false,
            false,
            env,
            document,
        )?;
    }
    if let Some(gap) = &style.gap {
        write!(
            code,
            " __style.gap = ::std::option::Option::Some({});",
            background_code(gap, env, document)?
        )
        .unwrap();
    }
    append_scroll_surface_style(
        code,
        &style.auto_scroll,
        "__style.auto_scroll",
        false,
        false,
        env,
        document,
    )?;
    if let Some(color) = &style.auto_scroll_icon {
        write!(
            code,
            " __style.auto_scroll.icon = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_scroll_surface_style(
    code: &mut String,
    options: &ContainerStyleOptions,
    target: &str,
    optional_background: bool,
    text: bool,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let mut options = options.clone();
    if !optional_background && let Some(background) = options.background.take() {
        write!(
            code,
            " {target}.background = {};",
            background_code(&background, env, document)?
        )
        .unwrap();
    }
    write!(code, " {{ let __style = &mut {target};").unwrap();
    append_surface_style_overrides(code, &options, env, document)?;
    if text && let Some(color) = &options.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            theme_color(document, color)
        )
        .unwrap();
    }
    code.push_str(" }");
    Ok(())
}

fn render_canvas(
    options: &CanvasOptions,
    locals: &[State],
    commands: &[CanvasCommand],
    events: &[CanvasEvent],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
) -> Result<String, Error> {
    let state_fields = locals
        .iter()
        .map(|local| format!("{}: {},", local.name, local.ty.rust(&document.structs)))
        .collect::<Vec<_>>()
        .join(" ");
    let state_initials = locals
        .iter()
        .map(|local| {
            format!(
                "{}: {},",
                local.name,
                initial_code(&local.initial, &local.ty, document)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut canvas_env = env.clone();
    for local in locals {
        canvas_env.insert(
            local.name.clone(),
            Binding {
                code: format!("__state.{}", local.name),
                ty: local.ty.clone(),
                local: false,
            },
        );
    }
    canvas_env.insert(
        "canvas_width".into(),
        Binding {
            code: "(__bounds.width as f64)".into(),
            ty: Type::F64,
            local: true,
        },
    );
    canvas_env.insert(
        "canvas_height".into(),
        Binding {
            code: "(__bounds.height as f64)".into(),
            ty: Type::F64,
            local: true,
        },
    );
    let draw_commands = canvas_commands_code(commands, &canvas_env, document)?;
    let use_cache = options.cache.is_some();
    let cache_key = if let Some(dependency) = &options.cache {
        let dependency = expr_code(dependency, env, document, ValueMode::Owned)?;
        format!(
            "::std::option::Option::Some({{ let mut __hasher = ::std::hash::DefaultHasher::new(); ::std::hash::Hash::hash(&({dependency}), &mut __hasher); ::std::hash::Hasher::finish(&__hasher) }})"
        )
    } else {
        "::std::option::Option::None".into()
    };
    let update = canvas_update_code(
        options,
        events,
        env,
        &canvas_env,
        document,
        message,
        use_cache,
    )?;
    let interaction = if let Some(interaction) = &options.interaction_expr {
        let interaction = expr_code(interaction, &canvas_env, document, ValueMode::Owned)?;
        format!(
            "{{ let __interaction = {interaction}; __ice_canvas_interaction(__interaction.as_str()) }}"
        )
    } else {
        format!(
            "::iced::mouse::Interaction::{}",
            options
                .interaction
                .map(mouse_interaction_code)
                .unwrap_or("None")
        )
    };
    let interaction_outside = options
        .interaction_outside
        .as_ref()
        .map(|outside| expr_code(outside, &canvas_env, document, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "false".into());
    let cache_group = options.cache_group.as_ref().map_or_else(
        || "::std::option::Option::None".into(),
        |group| {
            format!(
                "::std::option::Option::Some(*{}.get_or_init(::iced::widget::canvas::Group::unique))",
                canvas_group_symbol(group)
            )
        },
    );
    let cache_setup = if use_cache {
        "let __cache = __state.cache.get_or_init(|| match __cache_group { ::std::option::Option::Some(group) => ::iced::widget::canvas::Cache::with_group(group), ::std::option::Option::None => ::iced::widget::canvas::Cache::new() }); if __state.cache_key.get() != __cache_key { __cache.clear(); __state.cache_key.set(__cache_key); }"
    } else {
        ""
    };
    let geometry = if use_cache {
        "__cache.draw(__renderer, __bounds.size(), __paint)"
    } else {
        "{ let mut __frame = ::iced::widget::canvas::Frame::new(__renderer, __bounds.size()); __paint(&mut __frame); __frame.into_geometry() }"
    };
    let mut code = format!(
        "{{ #[allow(dead_code)] struct __IceCanvasState {{ cache: ::std::cell::OnceCell<::iced::widget::canvas::Cache>, cache_key: ::std::cell::Cell<::std::option::Option<u64>>, inside: bool, {state_fields} }} impl ::std::default::Default for __IceCanvasState {{ fn default() -> Self {{ Self {{ cache: ::std::cell::OnceCell::new(), cache_key: ::std::cell::Cell::new(::std::option::Option::None), inside: false, {state_initials} }} }} }} let __cache_key: ::std::option::Option<u64> = {cache_key}; let __cache_group: ::std::option::Option<::iced::widget::canvas::Group> = {cache_group}; let __program = __IceCanvasProgram::<__IceCanvasState, {message}, _, _, _> {{ draw: move |__state: &__IceCanvasState, __renderer: &::iced::Renderer, __theme: &::iced::Theme, __bounds: ::iced::Rectangle, __cursor: ::iced::mouse::Cursor| {{ let _ = (&__cache_key, &__cache_group); {cache_setup} let __paint = move |__frame: &mut ::iced::widget::canvas::Frame| {{ {draw_commands} }}; let __geometry = {geometry}; ::std::vec![__geometry] }}, update: {update}, interaction: move |__state: &__IceCanvasState, __bounds: ::iced::Rectangle, __cursor: ::iced::mouse::Cursor| {{ if ({interaction_outside}) || __cursor.is_over(__bounds) {{ {interaction} }} else {{ ::iced::mouse::Interaction::default() }} }}, message: ::std::marker::PhantomData }}; let __canvas = ::iced::widget::canvas(__program)"
    );
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    if let Some(height) = &options.height {
        write!(code, ".height({})", length_code(height, env, document)?).unwrap();
    }
    code.push_str("; __canvas.into() }");
    Ok(code)
}

fn canvas_update_code(
    options: &CanvasOptions,
    events: &[CanvasEvent],
    env: &HashMap<String, Binding>,
    canvas_env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
    use_cache: bool,
) -> Result<String, Error> {
    let capture = options
        .capture
        .as_ref()
        .map(|value| expr_code(value, env, document, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "false".into());
    let action = |message: String, capture: &str| {
        format!(
            "::std::option::Option::Some(if {capture} {{ ::iced::widget::canvas::Action::publish({message}).and_capture() }} else {{ ::iced::widget::canvas::Action::publish({message}) }})"
        )
    };
    let mut code = format!(
        "move |__state: &mut __IceCanvasState, __event: &::iced::widget::canvas::Event, __bounds: ::iced::Rectangle, __cursor: ::iced::mouse::Cursor| {{ let __capture = {capture};"
    );
    if options.enter.is_some() || options.exit.is_some() {
        code.push_str(" let __inside = __cursor.is_over(__bounds); if __inside != __state.inside { __state.inside = __inside;");
        if let Some(route) = &options.enter {
            let route = route_code(route, "", env, document, message)?;
            write!(
                code,
                " if __inside {{ return {}; }}",
                action(route, "__capture")
            )
            .unwrap();
        }
        if let Some(route) = &options.exit {
            let route = route_code(route, "", env, document, message)?;
            write!(
                code,
                " if !__inside {{ return {}; }}",
                action(route, "__capture")
            )
            .unwrap();
        }
        code.push_str(" }");
    }
    let has_pointer_routes = options.press.is_some()
        || options.release.is_some()
        || options.right_press.is_some()
        || options.right_release.is_some()
        || options.middle_press.is_some()
        || options.middle_release.is_some()
        || options.move_route.is_some()
        || options.scroll.is_some();
    if has_pointer_routes {
        code.push_str(
            " if let ::std::option::Option::Some(__point) = __cursor.position_in(__bounds) { match __event {",
        );
        for (route, event) in [
            (
                &options.press,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonPressed(::iced::mouse::Button::Left))",
            ),
            (
                &options.release,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonReleased(::iced::mouse::Button::Left))",
            ),
            (
                &options.right_press,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonPressed(::iced::mouse::Button::Right))",
            ),
            (
                &options.right_release,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonReleased(::iced::mouse::Button::Right))",
            ),
            (
                &options.middle_press,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonPressed(::iced::mouse::Button::Middle))",
            ),
            (
                &options.middle_release,
                "::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonReleased(::iced::mouse::Button::Middle))",
            ),
        ] {
            if let Some(route) = route {
                let route = ordered_route_code(
                    route,
                    &["__point.x as f64", "__point.y as f64"],
                    env,
                    document,
                    message,
                )?;
                write!(code, " {event} => return {},", action(route, "__capture")).unwrap();
            }
        }
        if let Some(route) = &options.move_route {
            let route = ordered_route_code(
                route,
                &["__point.x as f64", "__point.y as f64"],
                env,
                document,
                message,
            )?;
            write!(
                code,
                " ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::CursorMoved {{ .. }}) => return {},",
                action(route, "__capture")
            )
            .unwrap();
        }
        if let Some(route) = &options.scroll {
            let lines = ordered_route_code(
                route,
                &["__x as f64", "__y as f64", "false"],
                env,
                document,
                message,
            )?;
            let pixels = ordered_route_code(
                route,
                &["__x as f64", "__y as f64", "true"],
                env,
                document,
                message,
            )?;
            write!(
                code,
                " ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::WheelScrolled {{ delta }}) => return match delta {{ ::iced::mouse::ScrollDelta::Lines {{ x: __x, y: __y }} => {}, ::iced::mouse::ScrollDelta::Pixels {{ x: __x, y: __y }} => {} }},",
                action(lines, "__capture"),
                action(pixels, "__capture")
            )
            .unwrap();
        }
        code.push_str(" _ => {} } }");
    }
    for event in events {
        let filter = canvas_event_filter(&event.source);
        let payloads = canvas_event_payload_types(&event.source);
        let mut event_env = canvas_env.clone();
        for (binding, ty) in event.bindings.iter().zip(payloads) {
            event_env.insert(
                binding.clone(),
                Binding {
                    code: binding.clone(),
                    ty,
                    local: false,
                },
            );
        }
        let bindings = match event.bindings.as_slice() {
            [] => String::new(),
            [binding] => format!("let {binding} = __value;"),
            bindings => format!("let ({}) = __value;", bindings.join(", ")),
        };
        let mut updates = event
            .updates
            .iter()
            .enumerate()
            .map(|(index, update)| {
                Ok(format!(
                    "let __next_canvas_state_{index} = {}; __state.{} = __next_canvas_state_{index};",
                    expr_code(&update.value, &event_env, document, ValueMode::Owned)?,
                    update.name,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?
            .join(" ");
        if use_cache && !event.updates.is_empty() {
            updates.push_str(
                " if let ::std::option::Option::Some(__cache) = __state.cache.get() { __cache.clear(); }",
            );
        }
        let event_capture = if event.capture { "true" } else { "__capture" };
        let result = match &event.action {
            Some(CanvasEventAction::Route(route)) => {
                let route = if event.route_payload {
                    canvas_event_route_code(&event.source, route, env, document, message)?
                } else {
                    route_code(route, "", &event_env, document, message)?
                };
                action(route, event_capture)
            }
            Some(CanvasEventAction::Redraw { after_ms }) => {
                let redraw = after_ms.map_or_else(
                    || "::iced::widget::canvas::Action::request_redraw()".into(),
                    |milliseconds| {
                        format!(
                            "::iced::widget::canvas::Action::request_redraw_at(::iced::time::Instant::now() + ::iced::time::Duration::from_millis({milliseconds}))"
                        )
                    },
                );
                format!(
                    "::std::option::Option::Some(if {event_capture} {{ {redraw}.and_capture() }} else {{ {redraw} }})"
                )
            }
            None => format!(
                "if {event_capture} {{ ::std::option::Option::Some(::iced::widget::canvas::Action::capture()) }} else {{ ::std::option::Option::None }}"
            ),
        };
        write!(
            code,
            " if let ::std::option::Option::Some(__value) = {filter} {{ let _ = &__value; {bindings} {updates} return {result}; }}"
        )
        .unwrap();
    }
    code.push_str(" ::std::option::Option::None }");
    Ok(code)
}

fn canvas_event_filter(source: &SubscriptionSource) -> String {
    match source {
        SubscriptionSource::InputMethod(event) => match event {
            InputMethodEvent::Opened => "matches!(__event, ::iced::widget::canvas::Event::InputMethod(::iced::advanced::input_method::Event::Opened)).then_some(())".into(),
            InputMethodEvent::Preedit => "match __event { ::iced::widget::canvas::Event::InputMethod(::iced::advanced::input_method::Event::Preedit(content, range)) => { let (start, end) = range.as_ref().map_or((::std::option::Option::None, ::std::option::Option::None), |range| (::std::option::Option::Some(i64::try_from(range.start).unwrap_or(i64::MAX)), ::std::option::Option::Some(i64::try_from(range.end).unwrap_or(i64::MAX)))); ::std::option::Option::Some((content.clone(), start, end)) }, _ => ::std::option::Option::None }".into(),
            InputMethodEvent::Commit => "match __event { ::iced::widget::canvas::Event::InputMethod(::iced::advanced::input_method::Event::Commit(content)) => ::std::option::Option::Some(content.clone()), _ => ::std::option::Option::None }".into(),
            InputMethodEvent::Closed => "matches!(__event, ::iced::widget::canvas::Event::InputMethod(::iced::advanced::input_method::Event::Closed)).then_some(())".into(),
        },
        SubscriptionSource::Keyboard(event) => match event {
            KeyboardEvent::Press => "match __event { ::iced::widget::canvas::Event::Keyboard(::iced::keyboard::Event::KeyPressed { key, modified_key, physical_key, location, modifiers, text, repeat }) => ::std::option::Option::Some(__IceKeyPress { key: __ice_key(key.clone()), modified_key: __ice_key(modified_key.clone()), physical_key: ::std::format!(\"{physical_key:?}\"), location: __ice_key_location(*location), modifiers: __ice_key_modifiers(*modifiers), text: text.as_ref().map(::std::string::ToString::to_string), repeat: *repeat }), _ => ::std::option::Option::None }".into(),
            KeyboardEvent::Release => "match __event { ::iced::widget::canvas::Event::Keyboard(::iced::keyboard::Event::KeyReleased { key, modified_key, physical_key, location, modifiers }) => ::std::option::Option::Some(__IceKeyRelease { key: __ice_key(key.clone()), modified_key: __ice_key(modified_key.clone()), physical_key: ::std::format!(\"{physical_key:?}\"), location: __ice_key_location(*location), modifiers: __ice_key_modifiers(*modifiers) }), _ => ::std::option::Option::None }".into(),
            KeyboardEvent::Modifiers => "match __event { ::iced::widget::canvas::Event::Keyboard(::iced::keyboard::Event::ModifiersChanged(modifiers)) => ::std::option::Option::Some(__ice_key_modifiers(*modifiers)), _ => ::std::option::Option::None }".into(),
        },
        SubscriptionSource::Mouse(event) => match event {
            MouseEvent::Entered => "matches!(__event, ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::CursorEntered)).then_some(())".into(),
            MouseEvent::Left => "matches!(__event, ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::CursorLeft)).then_some(())".into(),
            MouseEvent::Moved => "match __event { ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::CursorMoved { position }) => ::std::option::Option::Some((position.x as f64, position.y as f64)), _ => ::std::option::Option::None }".into(),
            MouseEvent::Pressed => "match __event { ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonPressed(button)) => ::std::option::Option::Some(__ice_mouse_button(*button)), _ => ::std::option::Option::None }".into(),
            MouseEvent::Released => "match __event { ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::ButtonReleased(button)) => ::std::option::Option::Some(__ice_mouse_button(*button)), _ => ::std::option::Option::None }".into(),
            MouseEvent::Wheel => "match __event { ::iced::widget::canvas::Event::Mouse(::iced::mouse::Event::WheelScrolled { delta }) => { let (x, y, pixels) = match delta { ::iced::mouse::ScrollDelta::Lines { x, y } => (*x as f64, *y as f64, false), ::iced::mouse::ScrollDelta::Pixels { x, y } => (*x as f64, *y as f64, true) }; ::std::option::Option::Some((x, y, pixels)) }, _ => ::std::option::Option::None }".into(),
        },
        SubscriptionSource::Touch(event) => {
            let variant = match event {
                TouchEvent::Pressed => "FingerPressed",
                TouchEvent::Moved => "FingerMoved",
                TouchEvent::Lifted => "FingerLifted",
                TouchEvent::Lost => "FingerLost",
            };
            format!("match __event {{ ::iced::widget::canvas::Event::Touch(::iced::touch::Event::{variant} {{ id, position }}) => ::std::option::Option::Some((id.0.to_string(), position.x as f64, position.y as f64)), _ => ::std::option::Option::None }}")
        }
        SubscriptionSource::Window(event) => match event {
            WindowEvent::Frame => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::RedrawRequested(_))).then_some(())".into(),
            WindowEvent::Opened => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::Opened { position, size }) => { let (x, y) = position.as_ref().map_or((::std::option::Option::None, ::std::option::Option::None), |position| (::std::option::Option::Some(position.x as f64), ::std::option::Option::Some(position.y as f64))); ::std::option::Option::Some((x, y, size.width as f64, size.height as f64)) }, _ => ::std::option::Option::None }".into(),
            WindowEvent::Closed => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::Closed)).then_some(())".into(),
            WindowEvent::Moved => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::Moved(position)) => ::std::option::Option::Some((position.x as f64, position.y as f64)), _ => ::std::option::Option::None }".into(),
            WindowEvent::Resized => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::Resized(size)) => ::std::option::Option::Some((size.width as f64, size.height as f64)), _ => ::std::option::Option::None }".into(),
            WindowEvent::Rescaled => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::Rescaled(scale)) => ::std::option::Option::Some(*scale as f64), _ => ::std::option::Option::None }".into(),
            WindowEvent::CloseRequested => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::CloseRequested)).then_some(())".into(),
            WindowEvent::Focused => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::Focused)).then_some(())".into(),
            WindowEvent::Unfocused => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::Unfocused)).then_some(())".into(),
            WindowEvent::FileHovered => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::FileHovered(path)) => ::std::option::Option::Some(path.to_string_lossy().into_owned()), _ => ::std::option::Option::None }".into(),
            WindowEvent::FileDropped => "match __event { ::iced::widget::canvas::Event::Window(::iced::window::Event::FileDropped(path)) => ::std::option::Option::Some(path.to_string_lossy().into_owned()), _ => ::std::option::Option::None }".into(),
            WindowEvent::FilesHoveredLeft => "matches!(__event, ::iced::widget::canvas::Event::Window(::iced::window::Event::FilesHoveredLeft)).then_some(())".into(),
        },
        _ => unreachable!("parser rejects non-event canvas sources"),
    }
}

fn canvas_event_payload_types(source: &SubscriptionSource) -> Vec<Type> {
    match source {
        SubscriptionSource::InputMethod(event) => match event {
            InputMethodEvent::Opened | InputMethodEvent::Closed => Vec::new(),
            InputMethodEvent::Preedit => vec![
                Type::Str,
                Type::Option(Box::new(Type::I64)),
                Type::Option(Box::new(Type::I64)),
            ],
            InputMethodEvent::Commit => vec![Type::Str],
        },
        SubscriptionSource::Keyboard(event) => vec![match event {
            KeyboardEvent::Press => Type::KeyPress,
            KeyboardEvent::Release => Type::KeyRelease,
            KeyboardEvent::Modifiers => Type::KeyModifiers,
        }],
        SubscriptionSource::Mouse(event) => match event {
            MouseEvent::Entered | MouseEvent::Left => Vec::new(),
            MouseEvent::Moved => vec![Type::F64, Type::F64],
            MouseEvent::Pressed | MouseEvent::Released => vec![Type::Str],
            MouseEvent::Wheel => vec![Type::F64, Type::F64, Type::Bool],
        },
        SubscriptionSource::Touch(_) => vec![Type::Str, Type::F64, Type::F64],
        SubscriptionSource::Window(event) => match event {
            WindowEvent::Frame
            | WindowEvent::Closed
            | WindowEvent::CloseRequested
            | WindowEvent::Focused
            | WindowEvent::Unfocused
            | WindowEvent::FilesHoveredLeft => Vec::new(),
            WindowEvent::Opened => vec![
                Type::Option(Box::new(Type::F64)),
                Type::Option(Box::new(Type::F64)),
                Type::F64,
                Type::F64,
            ],
            WindowEvent::Moved | WindowEvent::Resized => vec![Type::F64, Type::F64],
            WindowEvent::Rescaled => vec![Type::F64],
            WindowEvent::FileHovered | WindowEvent::FileDropped => vec![Type::Str],
        },
        _ => unreachable!("parser rejects non-event canvas sources"),
    }
}

fn canvas_event_route_code(
    source: &SubscriptionSource,
    route: &Route,
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    match source {
        SubscriptionSource::InputMethod(event) => match event {
            InputMethodEvent::Opened | InputMethodEvent::Closed => {
                route_code(route, "", env, document, message)
            }
            InputMethodEvent::Preedit => ordered_route_code(
                route,
                &["__value.0", "__value.1", "__value.2"],
                env,
                document,
                message,
            ),
            InputMethodEvent::Commit => route_code(route, "__value", env, document, message),
        },
        SubscriptionSource::Keyboard(_) => route_code(route, "__value", env, document, message),
        SubscriptionSource::Mouse(event) => match event {
            MouseEvent::Entered | MouseEvent::Left => route_code(route, "", env, document, message),
            MouseEvent::Moved => {
                ordered_route_code(route, &["__value.0", "__value.1"], env, document, message)
            }
            MouseEvent::Pressed | MouseEvent::Released => {
                route_code(route, "__value", env, document, message)
            }
            MouseEvent::Wheel => ordered_route_code(
                route,
                &["__value.0", "__value.1", "__value.2"],
                env,
                document,
                message,
            ),
        },
        SubscriptionSource::Touch(_) => ordered_route_code(
            route,
            &["__value.0", "__value.1", "__value.2"],
            env,
            document,
            message,
        ),
        SubscriptionSource::Window(event) => match event {
            WindowEvent::Opened => ordered_route_code(
                route,
                &["__value.0", "__value.1", "__value.2", "__value.3"],
                env,
                document,
                message,
            ),
            WindowEvent::Moved | WindowEvent::Resized => {
                ordered_route_code(route, &["__value.0", "__value.1"], env, document, message)
            }
            WindowEvent::Rescaled | WindowEvent::FileHovered | WindowEvent::FileDropped => {
                route_code(route, "__value", env, document, message)
            }
            WindowEvent::Frame
            | WindowEvent::Closed
            | WindowEvent::CloseRequested
            | WindowEvent::Focused
            | WindowEvent::Unfocused
            | WindowEvent::FilesHoveredLeft => route_code(route, "", env, document, message),
        },
        _ => unreachable!("parser rejects non-event canvas sources"),
    }
}

fn canvas_commands_code(
    commands: &[CanvasCommand],
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let mut code = String::new();
    for command in commands {
        match command {
            CanvasCommand::Rectangle {
                x,
                y,
                width,
                height,
                radius,
                paint,
                ..
            } => {
                let point = canvas_point_code(x, y, env, document)?;
                let size = canvas_size_code(width, height, env, document)?;
                if canvas_radius_is_empty(radius) {
                    if let Some(fill) = &paint.fill {
                        write!(
                            code,
                            " __frame.fill_rectangle({point}, {size}, {});",
                            canvas_fill_code(fill, paint.fill_rule, env, document)?
                        )
                        .unwrap();
                    }
                    if let Some(stroke) = &paint.stroke {
                        write!(
                            code,
                            " __frame.stroke_rectangle({point}, {size}, {});",
                            canvas_stroke_code(stroke, env, document)?
                        )
                        .unwrap();
                    }
                } else {
                    let radius = canvas_radius_code(radius, env, document)?;
                    write!(
                        code,
                        " {{ let __path = ::iced::widget::canvas::Path::rounded_rectangle({point}, {size}, {radius}); {} }}",
                        canvas_paint_code(paint, "&__path", env, document)?
                    )
                    .unwrap();
                }
            }
            CanvasCommand::Circle {
                x,
                y,
                radius,
                paint,
                ..
            } => {
                let point = canvas_point_code(x, y, env, document)?;
                let radius = canvas_expr_code(radius, env, document)?;
                write!(
                    code,
                    " {{ let __path = ::iced::widget::canvas::Path::circle({point}, {radius} as f32); {} }}",
                    canvas_paint_code(paint, "&__path", env, document)?
                )
                .unwrap();
            }
            CanvasCommand::Line {
                x1,
                y1,
                x2,
                y2,
                stroke,
                ..
            } => {
                let from = canvas_point_code(x1, y1, env, document)?;
                let to = canvas_point_code(x2, y2, env, document)?;
                write!(
                    code,
                    " {{ let __path = ::iced::widget::canvas::Path::line({from}, {to}); __frame.stroke(&__path, {}); }}",
                    canvas_stroke_code(stroke, env, document)?
                )
                .unwrap();
            }
            CanvasCommand::Text {
                value,
                x,
                y,
                max_width,
                color,
                size,
                line_height,
                font,
                align_x,
                align_y,
                shaping,
                span,
            } => {
                let ty = expr_type(
                    value,
                    &env.iter()
                        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                        .collect(),
                    document,
                    span,
                )?;
                let value = expr_code(value, env, document, ValueMode::Owned)?;
                let content = if ty == Type::Str {
                    value
                } else {
                    format!("::std::format!(\"{{}}\", {value})")
                };
                let position = canvas_point_code(x, y, env, document)?;
                let max_width = max_width
                    .as_ref()
                    .map(|value| canvas_expr_code(value, env, document))
                    .transpose()?
                    .map_or_else(|| "f32::INFINITY".into(), |value| format!("{value} as f32"));
                let color = color.as_ref().map_or_else(
                    || theme_color(document, "foreground"),
                    |color| theme_color(document, color),
                );
                let size = size
                    .as_ref()
                    .map(|value| canvas_expr_code(value, env, document))
                    .transpose()?
                    .unwrap_or_else(|| "16.0".into());
                let line_height = match line_height {
                    Some(TextLineHeight::Relative(value)) => format!(
                        "::iced::widget::text::LineHeight::Relative({} as f32)",
                        canvas_expr_code(value, env, document)?
                    ),
                    Some(TextLineHeight::Absolute(value)) => format!(
                        "::iced::widget::text::LineHeight::Absolute(::iced::Pixels({} as f32))",
                        canvas_expr_code(value, env, document)?
                    ),
                    None => "::iced::widget::text::LineHeight::default()".into(),
                };
                let font = font
                    .as_ref()
                    .map(|font| font_preset_code(font, document))
                    .transpose()?
                    .unwrap_or_else(|| "::iced::Font::DEFAULT".into());
                let align_x = align_x.map_or("Default", |value| text_alignment_code(value));
                let align_y = match align_y {
                    None | Some(VerticalAlignment::Top) => "Top",
                    Some(VerticalAlignment::Center) => "Center",
                    Some(VerticalAlignment::Bottom) => "Bottom",
                };
                let shaping = shaping.map_or("Auto", text_shaping_code);
                write!(
                    code,
                    " __frame.fill_text(::iced::widget::canvas::Text {{ content: {content}, position: {position}, max_width: {max_width}, color: {color}, size: ::iced::Pixels({size} as f32), line_height: {line_height}, font: {font}, align_x: ::iced::widget::text::Alignment::{align_x}, align_y: ::iced::alignment::Vertical::{align_y}, shaping: ::iced::widget::text::Shaping::{shaping} }});"
                )
                .unwrap();
            }
            CanvasCommand::Image {
                source,
                x,
                y,
                width,
                height,
                filter,
                rotation,
                opacity,
                snap,
                radius,
                span,
            } => {
                let source_ty = expr_type(
                    source,
                    &env.iter()
                        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                        .collect(),
                    document,
                    span,
                )?;
                let source = expr_code(source, env, document, ValueMode::Owned)?;
                let handle = if source_ty == Type::Str {
                    format!("::iced::widget::image::Handle::from_path({source})")
                } else {
                    source
                };
                let filter = match filter {
                    ImageFilter::Linear => "Linear",
                    ImageFilter::Nearest => "Nearest",
                };
                write!(
                    code,
                    " __frame.draw_image(::iced::Rectangle::new({}, {}), ::iced::widget::canvas::Image {{ handle: {handle}, filter_method: ::iced::widget::image::FilterMethod::{filter}, rotation: ::iced::Radians({} as f32), border_radius: {}, opacity: {} as f32, snap: {} }});",
                    canvas_point_code(x, y, env, document)?,
                    canvas_size_code(width, height, env, document)?,
                    canvas_expr_code(rotation, env, document)?,
                    canvas_radius_code(radius, env, document)?,
                    canvas_expr_code(opacity, env, document)?,
                    canvas_expr_code(snap, env, document)?
                )
                .unwrap();
            }
            CanvasCommand::Svg {
                source,
                memory,
                x,
                y,
                width,
                height,
                color,
                rotation,
                opacity,
                span,
            } => {
                let source_ty = expr_type(
                    source,
                    &env.iter()
                        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                        .collect(),
                    document,
                    span,
                )?;
                let source = expr_code(source, env, document, ValueMode::Owned)?;
                let handle = if *memory && source_ty == Type::Bytes {
                    format!("::iced::advanced::svg::Handle::from_memory({source})")
                } else if *memory {
                    format!("::iced::advanced::svg::Handle::from_memory(({source}).into_bytes())")
                } else {
                    format!("::iced::advanced::svg::Handle::from_path({source})")
                };
                let color = color.as_ref().map_or_else(
                    || "::std::option::Option::None".into(),
                    |color| {
                        format!(
                            "::std::option::Option::Some({})",
                            theme_color(document, color)
                        )
                    },
                );
                write!(
                    code,
                    " __frame.draw_svg(::iced::Rectangle::new({}, {}), ::iced::advanced::svg::Svg {{ handle: {handle}, color: {color}, rotation: ::iced::Radians({} as f32), opacity: {} as f32 }});",
                    canvas_point_code(x, y, env, document)?,
                    canvas_size_code(width, height, env, document)?,
                    canvas_expr_code(rotation, env, document)?,
                    canvas_expr_code(opacity, env, document)?
                )
                .unwrap();
            }
            CanvasCommand::Path {
                segments, paint, ..
            } => {
                let path = canvas_path_code(segments, env, document)?;
                write!(
                    code,
                    " {{ let __path = {path}; {} }}",
                    canvas_paint_code(paint, "&__path", env, document)?
                )
                .unwrap();
            }
            CanvasCommand::Group {
                transform,
                commands,
                ..
            } => {
                let inner = canvas_commands_code(commands, env, document)?;
                let mut body = String::new();
                if transform.x.is_some() || transform.y.is_some() {
                    let x = transform
                        .x
                        .as_ref()
                        .map(|value| canvas_expr_code(value, env, document))
                        .transpose()?
                        .unwrap_or_else(|| "0.0".into());
                    let y = transform
                        .y
                        .as_ref()
                        .map(|value| canvas_expr_code(value, env, document))
                        .transpose()?
                        .unwrap_or_else(|| "0.0".into());
                    write!(
                        body,
                        " __frame.translate(::iced::Vector::new({x} as f32, {y} as f32));"
                    )
                    .unwrap();
                }
                if let Some(value) = &transform.rotate {
                    write!(
                        body,
                        " __frame.rotate({} as f32);",
                        canvas_expr_code(value, env, document)?
                    )
                    .unwrap();
                }
                if let Some(value) = &transform.scale {
                    write!(
                        body,
                        " __frame.scale({} as f32);",
                        canvas_expr_code(value, env, document)?
                    )
                    .unwrap();
                }
                if transform.scale_x.is_some() || transform.scale_y.is_some() {
                    let x = transform
                        .scale_x
                        .as_ref()
                        .map(|value| canvas_expr_code(value, env, document))
                        .transpose()?
                        .unwrap_or_else(|| "1.0".into());
                    let y = transform
                        .scale_y
                        .as_ref()
                        .map(|value| canvas_expr_code(value, env, document))
                        .transpose()?
                        .unwrap_or_else(|| "1.0".into());
                    write!(
                        body,
                        " __frame.scale_nonuniform(::iced::Vector::new({x} as f32, {y} as f32));"
                    )
                    .unwrap();
                }
                if let Some([x, y, width, height]) = &transform.clip {
                    let point = canvas_point_code(x, y, env, document)?;
                    let size = canvas_size_code(width, height, env, document)?;
                    write!(
                        body,
                        " __frame.with_clip(::iced::Rectangle {{ x: {point}.x, y: {point}.y, width: {size}.width, height: {size}.height }}, |__frame| {{ {inner} }});"
                    )
                    .unwrap();
                } else {
                    body.push_str(&inner);
                }
                write!(code, " __frame.with_save(|__frame| {{ {body} }});").unwrap();
            }
            CanvasCommand::If {
                condition,
                commands,
                ..
            } => {
                let condition = expr_code(condition, env, document, ValueMode::Owned)?;
                write!(
                    code,
                    " if {condition} {{ {} }}",
                    canvas_commands_code(commands, env, document)?
                )
                .unwrap();
            }
            CanvasCommand::For {
                item,
                items,
                commands,
                span,
            } => {
                let Type::List(inner) = expr_type(
                    items,
                    &env.iter()
                        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                        .collect(),
                    document,
                    span,
                )?
                else {
                    return Err(Error::new("E190", span, "canvas for expects a list"));
                };
                let items = expr_code(items, env, document, ValueMode::Borrowed)?;
                let mut child_env = env.clone();
                child_env.insert(
                    item.clone(),
                    Binding {
                        code: item.clone(),
                        ty: *inner,
                        local: false,
                    },
                );
                write!(
                    code,
                    " for {item} in {items}.iter() {{ {} }}",
                    canvas_commands_code(commands, &child_env, document)?
                )
                .unwrap();
            }
        }
    }
    Ok(code)
}

fn canvas_path_code(
    segments: &[CanvasPathSegment],
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let mut code = String::from("::iced::widget::canvas::Path::new(|__path| {");
    for segment in segments {
        match segment {
            CanvasPathSegment::Move(x, y) => write!(
                code,
                " __path.move_to({});",
                canvas_point_code(x, y, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Line(x, y) => write!(
                code,
                " __path.line_to({});",
                canvas_point_code(x, y, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Arc {
                x,
                y,
                radius,
                start,
                end,
            } => write!(
                code,
                " __path.arc(::iced::widget::canvas::path::Arc {{ center: {}, radius: {} as f32, start_angle: ::iced::Radians({} as f32), end_angle: ::iced::Radians({} as f32) }});",
                canvas_point_code(x, y, env, document)?,
                canvas_expr_code(radius, env, document)?,
                canvas_expr_code(start, env, document)?,
                canvas_expr_code(end, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::ArcTo {
                ax,
                ay,
                bx,
                by,
                radius,
            } => write!(
                code,
                " __path.arc_to({}, {}, {} as f32);",
                canvas_point_code(ax, ay, env, document)?,
                canvas_point_code(bx, by, env, document)?,
                canvas_expr_code(radius, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Ellipse {
                x,
                y,
                radius_x,
                radius_y,
                rotation,
                start,
                end,
            } => write!(
                code,
                " __path.ellipse(::iced::widget::canvas::path::arc::Elliptical {{ center: {}, radii: ::iced::Vector::new({} as f32, {} as f32), rotation: ::iced::Radians({} as f32), start_angle: ::iced::Radians({} as f32), end_angle: ::iced::Radians({} as f32) }});",
                canvas_point_code(x, y, env, document)?,
                canvas_expr_code(radius_x, env, document)?,
                canvas_expr_code(radius_y, env, document)?,
                canvas_expr_code(rotation, env, document)?,
                canvas_expr_code(start, env, document)?,
                canvas_expr_code(end, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Bezier {
                control_ax,
                control_ay,
                control_bx,
                control_by,
                x,
                y,
            } => write!(
                code,
                " __path.bezier_curve_to({}, {}, {});",
                canvas_point_code(control_ax, control_ay, env, document)?,
                canvas_point_code(control_bx, control_by, env, document)?,
                canvas_point_code(x, y, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Quadratic {
                control_x,
                control_y,
                x,
                y,
            } => write!(
                code,
                " __path.quadratic_curve_to({}, {});",
                canvas_point_code(control_x, control_y, env, document)?,
                canvas_point_code(x, y, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Rectangle {
                x,
                y,
                width,
                height,
            } => write!(
                code,
                " __path.rectangle({}, {});",
                canvas_point_code(x, y, env, document)?,
                canvas_size_code(width, height, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::RoundedRectangle {
                x,
                y,
                width,
                height,
                radius,
            } => write!(
                code,
                " __path.rounded_rectangle({}, {}, {});",
                canvas_point_code(x, y, env, document)?,
                canvas_size_code(width, height, env, document)?,
                canvas_radius_code(radius, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Circle { x, y, radius } => write!(
                code,
                " __path.circle({}, {} as f32);",
                canvas_point_code(x, y, env, document)?,
                canvas_expr_code(radius, env, document)?
            )
            .unwrap(),
            CanvasPathSegment::Close => code.push_str(" __path.close();"),
        }
    }
    code.push_str(" })");
    Ok(code)
}

fn canvas_paint_code(
    paint: &CanvasPaint,
    path: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let mut code = String::new();
    if let Some(fill) = &paint.fill {
        write!(
            code,
            " __frame.fill({path}, {});",
            canvas_fill_code(fill, paint.fill_rule, env, document)?
        )
        .unwrap();
    }
    if let Some(stroke) = &paint.stroke {
        write!(
            code,
            " __frame.stroke({path}, {});",
            canvas_stroke_code(stroke, env, document)?
        )
        .unwrap();
    }
    Ok(code)
}

fn canvas_fill_code(
    fill: &BackgroundValue,
    rule: CanvasFillRule,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let rule = match rule {
        CanvasFillRule::NonZero => "NonZero",
        CanvasFillRule::EvenOdd => "EvenOdd",
    };
    Ok(format!(
        "::iced::widget::canvas::Fill {{ style: {}, rule: ::iced::widget::canvas::fill::Rule::{rule} }}",
        canvas_style_code(fill, env, document)?
    ))
}

fn canvas_stroke_code(
    stroke: &CanvasStroke,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let cap = match stroke.cap {
        CanvasLineCap::Butt => "Butt",
        CanvasLineCap::Square => "Square",
        CanvasLineCap::Round => "Round",
    };
    let join = match stroke.join {
        CanvasLineJoin::Miter => "Miter",
        CanvasLineJoin::Round => "Round",
        CanvasLineJoin::Bevel => "Bevel",
    };
    let dash = stroke
        .dash
        .iter()
        .map(|value| canvas_expr_code(value, env, document).map(|value| format!("{value} as f32")))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(format!(
        "::iced::widget::canvas::Stroke {{ style: {}, width: {} as f32, line_cap: ::iced::widget::canvas::LineCap::{cap}, line_join: ::iced::widget::canvas::LineJoin::{join}, line_dash: ::iced::widget::canvas::LineDash {{ segments: &[{dash}], offset: usize::try_from({}).unwrap_or(usize::MAX) }} }}",
        canvas_style_code(&stroke.style, env, document)?,
        canvas_expr_code(&stroke.width, env, document)?,
        canvas_expr_code(&stroke.dash_offset, env, document)?
    ))
}

fn canvas_style_code(
    style: &BackgroundValue,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(match style {
        BackgroundValue::Color(color) => format!(
            "::iced::widget::canvas::Style::Solid({})",
            theme_color(document, color)
        ),
        BackgroundValue::Linear { angle, stops } => {
            let mut gradient =
                String::from("::iced::widget::canvas::gradient::Linear::new(__start, __end)");
            for stop in stops {
                write!(
                    gradient,
                    ".add_stop({} as f32, {})",
                    canvas_expr_code(&stop.offset, env, document)?,
                    theme_color(document, &stop.color)
                )
                .unwrap();
            }
            format!(
                "{{ let __angle = {} as f32; let __direction = ::iced::Vector::new(__angle.cos(), __angle.sin()); let __center = ::iced::Point::new(__bounds.width / 2.0, __bounds.height / 2.0); let __extent = (__bounds.width * __direction.x.abs() + __bounds.height * __direction.y.abs()) / 2.0; let __start = ::iced::Point::new(__center.x - __direction.x * __extent, __center.y - __direction.y * __extent); let __end = ::iced::Point::new(__center.x + __direction.x * __extent, __center.y + __direction.y * __extent); ::iced::widget::canvas::Style::Gradient(::iced::widget::canvas::Gradient::Linear({gradient})) }}",
                canvas_expr_code(angle, env, document)?
            )
        }
    })
}

fn canvas_radius_is_empty(radius: &CanvasRadius) -> bool {
    radius.all.is_none()
        && radius.top_left.is_none()
        && radius.top_right.is_none()
        && radius.bottom_right.is_none()
        && radius.bottom_left.is_none()
}

fn canvas_radius_code(
    radius: &CanvasRadius,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    radius_code(
        radius.all.as_ref(),
        [
            radius.top_left.as_ref(),
            radius.top_right.as_ref(),
            radius.bottom_right.as_ref(),
            radius.bottom_left.as_ref(),
        ],
        env,
        document,
    )
    .map(|radius| radius.unwrap_or_else(|| "::iced::border::Radius::default()".into()))
}

fn canvas_point_code(
    x: &Expr,
    y: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(format!(
        "::iced::Point::new({} as f32, {} as f32)",
        canvas_expr_code(x, env, document)?,
        canvas_expr_code(y, env, document)?
    ))
}

fn canvas_size_code(
    width: &Expr,
    height: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(format!(
        "::iced::Size::new({} as f32, {} as f32)",
        canvas_expr_code(width, env, document)?,
        canvas_expr_code(height, env, document)?
    ))
}

fn canvas_expr_code(
    value: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    expr_code(value, env, document, ValueMode::Owned)
}

fn render_children(
    out: &mut String,
    children: &[ViewNode],
    document: &Document,
    message: &str,
    env: &HashMap<String, Binding>,
    scope: &str,
    slot: Option<&SlotContext>,
) -> Result<(), Error> {
    for child in children {
        match child {
            ViewNode::If {
                condition,
                children,
                ..
            } => {
                let condition = expr_code(condition, env, document, ValueMode::Owned)?;
                write!(out, " if {condition} {{").unwrap();
                render_children(out, children, document, message, env, scope, slot)?;
                out.push_str(" }");
            }
            ViewNode::For {
                item,
                items,
                children,
                span,
            } => {
                let Type::List(inner) = expr_type(
                    items,
                    &env.iter()
                        .map(|(name, binding)| (name.clone(), binding.ty.clone()))
                        .collect(),
                    document,
                    span,
                )?
                else {
                    return Err(Error::new("E121", span, "for expects a list"));
                };
                let items = expr_code(items, env, document, ValueMode::Borrowed)?;
                write!(out, " for {item} in {items}.iter() {{").unwrap();
                let mut child_env = env.clone();
                child_env.insert(
                    item.clone(),
                    Binding {
                        code: item.clone(),
                        ty: *inner,
                        local: false,
                    },
                );
                render_children(out, children, document, message, &child_env, scope, slot)?;
                out.push_str(" }");
            }
            _ => {
                let child = render_node(child, document, message, env, scope, slot)?;
                write!(out, " __children.push({child});").unwrap();
            }
        }
    }
    Ok(())
}

#[derive(Clone)]
struct Binding {
    code: String,
    ty: Type,
    local: bool,
}

#[derive(Clone)]
struct SlotContext {
    entries: Vec<SlotContent>,
    parent: Option<Box<SlotContext>>,
}

#[derive(Clone)]
struct SlotContent {
    name: String,
    node: ViewNode,
    env: HashMap<String, Binding>,
}

#[derive(Clone, Copy)]
enum ValueMode {
    Owned,
    Borrowed,
}

fn state_env(document: &Document, name: &str) -> HashMap<String, Binding> {
    document
        .states
        .iter()
        .map(|state| {
            (
                state.name.clone(),
                Binding {
                    code: format!("{name}.{}", state.name),
                    ty: state.ty.clone(),
                    local: false,
                },
            )
        })
        .collect()
}

fn expr_code(
    expr: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
    mode: ValueMode,
) -> Result<String, Error> {
    Ok(match expr {
        Expr::Bool(value) => value.to_string(),
        Expr::I64(value) => value.to_string(),
        Expr::F64(value) => rust_f64(*value),
        Expr::Str(value) => match mode {
            ValueMode::Owned => format!("{}.to_owned()", rust_string(value)),
            ValueMode::Borrowed => rust_string(value),
        },
        Expr::Bytes(values) => format!(
            "::std::vec![{}]",
            values
                .iter()
                .map(|value| format!("0x{value:02x}u8"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::EmptyList => "::std::vec::Vec::new()".into(),
        Expr::List(values) => format!(
            "::std::vec![{}]",
            values
                .iter()
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ),
        Expr::None => "::std::option::Option::None".into(),
        Expr::Path(path) => {
            let binding = env.get(&path[0]).ok_or_else(|| {
                Error::new(
                    "E150",
                    &Span::line(1),
                    format!("unknown value `{}`", path[0]),
                )
            })?;
            let mut code = binding.code.clone();
            let mut ty = binding.ty.clone();
            let mut owned_projection = false;
            for field in &path[1..] {
                if let Type::Option(inner) = &ty
                    && **inner == Type::WidgetTarget
                {
                    code = format!("({code}).as_ref().map(|value| value.{field}.clone())");
                    ty = Type::Option(Box::new(
                        widget_target_field_type(field).unwrap_or(Type::Unknown),
                    ));
                    owned_projection = true;
                    continue;
                }
                write!(code, ".{field}").unwrap();
                if let Type::Named(name) = &ty {
                    ty = document
                        .structs
                        .iter()
                        .find(|item| item.name == *name)
                        .and_then(|item| item.fields.iter().find(|(name, _)| name == field))
                        .map(|(_, ty)| ty.clone())
                        .unwrap_or(Type::Unknown);
                } else if ty == Type::WidgetTarget {
                    ty = widget_target_field_type(field).unwrap_or(Type::Unknown);
                }
            }
            let clone_unnecessary = matches!(ty, Type::Bool | Type::I64 | Type::F64 | Type::Unit)
                || (binding.local && path.len() == 1)
                || owned_projection;
            if matches!(mode, ValueMode::Owned) && !clone_unnecessary {
                code.push_str(".clone()");
            }
            code
        }
        Expr::Call { name, args } => match name.as_str() {
            "len" => format!(
                "({}).len() as i64",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "empty" => format!(
                "({}).is_empty()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "trim" => format!(
                "({}).trim().to_owned()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "some" => format!(
                "::std::option::Option::Some({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "markdown" => format!(
                "::iced::widget::markdown::Content::parse(&{})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "markdown_images" => format!(
                "({}).images().iter().cloned().collect::<::std::vec::Vec<_>>()",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            "editor" => format!(
                "::iced::widget::text_editor::Content::with_text(&{})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "encoded" => format!(
                "::iced::widget::image::Handle::from_bytes({})",
                expr_code(&args[0], env, document, ValueMode::Owned)?
            ),
            "rgba" => format!(
                "::iced::widget::image::Handle::from_rgba({}, {}, {})",
                u32_code(&args[0], env, document)?,
                u32_code(&args[1], env, document)?,
                expr_code(&args[2], env, document, ValueMode::Owned)?
            ),
            "aborted" => format!(
                "({}).as_ref().is_some_and(::iced::task::Handle::is_aborted)",
                expr_code(&args[0], env, document, ValueMode::Borrowed)?
            ),
            _ => {
                let function = document
                    .functions
                    .iter()
                    .find(|function| function.name == *name && function.kind == ExternKind::Sync)
                    .expect("checker accepts only declared sync calls");
                let args = args
                    .iter()
                    .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                format!("{}({args})", function.rust_path)
            }
        },
        Expr::Unary { op, value } => format!(
            "({}{})",
            match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
            },
            expr_code(value, env, document, ValueMode::Owned)?
        ),
        Expr::Binary { left, op, right } => {
            let mode = if matches!(
                op,
                BinaryOp::Eq
                    | BinaryOp::NotEq
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::Gt
                    | BinaryOp::GtEq
            ) {
                ValueMode::Borrowed
            } else {
                ValueMode::Owned
            };
            format!(
                "({} {} {})",
                expr_code(left, env, document, mode)?,
                match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Eq => "==",
                    BinaryOp::NotEq => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::LtEq => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::GtEq => ">=",
                    BinaryOp::And => "&&",
                    BinaryOp::Or => "||",
                },
                expr_code(right, env, document, mode)?
            )
        }
    })
}

fn widget_target_field_type(field: &str) -> Option<Type> {
    match field {
        "kind" => Some(Type::Str),
        "id" => Some(Type::Option(Box::new(Type::WidgetId))),
        "x" | "y" | "width" | "height" => Some(Type::F64),
        "visible_x" | "visible_y" | "visible_width" | "visible_height" | "content_x"
        | "content_y" | "content_width" | "content_height" | "translation_x" | "translation_y" => {
            Some(Type::Option(Box::new(Type::F64)))
        }
        "content" => Some(Type::Option(Box::new(Type::Str))),
        _ => None,
    }
}

fn u32_code(
    expr: &Expr,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(format!(
        "({}).clamp(0, u32::MAX as i64) as u32",
        expr_code(expr, env, document, ValueMode::Owned)?
    ))
}

fn route_code(
    route: &Route,
    payload: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    let variant = pascal(&route.handler);
    if route.args.is_empty() {
        return Ok(format!("{message}::{variant}"));
    }
    let args = route
        .args
        .iter()
        .map(|arg| match arg {
            RouteArg::Payload => Ok(payload.into()),
            RouteArg::Expr(expr) => expr_code(expr, env, document, ValueMode::Owned),
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    Ok(format!("{message}::{variant}({args})"))
}

fn size_route_code(
    route: &Route,
    size: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    ordered_route_code(
        route,
        &[
            &format!("{size}.width as f64"),
            &format!("{size}.height as f64"),
        ],
        env,
        document,
        message,
    )
}

fn ordered_route_code(
    route: &Route,
    payloads: &[&str],
    env: &HashMap<String, Binding>,
    document: &Document,
    message: &str,
) -> Result<String, Error> {
    let variant = pascal(&route.handler);
    if route.args.is_empty() {
        return Ok(format!("{message}::{variant}"));
    }
    let mut payload = payloads.iter();
    let args = route
        .args
        .iter()
        .map(|arg| match arg {
            RouteArg::Payload => Ok((*payload.next().expect("checked payload count")).to_owned()),
            RouteArg::Expr(expr) => expr_code(expr, env, document, ValueMode::Owned),
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    Ok(format!("{message}::{variant}({args})"))
}

fn initial_code(expr: &Expr, ty: &Type, document: &Document) -> String {
    match (expr, ty) {
        (Expr::Str(value), Type::Str) => format!("{}.to_owned()", rust_string(value)),
        (Expr::Str(value), Type::Markdown) => format!(
            "::iced::widget::markdown::Content::parse({})",
            rust_string(value)
        ),
        (Expr::Str(value), Type::Editor) => format!(
            "::iced::widget::text_editor::Content::with_text({})",
            rust_string(value)
        ),
        (Expr::EmptyList, Type::List(_)) => "::std::vec::Vec::new()".into(),
        (Expr::EmptyList, Type::Combo(_)) => {
            "::iced::widget::combo_box::State::new(::std::vec::Vec::new())".into()
        }
        (Expr::List(values), Type::Combo(_)) => format!(
            "::iced::widget::combo_box::State::new(::std::vec![{}])",
            values
                .iter()
                .map(|value| {
                    expr_code(value, &HashMap::new(), document, ValueMode::Owned)
                        .unwrap_or_else(|_| "::core::default::Default::default()".into())
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        (Expr::None, Type::Option(_)) => "::std::option::Option::None".into(),
        (Expr::Bool(value), _) => value.to_string(),
        (Expr::I64(value), _) => value.to_string(),
        (Expr::F64(value), _) => rust_f64(*value),
        _ => expr_code(expr, &HashMap::new(), document, ValueMode::Owned)
            .unwrap_or_else(|_| "::core::default::Default::default()".into()),
    }
}

fn pane_field(name: &str) -> String {
    format!("__pane_{name}")
}

fn pane_configuration_code(configuration: &PaneConfiguration) -> String {
    match configuration {
        PaneConfiguration::Pane(name) => format!(
            "::iced::widget::pane_grid::Configuration::Pane({})",
            rust_string(name)
        ),
        PaneConfiguration::Split { axis, ratio, a, b } => {
            let axis = match axis {
                PaneAxis::Horizontal => "Horizontal",
                PaneAxis::Vertical => "Vertical",
            };
            format!(
                "::iced::widget::pane_grid::Configuration::Split {{ axis: ::iced::widget::pane_grid::Axis::{axis}, ratio: {ratio:?}, a: ::std::boxed::Box::new({}), b: ::std::boxed::Box::new({}) }}",
                pane_configuration_code(a),
                pane_configuration_code(b)
            )
        }
    }
}

fn pane_resize_variant(name: &str) -> String {
    format!("__Pane{}Resize", pascal(name))
}

fn pane_drag_variant(name: &str) -> String {
    format!("__Pane{}Drag", pascal(name))
}

fn pane_grids(root: &ViewNode) -> Vec<&ViewNode> {
    fn collect<'a>(node: &'a ViewNode, output: &mut Vec<&'a ViewNode>) {
        match node {
            ViewNode::PaneGrid { panes, .. } => {
                output.push(node);
                for pane in panes {
                    for node in pane.nodes() {
                        collect(node, output);
                    }
                }
            }
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, output);
                }
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, output);
                collect(tip, output);
            }
            ViewNode::Overlay { content, layer, .. } => {
                collect(content, output);
                collect(layer, output);
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, output);
                    collect(&column.cell, output);
                }
            }
            ViewNode::MouseArea { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. } => collect(content, output),
            ViewNode::Button {
                content: Some(content),
                ..
            } => collect(content, output),
            ViewNode::Component { slots, .. } => {
                for slot in slots {
                    collect(&slot.content, output);
                }
            }
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, output);
                    collect(wide, output);
                }
                ResponsiveContent::Size { content, .. } => collect(content, output),
            },
            _ => {}
        }
    }
    let mut output = Vec::new();
    collect(root, &mut output);
    output
}

fn uses_canvas(document: &Document) -> bool {
    !canvases(document).is_empty()
}

fn canvases(document: &Document) -> Vec<(&CanvasOptions, &[CanvasEvent])> {
    fn collect<'a>(node: &'a ViewNode, output: &mut Vec<(&'a CanvasOptions, &'a [CanvasEvent])>) {
        match node {
            ViewNode::Canvas {
                options, events, ..
            } => output.push((options, events)),
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => {
                for child in children {
                    collect(child, output);
                }
            }
            ViewNode::Tooltip { content, tip, .. } => {
                collect(content, output);
                collect(tip, output);
            }
            ViewNode::Overlay { content, layer, .. } => {
                collect(content, output);
                collect(layer, output);
            }
            ViewNode::PaneGrid { panes, .. } => {
                for node in panes.iter().flat_map(PaneView::nodes) {
                    collect(node, output);
                }
            }
            ViewNode::Table { columns, .. } => {
                for column in columns {
                    collect(&column.header, output);
                    collect(&column.cell, output);
                }
            }
            ViewNode::MouseArea { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. }
            | ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. }
            | ViewNode::KeyedColumn { child: content, .. }
            | ViewNode::Lazy { child: content, .. } => collect(content, output),
            ViewNode::Component { slots, .. } => {
                for slot in slots {
                    collect(&slot.content, output);
                }
            }
            ViewNode::Button {
                content: Some(content),
                ..
            } => collect(content, output),
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    collect(narrow, output);
                    collect(wide, output);
                }
                ResponsiveContent::Size { content, .. } => collect(content, output),
            },
            _ => {}
        }
    }
    let mut output = Vec::new();
    collect(&document.view, &mut output);
    for component in &document.components {
        collect(&component.root, &mut output);
    }
    output
}

fn canvas_cache_groups(document: &Document) -> Vec<&str> {
    let mut groups = Vec::new();
    for group in canvases(document)
        .into_iter()
        .filter_map(|(options, _)| options.cache_group.as_deref())
    {
        if !groups.contains(&group) {
            groups.push(group);
        }
    }
    groups
}

fn canvas_events(document: &Document) -> Vec<&CanvasEvent> {
    canvases(document)
        .into_iter()
        .flat_map(|(_, events)| events)
        .collect()
}

fn canvas_group_symbol(group: &str) -> String {
    format!("__ICE_CANVAS_GROUP_{}", group.to_ascii_uppercase())
}

fn needs_extern_noop(document: &Document) -> bool {
    fn contains(node: &ViewNode) -> bool {
        match node {
            ViewNode::ExternComponent { route: None, .. }
            | ViewNode::Shader { route: None, .. } => true,
            ViewNode::Layout { children, .. }
            | ViewNode::If { children, .. }
            | ViewNode::For { children, .. } => children.iter().any(contains),
            ViewNode::Tooltip { content, tip, .. } => contains(content) || contains(tip),
            ViewNode::Overlay { .. } => true,
            ViewNode::PaneGrid { panes, .. } => {
                panes.iter().flat_map(PaneView::nodes).any(contains)
            }
            ViewNode::Table { columns, .. } => columns
                .iter()
                .any(|column| contains(&column.header) || contains(&column.cell)),
            ViewNode::MouseArea { content, .. }
            | ViewNode::Container { content, .. }
            | ViewNode::Theme { content, .. } => contains(content),
            ViewNode::Component { slots, .. } => slots.iter().any(|slot| contains(&slot.content)),
            ViewNode::KeyedColumn { child, .. } | ViewNode::Lazy { child, .. } => contains(child),
            ViewNode::Button {
                content: Some(content),
                ..
            } => contains(content),
            ViewNode::Float { content, .. }
            | ViewNode::Pin { content, .. }
            | ViewNode::Sensor { content, .. } => contains(content),
            ViewNode::Responsive { content, .. } => match content {
                ResponsiveContent::Breakpoint { narrow, wide, .. } => {
                    contains(narrow) || contains(wide)
                }
                ResponsiveContent::Size { content, .. } => contains(content),
            },
            _ => false,
        }
    }
    contains(&document.view) || document.components.iter().any(|item| contains(&item.root))
}

fn length_code(
    length: &LengthValue,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(match length {
        LengthValue::Fill => "::iced::Fill".into(),
        LengthValue::FillPortion(portion) => {
            format!("::iced::Length::FillPortion({portion})")
        }
        LengthValue::Shrink => "::iced::Shrink".into(),
        LengthValue::Fixed(value) => format!(
            "{} as f32",
            expr_code(value, env, document, ValueMode::Owned)?
        ),
    })
}

fn typed_padding_code(
    padding: &PaddingOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<Option<String>, Error> {
    if padding.all.is_none()
        && padding.x.is_none()
        && padding.y.is_none()
        && padding.top.is_none()
        && padding.right.is_none()
        && padding.bottom.is_none()
        && padding.left.is_none()
    {
        return Ok(None);
    }
    let code = |value: Option<&Expr>| {
        value
            .map(|value| expr_code(value, env, document, ValueMode::Owned))
            .transpose()
    };
    let all = code(padding.all.as_ref())?.unwrap_or_else(|| "0.0".into());
    let x = code(padding.x.as_ref())?.unwrap_or_else(|| all.clone());
    let y = code(padding.y.as_ref())?.unwrap_or_else(|| all.clone());
    let top = code(padding.top.as_ref())?.unwrap_or_else(|| y.clone());
    let right = code(padding.right.as_ref())?.unwrap_or_else(|| x.clone());
    let bottom = code(padding.bottom.as_ref())?.unwrap_or(y);
    let left = code(padding.left.as_ref())?.unwrap_or(x);
    Ok(Some(format!(
        "::iced::Padding {{ top: {top} as f32, right: {right} as f32, bottom: {bottom} as f32, left: {left} as f32 }}"
    )))
}

fn radius_code(
    uniform: Option<&Expr>,
    corners: [Option<&Expr>; 4],
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<Option<String>, Error> {
    if uniform.is_none() && corners.iter().all(Option::is_none) {
        return Ok(None);
    }
    let base = uniform
        .map(|value| expr_code(value, env, document, ValueMode::Owned))
        .transpose()?
        .unwrap_or_else(|| "0.0".to_owned());
    let mut values = Vec::with_capacity(4);
    for corner in corners {
        values.push(
            corner
                .map(|value| expr_code(value, env, document, ValueMode::Owned))
                .transpose()?
                .unwrap_or_else(|| base.clone()),
        );
    }
    Ok(Some(format!(
        "::iced::border::Radius {{ top_left: {} as f32, top_right: {} as f32, bottom_right: {} as f32, bottom_left: {} as f32 }}",
        values[0], values[1], values[2], values[3]
    )))
}

fn append_float_style(
    code: &mut String,
    style: &FloatStyleOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let radius = radius_code(
        style.radius.as_ref(),
        [
            style.radius_top_left.as_ref(),
            style.radius_top_right.as_ref(),
            style.radius_bottom_right.as_ref(),
            style.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )?;
    if style.shadow_color.is_none()
        && style.shadow_x.is_none()
        && style.shadow_y.is_none()
        && style.shadow_blur.is_none()
        && radius.is_none()
    {
        return Ok(());
    }
    code.push_str(".style(move |_| { let mut __style = ::iced::widget::float::Style::default();");
    if let Some(color) = &style.shadow_color {
        write!(
            code,
            " __style.shadow.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    for (value, field) in [
        (&style.shadow_x, "__style.shadow.offset.x"),
        (&style.shadow_y, "__style.shadow.offset.y"),
        (&style.shadow_blur, "__style.shadow.blur_radius"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                " {field} = {} as f32;",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(radius) = radius {
        write!(code, " __style.shadow_border_radius = {radius};").unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}

fn background_code(
    background: &BackgroundValue,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    match background {
        BackgroundValue::Color(color) => Ok(format!(
            "::iced::Background::Color({})",
            theme_color(document, color)
        )),
        BackgroundValue::Linear { angle, stops } => {
            let mut code = format!(
                "::iced::Background::from(::iced::gradient::Linear::new({} as f32)",
                expr_code(angle, env, document, ValueMode::Owned)?
            );
            for stop in stops {
                write!(
                    code,
                    ".add_stop({} as f32, {})",
                    expr_code(&stop.offset, env, document, ValueMode::Owned)?,
                    theme_color(document, &stop.color)
                )
                .unwrap();
            }
            code.push(')');
            Ok(code)
        }
    }
}

fn container_surface_style_value(
    utilities: &Style,
    options: &ContainerStyleOptions,
    custom: Option<&ExternCall>,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<Option<String>, Error> {
    let has_typed_style = options.background.is_some()
        || options.text_color.is_some()
        || options.border_color.is_some()
        || options.border_width.is_some()
        || options.radius.is_some()
        || options.radius_top_left.is_some()
        || options.radius_top_right.is_some()
        || options.radius_bottom_right.is_some()
        || options.radius_bottom_left.is_some()
        || options.shadow_color.is_some()
        || options.shadow_x.is_some()
        || options.shadow_y.is_some()
        || options.shadow_blur.is_some()
        || options.pixel_snap.is_some();
    let utility_style = container_style_value(utilities, document);
    let custom_style = custom
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::ContainerStyle)
                .expect("checker validates container style");
            let args = style
                .args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(format!(
                "{}(__theme{})",
                function.rust_path,
                args.iter()
                    .map(|arg| format!(", {arg}"))
                    .collect::<String>()
            ))
        })
        .transpose()?;
    if !has_typed_style && custom_style.is_none() {
        return Ok(utility_style);
    }
    if !has_typed_style && utility_style.is_none() {
        return Ok(custom_style);
    }

    let has_custom_style = custom_style.is_some();
    let base = custom_style
        .or_else(|| utility_style.clone())
        .unwrap_or_else(|| "::iced::widget::container::Style::default()".into());
    let mut code = format!("{{ let mut __style = {base};");
    if has_custom_style {
        append_container_utility_overrides(&mut code, utilities, document);
    }
    append_surface_style_overrides(&mut code, options, env, document)?;
    if let Some(color) = &options.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            theme_color(document, color)
        )
        .unwrap();
    }
    code.push_str(" __style }");
    Ok(Some(code))
}

fn append_container_utility_overrides(code: &mut String, style: &Style, document: &Document) {
    if let Some(background) = &style.background {
        write!(
            code,
            " __style.background = ::std::option::Option::Some({}.into());",
            theme_color(document, background)
        )
        .unwrap();
    }
    if let Some(text) = &style.text_color {
        write!(
            code,
            " __style.text_color = ::std::option::Option::Some({});",
            theme_color(document, text)
        )
        .unwrap();
    }
    if let Some(border) = &style.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, border)
        )
        .unwrap();
    }
    if style.border_width != 0 {
        write!(code, " __style.border.width = {}.0;", style.border_width).unwrap();
    }
    if style.radius != 0 {
        write!(code, " __style.border.radius = {}.0.into();", style.radius).unwrap();
    }
}

fn append_surface_style_overrides(
    code: &mut String,
    options: &ContainerStyleOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    if let Some(background) = &options.background {
        write!(
            code,
            " __style.background = ::std::option::Option::Some({});",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(color) = &options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(width) = &options.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = radius_code(
        options.radius.as_ref(),
        [
            options.radius_top_left.as_ref(),
            options.radius_top_right.as_ref(),
            options.radius_bottom_right.as_ref(),
            options.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    if let Some(color) = &options.shadow_color {
        write!(
            code,
            " __style.shadow.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(x) = &options.shadow_x {
        write!(
            code,
            " __style.shadow.offset.x = {} as f32;",
            expr_code(x, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(y) = &options.shadow_y {
        write!(
            code,
            " __style.shadow.offset.y = {} as f32;",
            expr_code(y, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(blur) = &options.shadow_blur {
        write!(
            code,
            " __style.shadow.blur_radius = {} as f32;",
            expr_code(blur, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(snap) = &options.pixel_snap {
        write!(
            code,
            " __style.snap = {};",
            expr_code(snap, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    Ok(())
}

fn append_slider_styles(
    code: &mut String,
    styles: &SliderStyleSet,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let custom = styles
        .custom
        .as_ref()
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::SliderStyle)
                .expect("checker validates slider style");
            let args = style
                .args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(format!(
                "{}(__theme, __status{})",
                function.rust_path,
                args.iter()
                    .map(|arg| format!(", {arg}"))
                    .collect::<String>()
            ))
        })
        .transpose()?;
    if styles.active.is_none() && styles.hovered.is_none() && styles.dragged.is_none() {
        if let Some(custom) = custom {
            write!(code, ".style(move |__theme, __status| {custom})").unwrap();
        }
        return Ok(());
    }
    let complete = styles.active.is_some() && styles.hovered.is_some() && styles.dragged.is_some();
    let base =
        custom.unwrap_or_else(|| "::iced::widget::slider::default(__theme, __status)".to_owned());
    write!(
        code,
        ".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{"
    )
    .unwrap();
    for (status, style) in [
        ("Active", &styles.active),
        ("Hovered", &styles.hovered),
        ("Dragged", &styles.dragged),
    ] {
        if let Some(style) = style {
            write!(code, " ::iced::widget::slider::Status::{status} => {{").unwrap();
            append_slider_style_fields(code, style, env, document)?;
            code.push_str(" }");
        }
    }
    if !complete {
        code.push_str(" _ => {}");
    }
    code.push_str(" } __style })");
    Ok(())
}

fn append_slider_style_fields(
    code: &mut String,
    style: &SliderStyle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    for (background, field) in [
        (&style.rail_start, "__style.rail.backgrounds.0"),
        (&style.rail_end, "__style.rail.backgrounds.1"),
        (&style.handle_color, "__style.handle.background"),
    ] {
        if let Some(background) = background {
            write!(
                code,
                " {field} = {};",
                background_code(background, env, document)?
            )
            .unwrap();
        }
    }
    for (color, field) in [
        (&style.rail_border_color, "__style.rail.border.color"),
        (&style.handle_border_color, "__style.handle.border_color"),
    ] {
        if let Some(color) = color {
            write!(code, " {field} = {}.into();", theme_color(document, color)).unwrap();
        }
    }
    for (value, field) in [
        (&style.rail_width, "__style.rail.width"),
        (&style.rail_border_width, "__style.rail.border.width"),
        (&style.handle_border_width, "__style.handle.border_width"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                " {field} = {} as f32;",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(radius) = radius_code(
        style.rail_radius.as_ref(),
        [
            style.rail_radius_top_left.as_ref(),
            style.rail_radius_top_right.as_ref(),
            style.rail_radius_bottom_right.as_ref(),
            style.rail_radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )? {
        write!(code, " __style.rail.border.radius = {radius};").unwrap();
    }
    if let Some(shape) = &style.handle_shape {
        let shape = match shape {
            SliderHandleShape::Circle(radius) => format!(
                "::iced::widget::slider::HandleShape::Circle {{ radius: {} as f32 }}",
                expr_code(radius, env, document, ValueMode::Owned)?
            ),
            SliderHandleShape::Rectangle { width } => {
                let radius = radius_code(
                    style.handle_radius.as_ref(),
                    [
                        style.handle_radius_top_left.as_ref(),
                        style.handle_radius_top_right.as_ref(),
                        style.handle_radius_bottom_right.as_ref(),
                        style.handle_radius_bottom_left.as_ref(),
                    ],
                    env,
                    document,
                )?
                .unwrap_or_else(|| "::iced::border::Radius::default()".to_owned());
                format!(
                    "::iced::widget::slider::HandleShape::Rectangle {{ width: {width}, border_radius: {radius} }}"
                )
            }
        };
        write!(code, " __style.handle.shape = {shape};").unwrap();
    }
    Ok(())
}

fn append_tooltip_style(
    code: &mut String,
    options: &TooltipOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let has_radius = options.radius.is_some()
        || options.radius_top_left.is_some()
        || options.radius_top_right.is_some()
        || options.radius_bottom_right.is_some()
        || options.radius_bottom_left.is_some();
    if options.style.is_none()
        && options.custom_style.is_none()
        && options.background.is_none()
        && options.text_color.is_none()
        && options.border_color.is_none()
        && options.border_width.is_none()
        && !has_radius
        && options.shadow_color.is_none()
        && options.shadow_x.is_none()
        && options.shadow_y.is_none()
        && options.shadow_blur.is_none()
        && options.pixel_snap.is_none()
    {
        return Ok(());
    }
    if let Some(style) = &options.custom_style {
        let function = document
            .functions
            .iter()
            .find(|item| item.name == style.function && item.kind == ExternKind::ContainerStyle)
            .expect("checker validates tooltip container style");
        let args = style
            .args
            .iter()
            .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
            .collect::<Result<Vec<_>, _>>()?;
        write!(
            code,
            ".style(move |__theme| {{ let mut __style = {}(__theme{});",
            function.rust_path,
            args.iter()
                .map(|arg| format!(", {arg}"))
                .collect::<String>()
        )
        .unwrap();
    } else {
        let preset = match options.style.unwrap_or(TooltipStyle::Transparent) {
            TooltipStyle::Transparent => "transparent",
            TooltipStyle::Rounded => "rounded_box",
            TooltipStyle::Bordered => "bordered_box",
            TooltipStyle::Dark => "dark",
            TooltipStyle::Primary => "primary",
            TooltipStyle::Secondary => "secondary",
            TooltipStyle::Success => "success",
            TooltipStyle::Warning => "warning",
            TooltipStyle::Danger => "danger",
        };
        write!(
            code,
            ".style(move |__theme| {{ let mut __style = ::iced::widget::container::{preset}(__theme);"
        )
        .unwrap();
    }
    if let Some(background) = &options.background {
        write!(
            code,
            " __style.background = Some({});",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(text) = &options.text_color {
        write!(
            code,
            " __style.text_color = Some({});",
            theme_color(document, text)
        )
        .unwrap();
    }
    if let Some(border) = &options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, border)
        )
        .unwrap();
    }
    if let Some(width) = &options.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if has_radius {
        let radius = radius_code(
            options.radius.as_ref(),
            [
                options.radius_top_left.as_ref(),
                options.radius_top_right.as_ref(),
                options.radius_bottom_right.as_ref(),
                options.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .expect("tooltip radius options were present");
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    if let Some(shadow) = &options.shadow_color {
        write!(
            code,
            " __style.shadow.color = {};",
            theme_color(document, shadow)
        )
        .unwrap();
    }
    for (value, field) in [
        (&options.shadow_x, "__style.shadow.offset.x"),
        (&options.shadow_y, "__style.shadow.offset.y"),
        (&options.shadow_blur, "__style.shadow.blur_radius"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                " {field} = {} as f32;",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(pixel_snap) = &options.pixel_snap {
        write!(
            code,
            " __style.snap = {};",
            expr_code(pixel_snap, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}

fn append_progress_options(
    code: &mut String,
    options: &ProgressOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let has_radius = options.radius.is_some()
        || options.radius_top_left.is_some()
        || options.radius_top_right.is_some()
        || options.radius_bottom_right.is_some()
        || options.radius_bottom_left.is_some();
    if options.style.is_none()
        && options.custom_style.is_none()
        && options.background.is_none()
        && options.bar.is_none()
        && options.border_color.is_none()
        && options.border_width.is_none()
        && !has_radius
    {
        return Ok(());
    }
    if let Some(style) = &options.custom_style {
        let function = document
            .functions
            .iter()
            .find(|item| item.name == style.function && item.kind == ExternKind::ProgressStyle)
            .expect("checker validates progress style");
        let args = style
            .args
            .iter()
            .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
            .collect::<Result<Vec<_>, _>>()?;
        write!(
            code,
            ".style(move |__theme| {{ let mut __style = {}(__theme{});",
            function.rust_path,
            args.iter()
                .map(|arg| format!(", {arg}"))
                .collect::<String>()
        )
        .unwrap();
    } else {
        let preset = match options.style.unwrap_or(ProgressStyle::Primary) {
            ProgressStyle::Primary => "primary",
            ProgressStyle::Secondary => "secondary",
            ProgressStyle::Success => "success",
            ProgressStyle::Warning => "warning",
            ProgressStyle::Danger => "danger",
        };
        write!(
            code,
            ".style(move |__theme| {{ let mut __style = ::iced::widget::progress_bar::{preset}(__theme);"
        )
        .unwrap();
    }
    if let Some(background) = &options.background {
        write!(
            code,
            " __style.background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(bar) = &options.bar {
        write!(
            code,
            " __style.bar = {};",
            background_code(bar, env, document)?
        )
        .unwrap();
    }
    if let Some(border) = &options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, border)
        )
        .unwrap();
    }
    if let Some(width) = &options.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if has_radius {
        let radius = radius_code(
            options.radius.as_ref(),
            [
                options.radius_top_left.as_ref(),
                options.radius_top_right.as_ref(),
                options.radius_bottom_right.as_ref(),
                options.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .expect("progress radius options were present");
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}

fn append_rule_options(
    code: &mut String,
    options: &RuleOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    let has_radius = options.radius.is_some()
        || options.radius_top_left.is_some()
        || options.radius_top_right.is_some()
        || options.radius_bottom_right.is_some()
        || options.radius_bottom_left.is_some();
    if options.style.is_none()
        && options.fill.is_none()
        && options.color.is_none()
        && !has_radius
        && options.snap.is_none()
    {
        return Ok(());
    }
    let preset = match options.style.unwrap_or(RuleStyle::Default) {
        RuleStyle::Default => "default",
        RuleStyle::Weak => "weak",
    };
    write!(
        code,
        ".style(move |__theme| {{ let mut __style = ::iced::widget::rule::{preset}(__theme);"
    )
    .unwrap();
    if let Some(fill) = &options.fill {
        let fill = match fill {
            RuleFill::Full => "::iced::widget::rule::FillMode::Full".to_owned(),
            RuleFill::Percent(value) => format!(
                "::iced::widget::rule::FillMode::Percent({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            ),
            RuleFill::Padded(value) => {
                format!("::iced::widget::rule::FillMode::Padded({value})")
            }
            RuleFill::AsymmetricPadding(first, second) => {
                format!("::iced::widget::rule::FillMode::AsymmetricPadding({first}, {second})")
            }
        };
        write!(code, " __style.fill_mode = {fill};").unwrap();
    }
    if let Some(color) = &options.color {
        write!(code, " __style.color = {};", theme_color(document, color)).unwrap();
    }
    if has_radius {
        let radius = radius_code(
            options.radius.as_ref(),
            [
                options.radius_top_left.as_ref(),
                options.radius_top_right.as_ref(),
                options.radius_bottom_right.as_ref(),
                options.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )?
        .expect("rule radius options were present");
        write!(code, " __style.radius = {radius};").unwrap();
    }
    if let Some(snap) = &options.snap {
        write!(
            code,
            " __style.snap = {};",
            expr_code(snap, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    code.push_str(" __style })");
    Ok(())
}

fn append_text_options(
    code: &mut String,
    options: &TextOptions,
    style: &Style,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    if let Some(size) = &options.size {
        write!(
            code,
            ".size({} as f32)",
            expr_code(size, env, document, ValueMode::Owned)?
        )
        .unwrap();
    } else if let Some(size) = style.text_size {
        write!(code, ".size({size})").unwrap();
    }
    for (length, method) in [(&options.width, "width"), (&options.height, "height")] {
        if let Some(length) = length {
            write!(code, ".{method}({})", length_code(length, env, document)?).unwrap();
        }
    }
    if let Some(line_height) = &options.line_height {
        let line_height = match line_height {
            TextLineHeight::Relative(value) => format!(
                "::iced::widget::text::LineHeight::Relative({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            ),
            TextLineHeight::Absolute(value) => format!(
                "::iced::widget::text::LineHeight::Absolute(({} as f32).into())",
                expr_code(value, env, document, ValueMode::Owned)?
            ),
        };
        write!(code, ".line_height({line_height})").unwrap();
    }
    if let Some(alignment) = options.align_x {
        write!(
            code,
            ".align_x(::iced::widget::text::Alignment::{})",
            text_alignment_code(alignment)
        )
        .unwrap();
    }
    if let Some(alignment) = options.align_y {
        let alignment = match alignment {
            VerticalAlignment::Top => "Top",
            VerticalAlignment::Center => "Center",
            VerticalAlignment::Bottom => "Bottom",
        };
        write!(code, ".align_y(::iced::alignment::Vertical::{alignment})").unwrap();
    }
    if let Some(shaping) = options.shaping {
        write!(
            code,
            ".shaping(::iced::widget::text::Shaping::{})",
            text_shaping_code(shaping)
        )
        .unwrap();
    }
    if let Some(wrapping) = options.wrapping {
        write!(
            code,
            ".wrapping(::iced::widget::text::Wrapping::{})",
            text_wrapping_code(wrapping)
        )
        .unwrap();
    }
    if let Some(font) = &options.font {
        let font = font_preset_code(font, document)?;
        if style.bold {
            write!(
                code,
                ".font(::iced::Font {{ weight: ::iced::font::Weight::Bold, ..{font} }})"
            )
            .unwrap();
        } else {
            write!(code, ".font({font})").unwrap();
        }
    } else if style.bold {
        code.push_str(
            ".font(::iced::Font { weight: ::iced::font::Weight::Bold, ..::iced::Font::DEFAULT })",
        );
    }
    if let Some(style) = &options.custom_style {
        let function = document
            .functions
            .iter()
            .find(|item| item.name == style.function && item.kind == ExternKind::TextStyle)
            .expect("checker validates text style");
        let args = style
            .args
            .iter()
            .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
            .collect::<Result<Vec<_>, _>>()?;
        write!(
            code,
            ".style(move |__theme| {}(__theme{}))",
            function.rust_path,
            args.iter()
                .map(|arg| format!(", {arg}"))
                .collect::<String>()
        )
        .unwrap();
    }
    Ok(())
}

fn append_bool_control_options(
    code: &mut String,
    options: &BoolControlOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
    toggler: bool,
) -> Result<(), Error> {
    for (value, method) in [
        (&options.size, "size"),
        (&options.spacing, "spacing"),
        (&options.text_size, "text_size"),
    ] {
        if let Some(value) = value {
            write!(
                code,
                ".{method}({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
    }
    if let Some(width) = &options.width {
        write!(code, ".width({})", length_code(width, env, document)?).unwrap();
    }
    if let Some(height) = &options.line_height {
        write!(
            code,
            ".text_line_height(::iced::widget::text::LineHeight::Relative({} as f32))",
            expr_code(height, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(shaping) = options.shaping {
        write!(
            code,
            ".text_shaping(::iced::widget::text::Shaping::{})",
            text_shaping_code(shaping)
        )
        .unwrap();
    }
    if let Some(wrapping) = options.wrapping {
        write!(
            code,
            ".text_wrapping(::iced::widget::text::Wrapping::{})",
            text_wrapping_code(wrapping)
        )
        .unwrap();
    }
    if let Some(font) = &options.font {
        write!(code, ".font({})", font_preset_code(font, document)?).unwrap();
    }
    if toggler {
        if let Some(alignment) = options.alignment {
            write!(
                code,
                ".text_alignment(::iced::widget::text::Alignment::{})",
                text_alignment_code(alignment)
            )
            .unwrap();
        }
    } else if let Some(icon) = options.icon {
        let size = options.icon_size.as_ref().map_or_else(
            || Ok("None".to_owned()),
            |value| {
                Ok::<_, Error>(format!(
                    "Some(({} as f32).into())",
                    expr_code(value, env, document, ValueMode::Owned)?
                ))
            },
        )?;
        let line_height = if let Some(value) = &options.icon_line_height {
            format!(
                "::iced::widget::text::LineHeight::Relative({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            )
        } else {
            "::iced::widget::text::LineHeight::default()".to_owned()
        };
        let shaping = options.icon_shaping.map_or("Auto", text_shaping_code);
        write!(
            code,
            ".icon(::iced::widget::checkbox::Icon {{ font: ::iced::Font::DEFAULT, code_point: {icon:?}, size: {size}, line_height: {line_height}, shaping: ::iced::widget::text::Shaping::{shaping} }})"
        )
        .unwrap();
    }
    Ok(())
}

fn checkbox_style_code(
    styles: &CheckboxStyleSet,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = styles
        .custom
        .as_ref()
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::CheckboxStyle)
                .expect("checker validates checkbox style");
            let args = style
                .args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(format!(
                "{}(__theme, __status{})",
                function.rust_path,
                args.iter()
                    .map(|arg| format!(", {arg}"))
                    .collect::<String>()
            ))
        })
        .transpose()?;
    let preset = match styles.preset {
        CheckboxStylePreset::Primary => "primary",
        CheckboxStylePreset::Secondary => "secondary",
        CheckboxStylePreset::Success => "success",
        CheckboxStylePreset::Danger => "danger",
    };
    let overrides = [
        ("Active", true, &styles.active_checked),
        ("Active", false, &styles.active_unchecked),
        ("Hovered", true, &styles.hovered_checked),
        ("Hovered", false, &styles.hovered_unchecked),
        ("Disabled", true, &styles.disabled_checked),
        ("Disabled", false, &styles.disabled_unchecked),
    ];
    if overrides.iter().all(|(_, _, style)| style.is_none()) {
        return Ok(if let Some(custom) = custom {
            format!(".style(move |__theme, __status| {custom})")
        } else if styles.preset == CheckboxStylePreset::Primary {
            String::new()
        } else {
            format!(".style(::iced::widget::checkbox::{preset})")
        });
    }

    let base =
        custom.unwrap_or_else(|| format!("::iced::widget::checkbox::{preset}(__theme, __status)"));
    let mut code =
        format!(".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{");
    for (status, checked, style) in overrides {
        let Some(style) = style else { continue };
        write!(
            code,
            " ::iced::widget::checkbox::Status::{status} {{ is_checked: {checked} }} => {{"
        )
        .unwrap();
        if let Some(background) = &style.background {
            write!(
                code,
                " __style.background = {};",
                background_code(background, env, document)?
            )
            .unwrap();
        }
        if let Some(color) = &style.icon_color {
            write!(
                code,
                " __style.icon_color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(color) = &style.text_color {
            write!(
                code,
                " __style.text_color = ::std::option::Option::Some({});",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(color) = &style.border_color {
            write!(
                code,
                " __style.border.color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(width) = &style.border_width {
            write!(
                code,
                " __style.border.width = {} as f32;",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(radius) = radius_code(
            style.radius.as_ref(),
            [
                style.radius_top_left.as_ref(),
                style.radius_top_right.as_ref(),
                style.radius_bottom_right.as_ref(),
                style.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )? {
            write!(code, " __style.border.radius = {radius};").unwrap();
        }
        code.push_str(" }");
    }
    code.push_str(" _ => {} } __style })");
    Ok(code)
}

fn toggler_style_code(
    styles: &TogglerStyleSet,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = styles
        .custom
        .as_ref()
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::TogglerStyle)
                .expect("checker validates toggler style");
            let args = style
                .args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(format!(
                "{}(__theme, __status{})",
                function.rust_path,
                args.iter()
                    .map(|arg| format!(", {arg}"))
                    .collect::<String>()
            ))
        })
        .transpose()?;
    let overrides = [
        ("Active", true, &styles.active_checked),
        ("Active", false, &styles.active_unchecked),
        ("Hovered", true, &styles.hovered_checked),
        ("Hovered", false, &styles.hovered_unchecked),
        ("Disabled", true, &styles.disabled_checked),
        ("Disabled", false, &styles.disabled_unchecked),
    ];
    if overrides.iter().all(|(_, _, style)| style.is_none()) {
        return Ok(custom
            .map(|custom| format!(".style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }

    let base =
        custom.unwrap_or_else(|| "::iced::widget::toggler::default(__theme, __status)".to_owned());
    let mut code =
        format!(".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{");
    for (status, checked, style) in overrides {
        let Some(style) = style else { continue };
        write!(
            code,
            " ::iced::widget::toggler::Status::{status} {{ is_toggled: {checked} }} => {{"
        )
        .unwrap();
        if let Some(background) = &style.background {
            write!(
                code,
                " __style.background = {};",
                background_code(background, env, document)?
            )
            .unwrap();
        }
        if let Some(color) = &style.background_border_color {
            write!(
                code,
                " __style.background_border_color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(width) = &style.background_border_width {
            write!(
                code,
                " __style.background_border_width = {} as f32;",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(foreground) = &style.foreground {
            write!(
                code,
                " __style.foreground = {};",
                background_code(foreground, env, document)?
            )
            .unwrap();
        }
        if let Some(color) = &style.foreground_border_color {
            write!(
                code,
                " __style.foreground_border_color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(width) = &style.foreground_border_width {
            write!(
                code,
                " __style.foreground_border_width = {} as f32;",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(color) = &style.text_color {
            write!(
                code,
                " __style.text_color = ::std::option::Option::Some({});",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(radius) = radius_code(
            style.radius.as_ref(),
            [
                style.radius_top_left.as_ref(),
                style.radius_top_right.as_ref(),
                style.radius_bottom_right.as_ref(),
                style.radius_bottom_left.as_ref(),
            ],
            env,
            document,
        )? {
            write!(
                code,
                " __style.border_radius = ::std::option::Option::Some({radius});"
            )
            .unwrap();
        }
        if let Some(ratio) = &style.padding_ratio {
            write!(
                code,
                " __style.padding_ratio = {} as f32;",
                expr_code(ratio, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        code.push_str(" }");
    }
    code.push_str(" _ => {} } __style })");
    Ok(code)
}

fn radio_style_code(
    styles: &RadioStyleSet,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = styles
        .custom
        .as_ref()
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::RadioStyle)
                .expect("checker validates radio style");
            let args = style
                .args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(format!(
                "{}(__theme, __status{})",
                function.rust_path,
                args.iter()
                    .map(|arg| format!(", {arg}"))
                    .collect::<String>()
            ))
        })
        .transpose()?;
    let overrides = [
        ("Active", true, &styles.active_selected),
        ("Active", false, &styles.active_unselected),
        ("Hovered", true, &styles.hovered_selected),
        ("Hovered", false, &styles.hovered_unselected),
    ];
    if overrides.iter().all(|(_, _, style)| style.is_none()) {
        return Ok(custom
            .map(|custom| format!(".style(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }

    let base =
        custom.unwrap_or_else(|| "::iced::widget::radio::default(__theme, __status)".to_owned());
    let mut code =
        format!(".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{");
    for (status, selected, style) in overrides {
        let Some(style) = style else { continue };
        write!(
            code,
            " ::iced::widget::radio::Status::{status} {{ is_selected: {selected} }} => {{"
        )
        .unwrap();
        if let Some(background) = &style.background {
            write!(
                code,
                " __style.background = {};",
                background_code(background, env, document)?
            )
            .unwrap();
        }
        if let Some(color) = &style.dot_color {
            write!(
                code,
                " __style.dot_color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(color) = &style.border_color {
            write!(
                code,
                " __style.border_color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        if let Some(width) = &style.border_width {
            write!(
                code,
                " __style.border_width = {} as f32;",
                expr_code(width, env, document, ValueMode::Owned)?
            )
            .unwrap();
        }
        if let Some(color) = &style.text_color {
            write!(
                code,
                " __style.text_color = ::std::option::Option::Some({});",
                theme_color(document, color)
            )
            .unwrap();
        }
        code.push_str(" }");
    }
    code.push_str(" _ => {} } __style })");
    Ok(code)
}

fn pick_list_handle_code(
    handle: &PickListHandle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    Ok(match handle {
        PickListHandle::Arrow { size } => {
            let size = size.as_ref().map_or_else(
                || Ok("::std::option::Option::None".to_owned()),
                |value| {
                    Ok::<_, Error>(format!(
                        "::std::option::Option::Some(({} as f32).into())",
                        expr_code(value, env, document, ValueMode::Owned)?
                    ))
                },
            )?;
            format!("::iced::widget::pick_list::Handle::Arrow {{ size: {size} }}")
        }
        PickListHandle::Static(icon) => format!(
            "::iced::widget::pick_list::Handle::Static({})",
            pick_list_icon_code(icon, env, document)?
        ),
        PickListHandle::Dynamic { closed, open } => format!(
            "::iced::widget::pick_list::Handle::Dynamic {{ closed: {}, open: {} }}",
            pick_list_icon_code(closed, env, document)?,
            pick_list_icon_code(open, env, document)?
        ),
        PickListHandle::None => "::iced::widget::pick_list::Handle::None".to_owned(),
    })
}

fn pick_list_icon_code(
    icon: &PickListIcon,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let font = icon.font.as_ref().map_or_else(
        || Ok("::iced::Font::DEFAULT".to_owned()),
        |font| font_preset_code(font, document),
    )?;
    let size = icon.size.as_ref().map_or_else(
        || Ok("::std::option::Option::None".to_owned()),
        |value| {
            Ok::<_, Error>(format!(
                "::std::option::Option::Some(({} as f32).into())",
                expr_code(value, env, document, ValueMode::Owned)?
            ))
        },
    )?;
    let line_height = icon.line_height.as_ref().map_or_else(
        || Ok("::iced::widget::text::LineHeight::default()".to_owned()),
        |value| {
            Ok::<_, Error>(format!(
                "::iced::widget::text::LineHeight::Relative({} as f32)",
                expr_code(value, env, document, ValueMode::Owned)?
            ))
        },
    )?;
    let shaping = icon.shaping.map_or_else(
        || "::iced::widget::text::Shaping::default()".to_owned(),
        |shaping| {
            format!(
                "::iced::widget::text::Shaping::{}",
                text_shaping_code(shaping)
            )
        },
    );
    Ok(format!(
        "::iced::widget::pick_list::Icon {{ font: {font}, code_point: {:?}, size: {size}, line_height: {line_height}, shaping: {shaping} }}",
        icon.code_point
    ))
}

fn pick_list_style_code(
    options: &PickListOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = options
        .custom_style
        .as_ref()
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::PickListStyle)
                .expect("checker validates pick-list style");
            let args = style
                .args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(format!(
                "{}(__theme, __status{})",
                function.rust_path,
                args.iter()
                    .map(|arg| format!(", {arg}"))
                    .collect::<String>()
            ))
        })
        .transpose()?;
    let overrides = [
        ("Active", &options.style.active),
        ("Hovered", &options.style.hovered),
        ("Opened { is_hovered: false }", &options.style.opened),
        ("Opened { is_hovered: true }", &options.style.opened_hovered),
    ];
    let mut code = String::new();
    if overrides.iter().any(|(_, style)| style.is_some()) {
        let base = custom
            .unwrap_or_else(|| "::iced::widget::pick_list::default(__theme, __status)".to_owned());
        write!(
            code,
            ".style(move |__theme, __status| {{ let mut __style = {base}; match __status {{"
        )
        .unwrap();
        for (status, style) in overrides {
            let Some(style) = style else { continue };
            write!(code, " ::iced::widget::pick_list::Status::{status} => {{").unwrap();
            append_select_surface_overrides(&mut code, &style.options, env, document, false)?;
            if let Some(color) = &style.placeholder_color {
                write!(
                    code,
                    " __style.placeholder_color = {};",
                    theme_color(document, color)
                )
                .unwrap();
            }
            if let Some(color) = &style.handle_color {
                write!(
                    code,
                    " __style.handle_color = {};",
                    theme_color(document, color)
                )
                .unwrap();
            }
            code.push_str(" }");
        }
        code.push_str(" _ => {} } __style })");
    } else if let Some(custom) = custom {
        write!(code, ".style(move |__theme, __status| {custom})").unwrap();
    }
    code.push_str(&menu_style_code(
        options.menu_style.as_deref(),
        options.custom_menu_style.as_ref(),
        env,
        document,
    )?);
    Ok(code)
}

fn menu_style_code(
    style: Option<&MenuStyleOptions>,
    custom: Option<&ExternCall>,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let custom = custom
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::MenuStyle)
                .expect("checker validates menu style");
            let args = style
                .args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(format!(
                "{}(__theme{})",
                function.rust_path,
                args.iter()
                    .map(|arg| format!(", {arg}"))
                    .collect::<String>()
            ))
        })
        .transpose()?;
    let Some(style) = style else {
        return Ok(custom
            .map(|custom| format!(".menu_style(move |__theme| {custom})"))
            .unwrap_or_default());
    };
    let base = custom.unwrap_or_else(|| "::iced::overlay::menu::default(__theme)".to_owned());
    let mut code = String::new();
    write!(
        code,
        ".menu_style(move |__theme| {{ let mut __style = {base};"
    )
    .unwrap();
    append_select_surface_overrides(&mut code, &style.options, env, document, true)?;
    if let Some(color) = &style.selected_text_color {
        write!(
            code,
            " __style.selected_text_color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(background) = &style.selected_background {
        write!(
            code,
            " __style.selected_background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    code.push_str(" __style })");
    Ok(code)
}

fn append_select_surface_overrides(
    code: &mut String,
    options: &ContainerStyleOptions,
    env: &HashMap<String, Binding>,
    document: &Document,
    shadow: bool,
) -> Result<(), Error> {
    if let Some(background) = &options.background {
        write!(
            code,
            " __style.background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(color) = &options.text_color {
        write!(
            code,
            " __style.text_color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(color) = &options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(width) = &options.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = radius_code(
        options.radius.as_ref(),
        [
            options.radius_top_left.as_ref(),
            options.radius_top_right.as_ref(),
            options.radius_bottom_right.as_ref(),
            options.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    if shadow {
        if let Some(color) = &options.shadow_color {
            write!(
                code,
                " __style.shadow.color = {};",
                theme_color(document, color)
            )
            .unwrap();
        }
        for (value, field) in [
            (&options.shadow_x, "__style.shadow.offset.x"),
            (&options.shadow_y, "__style.shadow.offset.y"),
            (&options.shadow_blur, "__style.shadow.blur_radius"),
        ] {
            if let Some(value) = value {
                write!(
                    code,
                    " {field} = {} as f32;",
                    expr_code(value, env, document, ValueMode::Owned)?
                )
                .unwrap();
            }
        }
    }
    Ok(())
}

fn text_input_icon_code(
    icon: &TextInputIcon,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let font = icon.font.as_ref().map_or_else(
        || Ok("::iced::Font::DEFAULT".to_owned()),
        |font| font_preset_code(font, document),
    )?;
    let size = icon.size.as_ref().map_or_else(
        || Ok("::std::option::Option::None".to_owned()),
        |value| {
            Ok::<_, Error>(format!(
                "::std::option::Option::Some(({} as f32).into())",
                expr_code(value, env, document, ValueMode::Owned)?
            ))
        },
    )?;
    let spacing = icon.spacing.as_ref().map_or_else(
        || Ok("0.0".to_owned()),
        |value| expr_code(value, env, document, ValueMode::Owned),
    )?;
    let side = match icon.side {
        IconSide::Left => "Left",
        IconSide::Right => "Right",
    };
    Ok(format!(
        "::iced::widget::text_input::Icon {{ font: {font}, code_point: {:?}, size: {size}, spacing: {spacing} as f32, side: ::iced::widget::text_input::Side::{side} }}",
        icon.code_point
    ))
}

fn text_input_style_code(
    styles: &TextInputStyleSet,
    custom: Option<&ExternCall>,
    utilities: Option<&Style>,
    env: &HashMap<String, Binding>,
    document: &Document,
    method: &str,
    widget: &str,
) -> Result<String, Error> {
    let custom_kind = if widget == "text_editor" {
        ExternKind::EditorStyle
    } else {
        ExternKind::InputStyle
    };
    let custom = custom
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == custom_kind)
                .expect("checker validates input style");
            let args = style
                .args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(format!(
                "{}(__theme, __status{})",
                function.rust_path,
                args.iter()
                    .map(|arg| format!(", {arg}"))
                    .collect::<String>()
            ))
        })
        .transpose()?;
    let has_utilities = utilities.is_some_and(|style| {
        style.background.is_some()
            || style.border_color.is_some()
            || style.border_width != 0
            || style.radius != 0
            || style.focus_border_color.is_some()
    });
    let overrides = [
        ("Active", &styles.active),
        ("Hovered", &styles.hovered),
        ("Focused { is_hovered: false }", &styles.focused),
        ("Focused { is_hovered: true }", &styles.focused_hovered),
        ("Disabled", &styles.disabled),
    ];
    let has_overrides = overrides.iter().any(|(_, style)| style.is_some());
    if !has_overrides && !has_utilities {
        return Ok(custom
            .map(|custom| format!(".{method}(move |__theme, __status| {custom})"))
            .unwrap_or_default());
    }
    let base =
        custom.unwrap_or_else(|| format!("::iced::widget::{widget}::default(__theme, __status)"));
    let mut code = format!(".{method}(move |__theme, __status| {{ let mut __style = {base};");
    if let Some(style) = utilities.filter(|_| has_utilities) {
        if let Some(background) = &style.background {
            write!(
                code,
                " __style.background = {}.into();",
                theme_color(document, background)
            )
            .unwrap();
        }
        if let Some(border) = &style.border_color {
            write!(
                code,
                " __style.border.color = {};",
                theme_color(document, border)
            )
            .unwrap();
        }
        if style.border_width != 0 {
            write!(code, " __style.border.width = {}.0;", style.border_width).unwrap();
        }
        if style.radius != 0 {
            write!(code, " __style.border.radius = {}.0.into();", style.radius).unwrap();
        }
        if let Some(focus) = &style.focus_border_color {
            write!(
                code,
                " if matches!(__status, ::iced::widget::text_input::Status::Focused {{ .. }}) {{ __style.border.color = {}; }}",
                theme_color(document, focus)
            )
            .unwrap();
        }
    }
    if has_overrides {
        code.push_str(" match __status {");
        for (status, style) in overrides {
            let Some(style) = style else { continue };
            write!(code, " ::iced::widget::{widget}::Status::{status} => {{").unwrap();
            append_text_input_style_overrides(&mut code, style, env, document)?;
            code.push_str(" }");
        }
        code.push_str(" _ => {} }");
    }
    code.push_str(" __style })");
    Ok(code)
}

fn append_text_input_style_overrides(
    code: &mut String,
    style: &TextInputStatusStyle,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(), Error> {
    if let Some(background) = &style.options.background {
        write!(
            code,
            " __style.background = {};",
            background_code(background, env, document)?
        )
        .unwrap();
    }
    if let Some(color) = &style.options.border_color {
        write!(
            code,
            " __style.border.color = {};",
            theme_color(document, color)
        )
        .unwrap();
    }
    if let Some(width) = &style.options.border_width {
        write!(
            code,
            " __style.border.width = {} as f32;",
            expr_code(width, env, document, ValueMode::Owned)?
        )
        .unwrap();
    }
    if let Some(radius) = radius_code(
        style.options.radius.as_ref(),
        [
            style.options.radius_top_left.as_ref(),
            style.options.radius_top_right.as_ref(),
            style.options.radius_bottom_right.as_ref(),
            style.options.radius_bottom_left.as_ref(),
        ],
        env,
        document,
    )? {
        write!(code, " __style.border.radius = {radius};").unwrap();
    }
    for (color, field) in [
        (&style.icon_color, "__style.icon"),
        (&style.placeholder_color, "__style.placeholder"),
        (&style.value_color, "__style.value"),
        (&style.selection_color, "__style.selection"),
    ] {
        if let Some(color) = color {
            write!(code, " {field} = {};", theme_color(document, color)).unwrap();
        }
    }
    Ok(())
}

fn text_shaping_code(shaping: TextShaping) -> &'static str {
    match shaping {
        TextShaping::Auto => "Auto",
        TextShaping::Basic => "Basic",
        TextShaping::Advanced => "Advanced",
    }
}

fn text_wrapping_code(wrapping: TextWrapping) -> &'static str {
    match wrapping {
        TextWrapping::None => "None",
        TextWrapping::Word => "Word",
        TextWrapping::Glyph => "Glyph",
        TextWrapping::WordOrGlyph => "WordOrGlyph",
    }
}

fn font_preset_code(font: &FontPreset, document: &Document) -> Result<String, Error> {
    match font {
        FontPreset::Default => Ok("::iced::Font::DEFAULT".into()),
        FontPreset::Monospace => Ok("::iced::Font::MONOSPACE".into()),
        FontPreset::Named(name) => document
            .fonts
            .iter()
            .find(|font| font.name == *name)
            .map(font_decl_code)
            .ok_or_else(|| Error::new("E171", &Span::line(1), format!("unknown font `{name}`"))),
    }
}

fn font_decl_code(font: &FontDecl) -> String {
    let family = match &font.family {
        FontFamily::Named(name) => format!("::iced::font::Family::Name({})", rust_string(name)),
        FontFamily::Serif => "::iced::font::Family::Serif".into(),
        FontFamily::SansSerif => "::iced::font::Family::SansSerif".into(),
        FontFamily::Cursive => "::iced::font::Family::Cursive".into(),
        FontFamily::Fantasy => "::iced::font::Family::Fantasy".into(),
        FontFamily::Monospace => "::iced::font::Family::Monospace".into(),
    };
    let weight = match font.weight {
        FontWeight::Thin => "Thin",
        FontWeight::ExtraLight => "ExtraLight",
        FontWeight::Light => "Light",
        FontWeight::Normal => "Normal",
        FontWeight::Medium => "Medium",
        FontWeight::Semibold => "Semibold",
        FontWeight::Bold => "Bold",
        FontWeight::ExtraBold => "ExtraBold",
        FontWeight::Black => "Black",
    };
    let stretch = match font.stretch {
        FontStretch::UltraCondensed => "UltraCondensed",
        FontStretch::ExtraCondensed => "ExtraCondensed",
        FontStretch::Condensed => "Condensed",
        FontStretch::SemiCondensed => "SemiCondensed",
        FontStretch::Normal => "Normal",
        FontStretch::SemiExpanded => "SemiExpanded",
        FontStretch::Expanded => "Expanded",
        FontStretch::ExtraExpanded => "ExtraExpanded",
        FontStretch::UltraExpanded => "UltraExpanded",
    };
    let style = match font.style {
        FontStyle::Normal => "Normal",
        FontStyle::Italic => "Italic",
        FontStyle::Oblique => "Oblique",
    };
    format!(
        "::iced::Font {{ family: {family}, weight: ::iced::font::Weight::{weight}, stretch: ::iced::font::Stretch::{stretch}, style: ::iced::font::Style::{style} }}"
    )
}

fn text_alignment_code(alignment: TextAlignment) -> &'static str {
    match alignment {
        TextAlignment::Default => "Default",
        TextAlignment::Left => "Left",
        TextAlignment::Center => "Center",
        TextAlignment::Right => "Right",
        TextAlignment::Justified => "Justified",
    }
}

fn mouse_interaction_code(interaction: MouseInteraction) -> &'static str {
    match interaction {
        MouseInteraction::None => "None",
        MouseInteraction::Hidden => "Hidden",
        MouseInteraction::Idle => "Idle",
        MouseInteraction::ContextMenu => "ContextMenu",
        MouseInteraction::Help => "Help",
        MouseInteraction::Pointer => "Pointer",
        MouseInteraction::Progress => "Progress",
        MouseInteraction::Wait => "Wait",
        MouseInteraction::Cell => "Cell",
        MouseInteraction::Crosshair => "Crosshair",
        MouseInteraction::Text => "Text",
        MouseInteraction::Alias => "Alias",
        MouseInteraction::Copy => "Copy",
        MouseInteraction::Move => "Move",
        MouseInteraction::NoDrop => "NoDrop",
        MouseInteraction::NotAllowed => "NotAllowed",
        MouseInteraction::Grab => "Grab",
        MouseInteraction::Grabbing => "Grabbing",
        MouseInteraction::ResizingHorizontally => "ResizingHorizontally",
        MouseInteraction::ResizingVertically => "ResizingVertically",
        MouseInteraction::ResizingDiagonallyUp => "ResizingDiagonallyUp",
        MouseInteraction::ResizingDiagonallyDown => "ResizingDiagonallyDown",
        MouseInteraction::ResizingColumn => "ResizingColumn",
        MouseInteraction::ResizingRow => "ResizingRow",
        MouseInteraction::AllScroll => "AllScroll",
        MouseInteraction::ZoomIn => "ZoomIn",
        MouseInteraction::ZoomOut => "ZoomOut",
    }
}

fn binding_variant(binding: &str) -> String {
    format!("__Bind{}", pascal(binding))
}

fn editor_variant(binding: &str) -> String {
    format!("__Edit{}", pascal(binding))
}

fn controlled_state_name(code: &str, widget: &str, span: &Span) -> Result<String, Error> {
    let Some(name) = code.strip_prefix("self.") else {
        return Err(Error::new(
            "E139",
            span,
            format!("{widget} binding must resolve to an app state"),
        ));
    };
    if name.contains('.') {
        return Err(Error::new(
            "E139",
            span,
            format!("{widget} binding must resolve to one app state"),
        ));
    }
    Ok(name.to_owned())
}

fn id_code(
    id: &Id,
    scope: &str,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    if let Some(key) = &id.key {
        Ok(format!(
            "format!(\"{{}}/{}({{}})\", {scope}, {})",
            id.name,
            expr_code(key, env, document, ValueMode::Borrowed)?
        ))
    } else {
        Ok(format!("format!(\"{{}}/{}\", {scope})", id.name))
    }
}

fn widget_target_code(
    target: &WidgetTarget,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    if target.segments.iter().all(|segment| segment.key.is_none()) {
        return Ok(format!(
            "::iced::widget::Id::new({})",
            rust_string(&format!(
                "{}/{}",
                document.app,
                target
                    .segments
                    .iter()
                    .map(|segment| segment.name.as_str())
                    .collect::<Vec<_>>()
                    .join("/")
            ))
        ));
    }
    let mut scope = rust_string(&document.app);
    for segment in &target.segments {
        scope = id_code(segment, &scope, env, document)?;
    }
    Ok(format!("::iced::widget::Id::from({scope})"))
}

fn widget_selector_code(
    selector: &WidgetSelector,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<(String, Option<&'static str>), Error> {
    match selector {
        WidgetSelector::Id(target) => Ok((
            format!(
                "::iced::widget::selector::id({})",
                widget_target_code(target, env, document)?
            ),
            Some("__ice_widget_target_from_target"),
        )),
        WidgetSelector::Text(value) => Ok((
            expr_code(value, env, document, ValueMode::Owned)?,
            Some("__ice_widget_target_from_text"),
        )),
        WidgetSelector::Point { x, y } => Ok((
            format!(
                "::iced::Point::new(({}) as f32, ({}) as f32)",
                expr_code(x, env, document, ValueMode::Owned)?,
                expr_code(y, env, document, ValueMode::Owned)?
            ),
            Some("__ice_widget_target_from_target"),
        )),
        WidgetSelector::Focused => Ok((
            "::iced::widget::selector::is_focused()".into(),
            Some("__ice_widget_target_from_target"),
        )),
        WidgetSelector::Extern { function, args } => {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == *function && item.kind == ExternKind::Selector)
                .expect("checker validates selectors");
            Ok((
                format!(
                    "{}({})",
                    function.rust_path,
                    args.iter()
                        .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                        .collect::<Result<Vec<_>, _>>()?
                        .join(", ")
                ),
                None,
            ))
        }
    }
}

#[derive(Default)]
struct Style {
    width_fill: bool,
    height_fill: bool,
    max_width: Option<u16>,
    padding: [u16; 4],
    gap: Option<u16>,
    items_center: bool,
    self_center: bool,
    text_size: Option<u16>,
    bold: bool,
    text_color: Option<String>,
    background: Option<String>,
    hover_background: Option<String>,
    pressed_background: Option<String>,
    border_color: Option<String>,
    focus_border_color: Option<String>,
    border_width: u16,
    radius: u16,
    disabled_opacity: Option<f32>,
}

impl Style {
    fn parse(tokens: &[String], document: &Document) -> Self {
        let mut style = Self::default();
        for token in tokens {
            let (variant, utility) = token
                .split_once(':')
                .map_or((None, token.as_str()), |(a, b)| (Some(a), b));
            if variant == Some("hover") && utility.starts_with("bg-") {
                style.hover_background = Some(utility[3..].into());
                continue;
            }
            if variant == Some("pressed") && utility.starts_with("bg-") {
                style.pressed_background = Some(utility[3..].into());
                continue;
            }
            if variant == Some("focus") && utility.starts_with("border-") {
                style.focus_border_color = Some(utility[7..].into());
                continue;
            }
            if variant == Some("disabled") && utility.starts_with("opacity-") {
                style.disabled_opacity =
                    utility[8..].parse::<f32>().ok().map(|value| value / 100.0);
                continue;
            }
            if variant.is_some() {
                continue;
            }
            match utility {
                "w-full" => style.width_fill = true,
                "h-full" => style.height_fill = true,
                "max-w-sm" => style.max_width = Some(384),
                "max-w-md" => style.max_width = Some(448),
                "max-w-lg" => style.max_width = Some(512),
                "max-w-xl" => style.max_width = Some(576),
                "max-w-2xl" => style.max_width = Some(672),
                "items-center" => style.items_center = true,
                "self-center" => style.self_center = true,
                "text-xs" => style.text_size = Some(12),
                "text-sm" => style.text_size = Some(14),
                "text-base" => style.text_size = Some(16),
                "text-lg" => style.text_size = Some(18),
                "text-xl" => style.text_size = Some(20),
                "text-2xl" => style.text_size = Some(24),
                "font-bold" => style.bold = true,
                "border" => style.border_width = 1,
                "border-2" => style.border_width = 2,
                "rounded-sm" => style.radius = 2,
                "rounded" | "rounded-md" => style.radius = 6,
                "rounded-lg" => style.radius = 10,
                "rounded-full" => style.radius = 999,
                _ if utility.starts_with("gap-") => style.gap = spacing(&utility[4..]),
                _ if utility.starts_with("p-") => {
                    if let Some(value) = spacing(&utility[2..]) {
                        style.padding = [value; 4];
                    }
                }
                _ if utility.starts_with("px-") => {
                    if let Some(value) = spacing(&utility[3..]) {
                        style.padding[1] = value;
                        style.padding[3] = value;
                    }
                }
                _ if utility.starts_with("py-") => {
                    if let Some(value) = spacing(&utility[3..]) {
                        style.padding[0] = value;
                        style.padding[2] = value;
                    }
                }
                _ if utility.starts_with("bg-") => style.background = Some(utility[3..].into()),
                _ if utility.starts_with("text-") && document.theme.contains_key(&utility[5..])
                    || matches!(utility, "text-white" | "text-black") =>
                {
                    style.text_color = Some(utility[5..].into())
                }
                _ if utility.starts_with("border-") => {
                    style.border_color = Some(utility[7..].into())
                }
                _ => {}
            }
        }
        style
    }

    fn padding_code(&self) -> Option<String> {
        (self.padding != [0; 4]).then(|| {
            format!(
                "::iced::Padding {{ top: {}.0, right: {}.0, bottom: {}.0, left: {}.0 }}",
                self.padding[0], self.padding[1], self.padding[2], self.padding[3]
            )
        })
    }
}

fn append_size(code: &mut String, style: &Style) {
    if style.width_fill {
        code.push_str(".width(::iced::Fill)");
    }
    if style.height_fill {
        code.push_str(".height(::iced::Fill)");
    }
}

fn container_style_code(style: &Style, document: &Document) -> String {
    container_style_value(style, document)
        .map(|style| format!(".style(|_| {style})"))
        .unwrap_or_default()
}

fn container_style_value(style: &Style, document: &Document) -> Option<String> {
    if style.background.is_none() && style.border_width == 0 && style.text_color.is_none() {
        return None;
    }
    let background = style
        .background
        .as_ref()
        .map(|color| format!("Some({}.into())", theme_color(document, color)))
        .unwrap_or_else(|| "None".into());
    let text = style
        .text_color
        .as_ref()
        .map(|color| format!("Some({})", theme_color(document, color)))
        .unwrap_or_else(|| "None".into());
    let border = style
        .border_color
        .as_ref()
        .map(|color| theme_color(document, color))
        .unwrap_or_else(|| "::iced::Color::TRANSPARENT".into());
    Some(format!(
        "::iced::widget::container::Style {{ background: {background}, text_color: {text}, border: ::iced::Border {{ color: {border}, width: {}.0, radius: {}.0.into() }}, ..::iced::widget::container::Style::default() }}",
        style.border_width, style.radius
    ))
}

fn button_style_code(
    style: &Style,
    typed: &ButtonStyleSet,
    env: &HashMap<String, Binding>,
    document: &Document,
) -> Result<String, Error> {
    let has_utilities = style.background.is_some()
        || style.hover_background.is_some()
        || style.pressed_background.is_some()
        || style.text_color.is_some()
        || style.radius != 0
        || style.disabled_opacity.is_some();
    let has_typed = typed.active.is_some()
        || typed.hovered.is_some()
        || typed.pressed.is_some()
        || typed.disabled.is_some();
    let custom = typed
        .custom
        .as_ref()
        .map(|style| {
            let function = document
                .functions
                .iter()
                .find(|item| item.name == style.function && item.kind == ExternKind::ButtonStyle)
                .expect("checker validates button style");
            let args = style
                .args
                .iter()
                .map(|arg| expr_code(arg, env, document, ValueMode::Owned))
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, Error>(format!(
                "{}(__theme, __status{})",
                function.rust_path,
                args.iter()
                    .map(|arg| format!(", {arg}"))
                    .collect::<String>()
            ))
        })
        .transpose()?;
    let preset = match typed.preset {
        ButtonStylePreset::Primary => "primary",
        ButtonStylePreset::Secondary => "secondary",
        ButtonStylePreset::Success => "success",
        ButtonStylePreset::Warning => "warning",
        ButtonStylePreset::Danger => "danger",
        ButtonStylePreset::Text => "text",
        ButtonStylePreset::Background => "background",
        ButtonStylePreset::Subtle => "subtle",
    };
    if !has_utilities && !has_typed {
        return Ok(if let Some(custom) = custom {
            format!(".style(move |__theme, __status| {custom})")
        } else if typed.preset == ButtonStylePreset::Primary {
            String::new()
        } else {
            format!(".style(::iced::widget::button::{preset})")
        });
    }

    let base =
        custom.unwrap_or_else(|| format!("::iced::widget::button::{preset}(__theme, __status)"));
    let mut code = format!(".style(move |__theme, __status| {{ let mut __style = {base};");
    if has_utilities {
        let normal = style
            .background
            .as_ref()
            .map(|color| theme_color(document, color));
        let hover = style
            .hover_background
            .as_ref()
            .map(|color| theme_color(document, color))
            .or_else(|| normal.clone());
        let pressed = style
            .pressed_background
            .as_ref()
            .map(|color| theme_color(document, color))
            .or_else(|| hover.clone())
            .or_else(|| normal.clone());
        let option = |color: Option<String>| {
            color.map_or_else(|| "None".into(), |color| format!("Some({color})"))
        };
        write!(
            code,
            " let __background: Option<::iced::Color> = match __status {{ ::iced::widget::button::Status::Hovered => {}, ::iced::widget::button::Status::Pressed => {}, ::iced::widget::button::Status::Disabled => {}, _ => {} }}; if let Some(__background) = __background {{ __style.background = Some(::iced::Background::Color(__background)); }}",
            option(hover),
            option(pressed),
            option(normal.clone()),
            option(normal),
        )
        .unwrap();
        if let Some(text) = &style.text_color {
            write!(
                code,
                " __style.text_color = {};",
                theme_color(document, text)
            )
            .unwrap();
        }
        if style.radius > 0 {
            write!(code, " __style.border.radius = {}.0.into();", style.radius).unwrap();
        }
        if style.background.is_some()
            || style.text_color.is_some()
            || style.disabled_opacity.is_some()
        {
            let disabled = style.disabled_opacity.unwrap_or(0.5);
            write!(code, " if matches!(__status, ::iced::widget::button::Status::Disabled) {{ __style.text_color.a *= {disabled}; if let Some(::iced::Background::Color(mut __color)) = __style.background {{ __color.a *= {disabled}; __style.background = Some(::iced::Background::Color(__color)); }} }}").unwrap();
        }
    }
    if has_typed {
        code.push_str(" match __status {");
        for (variant, status) in [
            ("Active", &typed.active),
            ("Hovered", &typed.hovered),
            ("Pressed", &typed.pressed),
            ("Disabled", &typed.disabled),
        ] {
            write!(code, " ::iced::widget::button::Status::{variant} => {{").unwrap();
            if let Some(status) = status {
                append_surface_style_overrides(&mut code, &status.options, env, document)?;
                if let Some(color) = &status.options.text_color {
                    write!(
                        code,
                        " __style.text_color = {};",
                        theme_color(document, color)
                    )
                    .unwrap();
                }
            }
            code.push_str(" }");
        }
        code.push_str(" }");
    }
    code.push_str(" __style })");
    Ok(code)
}

fn theme_color(document: &Document, token: &str) -> String {
    let (name, opacity) = token
        .split_once('/')
        .map_or((token, None), |(name, opacity)| {
            (name, opacity.parse::<u8>().ok())
        });
    let value = match name {
        "white" => "#ffffff",
        "black" => "#000000",
        "transparent" => "#00000000",
        name => document
            .theme
            .get(name)
            .map(String::as_str)
            .unwrap_or("#000000"),
    };
    color_code(value, opacity)
}

fn theme_preset_code(preset: &ThemePreset) -> String {
    match preset {
        ThemePreset::Default => "::std::option::Option::None".into(),
        ThemePreset::App => "::std::option::Option::Some(Self::__app_theme())".into(),
        ThemePreset::BuiltIn(name) => format!(
            "::std::option::Option::Some(::iced::Theme::{})",
            pascal(name)
        ),
    }
}

fn qr_data_code(qr: &QrData) -> String {
    let module = "::iced::widget::qr_code";
    let data = match &qr.data {
        QrPayload::Text(value) => rust_string(value),
        QrPayload::Bytes(values) => format!(
            "&[{}][..]",
            values
                .iter()
                .map(|value| format!("0x{value:02x}u8"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    let correction = |value| match value {
        QrCorrection::Low => format!("{module}::ErrorCorrection::Low"),
        QrCorrection::Medium => format!("{module}::ErrorCorrection::Medium"),
        QrCorrection::Quartile => format!("{module}::ErrorCorrection::Quartile"),
        QrCorrection::High => format!("{module}::ErrorCorrection::High"),
    };
    let constructor = if let Some(version) = qr.version {
        let version = match version {
            QrVersion::Normal(value) => format!("{module}::Version::Normal({value})"),
            QrVersion::Micro(value) => format!("{module}::Version::Micro({value})"),
        };
        let correction = correction(qr.correction.unwrap_or(QrCorrection::Medium));
        format!("{module}::Data::with_version({data}, {version}, {correction})")
    } else if let Some(value) = qr.correction {
        format!(
            "{module}::Data::with_error_correction({data}, {})",
            correction(value)
        )
    } else {
        format!("{module}::Data::new({data})")
    };
    format!("{constructor}.expect(\"invalid qr data `{}`\")", qr.name)
}

fn color_code(value: &str, opacity: Option<u8>) -> String {
    let hex = value.trim_start_matches('#');
    let byte = |range: std::ops::Range<usize>| u8::from_str_radix(&hex[range], 16).unwrap_or(0);
    let alpha = opacity
        .map(|value| value as f32 / 100.0)
        .or_else(|| (hex.len() == 8).then(|| byte(6..8) as f32 / 255.0))
        .unwrap_or(1.0);
    format!(
        "::iced::Color::from_rgba8({}, {}, {}, {alpha:.6})",
        byte(0..2),
        byte(2..4),
        byte(4..6)
    )
}

fn spacing(value: &str) -> Option<u16> {
    value.parse::<u16>().ok().map(|value| value * 4)
}

fn rust_string(value: &str) -> String {
    format!("{value:?}")
}

fn rust_f64(value: f64) -> String {
    format!("{value:?}")
}

fn pascal(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::compile;

    #[test]
    fn lowers_complete_common_application_and_window_settings() {
        let source = r#"app Configured
  title "Configured app"
  theme "dark"
  background "123456"
  text-color "abcdef"
  id "dev.example.configured"
  executor iced::executor::Default
  font "fonts/Brand.ttf"
  font "fonts/Icons.otf"
  default-text-size 15
  antialiasing false
  vsync false
  scale-factor 1.25
  window
    icon-rgba "assets/app.rgba" 2 1
    size 960 720
    maximized true
    fullscreen false
    position 10 -20
    min-size 480 360
    max-size 1920 1080
    visible true
    resizable false
    closeable false
    minimizable false
    decorations false
    transparent true
    blur true
    level always-on-top
    exit-on-close-request false
    platform linux
      application-id "dev.example.configured"
      override-redirect true
    platform windows
      drag-and-drop false
      skip-taskbar true
      undecorated-shadow true
      corner round-small
    platform macos
      title-hidden true
      titlebar-transparent true
      fullsize-content-view true
    platform wasm
      target none
state
  ready = false
extern crate::backend
  task seed() -> bool
preset ready
  state
    ready = true
  boot
    task seed() -> seeded _
on seeded(value)
  ready = value
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
view
  text "Configured"
"#;
        let generated = compile(source, "configured.ice").unwrap();
        for expected in [
            ".title(Self::__title)",
            ".theme(Self::__theme).style(Self::__style)",
            "fn __title(&self) -> ::std::string::String",
            "\"dark\" => ::iced::Theme::Dark",
            "fn __style(&self, __theme: &::iced::Theme)",
            "parse::<::iced::Color>()",
            ".executor::<iced::executor::Default>()",
            ".presets([::iced::Preset::new(\"ready\", Self::__preset_0)])",
            "fn __preset_0()",
            "state.ready = true",
            "crate::backend::seed().map(|value| __ConfiguredMessage::Seeded(value))",
            "id: ::std::option::Option::Some(\"dev.example.configured\".to_owned())",
            ".font(include_bytes!(\"fonts/Brand.ttf\").as_slice())",
            ".font(include_bytes!(\"fonts/Icons.otf\").as_slice())",
            "default_text_size: ::iced::Pixels(15 as f32)",
            "antialiasing: false",
            "vsync: false",
            "size: ::iced::Size::new(960 as f32, 720 as f32)",
            "maximized: true",
            "fullscreen: false",
            "Position::Specific(::iced::Point::new(10 as f32, -20 as f32))",
            "min_size: ::std::option::Option::Some(::iced::Size::new(480 as f32, 360 as f32))",
            "max_size: ::std::option::Option::Some(::iced::Size::new(1920 as f32, 1080 as f32))",
            "visible: true",
            "resizable: false",
            "closeable: false",
            "minimizable: false",
            "decorations: false",
            "transparent: true",
            "blur: true",
            "level: ::iced::window::Level::AlwaysOnTop",
            "const __ICE_RGBA: &[u8] = include_bytes!(\"assets/app.rgba\")",
            "__ICE_RGBA.len() == 8",
            "window::icon::from_rgba(__ICE_RGBA.to_vec(), 2, 1)",
            "exit_on_close_request: false",
            "__platform.application_id = \"dev.example.configured\".to_owned()",
            "__platform.override_redirect = true",
            "__platform.drag_and_drop = false",
            "__platform.skip_taskbar = true",
            "__platform.undecorated_shadow = true",
            "CornerPreference::RoundSmall",
            "__platform.title_hidden = true",
            "__platform.titlebar_transparent = true",
            "__platform.fullsize_content_view = true",
            "__platform.target = ::std::option::Option::None",
            "#[cfg(target_os = \"linux\")]",
            "#[cfg(target_os = \"windows\")]",
            "#[cfg(target_os = \"macos\")]",
            "#[cfg(target_arch = \"wasm32\")]",
            ".scale_factor(Self::__scale_factor)",
            "fn __scale_factor(&self) -> f32",
        ] {
            assert!(generated.contains(expected), "missing {expected}");
        }

        let error = compile(
            &source.replace("ready = true", "ready = 1"),
            "configured.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E101");

        for (from, to, expected) in [
            ("title \"Configured app\"", "title ready", "expected `str`"),
            ("theme \"dark\"", "theme \"unknown\"", "unknown iced theme"),
            (
                "background \"123456\"",
                "background \"not-a-color\"",
                "hexadecimal",
            ),
            ("scale-factor 1.25", "scale-factor 0", "greater than zero"),
        ] {
            let error = compile(&source.replace(from, to), "configured.ice").unwrap_err();
            assert!(error.message.contains(expected), "{error:?}");
        }
    }

    #[test]
    fn emits_a_probe_for_every_extern_function() {
        let source = r#"app Demo
extern crate::backend
  Item(id:i64)
  AppError(message:str)
  load(id:i64) -> [Item] ! AppError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  items:[Item] = []
on mount
  return if false
  run load(1) -> loaded _ | failed _
on loaded(next)
  items = next
on failed(error)
  items = []
view
  text len(items)
"#;
        let generated = compile(source, "demo.ice").unwrap();
        assert!(generated.contains("async fn __ui_lang_check_load"));
        assert!(generated.contains("crate::backend::load(arg0).await"));
        assert!(generated.contains("let task = (||"));
    }

    #[test]
    fn lowers_structured_task_groups_to_native_combinators() {
        let source = r#"app Grouped
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on start
  parallel
    task system theme -> theme_read _
    sequential
      task clipboard read -> clipboard_read _
      task system info -> info_read _
on theme_read(next)
on clipboard_read(next)
on info_read(info)
view
  text "Tasks"
"#;
        let generated = compile(source, "grouped.ice").unwrap();
        assert!(generated.contains("return ::iced::Task::batch(["));
        assert!(generated.contains("::iced::Task::none().chain({"));
        assert!(generated.contains(".chain({ ::iced::system::information()"));
        assert!(generated.contains("fn __ice_system_info"));
    }

    #[test]
    fn lowers_native_task_cancellation() {
        let source = r#"app Cancel
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  request:task-handle? = none
on start
  abortable request abort-on-drop
    task system theme -> loaded _
on loaded(next)
on cancel
  abort request
view
  col
    if aborted(request)
      text "Canceled"
"#;
        let generated = compile(source, "cancel.ice").unwrap();
        assert!(
            generated.contains("pub(crate) request: ::std::option::Option<::iced::task::Handle>")
        );
        assert!(generated.contains("let (__task, __handle) = ({"));
        assert!(generated.contains("}).abortable()"));
        assert!(generated.contains("Some(__handle.abort_on_drop())"));
        assert!(generated.contains("__handle.abort()"));
        assert!(generated.contains("is_some_and(::iced::task::Handle::is_aborted)"));
    }

    #[test]
    fn lowers_typed_task_streams() {
        let source = r#"app Streams
extern crate::backend
  AppError(message:str)
  stream numbers(limit:i64) -> i64
  stream range(start:i64, limit:i64) -> i64
  stream fallible() -> str ! AppError
  recipe snapshot(id:i64) -> str
  event-filter raw_event() -> str
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on start
  parallel
    stream numbers(3) -> number _
    stream fallible() -> text _ | failed _
on number(value)
on text(value)
on failed(error)
on observed(result)
subscribe
  run fallible() -> observed _
  run numbers(3) -> number _
  run range(1, 3) -> number _
  recipe snapshot(3) -> text _
  events 3 using=raw_event -> text _
view
  text "Streams"
"#;
        let generated = compile(source, "streams.ice").unwrap();
        assert!(generated.contains("fn __ui_lang_check_stream_numbers"));
        assert!(generated.contains("Task::run(crate::backend::numbers(arg0), |value| value)"));
        assert!(generated.contains("Task::run(crate::backend::numbers(3), |value|"));
        assert!(generated.contains("Task::run(crate::backend::fallible(), |result| match result"));
        assert!(generated.contains("Result::Err(error) => __StreamsMessage::Failed(error)"));
        assert!(generated.contains(
            "Subscription::run(crate::backend::fallible).map(move |__value| __StreamsMessage::Observed(__value))"
        ));
        assert!(generated.contains(
            "Subscription::run_with(3, |__data: &i64| crate::backend::numbers(__data.clone()))"
        ));
        assert!(generated.contains(
            "Subscription::run_with((1, 3,), |__data: &(i64, i64,)| crate::backend::range(__data.0.clone(), __data.1.clone()))"
        ));
        assert!(generated.contains("fn __ui_lang_check_recipe_snapshot"));
        assert!(generated.contains(
            "advanced::subscription::from_recipe(crate::backend::snapshot(3)).map(move |__value| __StreamsMessage::Text(__value))"
        ));
        assert!(generated.contains("fn __ui_lang_check_event_filter_raw_event"));
        assert!(generated.contains(
            "advanced::subscription::from_recipe(__IceEventFilterRawEvent { id: 3 }).map(move |__value| __StreamsMessage::Text(__value))"
        ));
    }

    #[test]
    fn lowers_typed_task_sips() {
        let source = r#"app Sips
extern crate::backend
  AppError(message:str)
  sip transfer(size:i64) progress=f64 -> bytes
  sip fallible() progress=i64 -> str ! AppError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on start
  parallel
    sip transfer(3)
      progress -> advanced _
      done -> downloaded _
    sip fallible()
      progress -> counted _
      done -> finished _
      error -> failed _
on advanced(value)
on downloaded(value)
on counted(value)
on finished(value)
on failed(error)
view
  text "Sips"
"#;
        let generated = compile(source, "sips.ice").unwrap();
        assert!(generated.contains("fn __ui_lang_check_sip_transfer"));
        assert!(generated.contains("let _: f64 = value"));
        assert!(generated.contains("Task::sip(crate::backend::transfer(3), |value|"));
        assert!(generated.contains("Task::sip(crate::backend::fallible(), |value|"));
        assert!(generated.contains("Result::Err(error) => __SipsMessage::Failed(error)"));
    }

    #[test]
    fn lowers_structured_task_flows() {
        let source = r#"app Flows
extern crate::backend
  AppError(message:str)
  stream numbers(limit:i64) -> i64
  task double(value:i64) -> i64
  task fallible(value:i64) -> i64 ! AppError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on start
  parallel
    flow
      from stream numbers(3)
      then value -> task double(value)
      collect
      done -> collected _
      units -> planned _
    flow
      from task fallible(2)
      and-then value -> task fallible(value)
      done -> finished _
      error -> failed _
    flow
      from stream numbers(1)
      discard
on collected(values)
on planned(units)
on finished(value)
on failed(error)
view
  text "Flows"
"#;
        let generated = compile(source, "flows.ice").unwrap();
        assert!(generated.contains("Task::run(crate::backend::numbers(3), |value| value)"));
        assert!(generated.contains(".then(move |value| crate::backend::double(value))"));
        assert!(generated.contains(".and_then(move |value| crate::backend::fallible(value))"));
        assert!(generated.contains(".collect()"));
        assert!(generated.contains(".discard::<__FlowsMessage>()"));
        assert!(generated.contains("i64::try_from(__task.units())"));
    }

    #[test]
    fn lowers_task_error_mapping_and_native_sources() {
        let source = r#"app Errors
extern crate::backend
  NetworkError(message:str)
  AppError(message:str)
  sync normalize(error:NetworkError) -> AppError
  task request() -> i64 ! NetworkError
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  results:[result[i64,AppError]] = []
on start
  parallel
    flow
      from task request()
      map-error reason -> normalize(reason)
      collect
      done -> collected _
    flow
      from done 1
      then value -> done value + 1
      done -> finished _
    flow
      from none i64
      done -> finished _
on collected(values)
  results = values
on finished(value)
view
  text len(results)
"#;
        let generated = compile(source, "errors.ice").unwrap();
        assert!(generated.contains("fn __ui_lang_check_sync_normalize"));
        assert!(
            generated.contains(".map_err(move |reason| crate::backend::normalize(reason.clone()))")
        );
        assert!(generated.contains(".collect()"));
        assert!(generated.contains("Task::done(1)"));
        assert!(generated.contains("Task::done((value + 1))"));
        assert!(generated.contains("Task::<i64>::none()"));
        assert!(generated.contains("Vec<::std::result::Result<i64, crate::backend::AppError>>"));
    }

    #[test]
    fn lowers_qr_data_and_widget_options() {
        let source = r#"app Codes
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
qr automatic "one"
qr corrected "two" correction=quartile
qr fixed "three" correction=low version=micro(4)
qr binary bytes(00 ff a4)
view
  col
    qr automatic cell-size=5.0
    qr corrected total-size=120.0 cell=primary background=white
    qr fixed
    qr binary
"#;
        let generated = compile(source, "codes.ice").unwrap();
        assert!(generated.contains("qr_code::Data::new(\"one\")"));
        assert!(generated.contains("qr_code::Data::with_error_correction(\"two\", ::iced::widget::qr_code::ErrorCorrection::Quartile)"));
        assert!(generated.contains("qr_code::Data::with_version(\"three\", ::iced::widget::qr_code::Version::Micro(4), ::iced::widget::qr_code::ErrorCorrection::Low)"));
        assert!(generated.contains("qr_code::Data::new(&[0x00u8, 0xffu8, 0xa4u8][..])"));
        assert!(
            generated.contains("::iced::widget::qr_code(&self.automatic).cell_size(5.0 as f32)")
        );
        assert!(generated.contains(
            "::iced::widget::qr_code(&self.corrected).total_size(120.0 as f32).style(|theme|"
        ));
        assert!(generated.contains("qr_code::Style { cell: ::iced::Color"));
    }

    #[test]
    fn lowers_nested_iced_themes() {
        let source = r#"app Themes
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
  surface #111111
view
  col
    theme app
      text "App theme"
    theme tokyo-night text=foreground background=linear(1.57, surface@0.0, background@1.0)
      text "Built-in theme"
    theme dark background=surface
      text "Solid background"
    theme
      text "Default mode"
"#;
        let generated = compile(source, "themes.ice").unwrap();
        assert!(generated.contains("themer(::std::option::Option::Some(Self::__app_theme())"));
        assert!(
            generated.contains("themer(::std::option::Option::Some(::iced::Theme::TokyoNight)")
        );
        assert!(generated.contains(".text_color(|_| ::iced::Color"));
        assert!(generated.contains(".background(|_| ::iced::Background::Color"));
        assert!(generated.contains(".background(|_| ::iced::Background::from(::iced::gradient::Linear::new(1.57 as f32).add_stop(0.0 as f32"));
        assert!(generated.contains("themer(::std::option::Option::None"));
    }

    #[test]
    fn lowers_component_children_and_slot_forwarding() {
        let source = r#"app Composition
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  draft = ""
component Card(title:str)
  col #card
    text title
    slot
component Wrapper(title:str)
  Card title=title
    slot
view
  Wrapper title="Editor" #editor
    input "Name" #name <-> draft
"#;
        let generated = compile(source, "composition.ice").unwrap();
        assert!(generated.contains("__BindDraft(::std::string::String)"));
        assert!(generated.contains("::iced::widget::text_input(\"\", &self.draft)"));
        assert!(generated.contains(
            "format!(\"{}/name\", format!(\"{}/card\", format!(\"{}/Card\", format!(\"{}/editor\", \"Composition\"))))"
        ));
    }

    #[test]
    fn lowers_named_slots_and_named_slot_forwarding() {
        let source = r#"app Composition
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Frame()
  col
    slot heading
    slot body
component Dialog()
  Frame
    heading:
      slot title
    body:
      col
        slot content
        slot actions
on cancel
on delete
view
  Dialog
    title:
      text "Delete task?"
    content:
      text "This cannot be undone."
    actions:
      row
        button "Cancel" -> cancel
        button "Delete" -> delete
"#;
        let generated = compile(source, "composition.ice").unwrap();
        assert!(generated.contains("Delete task?"));
        assert!(generated.contains("This cannot be undone."));
        assert!(generated.contains("Cancel"));
        assert!(generated.contains("Delete"));
    }

    #[test]
    fn lowers_compound_components_into_named_slots() {
        let source = r#"app Composition
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
component Dialog()
  col
    slot Header
    slot Body
component Dialog.Header()
  container #root
    slot
component Dialog.Body()
  container #root
    slot
view
  Dialog
    Dialog.Header
      text "Compound title"
    Dialog.Body
      text "Structured body"
"#;
        let generated = compile(source, "composition.ice").unwrap();
        assert!(generated.contains("Compound title"));
        assert!(generated.contains("Structured body"));
        assert!(generated.contains("format!(\"{}/Dialog.Header\""));
        assert!(generated.contains("format!(\"{}/Dialog.Body\""));
    }

    #[test]
    fn lowers_fully_configured_keyed_columns() {
        let source = r#"app Keyed
extern crate::backend
  Item(id:i64, name:str)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  items:[Item] = []
view
  keyed item in items by=item.id width=fill(2) height=120.0 spacing=8.0 padding=4.0 padding-left=12.0 max-width=640.0 align=end
    scroll #row
      text item.name
"#;
        let generated = compile(source, "keyed.ice").unwrap();
        assert!(generated.contains("for item in self.items.iter()"));
        assert!(generated.contains("__children.push((__key, __child))"));
        assert!(generated.contains("::iced::widget::keyed_column(__children)"));
        assert!(generated.contains(".spacing(8.0 as f32)"));
        assert!(generated.contains("left: 12.0 as f32"));
        assert!(generated.contains(".width(::iced::Length::FillPortion(2))"));
        assert!(generated.contains(".height(120.0 as f32)"));
        assert!(generated.contains(".max_width(640.0 as f32)"));
        assert!(generated.contains(".align_items(::iced::Alignment::End)"));
        assert!(generated.contains("format!(\"{}/key({})\""));
    }

    #[test]
    fn lowers_lazy_to_an_owned_static_subtree() {
        let source = r#"app LazyDemo
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  title = "Hello"
view
  lazy title as cached
    col
      text cached
      text len(cached)
"#;
        let generated = compile(source, "lazy.ice").unwrap();
        assert!(
            generated
                .contains("::iced::widget::lazy((self.title.clone(), (\"LazyDemo\").to_owned())")
        );
        assert!(generated.contains("let cached: ::std::string::String = __dependency.0.clone()"));
        assert!(generated.contains("let __lazy_content: ::iced::Element<'static,"));
        assert!(generated.contains("let __lazy_scope = __dependency.1.clone()"));
    }

    #[test]
    fn lowers_parsed_markdown_with_complete_sizes_and_link_route() {
        let source = r##"app Docs
font ui family=sans
extern crate::backend
  markdown-viewer docs_viewer(prefix:str) -> str
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  docs:markdown = "# Hello"
  images:[str] = []
on open(url)
on reset
  docs = markdown("# Reset")
on extend
  markdown docs append "\n![Ice](asset://ice)"
  images = markdown_images(docs)
view
  markdown docs text-size=16.0 h1-size=32.0 h2-size=28.0 h3-size=24.0 h4-size=20.0 h5-size=18.0 h6-size=16.0 code-size=13.0 spacing=12.0 viewer=docs_viewer("docs") -> open _
    style font=ui inline-code-background=linear(1.57, background@0.0, primary@1.0) inline-code-color=foreground inline-code-font=mono code-block-font=mono link=primary inline-code-padding=2.0 inline-code-padding-x=3.0 inline-code-padding-y=4.0 inline-code-padding-top=5.0 inline-code-padding-right=6.0 inline-code-padding-bottom=7.0 inline-code-padding-left=8.0 inline-code-border=primary inline-code-border-width=1.0 inline-code-radius=4.0 inline-code-radius-tl=1.0 inline-code-radius-tr=2.0 inline-code-radius-br=3.0 inline-code-radius-bl=4.0
"##;
        let generated = compile(source, "docs.ice").unwrap();
        assert!(generated.contains("docs: ::iced::widget::markdown::Content::parse(\"# Hello\")"));
        assert!(generated.contains(
            "self.docs = ::iced::widget::markdown::Content::parse(&\"# Reset\".to_owned())"
        ));
        for field in [
            "text_size",
            "h1_size",
            "h2_size",
            "h3_size",
            "h4_size",
            "h5_size",
            "h6_size",
            "code_size",
            "spacing",
        ] {
            assert!(generated.contains(&format!("__markdown_settings.{field} =")));
        }
        assert!(generated.contains("self.docs.push_str(&\"\\n![Ice](asset://ice)\".to_owned())"));
        assert!(generated.contains(".images().iter().cloned().collect"));
        assert!(generated.contains("::iced::widget::markdown::view_with(self.docs.items()"));
        assert!(generated.contains("crate::backend::docs_viewer(\"docs\".to_owned())"));
        assert!(generated.contains("map(move |__event| __DocsMessage::Open(__event))"));
        assert!(generated.contains("fn __ui_lang_check_markdown_viewer_docs_viewer"));
        for field in [
            "style.font",
            "style.inline_code_highlight.background",
            "style.inline_code_color",
            "style.inline_code_font",
            "style.code_block_font",
            "style.link_color",
            "style.inline_code_padding",
            "style.inline_code_highlight.border.color",
            "style.inline_code_highlight.border.width",
            "style.inline_code_highlight.border.radius",
        ] {
            assert!(generated.contains(&format!("__markdown_settings.{field} =")));
        }
    }

    #[test]
    fn lowers_structured_tables_with_complete_native_options() {
        let source = r#"app Rows
extern crate::backend
  Item(name:str, done:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  rows:[Item] = []
view
  table row in rows width=fill padding=4.0 padding-x=8.0 padding-y=6.0 separator=1.0 separator-x=2.0 separator-y=3.0
    column width=fill(2) align-x=right align-y=bottom
      header
        text "Name"
      cell
        scroll #value
          text row.name
"#;
        let generated = compile(source, "rows.ice").unwrap();
        assert!(generated.contains("table::table(::std::vec!["));
        assert!(generated.contains("self.rows.clone().into_iter().enumerate()"));
        assert!(generated.contains("move |(__row, row): (usize, crate::backend::Item)|"));
        assert!(generated.contains(".width(::iced::Length::FillPortion(2))"));
        assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Right)"));
        assert!(generated.contains(".align_y(::iced::alignment::Vertical::Bottom)"));
        for method in [
            "padding(4.0 as f32)",
            "padding_x(8.0 as f32)",
            "padding_y(6.0 as f32)",
            "separator(1.0 as f32)",
            "separator_x(2.0 as f32)",
            "separator_y(3.0 as f32)",
        ] {
            assert!(generated.contains(method));
        }
        assert!(generated.contains("format!(\"{}/row({})/column(0)\""));
    }

    #[test]
    fn lowers_bound_text_editors_and_internal_actions() {
        let source = r#"app Notes
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  body:editor = "fn main() {}"
  locked = false
view
  editor #body <-> body placeholder="Write" width=640.0 height=fill min-height=80.0 max-height=240.0 size=14.0 line-height-px=18.0 padding=8.0 wrapping=word-or-glyph font=mono highlight="rs" highlight-theme=inspired-github disabled=locked
    active background=background border=foreground border-width=1.0 radius=4.0 placeholder=danger value=foreground selection=primary
    hovered background=background border=primary placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
"#;
        let generated = compile(source, "notes.ice").unwrap();
        assert!(generated.contains("body: ::iced::widget::text_editor::Content::with_text"));
        assert!(generated.contains("__EditBody(::iced::widget::text_editor::Action)"));
        assert!(generated.contains("self.body.perform(action)"));
        assert!(generated.contains("::iced::widget::text_editor(&self.body)"));
        assert!(generated.contains(".width(640.0 as f32)"));
        assert!(generated.contains(".height(::iced::Fill)"));
        assert!(generated.contains(".min_height(80.0 as f32)"));
        assert!(generated.contains(".max_height(240.0 as f32)"));
        assert!(generated.contains("LineHeight::Absolute((18.0 as f32).into())"));
        assert!(generated.contains("Wrapping::WordOrGlyph"));
        assert!(generated.contains(".font(::iced::Font::MONOSPACE)"));
        assert!(
            generated.contains(".highlight(\"rs\", ::iced::highlighter::Theme::InspiredGitHub)")
        );
        assert!(generated.contains("::iced::widget::text_editor::default"));
        assert!(generated.contains("text_editor::Status::Focused { is_hovered: true }"));
        assert!(generated.contains("__style.placeholder ="));
        assert!(generated.contains("__style.selection ="));
        assert!(generated.contains("if self.locked"));
        assert!(generated.contains(".on_action(__NotesMessage::__EditBody"));
    }

    #[test]
    fn lowers_component_controls_and_editor_extensions() {
        let source = r#"app Notes
extern crate::backend
  EditorCommand(save:bool)
  editor-binding editor_keys(readonly:bool) -> EditorCommand
  editor-highlighter editor_highlight(language:str)
  editor-style editor_surface(readonly:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  body:editor = ""
  title = "Notes"
  locked = false
  language = "rs"
component EditorPanel(content:editor, heading:str, readonly:bool, syntax:str)
  col
    input "Title" <-> heading
    editor <-> content highlighter=editor_highlight(syntax) key-binding=editor_keys(readonly) style=editor_surface(readonly) -> command _
on command(value)
view
  EditorPanel(body, title, locked, language)
"#;
        let generated = compile(source, "notes.ice").unwrap();
        assert!(generated.contains("__BindTitle(::std::string::String)"));
        assert!(generated.contains("__EditBody(::iced::widget::text_editor::Action)"));
        assert!(generated.contains("text_input(\"\", &self.title)"));
        assert!(generated.contains("text_editor(&self.body)"));
        assert!(generated.contains("crate::backend::editor_keys(__key_press, self.locked)"));
        assert!(generated.contains("__ice_map_editor_binding"));
        assert!(generated.contains("__NotesMessage::Command(__value)"));
        assert!(generated.contains("crate::backend::editor_highlight("));
        assert!(generated.contains(", self.language.clone())"));
        assert!(generated.contains("fn __ui_lang_check_editor_binding_editor_keys"));
        assert!(generated.contains("fn __ui_lang_check_editor_highlighter_editor_highlight"));
        assert!(generated.contains("fn __ui_lang_check_editor_style_editor_surface"));
        assert!(
            generated.contains("crate::backend::editor_surface(__theme, __status, self.locked)")
        );
        assert!(generated.contains("self.title = value"));
        assert!(generated.contains("self.body.perform(action)"));
    }

    #[test]
    fn lowers_complex_native_controls() {
        let source = r#"app Controls
extern crate::backend
  SliderNumber()
  sync slider_number(value:f64) -> SliderNumber
  slider-style dynamic_slider(active:bool)
  progress-style dynamic_progress(active:bool)
  radio-style dynamic_radio(highlight:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 50.0
  precise:SliderNumber = slider_number(50.0)
  enabled = false
  choice = "first"
on amount_changed(next)
  amount = next
on precise_changed(next)
  precise = next
on released
on enabled_changed(next)
  enabled = next
on choice_changed(next)
  choice = next
view
  col
    grid columns=2 width=640.0 spacing=12.0 height=aspect(16.0,9.0) @gap-2
      toggler "Enabled" checked=enabled -> enabled_changed _
      slider amount min=0.0 max=100.0 step=0.5 default=50.0 shift-step=0.1 vertical width=20.0 height=fill(2) style=dynamic_slider(enabled) release=released -> amount_changed _
        active rail-start=linear(0.0, primary@0.0, danger@1.0) rail-end=linear(1.57, background@0.0, primary/25@1.0) rail-width=4.0 rail-border=transparent rail-border-width=1.0 rail-radius=2.0 rail-radius-tl=1.0 handle=circle(7.0) handle-color=linear(0.785, primary@0.0, foreground@1.0) handle-border=foreground handle-border-width=1.0
        hovered rail-start=foreground rail-end=background handle=rect(12) handle-color=foreground handle-radius=3.0 handle-radius-tl=1.0
        dragged rail-start=danger handle=circle(8.0) handle-color=danger
      slider amount min=0.0 max=100.0 step=1.0 width=fill height=18.0 style=dynamic_slider(enabled) -> amount_changed _
      slider precise min=slider_number(0.0) max=slider_number(100.0) step=slider_number(5.0) default=slider_number(50.0) shift-step=slider_number(1.0) -> precise_changed _
      progress amount vertical length=fill(2) girth=20.0 style=dynamic_progress(enabled) background=linear(1.57, background@0.0, primary/25@1.0) bar=linear(0.0, primary/75@0.0, danger@1.0) border=foreground border-width=1.0 radius=4.0 radius-tl=2.0
      progress amount style=success
      progress amount style=warning
      progress amount style=danger
      radio "First" value="first" selected=(choice == "first") style=dynamic_radio(enabled) size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=advanced wrapping=word-or-glyph font=mono -> choice_changed _
        active selected background=linear(1.57, primary@0.0, background@1.0) dot=foreground border=primary border-width=2.0 text=foreground
        active unselected background=background dot=primary border=foreground text=foreground
        hovered selected background=primary dot=foreground border=foreground text=foreground
        hovered unselected background=foreground dot=background border=primary text=primary
      rule horizontal thickness=2.0 style=weak fill=full color=primary/50 radius=4.0 radius-tl=2.0 snap=false
      rule horizontal fill=percent(75.0)
      rule horizontal fill=pad(4)
      rule horizontal fill=pad(4,8)
      space width=fill(2) height=shrink
      stack clip=true width=fill(2) height=120.0 under=1
        text "base"
        text "overlay"
    grid fluid=240.0 height=fill(2)
      text "fluid"
"#;
        let generated = compile(source, "controls.ice").unwrap();
        assert!(
            generated.contains("::iced::widget::grid(__children).spacing(8).spacing(12.0 as f32).width(640.0 as f32).height(::iced::widget::grid::aspect_ratio(16.0 as f32, 9.0 as f32)).columns(2 as usize)")
        );
        assert!(generated.contains(
            "::iced::widget::grid(__children).height(::iced::Length::FillPortion(2)).fluid(240.0 as f32)"
        ));
        assert!(generated.contains("::iced::widget::vertical_slider"));
        assert!(generated.contains(".default(50.0).shift_step(0.1).width(20.0 as f32).height(::iced::Length::FillPortion(2))"));
        assert!(generated.contains("::iced::widget::slider"));
        assert!(generated.contains(".width(::iced::Fill).height(18.0 as f32)"));
        assert!(generated.contains(".style(move |__theme, __status|"));
        assert!(generated.contains("fn __ui_lang_check_slider_style_dynamic_slider"));
        assert_eq!(
            generated
                .matches("crate::backend::dynamic_slider(__theme, __status, self.enabled)")
                .count(),
            2
        );
        assert!(generated.contains(
            "::iced::widget::slider((crate::backend::slider_number(0.0))..=(crate::backend::slider_number(100.0)), self.precise, move |__value| __ControlsMessage::PreciseChanged(__value)).step(crate::backend::slider_number(5.0))"
        ));
        assert!(!generated.contains("self.precise.clone()"));
        assert!(generated.contains("slider::Status::Active"));
        assert!(generated.contains("slider::Status::Hovered"));
        assert!(generated.contains("slider::Status::Dragged"));
        assert!(generated.contains("slider::HandleShape::Circle"));
        assert!(generated.contains("slider::HandleShape::Rectangle"));
        assert!(generated.contains("__style.rail.backgrounds.0"));
        assert!(generated.contains("__style.rail.backgrounds.0 = ::iced::Background::from"));
        assert!(generated.contains("__style.rail.backgrounds.1 = ::iced::Background::from"));
        assert!(generated.contains("__style.handle.background = ::iced::Background::from"));
        assert!(generated.contains("::iced::widget::progress_bar"));
        assert!(generated.contains(".vertical()"));
        assert!(generated.contains(".length(::iced::Length::FillPortion(2)).girth(20.0 as f32)"));
        assert!(generated.contains("crate::backend::dynamic_progress(__theme, self.enabled)"));
        assert!(generated.contains("fn __ui_lang_check_progress_style_dynamic_progress"));
        assert!(generated.contains("progress_bar::success(__theme)"));
        assert!(generated.contains("progress_bar::warning(__theme)"));
        assert!(generated.contains("progress_bar::danger(__theme)"));
        assert!(generated.contains("__style.background = ::iced::Background::from"));
        assert!(generated.contains("__style.bar = ::iced::Background::from"));
        assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
        assert!(generated.contains("::iced::gradient::Linear::new(0.0 as f32)"));
        assert!(generated.contains("__style.border.radius"));
        assert!(generated.contains("::iced::widget::radio(\"First\".to_owned(), true"));
        assert!(
            generated.contains("move |_| __ControlsMessage::ChoiceChanged(\"first\".to_owned())")
        );
        assert!(generated.contains(".size(20.0 as f32).spacing(8.0 as f32)"));
        assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
        assert!(generated.contains(".text_wrapping(::iced::widget::text::Wrapping::WordOrGlyph)"));
        assert!(generated.contains(".font(::iced::Font::MONOSPACE)"));
        assert!(
            generated.contains("crate::backend::dynamic_radio(__theme, __status, self.enabled)")
        );
        assert!(generated.contains("fn __ui_lang_check_radio_style_dynamic_radio"));
        for (status, selected) in [
            ("Active", true),
            ("Active", false),
            ("Hovered", true),
            ("Hovered", false),
        ] {
            assert!(generated.contains(&format!(
                "radio::Status::{status} {{ is_selected: {selected} }}"
            )));
        }
        assert!(generated.contains("__style.background = ::iced::Background::from"));
        assert!(generated.contains("__style.dot_color ="));
        assert!(generated.contains("__style.border_width = 2.0 as f32"));
        assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
        let default_radio = compile(
            &source.replace(" style=dynamic_radio(enabled)", ""),
            "controls.ice",
        )
        .unwrap();
        assert!(default_radio.contains("radio::default(__theme, __status)"));
        assert!(generated.contains("::iced::widget::rule::weak(__theme)"));
        assert!(generated.contains("rule::FillMode::Full"));
        assert!(generated.contains("rule::FillMode::Percent(75.0 as f32)"));
        assert!(generated.contains("rule::FillMode::Padded(4)"));
        assert!(generated.contains("rule::FillMode::AsymmetricPadding(4, 8)"));
        assert!(generated.contains("__style.snap = false"));
        assert!(generated.contains(
            "::iced::widget::space().width(::iced::Length::FillPortion(2)).height(::iced::Shrink)"
        ));
        assert!(generated.contains("__children.split_off(__under)"));
        assert!(generated.contains("::iced::widget::Stack::new()"));
        assert!(generated.contains("__stack.push_under(__child)"));
        assert!(
            generated
                .contains(".clip(true).width(::iced::Length::FillPortion(2)).height(120.0 as f32)")
        );
    }

    #[test]
    fn lowers_complete_flex_layouts_and_wrapping() {
        let source = r#"app Layouts
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  col width=fill height=shrink spacing=8.0 padding=1.0 padding-x=2.0 padding-y=3.0 padding-top=4.0 padding-right=5.0 padding-bottom=6.0 padding-left=7.0 max-width=640.0 align=center clip=true wrap wrap-spacing=12.0 wrap-align=end
    row width=fill(2) height=48.0 spacing=4.0 padding=2.0 align=end clip=false wrap wrap-spacing=6.0 wrap-align=start
      text "One"
      text "Two"
"#;
        let generated = compile(source, "layouts.ice").unwrap();
        assert!(generated.contains("::iced::widget::column(__children).spacing(8.0 as f32)"));
        assert!(generated.contains("::iced::Padding { top: 4.0 as f32, right: 5.0 as f32, bottom: 6.0 as f32, left: 7.0 as f32 }"));
        assert!(generated.contains(".width(::iced::Fill).height(::iced::Shrink)"));
        assert!(generated.contains(".max_width(640.0 as f32)"));
        assert!(generated.contains(
            ".align_x(::iced::alignment::Horizontal::Center).clip(true).wrap().horizontal_spacing(12.0 as f32).align_x(::iced::alignment::Vertical::Bottom)"
        ));
        assert!(generated.contains(".width(::iced::Length::FillPortion(2)).height(48.0 as f32)"));
        assert!(generated.contains(
            ".align_y(::iced::alignment::Vertical::Bottom).clip(false).wrap().vertical_spacing(6.0 as f32).align_x(::iced::alignment::Horizontal::Left)"
        ));
    }

    #[test]
    fn lowers_complete_container_layout() {
        let source = r#"app Boxed
extern crate::backend
  container-style dynamic_container(highlight:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  highlight = false
view
  container #card style=dynamic_container(highlight) width=fill height=80.0 max-width=640.0 max-height=120.0 align-x=center align-y=end clip=true padding=8.0 padding-left=12.0 background=linear(1.57, background@0.0, primary/25@1.0) text=foreground border=primary border-width=2.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=6.0 pixel-snap=true @w-full bg-background border border-foreground rounded-lg
    text "Card"
"#;
        let generated = compile(source, "boxed.ice").unwrap();
        assert!(generated.contains("::iced::widget::container(__container_content)"));
        assert!(generated.contains(".id(::iced::widget::Id::from("));
        assert!(generated.contains(".width(::iced::Fill).height(80.0 as f32)"));
        assert!(generated.contains(".max_width(640.0 as f32).max_height(120.0 as f32)"));
        assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Center)"));
        assert!(generated.contains(".align_y(::iced::alignment::Vertical::Bottom)"));
        assert!(generated.contains(".clip(true)"));
        assert!(generated.contains("crate::backend::dynamic_container(__theme, self.highlight)"));
        assert!(generated.contains("fn __ui_lang_check_container_style_dynamic_container"));
        assert!(generated.contains("::iced::widget::container::Style"));
        assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
        assert!(generated.contains("__style.border.radius"));
        assert!(generated.contains("__style.shadow.blur_radius = 6.0 as f32"));
        assert!(generated.contains("__style.snap = true"));
        assert!(generated.contains("__style.border.width = 1.0;"));
        assert!(generated.contains("__style.border.width = 2.0 as f32;"));
    }

    #[test]
    fn lowers_structured_overlays_to_native_overlay_widgets() {
        let source = r#"app Dialog
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  shown = true
on close
  shown = false
view
  overlay when=shown dismiss=close backdrop=black/60 padding=24.0 align-x=center align-y=end
    content
      text "Page"
    layer
      container width=320.0 padding=16.0 @bg-background rounded-lg
        text "Dialog"
"#;
        let generated = compile(source, "dialog.ice").unwrap();
        assert!(generated.contains("if self.shown"));
        assert!(generated.contains("::iced::widget::Stack::new()"));
        assert!(generated.contains("::iced::widget::float(__overlay_surface)"));
        assert!(generated.contains("::core::f32::EPSILON"));
        assert!(generated.contains("::iced::Color::from_rgba8(0, 0, 0, 0.600000)"));
        assert!(generated.contains(".on_press(__DialogMessage::Close)"));
        assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Center)"));
        assert!(generated.contains(".align_y(::iced::alignment::Vertical::Bottom)"));
        assert!(generated.contains("__DialogMessage::__ExternNoop"));
    }

    #[test]
    fn lowers_persistent_pane_grids() {
        let source = r#"app Workspace
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on clicked(name)
view
  pane-grid #work split=vertical ratio=0.7 width=fill height=fill spacing=8.0 min-size=120.0 resize=6.0 drag click=clicked(_)
    pane files
      text "Files"
    pane editor
      text "Editor"
"#;
        let generated = compile(source, "workspace.ice").unwrap();
        assert!(generated.contains("__pane_work: ::iced::widget::pane_grid::State"));
        assert!(generated.contains("pane_grid::Configuration::Split"));
        assert!(generated.contains("pane_grid::Axis::Vertical"));
        assert!(generated.contains("Configuration::Pane(\"files\")"));
        assert!(generated.contains("::iced::widget::pane_grid(&self.__pane_work"));
        assert!(generated.contains(".on_resize(6.0 as f32, __WorkspaceMessage::__PaneWorkResize)"));
        assert!(generated.contains(".on_drag(__WorkspaceMessage::__PaneWorkDrag)"));
        assert!(generated.contains("self.__pane_work.resize(__event.split, __event.ratio)"));
        assert!(generated.contains("self.__pane_work.drop(pane, target)"));
        assert!(generated.contains("__WorkspaceMessage::Clicked(__pane_name.to_owned())"));
    }

    #[test]
    fn lowers_nested_pane_configuration_and_closed_templates() {
        let source = r#"app Workspace
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on open_preview
  pane #work split editor preview horizontal ratio=0.4
view
  pane-grid #work width=fill height=fill
    split vertical ratio=0.7
      pane files
        text "Files"
      split horizontal ratio=0.6
        pane editor
          text "Editor"
        pane terminal
          text "Terminal"
    pane preview closed
      text "Preview"
"#;
        let generated = compile(source, "workspace.ice").unwrap();
        assert_eq!(
            generated.matches("pane_grid::Configuration::Split").count(),
            2
        );
        assert!(generated.contains("pane_grid::Axis::Vertical"));
        assert!(generated.contains("pane_grid::Axis::Horizontal"));
        assert!(generated.contains("Configuration::Pane(\"terminal\")"));
        assert!(!generated.contains("Configuration::Pane(\"preview\")"));
        assert!(generated.contains("\"preview\" =>"));
        assert!(generated.contains(".split(::iced::widget::pane_grid::Axis::Horizontal"));
    }

    #[test]
    fn lowers_structured_pane_titles_and_dynamic_controls() {
        let source = r#"app Workspace
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  filter = ""
on close
view
  pane-grid #work split=vertical
    style
      hovered-region background=linear(0.785, primary/25@0.0, background@0.5, danger@1.0) border=foreground border-width=2.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0
      hovered-split color=primary width=3.0
      picked-split color=danger width=4.0
    pane files background=linear(1.57, background@0.0, primary/25@1.0) text=foreground border=primary border-width=2.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=6.0 pixel-snap=true @bg-background border border-primary rounded
      title padding=4.0 padding-x=8.0 padding-top=6.0 always-controls background=primary/50 text=foreground border=danger border-width=1.0 radius=3.0 shadow=black/50 shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=false @bg-primary text-white
        text "Files"
      controls
        button "Close" -> close
      compact-controls
        button "×" -> close
      content
        input "Filter" #filter <-> filter
    pane editor
      title
        text "Editor"
      controls
        button "Close" -> close
      content
        text "Editor body"
"#;
        let generated = compile(source, "workspace.ice").unwrap();
        assert!(generated.contains("pane_grid::Content::new(__pane_content).style"));
        assert!(generated.contains(".title_bar(::iced::widget::pane_grid::TitleBar::new"));
        assert!(generated.contains("top: 6.0 as f32"));
        assert!(generated.contains("right: 8.0 as f32"));
        assert!(generated.contains("bottom: 4.0 as f32"));
        assert!(generated.contains("pane_grid::Controls::dynamic"));
        assert!(generated.contains("pane_grid::Controls::new"));
        assert!(generated.contains(".always_show_controls().style"));
        assert!(generated.contains("__BindFilter"));
        assert!(generated.contains("format!(\"{}/filter\""));
        assert!(generated.contains("pane_grid::default(__theme)"));
        assert!(generated.contains("__style.hovered_region.background"));
        assert!(generated.contains("::iced::gradient::Linear::new(0.785 as f32)"));
        assert!(generated.contains(".add_stop(0.5 as f32"));
        assert!(generated.contains("__style.hovered_region.border.color"));
        assert!(generated.contains("__style.hovered_region.border.width = 2.0 as f32"));
        assert!(generated.contains("top_left: 1.0 as f32"));
        assert!(generated.contains("top_right: 2.0 as f32"));
        assert!(generated.contains("bottom_right: 3.0 as f32"));
        assert!(generated.contains("bottom_left: 4.0 as f32"));
        assert!(generated.contains("__style.hovered_split.color"));
        assert!(generated.contains("__style.hovered_split.width = 3.0 as f32"));
        assert!(generated.contains("__style.picked_split.color"));
        assert!(generated.contains("__style.picked_split.width = 4.0 as f32"));
        assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
        assert!(generated.contains("__style.shadow.color"));
        assert!(generated.contains("__style.shadow.offset.x = (-1.0) as f32"));
        assert!(generated.contains("__style.shadow.offset.y = 2.0 as f32"));
        assert!(generated.contains("__style.shadow.blur_radius = 6.0 as f32"));
        assert!(generated.contains("__style.snap = true"));
        assert!(generated.contains("__style.snap = false"));
    }

    #[test]
    fn lowers_pane_state_operations_and_queries() {
        let source = r#"app Workspace
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on arrange
  pane #work maximize editor
  pane #work restore
  pane #work swap files editor
  pane #work move editor left
  pane #work resize 0.6
  pane #work drop editor files top
  pane #work split editor preview horizontal ratio=0.4
  pane #work close editor
on inspect
  pane #work maximized -> observed _
on inspect_neighbor
  pane #work adjacent files right -> observed _
on observed(name)
view
  pane-grid #work split=vertical
    pane files
      text "Files"
    pane editor
      text "Editor"
    pane preview closed
      text "Preview"
"#;
        let generated = compile(source, "workspace.ice").unwrap();
        assert!(generated.contains("self.__pane_work.maximize(__pane)"));
        assert!(generated.contains("self.__pane_work.restore()"));
        assert!(generated.contains("self.__pane_work.swap(__first, __second)"));
        assert!(generated.contains("move_to_edge(__pane, ::iced::widget::pane_grid::Edge::Left)"));
        assert!(generated.contains("layout().splits().next().copied()"));
        assert!(generated.contains("self.__pane_work.resize(__split, (0.6) as f32)"));
        assert!(generated.contains("pane_grid::Target::Pane(__target"));
        assert!(generated.contains("pane_grid::Region::Edge"));
        assert!(generated.contains(".split(::iced::widget::pane_grid::Axis::Horizontal"));
        assert!(generated.contains("\"preview\""));
        assert!(generated.contains("self.__pane_work.close(__pane)"));
        assert!(generated.contains("self.__pane_work.maximized()"));
        assert!(generated.contains("pane_grid::Direction::Right"));
        assert!(generated.contains("::iced::Task::done(__WorkspaceMessage::Observed(value))"));
    }

    #[test]
    fn lowers_list_literals_options_and_pick_lists() {
        let source = r#"app Selection
extern crate::backend
  pick-list-style dynamic_pick(busy:bool)
  menu-style dynamic_menu(busy:bool)
font ui family=sans
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  choices = ["List", "Board"]
  selected:str? = none
on selected(next)
  selected = some(next)
on opened
on closed
view
  pick choices selected placeholder="Choose" width=fill menu-height=120.0 padding=8.0 text-size=14.0 line-height=1.2 shaping=advanced font=ui open=opened close=closed style=dynamic_pick(busy) menu-style=dynamic_menu(busy) -> selected _
    active text=foreground placeholder=danger handle=primary background=background border=foreground border-width=1.0 radius=4.0
    hovered text=foreground
    opened text=foreground
    opened-hovered text=foreground
    menu text=foreground selected-text=background selected-background=primary background=background border=foreground border-width=1.0 radius=6.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0
    handle dynamic
      closed code="⌄" font=ui size=12.0 line-height=1.0 shaping=basic
      open code="⌃" font=ui size=13.0 line-height=1.1 shaping=advanced
"#;
        let generated = compile(source, "selection.ice").unwrap();
        assert!(
            generated.contains("pub(crate) selected: ::std::option::Option<::std::string::String>")
        );
        assert!(generated.contains("::std::vec![\"List\".to_owned(), \"Board\".to_owned()]"));
        assert!(
            generated
                .contains("::iced::widget::pick_list(self.choices.clone(), self.selected.clone()")
        );
        assert!(generated.contains(".on_open(__SelectionMessage::Opened)"));
        assert!(
            generated.contains(
                ".text_line_height(::iced::widget::text::LineHeight::Relative(1.2 as f32))"
            )
        );
        assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
        assert!(generated.contains("::iced::widget::pick_list::Handle::Dynamic"));
        assert!(generated.contains(
            "let mut __style = crate::backend::dynamic_pick(__theme, __status, self.busy); match __status"
        ));
        assert!(
            generated
                .contains("let mut __style = crate::backend::dynamic_menu(__theme, self.busy);")
        );
        assert!(generated.contains("fn __ui_lang_check_pick_list_style_dynamic_pick"));
        assert!(generated.contains("fn __ui_lang_check_menu_style_dynamic_menu"));
        assert!(generated.contains("Status::Opened { is_hovered: false }"));
        assert!(generated.contains("Status::Opened { is_hovered: true }"));
        assert!(generated.contains(".menu_style(move |__theme|"));
        assert!(generated.contains("__style.selected_background"));
        assert!(generated.contains("__style.shadow.blur_radius = 4.0 as f32"));
        assert!(generated.contains("self.selected = ::std::option::Option::Some(next);"));
        let defaults = compile(
            &source.replace(
                " style=dynamic_pick(busy) menu-style=dynamic_menu(busy)",
                "",
            ),
            "selection.ice",
        )
        .unwrap();
        assert!(defaults.contains("pick_list::default(__theme, __status)"));
        assert!(defaults.contains("menu::default(__theme)"));
    }

    #[test]
    fn lowers_searchable_combo_boxes() {
        let source = r#"app Search
extern crate::backend
  input-style dynamic_input(busy:bool)
  menu-style dynamic_menu(busy:bool)
font ui family=sans
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  modes:combo[str] = ["List", "Board"]
  selected:str? = none
  query = ""
on selected(next)
  selected = some(next)
on searched(next)
  query = next
on hovered(next)
on opened
on closed
on reset
  modes = ["Timeline"]
on add
  combo modes push "Calendar"
view
  combo modes selected "Search modes" width=fill menu-height=120.0 padding=8.0 text-size=14.0 line-height=1.2 shaping=advanced font=ui input=searched hover=hovered open=opened close=closed style=dynamic_input(busy) menu-style=dynamic_menu(busy) -> selected _
    active background=background border=foreground border-width=1.0 radius=4.0 icon=primary placeholder=danger value=foreground selection=primary
    hovered background=background icon=foreground placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
    menu text=foreground selected-text=background selected-background=primary background=background border=foreground border-width=1.0 radius=6.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0
    icon code="⌕" font=ui size=12.0 spacing=6.0 side=right
"#;
        let generated = compile(source, "search.ice").unwrap();
        assert!(
            generated.contains(
                "pub(crate) modes: ::iced::widget::combo_box::State<::std::string::String>"
            )
        );
        assert!(generated.contains(
            "::iced::widget::combo_box::State::new(::std::vec![\"List\".to_owned(), \"Board\".to_owned()])"
        ));
        assert!(generated.contains(
            "::iced::widget::combo_box(&self.modes, \"Search modes\", __combo_selection.as_ref()"
        ));
        assert!(generated.contains(".on_input(move |__value| __SearchMessage::Searched(__value))"));
        assert!(
            generated
                .contains(".on_option_hovered(move |__value| __SearchMessage::Hovered(__value))")
        );
        assert!(
            generated
                .contains(".line_height(::iced::widget::text::LineHeight::Relative(1.2 as f32))")
        );
        assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
        assert!(generated.contains("code_point: '⌕'"));
        assert!(generated.contains("Side::Right"));
        assert!(generated.contains(".input_style(move |__theme, __status|"));
        assert!(generated.contains("crate::backend::dynamic_input(__theme, __status, self.busy)"));
        assert!(generated.contains("crate::backend::dynamic_menu(__theme, self.busy)"));
        assert!(generated.contains("fn __ui_lang_check_input_style_dynamic_input"));
        assert!(generated.contains("fn __ui_lang_check_menu_style_dynamic_menu"));
        assert!(generated.contains("Status::Focused { is_hovered: true }"));
        assert!(generated.contains(".menu_style(move |__theme|"));
        assert!(generated.contains("__style.selected_background"));
        assert!(generated.contains(
            "self.modes = ::iced::widget::combo_box::State::new(::std::vec![\"Timeline\".to_owned()]);"
        ));
        assert!(generated.contains("self.modes.push(\"Calendar\".to_owned());"));
    }

    #[test]
    fn lowers_structural_widgets_and_size_events() {
        let source = r#"app Structure
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  sensor_key = 0
  width = 0.0
  height = 0.0
on shown(w, h)
  width = w
  height = h
on resized(w, h)
  width = w
  height = h
on hidden
view
  col
    float scale=1.1 x=(viewport_x + viewport_width - original_x - original_width) y=(viewport_y + viewport_height - original_y - original_height) shadow=black/50 shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 radius=8.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0
      text "Floating"
    pin width=fill height=80.0 x=12.0 y=8.0
      text "Pinned"
    sensor show=shown resize=resized hide=hidden key=sensor_key anticipate=32.0 delay=10
      text "Observed"
    responsive at=600.0 width=fill height=40.0
      text "Narrow"
      text "Wide"
    responsive size=(available_width, available_height) width=fill height=fill
      col
        if available_width < available_height
          text "Portrait"
        if available_width >= available_height
          text "Landscape"
"#;
        let generated = compile(source, "structure.ice").unwrap();
        assert!(generated.contains("::iced::widget::float(__float_content).scale(1.1 as f32)"));
        assert!(generated.contains("translate(move |__original, __viewport|"));
        assert!(generated.contains(
            "(((__viewport.x as f64) + (__viewport.width as f64)) - (__original.x as f64)) - (__original.width as f64)"
        ));
        assert!(generated.contains(
            "(((__viewport.y as f64) + (__viewport.height as f64)) - (__original.y as f64)) - (__original.height as f64)"
        ));
        assert!(generated.contains("::iced::widget::float::Style::default()"));
        assert!(generated.contains("__style.shadow.color = ::iced::Color::from_rgba8"));
        assert!(generated.contains("__style.shadow.offset.x = 1.0 as f32"));
        assert!(generated.contains("__style.shadow.offset.y = 2.0 as f32"));
        assert!(generated.contains("__style.shadow.blur_radius = 4.0 as f32"));
        assert!(generated.contains("__style.shadow_border_radius = ::iced::border::Radius"));
        assert!(generated.contains("top_left: 1.0 as f32"));
        assert!(generated.contains("top_right: 2.0 as f32"));
        assert!(generated.contains("bottom_right: 3.0 as f32"));
        assert!(generated.contains("bottom_left: 4.0 as f32"));
        assert!(generated.contains("::iced::widget::pin(__pin_content).x(12.0 as f32)"));
        assert!(generated.contains(
            ".on_show(move |__size| __StructureMessage::Shown(__size.width as f64, __size.height as f64))"
        ));
        assert!(generated.contains(".key(self.sensor_key)"));
        assert!(generated.contains("::iced::widget::responsive(move |__size|"));
        assert!(generated.contains("if __size.width < 600.0 as f32"));
        assert!(generated.contains("if ((__size.width as f64) < (__size.height as f64))"));
        assert!(generated.contains("if ((__size.width as f64) >= (__size.height as f64))"));
    }

    #[test]
    fn lowers_configured_scrollables_and_viewport_events() {
        let source = r#"app Scrolling
extern crate::backend
  scroll-style dynamic_scroll(busy:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  busy = false
  absolute_x = 0.0
  absolute_y = 0.0
  relative_x = 0.0
  relative_y = 0.0
on scrolled(ax, ay, rx, ry)
  absolute_x = ax
  absolute_y = ay
  relative_x = rx
  relative_y = ry
on viewport(ax, ay, reversed_x, reversed_y, rx, ry, bx, by, bw, bh, cx, cy, cw, ch)
view
  col
    scroll #feed direction=both width=fill height=200.0 bar=hidden bar-width=8.0 bar-margin=2.0 scroller-width=6.0 bar-spacing=4.0 anchor-x=end anchor-y=start auto=true scroll=scrolled style=dynamic_scroll(busy)
      text "Legacy offsets"
    scroll direction=both width=fill height=200.0 viewport=viewport style=dynamic_scroll(busy)
      col
        text "Complete viewport"
      active horizontal-disabled=false vertical-disabled=false
        container background=background text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=1.0 radius-tr=2.0 radius-br=3.0 radius-bl=4.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=true
        horizontal-rail background=background border=primary border-width=1.0 radius=2.0
        horizontal-scroller background=primary border=foreground border-width=1.0 radius=2.0
        vertical-rail background=background border=primary border-width=1.0 radius=2.0
        vertical-scroller background=primary border=foreground border-width=1.0 radius=2.0
        gap background=background
        auto background=background border=primary border-width=1.0 radius=4.0 shadow=danger shadow-x=1.0 shadow-y=2.0 shadow-blur=4.0 icon=foreground
      hovered horizontal-hovered=true vertical-hovered=false horizontal-disabled=false vertical-disabled=false
        horizontal-scroller background=foreground
      dragged horizontal-dragged=false vertical-dragged=true horizontal-disabled=false vertical-disabled=false
        vertical-scroller background=danger
"#;
        let generated = compile(source, "scrolling.ice").unwrap();
        assert!(generated.contains("scrollable::Direction::Both"));
        assert!(generated.contains("scrollable::Scrollbar::hidden().width(8.0 as f32)"));
        assert!(generated.contains(".anchor_x(::iced::widget::scrollable::Anchor::End)"));
        assert!(generated.contains(".auto_scroll(true)"));
        assert!(generated.contains("crate::backend::dynamic_scroll(__theme, __status, self.busy)"));
        assert!(generated.contains(
            ".style(move |__theme, __status| crate::backend::dynamic_scroll(__theme, __status, self.busy))"
        ));
        assert!(generated.contains(
            "let mut __style = crate::backend::dynamic_scroll(__theme, __status, self.busy); match __status"
        ));
        assert!(generated.contains("fn __ui_lang_check_scroll_style_dynamic_scroll"));
        assert!(generated.contains("let __absolute = __viewport.absolute_offset()"));
        assert!(generated.contains(
            "__ScrollingMessage::Scrolled(__absolute.x as f64, __absolute.y as f64, __relative.x as f64, __relative.y as f64)"
        ));
        assert!(generated.contains("absolute_offset_reversed()"));
        assert!(generated.contains("let __bounds = __viewport.bounds()"));
        assert!(generated.contains("let __content_bounds = __viewport.content_bounds()"));
        assert!(generated.contains("scrollable::Status::Hovered"));
        assert!(generated.contains("__horizontal_interaction == true"));
        assert!(generated.contains("let __style = &mut __style.container"));
        assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
        assert!(generated.contains("__style.horizontal_rail.scroller.background"));
        assert!(generated.contains("__style.vertical_rail.scroller.background"));
        assert!(generated.contains("__style.gap = ::std::option::Option::Some"));
        assert!(generated.contains("let __style = &mut __style.auto_scroll"));
        assert!(generated.contains("__style.shadow.blur_radius = 4.0 as f32"));
        assert!(generated.contains("__style.auto_scroll.icon"));
        let default_scroll = compile(
            &source.replace(" style=dynamic_scroll(busy)", ""),
            "scrolling.ice",
        )
        .unwrap();
        assert!(default_scroll.contains("scrollable::default(__theme, __status)"));
    }

    #[test]
    fn lowers_extended_text_input_behavior() {
        let source = r#"app Form
extern crate::backend
  input-style dynamic_input(disabled:bool)
font ui family=sans
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
  disabled = false
  secure = true
on submitted
on pasted(next)
  value = next
view
  input "Secret" #secret <-> value hint="Paste token" disabled=disabled secure=secure submit=submitted paste=pasted width=240.0 padding=8.0 text-size=14.0 line-height=1.2 align=center font=mono style=dynamic_input(disabled) @bg-background border border-primary rounded-lg focus:border-danger
    active background=background border=foreground border-width=1.0 radius=4.0 icon=primary placeholder=danger value=foreground selection=primary
    hovered background=background icon=foreground placeholder=danger value=foreground selection=primary
    focused background=background border=primary
    focused-hovered background=background border=foreground
    disabled background=background value=danger
    icon code="•" font=ui size=12.0 spacing=4.0 side=right
"#;
        let generated = compile(source, "form.ice").unwrap();
        assert!(generated.contains(".secure(self.secure)"));
        assert!(generated.contains(".width(240.0 as f32).padding(8.0 as f32).size(14.0 as f32)"));
        assert!(generated.contains("LineHeight::Relative(1.2 as f32)"));
        assert!(generated.contains(".align_x(::iced::alignment::Horizontal::Center)"));
        assert!(generated.contains(".font(::iced::Font::MONOSPACE)"));
        assert!(generated.contains("code_point: '•'"));
        assert!(generated.contains("family: ::iced::font::Family::SansSerif"));
        assert!(generated.contains("Side::Right"));
        assert!(generated.contains(".style(move |__theme, __status|"));
        assert!(
            generated.contains("crate::backend::dynamic_input(__theme, __status, self.disabled)")
        );
        assert!(generated.contains("fn __ui_lang_check_input_style_dynamic_input"));
        let custom = generated
            .find("crate::backend::dynamic_input(__theme, __status, self.disabled)")
            .unwrap();
        let utility = custom + generated[custom..].find(" __style.background =").unwrap();
        let statuses = utility + generated[utility..].find(" match __status").unwrap();
        assert!(custom < utility && utility < statuses);
        assert!(generated.contains("Status::Focused { is_hovered: true }"));
        assert!(generated.contains("__style.placeholder ="));
        assert!(generated.contains("__style.selection ="));
        assert!(generated.contains(".on_submit_maybe(if self.disabled"));
        assert!(generated.contains(".on_paste_maybe(if self.disabled"));
        let default_input = compile(
            &source.replace(" style=dynamic_input(disabled)", ""),
            "form.ice",
        )
        .unwrap();
        assert!(default_input.contains("text_input::default(__theme, __status)"));
    }

    #[test]
    fn lowers_button_children_and_typed_properties() {
        let source = r#"app Actions
extern crate::backend
  button-style dynamic_button(disabled:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  disabled = false
on pressed
view
  button #action disabled=disabled width=fill height=48.0 padding=8.0 clip=true style=dynamic_button(disabled) @bg-primary text-white rounded-lg disabled:opacity-50 -> pressed
    row
      text "Save"
      text "⌘S"
    active background=linear(1.57, primary@0.0, background@1.0) text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=4.0 pixel-snap=true
    hovered background=foreground text=background
    pressed background=primary
    disabled background=background text=foreground
"#;
        let generated = compile(source, "actions.ice").unwrap();
        assert!(generated.contains("let __button_content: ::iced::Element"));
        assert!(generated.contains("::iced::widget::row(__children)"));
        assert!(generated.contains(".width(::iced::Fill).height(48.0 as f32)"));
        assert!(generated.contains(".padding(8.0 as f32).clip(true)"));
        assert!(generated.contains(".on_press_maybe(if self.disabled"));
        assert!(
            generated.contains("crate::backend::dynamic_button(__theme, __status, self.disabled)")
        );
        assert!(generated.contains("fn __ui_lang_check_button_style_dynamic_button"));
        assert!(generated.contains("button::Status::Active =>"));
        assert!(generated.contains("button::Status::Hovered =>"));
        assert!(generated.contains("button::Status::Pressed =>"));
        assert!(generated.contains("button::Status::Disabled =>"));
        assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
        assert!(generated.contains("__style.shadow.offset.x = (-1.0) as f32"));
        assert!(generated.contains("__style.snap = true"));
        for preset in [
            "primary",
            "secondary",
            "success",
            "warning",
            "danger",
            "text",
            "background",
            "subtle",
        ] {
            let generated = compile(
                &source.replace("style=dynamic_button(disabled)", &format!("style={preset}")),
                "actions.ice",
            )
            .unwrap();
            assert!(generated.contains(&format!("button::{preset}(__theme, __status)")));
        }
    }

    #[test]
    fn lowers_complete_boolean_control_styles_and_typography() {
        let source = r#"app Preferences
extern crate::backend
  checkbox-style dynamic_checkbox(disabled:bool)
  toggler-style dynamic_toggler(disabled:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  enabled = false
on changed(next)
  enabled = next
view
  col
    checkbox "Checkbox" checked=enabled style=dynamic_checkbox(enabled) size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=advanced wrapping=word-or-glyph font=mono icon="✓" icon-size=12.0 icon-line-height=1.0 icon-shaping=basic -> changed _
      active checked background=linear(1.57, primary@0.0, background@1.0) icon=foreground text=foreground border=primary border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0
      active unchecked background=background icon=primary text=foreground border=foreground
      hovered checked background=primary icon=foreground text=foreground border=primary
      hovered unchecked background=foreground icon=background text=primary border=primary
      disabled checked background=background icon=foreground text=foreground border=foreground
      disabled unchecked background=background icon=primary text=foreground border=primary
    toggler "Toggler" checked=enabled style=dynamic_toggler(enabled) size=20.0 width=fill spacing=8.0 text-size=14.0 line-height=1.2 shaping=auto wrapping=glyph font=default align=right -> changed _
      active checked background=linear(1.57, primary@0.0, background@1.0) background-border=primary background-border-width=1.0 foreground=linear(0.0, foreground@0.0, primary@1.0) foreground-border=foreground foreground-border-width=2.0 text=foreground radius=7.0 radius-tl=6.0 radius-tr=7.0 radius-br=8.0 radius-bl=9.0 padding-ratio=0.125
      active unchecked background=background foreground=foreground text=primary
      hovered checked background=primary foreground=foreground text=foreground
      hovered unchecked background=foreground foreground=background text=primary
      disabled checked background=background foreground=foreground text=foreground
      disabled unchecked background=background foreground=primary text=foreground
"#;
        let generated = compile(source, "preferences.ice").unwrap();
        assert!(generated.contains(".size(20.0 as f32).spacing(8.0 as f32)"));
        assert!(generated.contains(".width(::iced::Fill)"));
        assert!(generated.contains(".text_shaping(::iced::widget::text::Shaping::Advanced)"));
        assert!(generated.contains(".text_wrapping(::iced::widget::text::Wrapping::WordOrGlyph)"));
        assert!(generated.contains("checkbox::Icon"));
        assert!(generated.contains("code_point: '✓'"));
        assert!(generated.contains(".text_alignment(::iced::widget::text::Alignment::Right)"));
        assert!(
            generated.contains("crate::backend::dynamic_checkbox(__theme, __status, self.enabled)")
        );
        assert!(generated.contains("fn __ui_lang_check_checkbox_style_dynamic_checkbox"));
        for (status, checked) in [
            ("Active", true),
            ("Active", false),
            ("Hovered", true),
            ("Hovered", false),
            ("Disabled", true),
            ("Disabled", false),
        ] {
            assert!(generated.contains(&format!(
                "checkbox::Status::{status} {{ is_checked: {checked} }}"
            )));
        }
        assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
        assert!(generated.contains("__style.icon_color ="));
        assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
        assert!(generated.contains("__style.border.width = 1.0 as f32"));
        assert!(generated.contains("top_left: 2.0 as f32"));
        for preset in ["primary", "secondary", "success", "danger"] {
            let generated = compile(
                &source.replace(
                    "style=dynamic_checkbox(enabled)",
                    &format!("style={preset}"),
                ),
                "preferences.ice",
            )
            .unwrap();
            assert!(generated.contains(&format!("checkbox::{preset}(__theme, __status)")));
        }
        assert!(
            generated.contains("crate::backend::dynamic_toggler(__theme, __status, self.enabled)")
        );
        assert!(generated.contains("fn __ui_lang_check_toggler_style_dynamic_toggler"));
        for (status, checked) in [
            ("Active", true),
            ("Active", false),
            ("Hovered", true),
            ("Hovered", false),
            ("Disabled", true),
            ("Disabled", false),
        ] {
            assert!(generated.contains(&format!(
                "toggler::Status::{status} {{ is_toggled: {checked} }}"
            )));
        }
        assert!(generated.contains("__style.background_border_width = 1.0 as f32"));
        assert!(generated.contains("__style.foreground = ::iced::Background"));
        assert!(generated.contains("__style.foreground_border_width = 2.0 as f32"));
        assert!(generated.contains("__style.text_color = ::std::option::Option::Some"));
        assert!(generated.contains("__style.border_radius = ::std::option::Option::Some"));
        assert!(generated.contains("top_left: 6.0 as f32"));
        assert!(generated.contains("__style.padding_ratio = 0.125 as f32"));
        let generated = compile(
            &source.replace(" style=dynamic_toggler(enabled)", ""),
            "preferences.ice",
        )
        .unwrap();
        assert!(generated.contains("toggler::default(__theme, __status)"));
    }

    #[test]
    fn lowers_full_text_format() {
        let source = r#"app Typography
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  text "Long text" width=fill height=40.0 size=16.0 line-height-px=20.0 font=mono align-x=justified align-y=center shaping=advanced wrapping=word-or-glyph @font-bold
"#;
        let generated = compile(source, "typography.ice").unwrap();
        assert!(generated.contains(".width(::iced::Fill).height(40.0 as f32)"));
        assert!(generated.contains("LineHeight::Absolute((20.0 as f32).into())"));
        assert!(generated.contains("text::Alignment::Justified"));
        assert!(generated.contains("alignment::Vertical::Center"));
        assert!(generated.contains("text::Shaping::Advanced"));
        assert!(generated.contains("text::Wrapping::WordOrGlyph"));
        assert!(generated.contains("..::iced::Font::MONOSPACE"));
    }

    #[test]
    fn lowers_native_text_style_callbacks() {
        let source = r#"app Typography
extern crate::backend
  text-style dynamic_text(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  active = true
view
  col
    text "Styled" style=dynamic_text(active)
    rich-text style=dynamic_text(active) color=foreground
      span "Rich"
"#;
        let generated = compile(source, "typography.ice").unwrap();
        assert!(generated.contains(
            "fn __ui_lang_check_text_style_dynamic_text(theme: &::iced::Theme, arg0: bool)"
        ));
        assert_eq!(
            generated
                .matches(
                    ".style(move |__theme| crate::backend::dynamic_text(__theme, self.active))"
                )
                .count(),
            2
        );
        assert!(generated.contains(
            ".style(move |__theme| crate::backend::dynamic_text(__theme, self.active)).color("
        ));
    }

    #[test]
    fn lowers_structured_rich_text_spans() {
        let source = r#"app Typography
font ui family=sans weight=medium stretch=normal style=normal
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
on link(url)
view
  rich-text width=fill height=48.0 size=16.0 line-height=1.2 font=ui align-x=justified align-y=center wrapping=word color=foreground @font-bold -> link _
    span "Ice " size=18.0 line-height-px=22.0 font=ui color=primary background=linear(1.57, background@0.0, primary@1.0) border=foreground border-width=1.0 radius=4.0 radius-tl=2.0 radius-tr=3.0 radius-br=5.0 radius-bl=6.0 padding=2.0 padding-left=4.0 underline strike=false
    span "language" link="https://example.com" background=background @text-lg font-bold text-primary
"#;
        let generated = compile(source, "rich.ice").unwrap();
        assert!(generated.contains("::iced::widget::rich_text(__rich_spans)"));
        assert!(generated.contains("::iced::widget::span(\"Ice \".to_owned())"));
        assert!(generated.contains(".size(18.0 as f32)"));
        assert!(generated.contains("LineHeight::Absolute((22.0 as f32).into())"));
        assert!(generated.contains(".background(::iced::Background::Color("));
        assert!(generated.contains(".background(::iced::Background::from(::iced::gradient::Linear::new(1.57 as f32).add_stop(0.0 as f32"));
        assert!(generated.contains(".border(::iced::Border"));
        assert!(generated.contains(".padding(::iced::Padding"));
        assert!(generated.contains(".underline(true).strikethrough(false)"));
        assert!(generated.contains(".link(\"https://example.com\".to_owned())"));
        assert!(
            generated.contains(".on_link_click(move |__link| __TypographyMessage::Link(__link))")
        );
        assert!(generated.contains(".width(::iced::Fill).height(48.0 as f32)"));
        assert!(generated.contains("text::Wrapping::Word"));
    }

    #[test]
    fn lowers_declared_font_descriptors_and_app_default() {
        let source = r#"app Typography
font brand family="Inter" weight=semibold stretch=semi-expanded style=italic default=true
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
view
  text "Brand" font=brand @font-bold
"#;
        let generated = compile(source, "typography.ice").unwrap();
        assert!(generated.contains(".default_font(::iced::Font"));
        assert!(generated.contains("Family::Name(\"Inter\")"));
        assert!(generated.contains("Weight::Semibold"));
        assert!(generated.contains("Stretch::SemiExpanded"));
        assert!(generated.contains("Style::Italic"));
        assert!(generated.contains("weight: ::iced::font::Weight::Bold, ..::iced::Font"));
    }

    #[test]
    fn lowers_typed_iced_extern_boundaries() {
        let source = r#"app Interop
extern crate::backend
  Failure(code:i64)
  component native_meter(value:f64) -> f64
  component passive() -> unit
  shader native_shader(value:f64) -> bool
  shader passive_shader() -> unit
  task focus_next() -> unit
  task save() -> i64 ! Failure
  subscription events() -> bool
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  amount = 1.0
  count = 0
  seen = false
on changed(next)
  amount = next
on focused
on focus
  task focus_next() -> focused
on save
  task save() -> saved _ | failed _
on saved(next)
  count = next
on failed(error)
  count = error.code
on event(next)
  seen = next
on shaded(next)
  seen = next
subscribe
  events() -> event _
view
  col
    extern native_meter(amount) -> changed _
    extern passive()
    shader native_shader(amount) width=fill height=64.0 -> shaded _
    shader passive_shader()
    button "Focus" -> focus
    button "Save" -> save
"#;
        let generated = compile(source, "interop.ice").unwrap();
        assert!(generated.contains("::iced::Element<'static, f64>"));
        assert!(generated.contains("::iced::Task<()>"));
        assert!(generated.contains("::iced::Subscription<bool>"));
        assert!(generated.contains("fn __ui_lang_check_shader_native_shader"));
        assert!(generated.contains("::iced::widget::shader::Program<bool>"));
        assert!(
            generated.contains(
                "::iced::widget::Shader::new(crate::backend::native_shader(self.amount))"
            )
        );
        assert!(generated.contains(".width(::iced::Fill).height(64.0 as f32)"));
        assert!(generated.contains(".subscription(Self::__subscription)"));
        assert!(generated.contains("native_meter(self.amount).map"));
        assert!(generated.contains("passive().map(move |__value| __InteropMessage::__ExternNoop)"));
        assert!(generated.contains("focus_next().map(|value| __InteropMessage::Focused)"));
        assert!(generated.contains("save().map(|result| match result"));
        assert!(generated.contains("Result::Err(error) => __InteropMessage::Failed(error)"));
    }

    #[test]
    fn lowers_native_keyboard_subscriptions() {
        let source = r#"app Shortcuts
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  key = ""
  command = false
on pressed(event)
  key = event.key
  command = event.modifiers.command
on released(event)
  key = event.key
on modifiers_changed(modifiers)
  command = modifiers.command
subscribe
  keyboard press status=ignored -> pressed _
  keyboard release -> released _
  keyboard modifiers -> modifiers_changed _
view
  text key
"#;
        let generated = compile(source, "shortcuts.ice").unwrap();
        assert!(generated.contains("struct __IceKeyPress"));
        assert!(generated.contains("struct __IceKeyRelease"));
        assert!(generated.contains("struct __IceKeyModifiers"));
        assert!(generated.contains("::iced::keyboard::listen().filter_map"));
        assert!(generated.contains("::iced::keyboard::Event::KeyPressed"));
        assert!(generated.contains("::iced::keyboard::Event::KeyReleased"));
        assert!(generated.contains("::iced::keyboard::Event::ModifiersChanged"));
        assert!(generated.contains("::iced::event::Status::Ignored"));
        assert!(generated.contains("self.key = event.key.clone()"));
        assert!(generated.contains("self.command = event.modifiers.command.clone()"));
    }

    #[test]
    fn lowers_native_timer_subscription() {
        let source = include_str!("../../../examples/iced-app/src/ui/timer.ice");
        let generated = compile(source, "timer.ice").unwrap();
        assert!(generated.contains("::iced::time::every(::std::time::Duration::from_millis(250))"));
        assert!(generated.contains("if self.auto_refresh { ::iced::Subscription::batch(["));
        assert!(generated.contains("]) } else { ::iced::Subscription::none() }"));
        assert!(generated.contains("::iced::time::now().map"));
        assert!(generated.contains("__TimerEventsMessage::Tick(__value)"));
        assert!(generated.contains(
            "::iced::time::repeat(crate::backend::refresh_time, ::std::time::Duration::from_millis(1000))"
        ));
        assert!(generated.contains(
            ".filter_map(|__value| crate::backend::even_refresh(__value)).with(self.generation)"
        ));
        assert!(generated.contains(
            ".filter_map(|__value| crate::backend::visible_pointer(__value.0, __value.1)).with(self.generation)"
        ));
        assert!(generated.contains(".filter_map(|_| crate::backend::allow_frame())"));
        assert!(generated.contains("__TimerEventsMessage::Refreshed(__value.0, __value.1)"));
    }

    #[test]
    fn lowers_generic_event_values_to_all_native_listeners() {
        let source = r#"app Events
extern crate::backend
  sync event_name(value:event) -> str
  sync event_label(value:event) -> str?
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on received(value)
on labeled(value)
on identified(id, value)
subscribe
  event -> received _
  event filter=event_label status=any -> labeled _
  event with-id status=ignored -> identified _ _
  event raw status=captured -> received _
  event raw with-id -> identified _ _
view
  text "Events"
"#;
        let generated = compile(source, "events.ice").unwrap();
        assert!(generated.contains("fn __ui_lang_check_sync_event_name"));
        assert!(generated.contains("arg0: ::iced::Event"));
        assert!(generated.contains("::iced::event::listen().map"));
        assert!(generated.contains("::iced::event::listen_with"));
        assert!(generated.contains("::iced::event::listen_raw"));
        assert!(generated.contains("::iced::event::Status::Ignored"));
        assert!(generated.contains("::iced::event::Status::Captured"));
        assert!(generated.contains("Some((__id, __event))"));
        assert!(generated.contains("filter_map(|__value| crate::backend::event_label(__value))"));
        assert!(generated.contains("__EventsMessage::Received(__value)"));
        assert!(generated.contains("__EventsMessage::Identified(__value.0, __value.1)"));
    }

    #[test]
    fn lowers_a_condition_around_window_frames() {
        let source = include_str!("../../../examples/iced-app/src/ui/window_events.ice");
        let generated = compile(source, "window_events.ice").unwrap();
        assert!(generated.contains(
            "if self.listen_frames { ::iced::Subscription::batch([::iced::window::frames()"
        ));
        assert!(generated.contains("]) } else { ::iced::Subscription::none() }"));
        assert!(generated.contains("::iced::Event::Window(__event)"));
        assert!(generated.contains("::iced::event::Status::Captured"));
        assert!(generated.contains("::iced::window::events().filter_map(|(__id, __event)|"));
        assert!(generated.contains("::iced::event::listen_with(|__event, __status, __id|"));
        assert!(generated.contains("(__id, __value.0, __value.1, __value.2, __value.3)"));
        assert!(generated.contains(".map(|_| __id)"));
        assert!(generated.contains(".map(|__value| (__id, __value))"));
        assert!(generated.contains(
            "__WindowEventsMessage::Opened(__value.0, __value.1, __value.2, __value.3, __value.4)"
        ));

        let legacy = source
            .replace("on focused(id)\n  last_window = some(id)", "on focused")
            .replace(
                "window focused with-id -> focused _",
                "window focused -> focused",
            );
        let generated = compile(&legacy, "window_events.ice").unwrap();
        assert!(generated.contains("map(move |__value| __WindowEventsMessage::Focused)"));
    }

    #[test]
    fn lowers_all_native_input_method_subscriptions() {
        let source = include_str!("../../../examples/iced-app/src/ui/input_method_events.ice");
        let generated = compile(source, "input_method_events.ice").unwrap();
        assert!(generated.contains("::iced::advanced::input_method::Event::Opened"));
        assert!(generated.contains("::iced::advanced::input_method::Event::Preedit"));
        assert!(generated.contains("::iced::advanced::input_method::Event::Commit"));
        assert!(generated.contains("::iced::advanced::input_method::Event::Closed"));
        assert!(generated.contains("i64::try_from(range.start)"));
        assert!(generated.contains("|__event, _, _|"));
    }

    #[test]
    fn lowers_all_native_mouse_subscriptions() {
        let source = include_str!("../../../examples/iced-app/src/ui/mouse_events.ice");
        let generated = compile(source, "mouse_events.ice").unwrap();
        assert!(generated.contains("fn __ice_mouse_button"));
        assert!(generated.contains("::iced::event::listen_with"));
        assert!(generated.contains("::iced::mouse::Event::CursorEntered"));
        assert!(generated.contains("::iced::mouse::Event::CursorLeft"));
        assert!(generated.contains("::iced::mouse::Event::CursorMoved"));
        assert!(generated.contains("::iced::mouse::Event::ButtonPressed"));
        assert!(generated.contains("::iced::mouse::Event::ButtonReleased"));
        assert!(generated.contains("::iced::mouse::Event::WheelScrolled"));
        assert!(generated.contains("::iced::mouse::ScrollDelta::Pixels"));
        assert!(generated.contains("::iced::event::Status::Captured"));
    }

    #[test]
    fn lowers_all_native_touch_subscriptions() {
        let source = include_str!("../../../examples/iced-app/src/ui/touch_events.ice");
        let generated = compile(source, "touch_events.ice").unwrap();
        assert!(generated.contains("::iced::touch::Event::FingerPressed"));
        assert!(generated.contains("::iced::touch::Event::FingerMoved"));
        assert!(generated.contains("::iced::touch::Event::FingerLifted"));
        assert!(generated.contains("::iced::touch::Event::FingerLost"));
        assert!(generated.contains("id.0.to_string()"));
        assert!(generated.contains("::iced::event::Status::Ignored"));
    }

    #[test]
    fn lowers_native_system_tasks_and_subscription() {
        let source = r#"app Diagnostics
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  cpu = ""
  mode = "none"
on inspect
  task system info -> inspected _
on inspected(info)
  cpu = info.cpu_brand
on read_theme
  task system theme -> theme_changed _
on theme_changed(next)
  mode = next
subscribe
  system theme -> theme_changed _
view
  text cpu
"#;
        let generated = compile(source, "diagnostics.ice").unwrap();
        assert!(generated.contains("struct __IceSystemInfo"));
        assert!(generated.contains("fn __ice_system_info(value: ::iced::system::Information)"));
        assert!(generated.contains("::iced::system::information().map(__ice_system_info)"));
        assert!(generated.contains("::iced::system::theme().map(__ice_system_theme)"));
        assert!(generated.contains("::iced::system::theme_changes().map(__ice_system_theme)"));
        assert!(generated.contains("self.cpu = info.cpu_brand.clone()"));
    }

    #[test]
    fn lowers_native_clipboard_tasks() {
        let source = r#"app Clipboard
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value:str? = none
on read
  task clipboard read -> read_done _
on read_done(next)
  value = next
on read_primary
  task clipboard read-primary -> read_done _
on write
  task clipboard write "copied"
on write_primary
  task clipboard write-primary "selected"
view
  text "Clipboard"
"#;
        let generated = compile(source, "clipboard.ice").unwrap();
        assert!(generated.contains("::iced::clipboard::read().map"));
        assert!(generated.contains("::iced::clipboard::read_primary().map"));
        assert!(generated.contains("::iced::clipboard::write::<__ClipboardMessage>"));
        assert!(generated.contains("::iced::clipboard::write_primary::<__ClipboardMessage>"));
    }

    #[test]
    fn lowers_native_runtime_font_loading() {
        let source = r#"app Fonts
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  font_bytes:bytes = bytes(00 01)
on load
  task font load font_bytes -> loaded _
on loaded(result)
view
  text "Fonts"
"#;
        let generated = compile(source, "fonts.ice").unwrap();
        assert!(generated.contains("::iced::font::load(self.font_bytes.clone()).map"));
        assert!(generated.contains("Result::Ok(value) => __FontsMessage::Loaded(value)"));
        assert!(generated.contains("Result::Err(error) => match error {}"));
    }

    #[test]
    fn lowers_all_static_widget_operations() {
        let source = r#"app Operations
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  value = ""
on checked(value)
on previous
  task widget focus-previous
on next
  task widget focus-next
on focus
  task widget focus #field
on check
  task widget focused #field -> checked _
on front
  task widget cursor-front #field
on end
  task widget cursor-end #field
on cursor
  task widget cursor #field 2
on all
  task widget select-all #field
on range
  task widget select #field 1 3
on snap
  task widget snap #list 0.0 1.0
on snap_end
  task widget snap-end #list
on scroll_to
  task widget scroll-to #list 0.0 24.0
on scroll_by
  task widget scroll-by #list -4.0 8.0
view
  col
    input "Value" #field <-> value
    scroll #list
      text "Content"
"#;
        let generated = compile(source, "operations.ice").unwrap();
        for function in [
            "focus_previous",
            "focus_next",
            "focus::<",
            "is_focused",
            "move_cursor_to_front",
            "move_cursor_to_end",
            "move_cursor_to::<",
            "select_all",
            "select_range",
            "snap_to::<",
            "snap_to_end",
            "scroll_to::<",
            "scroll_by::<",
        ] {
            assert!(generated.contains(function), "missing {function}");
        }
        assert!(generated.contains("Id::new(\"Operations/field\")"));
        assert!(generated.contains("Id::new(\"Operations/list\")"));
        assert!(generated.contains("RelativeOffset { x: (0.0) as f32, y: (1.0) as f32 }"));
        assert!(generated.contains("AbsoluteOffset"));
        assert!(generated.contains("(-4.0)"));
    }

    #[test]
    fn lowers_all_dynamic_widget_operations() {
        let source = r#"app DynamicOperations
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  ids = [1, 2]
  selected = 1
  value = ""
on checked(value)
on focus
  task widget focus #field(selected)
on check
  task widget focused #field(selected) -> checked _
on front
  task widget cursor-front #field(selected)
on end
  task widget cursor-end #field(selected)
on cursor
  task widget cursor #field(selected) 2
on all
  task widget select-all #field(selected)
on range
  task widget select #field(selected) 1 3
on snap
  task widget snap #list(selected) 0.0 1.0
on snap_end
  task widget snap-end #list(selected)
on scroll_to
  task widget scroll-to #list(selected) 0.0 24.0
on scroll_by
  task widget scroll-by #list(selected) -4.0 8.0
view
  col
    for id in ids
      input "Value" #field(id) <-> value
      scroll #list(id)
        text id
"#;
        let generated = compile(source, "dynamic_operations.ice").unwrap();
        for function in [
            "focus::<",
            "is_focused",
            "move_cursor_to_front",
            "move_cursor_to_end",
            "move_cursor_to::<",
            "select_all",
            "select_range",
            "snap_to::<",
            "snap_to_end",
            "scroll_to::<",
            "scroll_by::<",
        ] {
            assert!(generated.contains(function), "missing {function}");
        }
        assert!(
            generated.contains(
                "Id::from(format!(\"{}/field({})\", \"DynamicOperations\", self.selected))"
            )
        );
        assert!(
            generated.contains(
                "Id::from(format!(\"{}/list({})\", \"DynamicOperations\", self.selected))"
            )
        );
        assert!(generated.contains(
            ".id(::iced::widget::Id::from(format!(\"{}/field({})\", \"DynamicOperations\", id)))"
        ));
        assert!(generated.contains(
            ".id(::iced::widget::Id::from(format!(\"{}/list({})\", \"DynamicOperations\", id)))"
        ));
    }

    #[test]
    fn lowers_scoped_widget_operations() {
        let source = include_str!("../../../examples/iced-app/src/ui/scoped_widget_operations.ice");
        let generated = compile(source, "scoped_widget_operations.ice").unwrap();

        for id in [
            "Id::new(\"ScopedOperations/Field/field\")",
            "Id::new(\"ScopedOperations/frame/inner-frame/slot-field\")",
            "Id::new(\"ScopedOperations/details/list\")",
        ] {
            assert!(generated.contains(id), "missing {id}");
        }
        for path in [
            "format!(\"{}/field\", format!(\"{}/inner\", format!(\"{}/outer({})\", \"ScopedOperations\", self.selected)))",
            "format!(\"{}/field\", format!(\"{}/key({})\", \"ScopedOperations\", self.selected))",
            "format!(\"{}/filter\", format!(\"{}/header({})\", \"ScopedOperations\", self.column_index))",
            "format!(\"{}/cell\", format!(\"{}/column({})\", format!(\"{}/row({})\", \"ScopedOperations\", self.row_index), self.column_index))",
        ] {
            assert!(generated.contains(path), "missing {path}");
        }
    }

    #[test]
    fn lowers_widget_selectors() {
        let source = include_str!("../../../examples/iced-app/src/ui/widget_selectors.ice");
        let generated = compile(source, "widget_selectors.ice").unwrap();

        for expected in [
            "struct __IceWidgetTarget",
            "fn __ice_widget_target_from_target",
            "fn __ice_widget_target_from_text",
            "::iced::widget::selector::find(::iced::widget::selector::id(",
            "::iced::widget::selector::find(\"Search\".to_owned())",
            "::iced::widget::selector::find(::iced::Point::new(",
            "::iced::widget::selector::is_focused()",
            "::iced::widget::selector::find_all(\"Search\".to_owned())",
            "::iced::widget::selector::find_all(crate::backend::by_kind(",
            "fn __ui_lang_check_selector_by_kind",
            ".as_ref().map(|value| value.kind.clone())",
            ".as_ref().map(|value| value.x.clone())",
        ] {
            assert!(generated.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn checks_and_lowers_main_window_tasks() {
        let source = r#"app WindowTasks
  window child
    size 640 480
    position centered
extern crate::backend
  window describe_window(prefix:str) -> str
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
on closed
on size_read(width, height)
on bool_read(value)
on optional_bool_read(value)
on optional_pair_read(x, y)
on scale_read(value)
on mode_read(value)
on raw_id_read(value)
on text_read(value)
on screenshot_read(pixels, width, height, scale)
on opened(id)
  task window size target=id -> size_read _ _
on selected(id)
on close_target(id)
  task window close target=id
on open_child
  task window open child -> opened _
on open_default
  task window open -> close_target _
on read_oldest
  task window oldest -> selected _
on read_latest
  task window latest -> selected _
on close_window
  task window close
on drag_window
  task window drag
on drag_resize_window
  task window drag-resize north-east
on resize_window
  task window resize 800.0 600.0
on resizable_window
  task window resizable true
on min_size_window
  task window min-size 320.0 240.0
on clear_min_size
  task window min-size none
on max_size_window
  task window max-size 1920.0 1080.0
on resize_increments_window
  task window resize-increments 8.0 16.0
on read_size
  task window size -> size_read _ _
on read_maximized
  task window maximized -> bool_read _
on maximize_window
  task window maximize true
on read_minimized
  task window minimized -> optional_bool_read _
on minimize_window
  task window minimize false
on read_position
  task window position -> optional_pair_read _ _
on read_scale
  task window scale-factor -> scale_read _
on move_window
  task window move -10.0 20.0
on read_mode
  task window mode -> mode_read _
on mode_window
  task window set-mode fullscreen
on toggle_maximize_window
  task window toggle-maximize
on toggle_decorations_window
  task window toggle-decorations
on attention_window
  task window attention informational
on clear_attention
  task window attention none
on focus_window
  task window focus
on level_window
  task window level always-on-top
on system_menu_window
  task window system-menu
on read_raw_id
  task window raw-id -> raw_id_read _
on capture_window
  task window screenshot -> screenshot_read _ _ _ _
on passthrough_window
  task window mouse-passthrough false
on read_monitor
  task window monitor-size -> optional_pair_read _ _
on automatic_tabbing
  task window automatic-tabbing false
on change_icon
  task window icon bytes(ff 00 00 ff 00 ff 00 ff) 2 1
on describe_window
  task window describe_window("main") -> text_read _
view
  text "Window"
"#;
        let generated = compile(source, "window_tasks.ice").unwrap();
        for function in [
            "window::open",
            "window::oldest",
            "window::latest",
            "window::close",
            "window::drag",
            "window::drag_resize",
            "window::resize",
            "window::set_resizable",
            "window::set_min_size",
            "window::set_max_size",
            "window::set_resize_increments",
            "window::size",
            "window::is_maximized",
            "window::maximize",
            "window::is_minimized",
            "window::minimize",
            "window::position",
            "window::scale_factor",
            "window::move_to",
            "window::mode",
            "window::set_mode",
            "window::toggle_maximize",
            "window::toggle_decorations",
            "window::request_user_attention",
            "window::gain_focus",
            "window::set_level",
            "window::show_system_menu",
            "window::raw_id",
            "window::screenshot",
            "window::enable_mouse_passthrough",
            "window::disable_mouse_passthrough",
            "window::monitor_size",
            "window::allow_automatic_tabbing",
            "window::set_icon",
            "window::run",
        ] {
            assert!(generated.contains(function), "missing {function}");
        }
        assert!(generated.contains("fn __window_0() -> ::iced::window::Settings"));
        assert!(generated.contains("size: ::iced::Size::new(640 as f32, 480 as f32)"));
        assert!(generated.contains("::iced::window::open(Self::__window_0())"));
        assert!(generated.contains("::iced::window::open(::std::default::Default::default())"));
        assert!(generated.contains("::iced::window::size(id).map"));
        assert!(generated.contains("::iced::window::close::<__WindowTasksMessage>(id)"));
        assert!(generated.contains("value.to_string()"));
        assert!(generated.contains("value.rgba.to_vec()"));
        assert!(generated.contains("value.size.width as i64"));
        assert!(generated.contains("value.scale_factor as f64"));
        assert!(generated.contains("window::oldest().and_then"));
        assert!(
            generated.contains("crate::backend::describe_window(__window, \"main\".to_owned())")
        );
        assert!(generated.contains("fn __ui_lang_check_window_describe_window"));
        assert!(generated.contains("window: &dyn ::iced::window::Window"));
        assert!(generated.contains("__width.checked_mul(__height).is_some()"));

        let error = compile(
            &source.replacen("task window close\n", "task window close -> closed\n", 1),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E173");

        let error = compile(
            &source.replace("resize 800.0 600.0", "resize -1.0 600.0"),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E128");

        let error = compile(
            &source.replace("size -> size_read _ _", "size -> size_read _"),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E129");

        let error = compile(
            &source.replace("task window open child", "task window open missing"),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E173");

        let error = compile(
            &source.replace("task window oldest", "task window oldest target=id"),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E173");

        let error = compile(
            &source.replace("task window size target=id", "task window size target=true"),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E101");

        let error = compile(
            &source.replace(
                "task window screenshot -> screenshot_read _ _ _ _",
                "task window screenshot -> screenshot_read _ _ _",
            ),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E129");

        let error = compile(
            &source.replace(
                "bytes(ff 00 00 ff 00 ff 00 ff) 2 1",
                "bytes(ff 00 00 ff) 2 1",
            ),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E173");
        assert!(error.message.contains("width × height × 4"));

        let error = compile(
            &source.replace(
                "bytes(ff 00 00 ff 00 ff 00 ff) 2 1",
                "bytes(ff 00 00 ff 00 ff 00 ff) 4294967295 2",
            ),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E173");
        assert!(error.message.contains("dimensions are too large"));

        let error = compile(
            &source.replace("describe_window(\"main\")", "describe_window(true)"),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E101");

        let error = compile(
            &source.replace(
                "describe_window(\"main\") -> text_read _",
                "missing(\"main\") -> text_read _",
            ),
            "window_tasks.ice",
        )
        .unwrap_err();
        assert_eq!(error.code, "E130");
    }

    #[test]
    fn lowers_native_canvas_geometry_cache_and_events() {
        let source = r#"app Drawing
theme
  background #0f172a
  foreground #f8fafc
  primary #7c3aed
  danger #dc2626
state
  cached = true
  picture = rgba(1, 1, bytes(ff 00 ff ff))
on pressed(x, y)
on released(x, y)
on moved(x, y)
on scrolled(x, y, pixels)
on entered
on exited
view
  canvas width=fill height=240.0 cache=cached cache-group=drawings capture=true cursor=crosshair press=pressed release=released move=moved scroll=scrolled enter=entered exit=exited
    rect x=0.0 y=0.0 width=canvas_width height=canvas_height fill=linear(1.57, background@0.0, primary@1.0) stroke=foreground
    rect x=8.0 y=8.0 width=72.0 height=40.0 radius=8.0 radius-tl=4.0 stroke=foreground stroke-width=2.0 dash=(4.0, 2.0) dash-offset=1 cap=round join=bevel
    circle x=120.0 y=60.0 radius=24.0 fill=primary fill-rule=even-odd stroke=foreground
    line x1=16.0 y1=120.0 x2=200.0 y2=120.0 stroke=foreground stroke-width=3.0 cap=square
    text "Canvas" x=16.0 y=150.0 max-width=180.0 color=foreground size=18.0 line-height=1.2 font=default align-x=left align-y=top shaping=advanced
    image picture x=8.0 y=160.0 width=32.0 height=24.0 filter=nearest rotation=0.2 opacity=0.8 snap=true radius=4.0 radius-tl=2.0
    svg "<svg/>" memory x=48.0 y=160.0 width=24.0 height=24.0 color=foreground rotation=0.1 opacity=0.9
    path fill=primary stroke=foreground stroke-width=1.0
      move x=220.0 y=20.0
      line x=260.0 y=20.0
      arc x=260.0 y=40.0 radius=20.0 start=0.0 end=3.14
      arc-to ax=280.0 ay=60.0 bx=300.0 by=40.0 radius=8.0
      ellipse x=320.0 y=40.0 radius-x=20.0 radius-y=10.0 rotation=0.2 start=0.0 end=6.28
      bezier ax=340.0 ay=10.0 bx=360.0 by=70.0 x=380.0 y=40.0
      quadratic cx=400.0 cy=10.0 x=420.0 y=40.0
      rect x=220.0 y=80.0 width=30.0 height=20.0
      rounded x=260.0 y=80.0 width=30.0 height=20.0 radius=4.0
      circle x=320.0 y=90.0 radius=10.0
      close
    group x=10.0 y=10.0 rotate=0.1 scale=1.1 scale-x=1.0 scale-y=0.9 clip=(0.0, 0.0, 100.0, 100.0)
      circle x=20.0 y=20.0 radius=10.0 fill=foreground
    if cached
      circle x=360.0 y=180.0 radius=12.0 fill=primary
    for value in [12.0, 24.0]
      circle x=value y=210.0 radius=4.0 fill=foreground
"#;
        let generated = compile(source, "drawing.ice").unwrap();
        for expected in [
            "impl<State, Message, Draw, Update, Interaction> ::iced::widget::canvas::Program<Message>",
            "__state.cache.get_or_init",
            "Cache::with_group",
            "__ICE_CANVAS_GROUP_DRAWINGS",
            "::std::hash::Hash::hash",
            "::iced::widget::canvas::Path::rounded_rectangle",
            "__frame.fill_rectangle",
            "__frame.stroke_rectangle",
            "__frame.fill_text",
            "__frame.draw_image",
            "__frame.draw_svg",
            "::iced::advanced::svg::Svg",
            "__path.arc(",
            "__path.arc_to(",
            "__path.ellipse(",
            "__path.bezier_curve_to(",
            "__path.quadratic_curve_to(",
            "__frame.with_save",
            "__frame.with_clip",
            "__frame.scale_nonuniform",
            "::iced::mouse::Interaction::Crosshair",
            "::iced::widget::canvas::Action::publish",
            ".and_capture()",
        ] {
            assert!(generated.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn lowers_every_canvas_event_and_redraw_action() {
        let source = include_str!("../../../examples/iced-app/src/ui/canvas_events.ice");
        let generated = compile(source, "canvas_events.ice").unwrap();
        for expected in [
            "Event::InputMethod",
            "Event::Keyboard",
            "Event::Mouse",
            "Event::Touch",
            "Event::Window",
            "struct __IceKeyPress",
            "fn __ice_mouse_button",
            "KeyPressed",
            "KeyReleased",
            "ModifiersChanged",
            "CursorEntered",
            "CursorLeft",
            "CursorMoved",
            "ButtonPressed",
            "ButtonReleased",
            "WheelScrolled",
            "FingerPressed",
            "FingerMoved",
            "FingerLifted",
            "FingerLost",
            "RedrawRequested",
            "CloseRequested",
            "FileHovered",
            "FileDropped",
            "FilesHoveredLeft",
            "Action::publish",
            "Action::capture",
            "Action::request_redraw()",
            "Action::request_redraw_at",
            "Duration::from_millis(16)",
            ".and_capture()",
            "move_count: i64",
            "__state.move_count =",
            "fn __ice_canvas_interaction",
            "__ice_canvas_interaction(__interaction.as_str())",
            "__cursor.is_over(__bounds)",
        ] {
            assert!(generated.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn lowers_media_tooltip_and_pointer_events() {
        let source = r#"app Media
extern crate::backend
  svg-style dynamic_svg(active:bool)
  container-style dynamic_tooltip(active:bool)
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  active = true
  encoded_image = encoded(bytes(50 36 0a))
  rgba_image = rgba(1, 1, bytes(ff 00 00 ff))
on entered
on exited
on pressed
on moved(x, y)
on scrolled(x, y, pixels)
view
  col
    image "photo.ppm" width=fill height=64.0 fit=cover filter=nearest rotation=solid(0.5) opacity=0.8 scale=1.2 expand=true radius=4.0 radius-tl=1.0 radius-br=2.0 crop=(1, 2, 30, 40)
    image encoded_image
    image rgba_image
    viewer encoded_image width=fill(2) height=120.0 fit=contain filter=linear padding=8.0 min-scale=0.5 max-scale=4.0 scale-step=0.25
    viewer "photo.ppm" width=64.0 height=64.0
    svg "icon.svg" width=48.0 height=shrink fit=scale-down rotation=0.1 opacity=0.9 color=foreground hover=primary style=dynamic_svg(active)
    svg "<svg/>" memory width=16.0 color=foreground hover=none
    svg bytes(3c 73 76 67 2f 3e) memory width=16.0
    tooltip position=cursor gap=2.0 padding=5.0 delay=100 snap=false style=dynamic_tooltip(active) background=linear(1.57, background@0.0, primary/25@1.0) text=foreground border=primary/75 border-width=1.0 radius=5.0 radius-tl=2.0 shadow=black/50 shadow-x=-1.0 shadow-y=2.0 shadow-blur=8.0 pixel-snap=true
      mouse enter=entered exit=exited press=pressed move=moved scroll=scrolled cursor=pointer
        text "Hover"
      text "Tip"
"#;
        let generated = compile(source, "media.ice").unwrap();
        assert!(generated.contains("::iced::widget::image(\"photo.ppm\".to_owned())"));
        assert!(
            generated.contains(".rotation(::iced::Rotation::Solid(::iced::Radians(0.5 as f32)))")
        );
        assert!(generated.contains(".border_radius(::iced::border::Radius { top_left: 1.0 as f32, top_right: 4.0 as f32, bottom_right: 2.0 as f32, bottom_left: 4.0 as f32 })"));
        assert!(
            generated.contains("image::Handle::from_bytes(::std::vec![0x50u8, 0x36u8, 0x0au8])")
        );
        assert!(generated.contains("image::Handle::from_rgba((1).clamp(0, u32::MAX as i64) as u32, (1).clamp(0, u32::MAX as i64) as u32, ::std::vec![0xffu8, 0x00u8, 0x00u8, 0xffu8])"));
        assert!(generated.contains("::iced::widget::image::viewer(self.encoded_image.clone()).width(::iced::Length::FillPortion(2)).height(120.0 as f32).content_fit(::iced::ContentFit::Contain).filter_method(::iced::widget::image::FilterMethod::Linear).padding(8.0 as f32).min_scale(0.5 as f32).max_scale(4.0 as f32).scale_step(0.25 as f32)"));
        assert!(generated.contains("::iced::widget::image::viewer(::iced::widget::image::Handle::from_path(\"photo.ppm\".to_owned()))"));
        assert!(generated.contains(".crop(::iced::Rectangle { x: (1).clamp(0, u32::MAX as i64) as u32, y: (2).clamp(0, u32::MAX as i64) as u32, width: (30).clamp(0, u32::MAX as i64) as u32, height: (40).clamp(0, u32::MAX as i64) as u32 })"));
        assert!(generated.contains(".filter_method(::iced::widget::image::FilterMethod::Nearest)"));
        assert!(generated.contains("::iced::widget::svg(\"icon.svg\".to_owned())"));
        assert!(
            generated.contains("svg::Handle::from_memory((\"<svg/>\".to_owned()).into_bytes())")
        );
        assert!(generated.contains(
            "svg::Handle::from_memory(::std::vec![0x3cu8, 0x73u8, 0x76u8, 0x67u8, 0x2fu8, 0x3eu8])"
        ));
        assert!(generated.contains("crate::backend::dynamic_svg(__theme, __status, self.active)"));
        assert!(generated.contains("fn __ui_lang_check_svg_style_dynamic_svg"));
        assert!(generated.contains("svg::Status::Idle => __style.color = Some(::iced::Color"));
        assert!(generated.contains("svg::Status::Hovered => __style.color = Some(::iced::Color"));
        assert!(generated.contains("svg::Status::Hovered => __style.color = None"));
        let default_svg = compile(
            &source.replace(" style=dynamic_svg(active)", ""),
            "media.ice",
        )
        .unwrap();
        assert!(default_svg.contains("let mut __style = ::iced::widget::svg::Style::default()"));
        assert!(generated.contains("tooltip::Position::FollowCursor"));
        assert!(generated.contains(".delay(::std::time::Duration::from_millis(100 as u64))"));
        assert!(generated.contains("crate::backend::dynamic_tooltip(__theme, self.active)"));
        let preset_tooltip = compile(
            &source.replace("style=dynamic_tooltip(active)", "style=success"),
            "media.ice",
        )
        .unwrap();
        assert!(preset_tooltip.contains("container::success(__theme)"));
        assert!(generated.contains("__style.background = Some("));
        assert!(generated.contains("::iced::gradient::Linear::new(1.57 as f32)"));
        assert!(generated.contains("__style.border.radius"));
        assert!(generated.contains("__style.shadow.offset.x = (-1.0) as f32"));
        assert!(generated.contains("__style.shadow.blur_radius = 8.0 as f32"));
        assert!(generated.contains("__style.snap = true"));
        assert!(generated.contains(".on_enter(__MediaMessage::Entered)"));
        assert!(generated.contains(
            ".on_move(move |__point| __MediaMessage::Moved(__point.x as f64, __point.y as f64))"
        ));
        assert!(generated.contains("::iced::mouse::ScrollDelta::Lines"));
        assert!(generated.contains("__MediaMessage::Scrolled(__x as f64, __y as f64, true)"));
        assert!(generated.contains(".interaction(::iced::mouse::Interaction::Pointer)"));
    }
}
