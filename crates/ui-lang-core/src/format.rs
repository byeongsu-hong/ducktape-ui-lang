use crate::parser::syntax::{split_top_marker, split_words};
use crate::parser::view::composition::split_style_utilities;
use crate::{Error, parse};

pub fn format_source(source: &str) -> Result<String, Error> {
    parse(source)?;
    Ok(format_fragment(source))
}

pub fn format_fragment(source: &str) -> String {
    let mut output = String::new();
    let mut indents = vec![0usize];
    let mut blank = false;

    for raw in source.lines() {
        let text = raw.trim();
        if text.is_empty() {
            blank = !output.is_empty();
            continue;
        }
        if blank && !output.ends_with("\n\n") {
            output.push('\n');
        }
        blank = false;

        let indent_bytes = raw.len() - raw.trim_start().len();
        let indent = raw[..indent_bytes].chars().count();
        while indents.last().is_some_and(|current| indent < *current) {
            indents.pop();
        }
        if indent > *indents.last().unwrap_or(&0) {
            indents.push(indent);
        }
        output.push_str(&"  ".repeat(indents.len() - 1));
        output.push_str(&canonicalize_style_line(text));
        output.push('\n');
    }
    output
}

// Rewrite only utilities whose generated owner is the same builder as the
// corresponding typed property. The parser's tokenizers preserve quoted and
// nested `@`/`->` markers.
fn canonicalize_style_line(source: &str) -> String {
    let (without_route, route) = split_top_marker(source, "->")
        .map_or((source, None), |(left, right)| (left, Some(right.trim())));
    let (core, mut styles) = split_style_utilities(without_route);
    if styles.is_empty() {
        return source.to_owned();
    }

    let words = split_words(core);
    let Some(kind) = words.first().map(String::as_str) else {
        return source.to_owned();
    };
    let has = |prefix: &str| words.iter().skip(1).any(|word| word.starts_with(prefix));
    let has_any = |prefixes: &[&str]| prefixes.iter().any(|prefix| has(prefix));
    let mut properties = Vec::new();

    match kind {
        "scroll" => {
            canonical_flag(
                &mut styles,
                &mut properties,
                !has("width="),
                "w-full",
                "width=fill",
            );
            canonical_flag(
                &mut styles,
                &mut properties,
                !has("height="),
                "h-full",
                "height=fill",
            );
        }
        "container" | "box" => {
            canonical_flag(
                &mut styles,
                &mut properties,
                !has("width="),
                "w-full",
                "width=fill",
            );
            canonical_flag(
                &mut styles,
                &mut properties,
                !has("height="),
                "h-full",
                "height=fill",
            );
            canonical_mapped(
                &mut styles,
                &mut properties,
                !has("max-width="),
                max_width_property,
            );
            canonical_padding(
                &mut styles,
                &mut properties,
                !has_any(PADDING_PROPERTIES),
                true,
            );
            canonical_surface(core, &mut styles, &mut properties);
        }
        "row" | "col" | "flex" => {
            canonical_mapped(
                &mut styles,
                &mut properties,
                !has("spacing="),
                spacing_property,
            );
            canonical_padding(
                &mut styles,
                &mut properties,
                !has_any(PADDING_PROPERTIES),
                true,
            );
            canonical_flag(
                &mut styles,
                &mut properties,
                !has("align="),
                "items-center",
                "align=center",
            );
        }
        "grid" => canonical_mapped(
            &mut styles,
            &mut properties,
            !has("spacing="),
            spacing_property,
        ),
        "input" => {
            canonical_flag(
                &mut styles,
                &mut properties,
                !has("width="),
                "w-full",
                "width=fill",
            );
            canonical_padding(&mut styles, &mut properties, !has("padding="), false);
        }
        "button" => canonical_padding(&mut styles, &mut properties, !has("padding="), false),
        "text" | "rich-text" | "span" => canonical_mapped(
            &mut styles,
            &mut properties,
            !has("size="),
            text_size_property,
        ),
        "pane" | "title" => canonical_surface(core, &mut styles, &mut properties),
        _ => {}
    }

    if properties.is_empty() {
        return source.to_owned();
    }
    let mut output = core.trim().to_owned();
    for property in properties {
        output.push(' ');
        output.push_str(&property);
    }
    if !styles.is_empty() {
        output.push_str(" @");
        output.push_str(&styles.join(" "));
    }
    if let Some(route) = route {
        output.push_str(" -> ");
        output.push_str(route);
    }
    output
}

