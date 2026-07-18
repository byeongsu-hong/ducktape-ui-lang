use super::*;

pub(crate) fn split_style_utilities(source: &str) -> (&str, Vec<String>) {
    split_top_marker(source, "@").map_or_else(
        || (source.trim(), Vec::new()),
        |(core, styles)| {
            (
                core.trim(),
                styles.split_whitespace().map(str::to_owned).collect(),
            )
        },
    )
}

pub(in crate::parser) fn parse_component_slots(
    component: &str,
    line: &Line,
) -> Result<Vec<ComponentSlot>, Error> {
    if line.children.is_empty() {
        return Ok(Vec::new());
    }
    let named = line.children.iter().any(|child| child.text.ends_with(':'));
    if !named {
        let compound = line
            .children
            .iter()
            .map(|child| compound_slot_name(component, child))
            .collect::<Vec<_>>();
        if compound.iter().all(Option::is_some) {
            return line
                .children
                .iter()
                .zip(compound)
                .map(|(child, name)| {
                    Ok(ComponentSlot {
                        name: name.expect("all compound slots are present"),
                        content: Box::new(parse_view(child)?),
                        span: Span::line(child.number),
                    })
                })
                .collect();
        }
        if compound.iter().any(Option::is_some) {
            return Err(error(
                "E040",
                line,
                "cannot mix compound components with direct component children",
            )
            .hint(format!(
                "use only `{component}.Name` children, or wrap direct children in one layout"
            )));
        }
        return match line.children.as_slice() {
            [content] => Ok(vec![ComponentSlot {
                name: "children".into(),
                content: Box::new(parse_view(content)?),
                span: Span::line(content.number),
            }]),
            _ => Err(error(
                "E040",
                line,
                "component children need one root or named `slot:` blocks",
            )
            .hint("wrap siblings in row or col, or write `header:` and `body:` blocks")),
        };
    }

    line.children
        .iter()
        .map(|section| {
            let Some(name) = section.text.strip_suffix(':') else {
                return Err(error(
                    "E040",
                    section,
                    "cannot mix a direct child with named component slots",
                ));
            };
            if section.children.len() != 1 {
                return Err(error(
                    "E040",
                    section,
                    format!("component slot `{}` needs exactly one root", name.trim()),
                ));
            }
            Ok(ComponentSlot {
                name: identifier(name.trim(), section)?,
                content: Box::new(parse_view(&section.children[0])?),
                span: Span::line(section.number),
            })
        })
        .collect()
}

pub(in crate::parser) fn compound_slot_name(component: &str, line: &Line) -> Option<String> {
    let head = line.text.split_ascii_whitespace().next()?;
    let name = head.split_once('(').map_or(head, |(name, _)| name);
    let slot = name.strip_prefix(component)?.strip_prefix('.')?;
    (!slot.contains('.'))
        .then(|| identifier(slot, line).ok())
        .flatten()
}
