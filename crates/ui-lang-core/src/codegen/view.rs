use super::*;

pub(super) fn render_node(
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
pub(super) fn render_container(
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
pub(super) fn render_overlay(
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
pub(super) fn render_rich_text(
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
pub(super) fn render_pane_grid(
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

pub(super) fn append_pane_grid_style(
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

pub(super) fn render_pane_content(
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

pub(super) fn render_rich_span(
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
pub(super) fn render_table(
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
pub(super) fn render_keyed_column(
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
pub(super) fn render_layout(
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

pub(super) fn scroll_bar_code(
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

pub(super) fn scroll_style_code(
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

pub(super) fn scroll_selector_code(style: &ScrollStatusStyle) -> String {
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

pub(super) fn append_scroll_status_style(
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
pub(super) fn append_scroll_surface_style(
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