const PADDING_PROPERTIES: &[&str] = &[
    "padding=",
    "padding-x=",
    "padding-y=",
    "padding-top=",
    "padding-right=",
    "padding-bottom=",
    "padding-left=",
];

fn canonical_flag(
    styles: &mut Vec<String>,
    properties: &mut Vec<String>,
    enabled: bool,
    utility: &str,
    property: &str,
) {
    if enabled && take_last(styles, |style| style == utility).is_some() {
        properties.push(property.to_owned());
    }
}

fn canonical_mapped(
    styles: &mut Vec<String>,
    properties: &mut Vec<String>,
    enabled: bool,
    map: fn(&str) -> Option<String>,
) {
    if !enabled {
        return;
    }
    if let Some(property) = take_last(styles, |style| map(style).is_some())
        .as_deref()
        .and_then(map)
    {
        properties.push(property);
    }
}

fn canonical_padding(
    styles: &mut Vec<String>,
    properties: &mut Vec<String>,
    enabled: bool,
    supports_axes: bool,
) {
    if !enabled {
        return;
    }
    if !supports_axes
        && styles.iter().any(|style| {
            spacing_value(style, "px-").is_some() || spacing_value(style, "py-").is_some()
        })
    {
        return;
    }

    let original_styles = styles.clone();
    let mut padding = [0_u16; 4];
    let mut found = false;
    styles.retain(|style| {
        let Some((axis, value)) = padding_utility(style) else {
            return true;
        };
        found = true;
        match axis {
            "all" => padding = [value; 4],
            "x" => {
                padding[1] = value;
                padding[3] = value;
            }
            "y" => {
                padding[0] = value;
                padding[2] = value;
            }
            _ => unreachable!("known padding axis"),
        }
        false
    });
    if !found {
        return;
    }
    // On input and button, an omitted padding call preserves Iced's widget
    // default while an explicit zero overrides it. Legacy `@p-0` emits no call.
    if !supports_axes && padding == [0; 4] {
        *styles = original_styles;
        return;
    }
    if padding.iter().all(|value| *value == padding[0]) {
        properties.push(format!("padding={}.0", padding[0]));
    } else if padding[0] == padding[2] && padding[1] == padding[3] {
        properties.push(format!("padding-x={}.0", padding[1]));
        properties.push(format!("padding-y={}.0", padding[0]));
    } else {
        for (name, value) in [
            ("top", padding[0]),
            ("right", padding[1]),
            ("bottom", padding[2]),
            ("left", padding[3]),
        ] {
            properties.push(format!("padding-{name}={value}.0"));
        }
    }
}

fn canonical_surface(core: &str, styles: &mut Vec<String>, properties: &mut Vec<String>) {
    let words = split_words(core);
    let has = |prefix: &str| words.iter().skip(1).any(|word| word.starts_with(prefix));
    let radius_has_required_surface = styles
        .iter()
        .any(|style| style.starts_with("bg-") || matches!(style.as_str(), "border" | "border-2"));
    canonical_mapped(styles, properties, !has("border-w="), border_width_property);
    if radius_has_required_surface
        && !["r=", "r-tl=", "r-tr=", "r-br=", "r-bl="]
            .iter()
            .any(|prefix| has(prefix))
    {
        canonical_mapped(styles, properties, true, radius_property);
    }
}

fn take_last(styles: &mut Vec<String>, predicate: impl Fn(&str) -> bool) -> Option<String> {
    let mut last = None;
    styles.retain(|style| {
        if predicate(style) {
            last = Some(style.clone());
            false
        } else {
            true
        }
    });
    last
}

fn spacing_value(style: &str, prefix: &str) -> Option<u16> {
    let value = style.strip_prefix(prefix)?;
    if !matches!(
        value,
        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "8" | "10" | "12" | "16" | "20" | "24"
    ) {
        return None;
    }
    value.parse::<u16>().ok().map(|value| value * 4)
}

fn padding_utility(style: &str) -> Option<(&'static str, u16)> {
    spacing_value(style, "p-")
        .map(|value| ("all", value))
        .or_else(|| spacing_value(style, "px-").map(|value| ("x", value)))
        .or_else(|| spacing_value(style, "py-").map(|value| ("y", value)))
}

fn spacing_property(style: &str) -> Option<String> {
    spacing_value(style, "gap-").map(|value| format!("spacing={value}.0"))
}

