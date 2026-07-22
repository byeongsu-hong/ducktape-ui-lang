use super::*;

pub(in crate::check) fn check_id(
    id: &Option<Id>,
    env: &HashMap<String, Type>,
    document: &Document,
    ids: &mut HashSet<String>,
    span: &Span,
) -> Result<(), Error> {
    let Some(id) = id else {
        return Ok(());
    };
    if let Some(key) = &id.key {
        let ty = expr_type(key, env, document, span)?;
        if !matches!(ty, Type::I64 | Type::Str) {
            return Err(Error::new(
                "E160",
                span,
                "dynamic id keys must be i64 or str",
            ));
        }
    } else if !ids.insert(id.name.clone()) {
        return Err(Error::new(
            "E161",
            span,
            format!("duplicate local id `#{}`", id.name),
        ));
    }
    Ok(())
}

// Style ownership vocabulary (availability and precedence are target-specific):
//
// | Property key             | Canonical owner          | Compatibility owner       |
// |--------------------------|--------------------------|---------------------------|
// | layout.width             | width=                   | @w-full                   |
// | layout.height            | height=                  | @h-full                   |
// | layout.max_width         | max-width=               | @max-w-*                  |
// | layout.spacing           | spacing=                 | @gap-*                    |
// | layout.padding.*         | padding[-side]=          | @p-*/px-*/py-*            |
// | layout.cross_alignment   | align=                   | @items-center             |
// | layout.self_alignment    | (no typed form yet)      | @self-center              |
// | text.size                | size=/text-size=         | @text-xs/.../text-2xl     |
// | text.weight              | @font-bold               | font= descriptor          |
// | surface.background       | @bg-TOKEN                | property/status/callback  |
// | surface.text_color       | @text-TOKEN              | property/status/callback  |
// | surface.border_width     | border-w=                | @border/@border-2         |
// | surface.border_color     | @border-TOKEN            | property/status/callback  |
// | surface.radius.*         | r[-corner]=              | @rounded-*                |
// | button state background  | @hover:bg-/pressed:bg-   | button status background  |
// | input focused border     | @focus:border-*          | input focused border      |
// | button disabled opacity  | @disabled:opacity-*      | (no typed form)           |
//
// Current lowering order is container callback -> utilities -> typed fields; input callback ->
// utilities -> typed builder/status fields; and button preset/callback -> utilities -> typed
// builder/status fields. Explicit text size wins over @text-*; font= and @font-bold compose, with
// the utility selecting bold weight. Layout utilities may style an outer wrapper, sometimes in
// addition to the inner widget (stack sizing), so they have no global precedence. Utility
// collisions resolve in source order. A top-level `preset` is boot/state data, not a reusable
// visual style.
#[derive(Clone, Copy)]
pub(in crate::check) enum StyleTarget<'a> {
    Layout(Layout, &'a LayoutOptions),
    Container(&'a ContainerOptions),
    PaneContent(&'a ContainerStyleOptions),
    PaneTitle(&'a ContainerStyleOptions),
    Text(&'a TextOptions),
    RichText {
        options: &'a TextOptions,
        typed_color: bool,
    },
    RichSpan(&'a RichSpanOptions),
    Input(&'a InputOptions),
    Button(&'a ButtonOptions),
    Checkbox,
    Toggler,
    Slider,
    Progress,
    Radio,
    Rule,
    Space,
}

pub(in crate::check) fn valid_theme_color(value: &str, document: &Document) -> bool {
    let (name, opacity) = value
        .split_once('/')
        .map_or((value, None), |(name, opacity)| (name, Some(opacity)));
    (["white", "black", "transparent"].contains(&name) || document.theme.contains_key(name))
        && opacity.is_none_or(|opacity| opacity.parse::<u8>().is_ok_and(|opacity| opacity <= 100))
}

pub(in crate::check) fn check_styles(
    styles: &[String],
    document: &Document,
    span: &Span,
    target: StyleTarget<'_>,
) -> Result<(), Error> {
    let spacing = [
        "0", "1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20", "24",
    ];
    let is_linear = matches!(target, StyleTarget::Layout(Layout::Column | Layout::Row, _));
    let is_box = matches!(
        target,
        StyleTarget::Layout(
            Layout::Column | Layout::Row | Layout::Grid | Layout::Stack,
            _
        ) | StyleTarget::Container(_)
    );
    let is_visual_box = is_box
        || matches!(
            target,
            StyleTarget::PaneContent(_) | StyleTarget::PaneTitle(_)
        );
    let target_name = match target {
        StyleTarget::Layout(Layout::Column, _) => "col",
        StyleTarget::Layout(Layout::Row, _) => "row",
        StyleTarget::Layout(Layout::Scroll, _) => "scroll",
        StyleTarget::Layout(Layout::Grid, _) => "grid",
        StyleTarget::Layout(Layout::Stack, _) => "stack",
        StyleTarget::Container(_) => "container",
        StyleTarget::PaneContent(_) => "pane",
        StyleTarget::PaneTitle(_) => "pane title",
        StyleTarget::Text(_) | StyleTarget::RichText { .. } | StyleTarget::RichSpan(_) => "text",
        StyleTarget::Input(_) => "input",
        StyleTarget::Button(_) => "button",
        StyleTarget::Checkbox => "checkbox",
        StyleTarget::Toggler => "toggler",
        StyleTarget::Slider => "slider",
        StyleTarget::Progress => "progress",
        StyleTarget::Radio => "radio",
        StyleTarget::Rule => "rule",
        StyleTarget::Space => "space",
    };

    for original in styles {
        let (variant, utility) = original
            .split_once(':')
            .map_or((None, original.as_str()), |(variant, utility)| {
                (Some(variant), utility)
            });
        let color = ["bg-", "text-", "border-"]
            .iter()
            .find_map(|prefix| utility.strip_prefix(prefix));
        let valid_color = color.is_some_and(|value| valid_theme_color(value, document));
        let valid_spacing = ["p-", "px-", "py-", "gap-"].iter().any(|prefix| {
            utility
                .strip_prefix(prefix)
                .is_some_and(|value| spacing.contains(&value))
        });
        let known = matches!(
            utility,
            "w-full"
                | "h-full"
                | "max-w-sm"
                | "max-w-md"
                | "max-w-lg"
                | "max-w-xl"
                | "max-w-2xl"
                | "items-center"
                | "self-center"
                | "text-xs"
                | "text-sm"
                | "text-base"
                | "text-lg"
                | "text-xl"
                | "text-2xl"
                | "font-bold"
                | "border"
                | "border-2"
                | "rounded-sm"
                | "rounded"
                | "rounded-md"
                | "rounded-lg"
                | "rounded-full"
        ) || valid_spacing
            || valid_color
            || utility
                .strip_prefix("opacity-")
                .is_some_and(|value| ["0", "25", "50", "75", "100"].contains(&value));

        if !known {
            return Err(Error::new(
                "E041",
                span,
                format!("unsupported utility `{original}`"),
            ));
        }

        let supported = match variant {
            Some("hover" | "pressed") => {
                matches!(target, StyleTarget::Button(_)) && utility.starts_with("bg-")
            }
            Some("focus") => {
                matches!(target, StyleTarget::Input(_)) && utility.starts_with("border-")
            }
            Some("disabled") => {
                matches!(target, StyleTarget::Button(_)) && utility.starts_with("opacity-")
            }
            Some(_) => false,
            None => match utility {
                "w-full" => matches!(
                    target,
                    StyleTarget::Layout(_, _) | StyleTarget::Container(_) | StyleTarget::Input(_)
                ),
                "h-full" => {
                    matches!(
                        target,
                        StyleTarget::Layout(_, _) | StyleTarget::Container(_)
                    )
                }
                "max-w-sm" | "max-w-md" | "max-w-lg" | "max-w-xl" | "max-w-2xl" | "self-center" => {
                    is_box
                }
                "items-center" => is_linear,
                "text-xs" | "text-sm" | "text-base" | "text-lg" | "text-xl" | "text-2xl"
                | "font-bold" => matches!(
                    target,
                    StyleTarget::Text(_) | StyleTarget::RichText { .. } | StyleTarget::RichSpan(_)
                ),
                "border" | "border-2" => is_visual_box || matches!(target, StyleTarget::Input(_)),
                "rounded-sm" | "rounded" | "rounded-md" | "rounded-lg" | "rounded-full" => {
                    is_visual_box
                        || matches!(target, StyleTarget::Input(_) | StyleTarget::Button(_))
                }
                _ if utility.starts_with("gap-") => {
                    is_linear || matches!(target, StyleTarget::Layout(Layout::Grid, _))
                }
                _ if utility.starts_with("p-")
                    || utility.starts_with("px-")
                    || utility.starts_with("py-") =>
                {
                    is_box || matches!(target, StyleTarget::Input(_) | StyleTarget::Button(_))
                }
                _ if utility.starts_with("bg-") => {
                    is_visual_box
                        || matches!(target, StyleTarget::Input(_) | StyleTarget::Button(_))
                }
                _ if utility.starts_with("text-") => {
                    is_visual_box
                        || matches!(
                            target,
                            StyleTarget::Text(_)
                                | StyleTarget::RichText { .. }
                                | StyleTarget::RichSpan(_)
                                | StyleTarget::Button(_)
                        )
                }
                _ if utility.starts_with("border-") => {
                    is_visual_box || matches!(target, StyleTarget::Input(_))
                }
                _ => false,
            },
        };
        if !supported {
            return Err(Error::new(
                "E042",
                span,
                format!("utility `{original}` has no effect on `{target_name}`"),
            ));
        }
    }

    let has_border = styles
        .iter()
        .map(|style| base_utility(style))
        .any(|utility| matches!(utility, "border" | "border-2"));
    let has_typed_border = match target {
        StyleTarget::Container(options) => options.style.border_width.is_some(),
        StyleTarget::PaneContent(style) | StyleTarget::PaneTitle(style) => {
            style.border_width.is_some()
        }
        _ => false,
    };
    let has_border_color = styles
        .iter()
        .map(|style| base_utility(style))
        .any(|utility| utility.starts_with("border-") && utility != "border-2");
    if (is_visual_box || matches!(target, StyleTarget::Input(_)))
        && has_border_color
        && !has_border
        && !has_typed_border
    {
        return Err(Error::new(
            "E044",
            span,
            "border colors require `border-w=` (or deprecated `@border`/`@border-2`) on the same node",
        ));
    }
    let has_radius = styles
        .iter()
        .map(|style| base_utility(style))
        .any(|utility| utility.starts_with("rounded"));
    let has_background = styles
        .iter()
        .map(|style| base_utility(style))
        .any(|utility| utility.starts_with("bg-"));
    if is_visual_box && has_radius && !has_background && !has_border {
        return Err(Error::new(
            "E044",
            span,
            "rounded layout requires a background or border on the same node",
        ));
    }
    check_style_ownership(styles, span, target)?;
    Ok(())
}

fn check_style_ownership(
    styles: &[String],
    span: &Span,
    target: StyleTarget<'_>,
) -> Result<(), Error> {
    match target {
        StyleTarget::Layout(kind, options) => match kind {
            Layout::Scroll => {
                let scroll = options.scroll.as_ref().expect("scroll options");
                reject_duplicate_style_property(
                    span,
                    scroll.width.is_some(),
                    "width",
                    "width=",
                    true,
                    last_utility(styles, None, |utility| utility == "w-full"),
                )?;
                reject_duplicate_style_property(
                    span,
                    scroll.height.is_some(),
                    "height",
                    "height=",
                    true,
                    last_utility(styles, None, |utility| utility == "h-full"),
                )?;
            }
            Layout::Column | Layout::Row => {
                reject_duplicate_style_property(
                    span,
                    options.spacing.is_some(),
                    "spacing",
                    "spacing=",
                    true,
                    last_utility(styles, None, |utility| utility.starts_with("gap-")),
                )?;
                reject_duplicate_style_property(
                    span,
                    has_padding(&options.padding),
                    "padding",
                    "padding=",
                    true,
                    last_utility(styles, None, is_padding_utility),
                )?;
                reject_duplicate_style_property(
                    span,
                    options.align.is_some()
                        || options
                            .flexbox
                            .as_ref()
                            .is_some_and(|flexbox| flexbox.align_items.is_some()),
                    "alignment",
                    "align=",
                    true,
                    last_utility(styles, None, |utility| utility == "items-center"),
                )?;
            }
            Layout::Grid => reject_duplicate_style_property(
                span,
                options.spacing.is_some(),
                "spacing",
                "spacing=",
                true,
                last_utility(styles, None, |utility| utility.starts_with("gap-")),
            )?,
            Layout::Stack => {
                reject_stack_size_overlap(
                    span,
                    options.width.is_some(),
                    "width",
                    "width=",
                    last_utility(styles, None, |utility| utility == "w-full"),
                )?;
                reject_stack_size_overlap(
                    span,
                    options.height.is_some(),
                    "height",
                    "height=",
                    last_utility(styles, None, |utility| utility == "h-full"),
                )?;
            }
        },
        StyleTarget::Container(options) => {
            for (typed, property, owner, utility) in [
                (
                    options.width.is_some(),
                    "width",
                    "width=",
                    last_utility(styles, None, |utility| utility == "w-full"),
                ),
                (
                    options.height.is_some(),
                    "height",
                    "height=",
                    last_utility(styles, None, |utility| utility == "h-full"),
                ),
                (
                    options.max_width.is_some(),
                    "max-width",
                    "max-width=",
                    last_utility(styles, None, |utility| utility.starts_with("max-w-")),
                ),
                (
                    has_padding(&options.padding),
                    "padding",
                    "padding=",
                    last_utility(styles, None, is_padding_utility),
                ),
            ] {
                reject_duplicate_style_property(span, typed, property, owner, true, utility)?;
            }
            check_direct_surface_ownership(styles, span, &options.style)?;
        }
        StyleTarget::PaneContent(style) | StyleTarget::PaneTitle(style) => {
            check_direct_surface_ownership(styles, span, style)?;
        }
        StyleTarget::Text(options) => {
            check_text_size_ownership(styles, span, options.size.is_some())?;
        }
        StyleTarget::RichText {
            options,
            typed_color,
        } => {
            check_text_size_ownership(styles, span, options.size.is_some())?;
            reject_duplicate_style_property(
                span,
                typed_color,
                "text color",
                "color=",
                false,
                last_utility(styles, None, is_text_color_utility),
            )?;
        }
        StyleTarget::RichSpan(options) => {
            check_text_size_ownership(styles, span, options.size.is_some())?;
            reject_duplicate_style_property(
                span,
                options.color.is_some(),
                "text color",
                "color=",
                false,
                last_utility(styles, None, is_text_color_utility),
            )?;
        }
        StyleTarget::Input(options) => {
            reject_duplicate_style_property(
                span,
                options.width.is_some(),
                "width",
                "width=",
                true,
                last_utility(styles, None, |utility| utility == "w-full"),
            )?;
            reject_duplicate_style_property(
                span,
                options.padding.is_some(),
                "padding",
                "padding=",
                true,
                last_utility(styles, None, is_padding_utility),
            )?;
            for (name, status, focused) in [
                ("active", &options.style.active, false),
                ("hovered", &options.style.hovered, false),
                ("focused", &options.style.focused, true),
                ("focused-hovered", &options.style.focused_hovered, true),
                ("disabled", &options.style.disabled, false),
            ] {
                if let Some(status) = status {
                    check_input_status_ownership(styles, span, name, &status.options, focused)?;
                }
            }
        }
        StyleTarget::Button(options) => {
            reject_duplicate_style_property(
                span,
                options.padding.is_some(),
                "padding",
                "padding=",
                true,
                last_utility(styles, None, is_padding_utility),
            )?;
            for (name, status) in [
                ("active", &options.style.active),
                ("hovered", &options.style.hovered),
                ("pressed", &options.style.pressed),
                ("disabled", &options.style.disabled),
            ] {
                if let Some(status) = status {
                    check_button_status_ownership(styles, span, name, &status.options)?;
                }
            }
        }
        StyleTarget::Checkbox
        | StyleTarget::Toggler
        | StyleTarget::Slider
        | StyleTarget::Progress
        | StyleTarget::Radio
        | StyleTarget::Rule
        | StyleTarget::Space => {}
    }
    Ok(())
}

fn check_text_size_ownership(styles: &[String], span: &Span, typed: bool) -> Result<(), Error> {
    reject_duplicate_style_property(
        span,
        typed,
        "text size",
        "size=",
        true,
        last_utility(styles, None, is_text_size_utility),
    )
}

fn check_direct_surface_ownership(
    styles: &[String],
    span: &Span,
    style: &ContainerStyleOptions,
) -> Result<(), Error> {
    for (typed, property, owner, utility) in [
        (
            style.background.is_some(),
            "background",
            "bg=",
            last_utility(styles, None, |utility| utility.starts_with("bg-")),
        ),
        (
            style.text_color.is_some(),
            "text color",
            "text=",
            last_utility(styles, None, is_text_color_utility),
        ),
        (
            style.border_width.is_some(),
            "border width",
            "border-w=",
            last_utility(styles, None, |utility| {
                matches!(utility, "border" | "border-2")
            }),
        ),
        (
            style.border_color.is_some(),
            "border color",
            "border=",
            last_utility(styles, None, is_border_color_utility),
        ),
        (
            has_radius(style),
            "radius",
            "r=",
            last_utility(styles, None, |utility| utility.starts_with("rounded")),
        ),
    ] {
        reject_duplicate_style_property(span, typed, property, owner, false, utility)?;
    }
    Ok(())
}

fn check_input_status_ownership(
    styles: &[String],
    span: &Span,
    status: &str,
    options: &ContainerStyleOptions,
    focused: bool,
) -> Result<(), Error> {
    let background = last_utility(styles, None, |utility| utility.starts_with("bg-"));
    let border_color = focused
        .then(|| {
            last_utility(styles, Some("focus"), |utility| {
                utility.starts_with("border-")
            })
        })
        .flatten()
        .or_else(|| last_utility(styles, None, is_border_color_utility));
    let owners = [
        (
            options.background.is_some(),
            "background",
            "bg=",
            background,
        ),
        (
            options.border_width.is_some(),
            "border width",
            "border-w=",
            last_utility(styles, None, |utility| {
                matches!(utility, "border" | "border-2")
            }),
        ),
        (
            options.border_color.is_some(),
            "border color",
            "border=",
            border_color,
        ),
        (
            has_radius(options),
            "radius",
            "r=",
            last_utility(styles, None, |utility| utility.starts_with("rounded")),
        ),
    ];
    for (typed, property, owner, utility) in owners {
        let property = format!("{status} {property}");
        let owner = format!("{status} {owner}");
        reject_duplicate_style_property(span, typed, &property, &owner, false, utility)?;
    }
    Ok(())
}

fn check_button_status_ownership(
    styles: &[String],
    span: &Span,
    status: &str,
    options: &ContainerStyleOptions,
) -> Result<(), Error> {
    let background = match status {
        "hovered" => last_utility(styles, Some("hover"), |utility| utility.starts_with("bg-"))
            .or_else(|| last_utility(styles, None, |utility| utility.starts_with("bg-"))),
        "pressed" => last_utility(styles, Some("pressed"), |utility| {
            utility.starts_with("bg-")
        })
        .or_else(|| last_utility(styles, Some("hover"), |utility| utility.starts_with("bg-")))
        .or_else(|| last_utility(styles, None, |utility| utility.starts_with("bg-"))),
        _ => last_utility(styles, None, |utility| utility.starts_with("bg-")),
    };
    for (typed, property, owner, utility) in [
        (
            options.background.is_some(),
            "background",
            "bg=",
            background,
        ),
        (
            options.text_color.is_some(),
            "text color",
            "text=",
            last_utility(styles, None, is_text_color_utility),
        ),
        (
            has_radius(options),
            "radius",
            "r=",
            last_utility(styles, None, |utility| utility.starts_with("rounded")),
        ),
    ] {
        let property = format!("{status} {property}");
        let owner = format!("{status} {owner}");
        reject_duplicate_style_property(span, typed, &property, &owner, false, utility)?;
    }
    Ok(())
}

fn reject_duplicate_style_property(
    span: &Span,
    typed: bool,
    property: &str,
    typed_owner: &str,
    typed_is_canonical: bool,
    utility: Option<&str>,
) -> Result<(), Error> {
    let Some(utility) = utility.filter(|_| typed) else {
        return Ok(());
    };
    let hint = if typed_is_canonical {
        format!("remove `@{utility}`; `{typed_owner}` is the canonical spelling")
    } else {
        format!("choose one owner; `{typed_owner}` currently overrides `@{utility}` on this node")
    };
    Err(Error::new(
        "E045",
        span,
        format!("style property `{property}` is set by both `{typed_owner}` and `@{utility}`"),
    )
    .hint(hint))
}

fn reject_stack_size_overlap(
    span: &Span,
    typed: bool,
    property: &str,
    typed_owner: &str,
    utility: Option<&str>,
) -> Result<(), Error> {
    let Some(utility) = utility.filter(|_| typed) else {
        return Ok(());
    };
    Err(Error::new(
        "E045",
        span,
        format!("style property `{property}` is set by both `{typed_owner}` and `@{utility}`"),
    )
    .hint(format!(
        "remove `{typed_owner}`; `@{utility}` sizes both the stack and its generated outer wrapper"
    )))
}

fn last_utility<'a>(
    styles: &'a [String],
    variant: Option<&str>,
    predicate: impl Fn(&str) -> bool,
) -> Option<&'a str> {
    styles.iter().rev().find_map(|style| {
        let (actual_variant, utility) = style
            .split_once(':')
            .map_or((None, style.as_str()), |(variant, utility)| {
                (Some(variant), utility)
            });
        (actual_variant == variant && predicate(utility)).then_some(style.as_str())
    })
}

fn has_padding(padding: &PaddingOptions) -> bool {
    padding.all.is_some()
        || padding.x.is_some()
        || padding.y.is_some()
        || padding.top.is_some()
        || padding.right.is_some()
        || padding.bottom.is_some()
        || padding.left.is_some()
}

fn has_radius(style: &ContainerStyleOptions) -> bool {
    style.radius.is_some()
        || style.radius_top_left.is_some()
        || style.radius_top_right.is_some()
        || style.radius_bottom_right.is_some()
        || style.radius_bottom_left.is_some()
}

fn is_padding_utility(utility: &str) -> bool {
    ["p-", "px-", "py-"]
        .iter()
        .any(|prefix| utility.starts_with(prefix))
}

fn is_text_size_utility(utility: &str) -> bool {
    matches!(
        utility,
        "text-xs" | "text-sm" | "text-base" | "text-lg" | "text-xl" | "text-2xl"
    )
}

fn is_text_color_utility(utility: &str) -> bool {
    utility.starts_with("text-") && !is_text_size_utility(utility)
}

fn is_border_color_utility(utility: &str) -> bool {
    utility.starts_with("border-") && utility != "border-2"
}

pub(in crate::check) fn base_utility(style: &str) -> &str {
    style.split_once(':').map_or(style, |(_, utility)| utility)
}

pub(in crate::check) fn require_type(
    actual: &Type,
    expected: &Type,
    span: &Span,
) -> Result<(), Error> {
    if compatible(actual, expected) {
        Ok(())
    } else {
        Err(type_error(span, expected, actual))
    }
}

pub(in crate::check) fn compatible(left: &Type, right: &Type) -> bool {
    left == right
        || *left == Type::Unknown
        || *right == Type::Unknown
        || match (left, right) {
            (Type::List(left), Type::List(right)) | (Type::Option(left), Type::Option(right)) => {
                compatible(left, right)
            }
            (Type::Result(left_output, left_error), Type::Result(right_output, right_error)) => {
                compatible(left_output, right_output) && compatible(left_error, right_error)
            }
            _ => false,
        }
}

pub(in crate::check) fn type_error(span: &Span, expected: &Type, actual: &Type) -> Error {
    Error::new(
        "E101",
        span,
        format!(
            "expected `{}`, got `{}`",
            expected.display(),
            actual.display()
        ),
    )
}
