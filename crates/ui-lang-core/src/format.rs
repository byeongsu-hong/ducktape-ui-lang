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

        let indent = raw.len() - raw.trim_start().len();
        while indents.last().is_some_and(|current| indent < *current) {
            indents.pop();
        }
        if indent > *indents.last().unwrap_or(&0) {
            indents.push(indent);
        }
        output.push_str(&"  ".repeat(indents.len() - 1));
        output.push_str(text);
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::format_source;

    #[test]
    fn normalizes_indent_and_trailing_newline() {
        let source = "app Demo\nextern crate::backend\n    Item(id:i64)\n    load() -> [Item] ! Item\ntheme\n    background #000000\n    foreground #ffffff\n    primary #333333\n    danger #ff0000\nstate\n    items:[Item] = []\non mount\n    run load() -> loaded _ | failed _\non loaded(next)\n    items = next\non failed(error)\n    items = []\nview\n    text len(items) @text-sm";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("extern crate::backend\n  Item"));
        assert!(formatted.ends_with('\n'));
    }
}