fn max_width_property(style: &str) -> Option<String> {
    let value = match style {
        "max-w-sm" => 384,
        "max-w-md" => 448,
        "max-w-lg" => 512,
        "max-w-xl" => 576,
        "max-w-2xl" => 672,
        _ => return None,
    };
    Some(format!("max-width={value}.0"))
}

fn text_size_property(style: &str) -> Option<String> {
    let value = match style {
        "text-xs" => 12,
        "text-sm" => 14,
        "text-base" => 16,
        "text-lg" => 18,
        "text-xl" => 20,
        "text-2xl" => 24,
        _ => return None,
    };
    Some(format!("size={value}.0"))
}

fn border_width_property(style: &str) -> Option<String> {
    match style {
        "border" => Some("border-w=1.0".to_owned()),
        "border-2" => Some("border-w=2.0".to_owned()),
        _ => None,
    }
}

fn radius_property(style: &str) -> Option<String> {
    let value = match style {
        "rounded-sm" => 2,
        "rounded" | "rounded-md" => 6,
        "rounded-lg" => 10,
        "rounded-full" => 999,
        _ => return None,
    };
    Some(format!("r={value}.0"))
}

#[cfg(test)]
mod tests {
    use super::{canonicalize_style_line, format_source};
    use crate::{analyze, compile};

    #[test]
    fn canonicalizes_only_same_builder_style_owners() {
        assert_eq!(
            canonicalize_style_line(
                r#"container @w-full h-full max-w-md p-4 px-2 border border-primary rounded-lg bg-bg"#
            ),
            r#"container width=fill height=fill max-width=448.0 padding-x=8.0 padding-y=16.0 border-w=1.0 r=10.0 @border-primary bg-bg"#
        );
        assert_eq!(
            canonicalize_style_line(r#"button "Save -> now" @p-2 bg-primary -> save"#),
            r#"button "Save -> now" padding=8.0 @bg-primary -> save"#
        );
        assert_eq!(
            canonicalize_style_line("col @w-full max-w-lg p-2 gap-3 items-center self-center"),
            "col spacing=12.0 padding=8.0 align=center @w-full max-w-lg self-center"
        );
        assert_eq!(
            canonicalize_style_line("input \"Value\" #field <-> value @p-2 px-3 w-full"),
            "input \"Value\" #field <-> value width=fill @p-2 px-3"
        );
        assert_eq!(
            canonicalize_style_line("button \"Zero\" @p-2 p-0 -> save"),
            "button \"Zero\" @p-2 p-0 -> save"
        );
        assert_eq!(canonicalize_style_line("col @gap-7"), "col @gap-7");
        assert_eq!(
            canonicalize_style_line("flex direction=column @gap-2 items-center"),
            "flex direction=column spacing=8.0 align=center"
        );
        assert_eq!(
            canonicalize_style_line("box @w-full p-2"),
            "box width=fill padding=8.0"
        );
        assert_eq!(
            canonicalize_style_line("container @rounded"),
            "container @rounded"
        );
        assert_eq!(
            canonicalize_style_line(
                "container bg=linear(radians(1.57), primary@0.0, bg@1.0) @border border-primary"
            ),
            "container bg=linear(radians(1.57), primary@0.0, bg@1.0) border-w=1.0 @border-primary"
        );
    }

    #[test]
    fn formatted_style_migrations_are_valid_and_idempotent() {
        let source = r#"app Demo
theme
  bg #000000
  fg #ffffff
  primary #336699
  danger #ff0000
state
view
  container @border border-primary rounded-lg bg-bg
    text "Hello" @text-lg
"#;
        let formatted = format_source(source).unwrap();
        assert_eq!(format_source(&formatted).unwrap(), formatted);
        assert!(formatted.contains("container border-w=1.0 r=10.0 @border-primary bg-bg"));
        assert!(formatted.contains("text \"Hello\" size=18.0"));
        analyze(&formatted).unwrap();

        let color_only = source.replace(
            "@border border-primary rounded-lg bg-bg",
            "@border border-primary rounded-lg",
        );
        let color_only = format_source(&color_only).unwrap();
        let generated = compile(&color_only, "demo.ice").unwrap();
        assert!(generated.contains("__style.border.width = 1.0 as f32"));
        assert!(generated.contains("color: ::iced::Color::from_rgba8(51, 102, 153"));

        let invalid = source.replace("@border border-primary rounded-lg bg-bg", "@rounded");
        assert_eq!(analyze(&invalid).unwrap_err().code, "E044");
        let invalid_formatted = format_source(&invalid).unwrap();
        assert_eq!(analyze(&invalid_formatted).unwrap_err().code, "E044");
    }
}
