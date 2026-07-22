use serde_json::{Value, json};

pub const LANGUAGE_REVISION: &str = "1.59";
pub const ICED_VERSION: &str = "0.14.0";
pub const ICED_WIDGET_VERSION: &str = "0.14.2";
pub const UI_LANG_RUNTIME_VERSION: &str = "0.1.0";
pub const ACCESSKIT_VERSION: &str = "0.24.1";
pub const ACCESSKIT_UNIX_VERSION: &str = "0.22.1";
pub const ACCESSKIT_WINDOWS_VERSION: &str = "0.32.0";

#[derive(Clone, Copy)]
struct Completion {
    label: &'static str,
    category: &'static str,
    insert_text: &'static str,
}

impl Completion {
    const fn new(label: &'static str, category: &'static str, insert_text: &'static str) -> Self {
        Self {
            label,
            category,
            insert_text,
        }
    }
}

const COMPLETIONS: &[Completion] = &[
    Completion::new("app", "declaration", "app ${1:Name}\n  $0"),
    Completion::new("use", "declaration", "use \"${1:path}.ice\""),
    Completion::new("extern", "declaration", "extern ${1:crate::backend}\n  $0"),
    Completion::new("state", "declaration", "state\n  ${1:name} = ${2:value}"),
    Completion::new(
        "component",
        "declaration",
        "component ${1:Name}(${2})\n  $0",
    ),
    Completion::new("slot", "declaration", "slot ${1:Name}"),
    Completion::new("on", "declaration", "on ${1:event}\n  $0"),
    Completion::new("view", "declaration", "view\n  $0"),
    Completion::new("if", "control", "if ${1:condition}\n  $0"),
    Completion::new("match", "control", "match ${1:value}\n  ${2:case}\n    $0"),
    Completion::new("for", "control", "for ${1:item} in ${2:items}\n  $0"),
    Completion::new(
        "keyed",
        "control",
        "keyed ${1:item} in ${2:items} by=${3:item.id}\n  $0",
    ),
    Completion::new(
        "lazy",
        "control",
        "lazy ${1:dependency} as ${2:value}\n  $0",
    ),
    Completion::new("row", "layout", "row\n  $0"),
    Completion::new("col", "layout", "col\n  $0"),
    Completion::new("flex", "layout", "flex width=fill\n  $0"),
    Completion::new("stack", "layout", "stack\n  $0"),
    Completion::new("scroll", "layout", "scroll\n  $0"),
    Completion::new("container", "layout", "container\n  $0"),
    Completion::new("box", "layout", "box\n  $0"),
    Completion::new("text", "widget", "text ${1:\"Text\"}"),
    Completion::new("input", "widget", "input \"${1:Label}\" <-> ${2:state}"),
    Completion::new("button", "widget", "button \"${1:Label}\" -> ${2:handler}"),
    Completion::new(
        "checkbox",
        "widget",
        "checkbox ${1:label} checked=${2:value} -> ${3:handler} _",
    ),
    Completion::new("image", "widget", "image ${1:handle}"),
    Completion::new(
        "run",
        "effect",
        "run ${1:action}(${2}) -> ${3:succeeded} _ | ${4:failed} _",
    ),
    Completion::new("<->", "operator", "<-> ${1:state}"),
    Completion::new("->", "operator", "-> ${1:handler}"),
    Completion::new("_", "operator", "_"),
    Completion::new("#id", "operator", "#${1:id}"),
];

fn property(name: &str, value_type: &str, required: bool) -> Value {
    json!({ "name": name, "type": value_type, "required": required })
}

fn properties(items: &[(&str, &str, bool)]) -> Vec<Value> {
    items
        .iter()
        .map(|(name, value_type, required)| property(name, value_type, *required))
        .collect()
}

fn padding_properties() -> Vec<Value> {
    properties(&[
        ("padding", "number", false),
        ("padding-x", "number", false),
        ("padding-y", "number", false),
        ("padding-top", "number", false),
        ("padding-right", "number", false),
        ("padding-bottom", "number", false),
        ("padding-left", "number", false),
    ])
}

fn surface_properties() -> Vec<Value> {
    properties(&[
        ("background", "background", false),
        ("text", "color-token", false),
        ("border", "color-token", false),
        ("border-width", "number", false),
        ("radius", "number", false),
        ("radius-tl", "number", false),
        ("radius-tr", "number", false),
        ("radius-br", "number", false),
        ("radius-bl", "number", false),
        ("shadow", "color-token", false),
        ("shadow-x", "number", false),
        ("shadow-y", "number", false),
        ("shadow-blur", "number", false),
        ("pixel-snap", "bool-expression", false),
    ])
}

fn flex_properties(column: bool) -> Vec<Value> {
    let mut output = properties(&[
        ("width", "length", false),
        ("height", "length", false),
        ("clip", "bool-expression", false),
        ("spacing", "number", false),
        ("align", "enum(start|center|end)", false),
        ("wrap", "flag", false),
        ("wrap-spacing", "number", false),
        ("wrap-align", "enum(start|center|end)", false),
    ]);
    output.extend(padding_properties());
    if column {
        output.push(property("max-width", "number", false));
    }
    output
}

fn css_flex_properties() -> Vec<Value> {
    let mut output = properties(&[
        (
            "direction",
            "enum(row|row-reverse|column|column-reverse)",
            false,
        ),
        (
            "flex-direction",
            "enum(row|row-reverse|column|column-reverse)",
            false,
        ),
        ("flex-flow", "direction,nowrap|wrap|wrap-reverse", false),
        ("flex-wrap", "enum(nowrap|wrap|wrap-reverse)", false),
        (
            "justify-content",
            "enum(normal|start|end|left|right|flex-start|flex-end|center|stretch|space-between|space-around|space-evenly)",
            false,
        ),
        (
            "align-items",
            "enum(normal|start|end|self-start|self-end|flex-start|flex-end|center|baseline|stretch)",
            false,
        ),
        (
            "align-content",
            "enum(normal|start|end|flex-start|flex-end|center|stretch|space-between|space-around|space-evenly)",
            false,
        ),
        ("gap", "number", false),
        ("row-gap", "number", false),
        ("column-gap", "number", false),
        ("width", "length", false),
        ("height", "length", false),
        ("max-width", "number", false),
        ("max-height", "number", false),
        ("clip", "bool-expression", false),
        ("spacing", "number", false),
        ("align", "enum(start|center|end)", false),
        ("wrap", "flag", false),
        ("wrap-spacing", "number", false),
        ("wrap-align", "enum(start|center|end)", false),
    ]);
    output.extend(padding_properties());
    output
}

fn keyed_properties() -> Vec<Value> {
    let mut output = properties(&[
        ("width", "length", false),
        ("height", "length", false),
        ("spacing", "number", false),
        ("max-width", "number", false),
        ("align", "enum(start|center|end)", false),
    ]);
    output.extend(padding_properties());
    output
}

fn container_properties() -> Vec<Value> {
    let mut output = properties(&[
        ("width", "length", false),
        ("height", "length", false),
        ("max-width", "number", false),
        ("max-height", "number", false),
        ("align-x", "enum(start|center|end)", false),
        ("align-y", "enum(start|center|end)", false),
        ("clip", "bool-expression", false),
        ("order", "integer-expression", false),
        ("flex-grow", "number", false),
        ("flex-shrink", "number", false),
        ("flex-basis", "auto|content|number|percent(number)", false),
        ("flex", "none|auto|initial|grow[,shrink[,basis]]", false),
        (
            "align-self",
            "enum(auto|normal|start|end|self-start|self-end|flex-start|flex-end|center|baseline|stretch)",
            false,
        ),
        ("margin", "auto|number|percent(number)", false),
        ("margin-x", "auto|number|percent(number)", false),
        ("margin-y", "auto|number|percent(number)", false),
        ("margin-top", "auto|number|percent(number)", false),
        ("margin-right", "auto|number|percent(number)", false),
        ("margin-bottom", "auto|number|percent(number)", false),
        ("margin-left", "auto|number|percent(number)", false),
        ("style", "extern-call", false),
    ]);
    output.extend(padding_properties());
    output.extend(surface_properties());
    output
}

fn text_properties() -> Vec<Value> {
    properties(&[
        ("width", "length", false),
        ("height", "length", false),
        ("size", "number", false),
        ("line-height", "number", false),
        ("line-height-px", "number", false),
        ("font", "font", false),
        (
            "align-x",
            "enum(default|left|center|right|justified)",
            false,
        ),
        ("align-y", "enum(top|center|bottom)", false),
        ("shaping", "enum(auto|basic|advanced)", false),
        ("wrapping", "enum(none|word|glyph|word-or-glyph)", false),
        ("style", "extern-call", false),
    ])
}

fn child_shape(min: usize, max: Option<usize>, role: &str) -> Value {
    json!({ "min": min, "max": max, "role": role })
}

fn details(
    contexts: &[&str],
    syntax: &str,
    children: Value,
    binding: Value,
    route: Value,
    properties: Vec<Value>,
) -> Value {
    json!({
        "contexts": contexts,
        "syntax": syntax,
        "children": children,
        "binding": binding,
        "route": route,
        "properties": properties,
    })
}

fn construct_schema(item: &Completion) -> Value {
    let leaf = || child_shape(0, Some(0), "none");
    let no_binding = || Value::Null;
    let no_route = || Value::Null;
    let shape = match item.label {
        "app" => details(
            &["document"],
            "app <Name>",
            child_shape(0, None, "app-setting"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "use" => details(
            &["document"],
            "use \"<relative-path>.ice\"",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "extern" => details(
            &["document"],
            "extern <rust-path>\n  [sync|task|component] <name>(<param>:<type>, ...) -> <type>[ ! <error-type>]",
            child_shape(0, None, "typed-extern-signature"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "state" => details(
            &["document", "component"],
            "state\n  <name>[:<type>] = <expression>",
            child_shape(0, None, "state-entry"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "component" => details(
            &["document"],
            "component <Name>(<prop>:<type>, ...)",
            child_shape(1, None, "component-state|component-handler|view-root"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "slot" => details(
            &["component-view"],
            "slot [<Name>]",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "on" => details(
            &["document", "component"],
            "on <handler>[(<payload>, ...)]",
            child_shape(0, None, "statement"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "view" => details(
            &["document"],
            "view",
            child_shape(1, Some(1), "view-root"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "if" => details(
            &["view"],
            "if <bool-expression>",
            child_shape(0, None, "view-node"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "match" => details(
            &["view"],
            "match <expression>\n  <case-expression>|_\n    <view-node>...",
            child_shape(1, None, "match-arm"),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        "for" => details(
            &["view"],
            "for <item> in <list-expression>",
            child_shape(0, None, "view-template"),
            json!({ "required": true, "name": "item", "source": "list-expression" }),
            no_route(),
            Vec::new(),
        ),
        "keyed" => details(
            &["view"],
            "keyed <item> in <list-expression> by=<key-expression>",
            child_shape(1, Some(1), "view-template"),
            json!({ "required": true, "name": "item", "source": "list-expression" }),
            no_route(),
            keyed_properties(),
        ),
        "lazy" => details(
            &["view"],
            "lazy <dependency-expression> as <name>",
            child_shape(1, Some(1), "view-root"),
            json!({ "required": true, "name": "name", "source": "dependency-expression" }),
            no_route(),
            Vec::new(),
        ),
        "row" => details(
            &["view"],
            "row [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(0, None, "view-node"),
            no_binding(),
            no_route(),
            flex_properties(false),
        ),
        "col" => details(
            &["view"],
            "col [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(0, None, "view-node"),
            no_binding(),
            no_route(),
            flex_properties(true),
        ),
        "flex" => details(
            &["view"],
            "flex [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(0, None, "view-node"),
            no_binding(),
            no_route(),
            css_flex_properties(),
        ),
        "stack" => details(
            &["view"],
            "stack [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(0, None, "view-node"),
            no_binding(),
            no_route(),
            properties(&[
                ("width", "length", false),
                ("height", "length", false),
                ("clip", "bool-expression", false),
                ("under", "u16", false),
            ]),
        ),
        "scroll" => details(
            &["view"],
            "scroll [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(1, Some(1), "view-root"),
            no_binding(),
            no_route(),
            properties(&[
                ("direction", "enum(vertical|horizontal|both)", false),
                ("width", "length", false),
                ("height", "length", false),
                ("bar", "enum(visible|hidden)", false),
                ("bar-width", "number", false),
                ("bar-margin", "number", false),
                ("scroller-width", "number", false),
                ("bar-spacing", "number", false),
                ("anchor-x", "enum(start|end)", false),
                ("anchor-y", "enum(start|end)", false),
                ("auto", "bool-expression", false),
                ("scroll", "payload-route(x,y,dx,dy)", false),
                ("viewport", "payload-route(bounds...)", false),
                ("style", "extern-call", false),
            ]),
        ),
        "container" => details(
            &["view"],
            "container [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(1, Some(1), "view-root"),
            no_binding(),
            no_route(),
            container_properties(),
        ),
        "box" => details(
            &["view"],
            "box [#<id>] [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(1, Some(1), "view-root"),
            no_binding(),
            no_route(),
            container_properties(),
        ),
        "text" => details(
            &["view"],
            "text <expression> [<property>=<value> ...] [@<semantic-utility> ...]",
            leaf(),
            no_binding(),
            no_route(),
            text_properties(),
        ),
        "input" => details(
            &["view"],
            "input \"<label>\" [#<id>] <-> <state> [<property>=<value> ...] [@<semantic-utility> ...]",
            child_shape(0, None, "optional-status-extension"),
            json!({ "required": true, "operator": "<->", "target": "state-identifier" }),
            no_route(),
            properties(&[
                ("label", "str-expression", false),
                ("description", "str-expression", false),
                ("hint", "string", false),
                ("disabled", "bool-expression", false),
                ("secure", "bool-expression", false),
                ("submit", "route", false),
                ("paste", "payload-route(text)", false),
                ("width", "length", false),
                ("padding", "number", false),
                ("text-size", "number", false),
                ("line-height", "number", false),
                ("align", "enum(left|center|right)", false),
                ("font", "font", false),
                ("style", "extern-call", false),
                ("icon", "one-character-string", false),
                ("icon-font", "font", false),
                ("icon-side", "enum(left|right)", false),
                ("icon-size", "number", false),
                ("icon-spacing", "number", false),
            ]),
        ),
        "button" => {
            let mut button = properties(&[
                ("description", "str-expression", false),
                ("disabled", "bool-expression", false),
                ("width", "length", false),
                ("height", "length", false),
                ("padding", "number", false),
                ("clip", "bool-expression", false),
                ("style", "button-preset|extern-call", false),
            ]);
            button.insert(
                0,
                json!({
                    "name": "label",
                    "type": "str-expression",
                    "required": false,
                    "requiredWhen": "button uses child content instead of a string label",
                }),
            );
            details(
                &["view"],
                "button [\"<label>\"] [#<id>] [<property>=<value> ...] [@<semantic-utility> ...] -> <handler> [_]",
                json!({ "min": 0, "max": 1, "role": "view-root", "condition": "exactly one child when string label is omitted" }),
                no_binding(),
                json!({ "required": true, "operator": "->", "payload": "unit" }),
                button,
            )
        }
        "checkbox" => details(
            &["view"],
            "checkbox <label-expression> [#<id>] checked=<bool-expression> [<property>=<value> ...] -> <handler> _",
            child_shape(0, None, "optional-status-extension"),
            no_binding(),
            json!({ "required": true, "operator": "->", "payload": "bool", "placeholder": "_" }),
            properties(&[
                ("label", "str-expression", false),
                ("description", "str-expression", false),
                ("checked", "bool-expression", true),
                ("disabled", "bool-expression", false),
                ("size", "number", false),
                ("width", "length", false),
                ("spacing", "number", false),
                ("text-size", "number", false),
                ("line-height", "number", false),
                ("shaping", "enum(auto|basic|advanced)", false),
                ("wrapping", "enum(none|word|glyph|word-or-glyph)", false),
                ("font", "font", false),
                ("icon", "one-character-string", false),
                ("icon-size", "number", false),
                ("icon-line-height", "number", false),
                ("icon-shaping", "enum(auto|basic|advanced)", false),
                ("style", "checkbox-preset|extern-call", false),
            ]),
        ),
        "image" => {
            let mut image = properties(&[
                ("label", "str-expression", false),
                ("width", "length", false),
                ("height", "length", false),
                (
                    "fit",
                    "enum(contain|cover|fill|none|scale-down)|expression",
                    false,
                ),
                ("rotation", "number", false),
                ("opacity", "number", false),
                ("filter", "enum(linear|nearest)", false),
                ("scale", "number", false),
                ("expand", "number", false),
                ("radius", "number", false),
                ("radius-tl", "number", false),
                ("radius-tr", "number", false),
                ("radius-br", "number", false),
                ("radius-bl", "number", false),
                ("crop", "tuple(number,number,number,number)", false),
            ]);
            image.insert(
                1,
                json!({
                    "name": "description",
                    "type": "str-expression",
                    "required": false,
                    "forbiddenWhen": "label is absent",
                }),
            );
            details(
                &["view"],
                "image <source-expression> [<property>=<value> ...]",
                leaf(),
                no_binding(),
                no_route(),
                image,
            )
        }
        "run" => details(
            &["handler-statement"],
            "run <extern-future>(<args>) -> <success-handler> _ [| <failure-handler> _]",
            leaf(),
            no_binding(),
            json!({
                "required": true,
                "operator": "->",
                "success": { "required": true, "payload": "extern output" },
                "failure": {
                    "payload": "extern error",
                    "requiredWhen": "extern declaration has `! <error-type>`",
                    "forbiddenWhen": "extern declaration has no error type"
                }
            }),
            Vec::new(),
        ),
        "<->" => details(
            &["binding-position"],
            "<-> <state-identifier>",
            leaf(),
            json!({ "required": true, "operator": "<->", "target": "state-identifier" }),
            no_route(),
            Vec::new(),
        ),
        "->" => details(
            &["route-position"],
            "-> <handler> [<payload-expression>]",
            leaf(),
            no_binding(),
            json!({ "required": true, "operator": "->", "payload": "expression|_" }),
            Vec::new(),
        ),
        "_" => details(
            &["route-payload"],
            "_",
            leaf(),
            no_binding(),
            json!({ "placeholder": true, "meaning": "forward emitted payload" }),
            Vec::new(),
        ),
        "#id" => details(
            &["view-node-id"],
            "#<scoped-id>",
            leaf(),
            no_binding(),
            no_route(),
            Vec::new(),
        ),
        _ => unreachable!("every completion is an Ice Core construct"),
    };
    let mut object = json!({
        "label": item.label,
        "category": item.category,
        "insertText": item.insert_text,
        "canonical": true,
    });
    object
        .as_object_mut()
        .expect("construct schema is an object")
        .extend(
            shape
                .as_object()
                .expect("construct details are an object")
                .clone(),
        );
    object
}

fn style_compatibility() -> Value {
    json!({
        "utilitySyntax": "forms omit the leading `@` marker",
        "statusCascade": {
            "base": "active fields apply to every native interaction status",
            "checked": "checked/selected statuses inherit their matching active checked/unchecked or selected/unselected fields",
            "compound": {
                "focused-hovered": ["active", "focused", "focused-hovered"],
                "opened-hovered": ["active", "opened", "opened-hovered"],
            },
            "precedence": "later, more-specific fields override inherited fields",
        },
        "patternNotation": {
            "N": "unsigned integer multiplied by four pixels",
            "TOKEN": "checked semantic theme token",
        },
        "deprecatedDirectBuilderForms": {
            "status": "accepted but canonicalized by cargo ice fmt",
            "condition": "only on the listed targets and only when no canonical property owns the same field",
            "mappings": [
                {
                    "targets": ["scroll", "container", "box", "flex", "input"],
                    "forms": { "w-full": "width=fill" },
                },
                {
                    "targets": ["scroll", "container", "box", "flex"],
                    "forms": { "h-full": "height=fill" },
                },
                {
                    "targets": ["container", "box", "flex"],
                    "forms": {
                        "max-w-sm": "max-width=384.0",
                        "max-w-md": "max-width=448.0",
                        "max-w-lg": "max-width=512.0",
                        "max-w-xl": "max-width=576.0",
                        "max-w-2xl": "max-width=672.0",
                    },
                },
                {
                    "targets": ["row", "col", "flex", "grid"],
                    "pattern": "gap-N",
                    "canonical": "spacing=(N*4).0",
                },
                {
                    "targets": ["container", "box", "row", "col", "flex"],
                    "patterns": ["p-N", "px-N", "py-N"],
                    "canonicalProperties": ["padding", "padding-x", "padding-y", "padding-top", "padding-right", "padding-bottom", "padding-left"],
                    "resolution": "apply utilities in source order, then emit the shortest equivalent typed padding fields",
                },
                {
                    "targets": ["input", "button"],
                    "pattern": "p-N",
                    "canonical": "padding=(N*4).0",
                    "condition": "no px-N or py-N is present and the final effective padding is nonzero",
                },
                {
                    "targets": ["row", "col", "flex"],
                    "forms": { "items-center": "align=center" },
                },
                {
                    "targets": ["container", "box", "pane", "title"],
                    "forms": {
                        "border": "border-width=1.0",
                        "border-2": "border-width=2.0",
                    },
                },
                {
                    "targets": ["container", "box", "pane", "title"],
                    "forms": {
                        "rounded-sm": "radius=2.0",
                        "rounded": "radius=6.0",
                        "rounded-md": "radius=6.0",
                        "rounded-lg": "radius=10.0",
                        "rounded-full": "radius=999.0",
                    },
                    "condition": "the original utility sequence satisfies E044 by also providing a utility background or border",
                },
                {
                    "targets": ["text", "rich-text", "span"],
                    "forms": {
                        "text-xs": "size=12.0",
                        "text-sm": "size=14.0",
                        "text-base": "size=16.0",
                        "text-lg": "size=18.0",
                        "text-xl": "size=20.0",
                        "text-2xl": "size=24.0",
                    },
                },
            ],
        },
        "intentionalUtilities": {
            "wrapperOwnedGeometry": [
                {
                    "targets": ["row", "col", "grid"],
                    "forms": ["w-full", "h-full", "max-w-sm", "max-w-md", "max-w-lg", "max-w-xl", "max-w-2xl", "self-center"],
                    "ownership": "outer wrapper",
                },
                {
                    "targets": ["stack"],
                    "forms": ["max-w-sm", "max-w-md", "max-w-lg", "max-w-xl", "max-w-2xl", "self-center"],
                    "ownership": "outer wrapper",
                },
                {
                    "targets": ["grid", "stack"],
                    "patterns": ["p-N", "px-N", "py-N"],
                    "ownership": "outer wrapper",
                },
            ],
            "dualOwnerGeometry": [
                {
                    "targets": ["stack"],
                    "forms": ["w-full", "h-full"],
                    "ownership": "inner stack and outer wrapper",
                    "conflict": "E045 when combined with typed stack width or height",
                },
            ],
            "typedPropertyGaps": [
                {
                    "targets": ["input", "button"],
                    "patterns": ["px-N", "py-N"],
                    "reason": "no equivalent axis-specific top-level padding property",
                },
            ],
            "semantic": ["bg-TOKEN", "text-TOKEN", "border-TOKEN", "state variants", "font-bold"],
            "rule": "geometry forms not listed as deprecated remain target-specific utilities; never deprecate a utility globally",
        },
    })
}

pub fn document() -> Value {
    let constructs = COMPLETIONS.iter().map(construct_schema).collect::<Vec<_>>();

    json!({
        "schemaVersion": 1,
        "language": {
            "name": "Ice",
            "revision": LANGUAGE_REVISION,
            "fileExtension": ".ice",
            "encoding": "UTF-8",
            "indent": "two spaces",
            "treeSyntax": "indentation",
        },
        "backend": {
            "iced": ICED_VERSION,
            "iced_widget": ICED_WIDGET_VERSION,
            "runtime": {
                "package": "ui-lang-runtime",
                "version": UI_LANG_RUNTIME_VERSION,
                "generatedRustPath": "::ui_lang_runtime",
                "publicApi": ["accessible", "navigation", "snapshot", "Bridge", "Role", "StableId"],
                "accesskit": ACCESSKIT_VERSION,
                "accesskit_unix": ACCESSKIT_UNIX_VERSION,
                "accesskit_unixTarget": "linux",
                "accesskit_windows": ACCESSKIT_WINDOWS_VERSION,
                "accesskit_windowsTarget": "windows",
            },
            "compatibilityCommand": "cargo ice compat",
        },
        "lsp": {
            "transport": "stdio Content-Length framing",
            "diagnostics": {
                "supported": true,
                "source": "ui_lang_core::analyze_file_with_overlays for existing file URIs; ui_lang_core::analyze otherwise",
                "inMemory": true,
                "rootBufferOverlay": true,
                "diskImports": true,
                "importedBufferOverlays": true,
                "diskFallbackOnClose": true,
                "ownership": "app roots own reports; reports are aggregated by diagnostic URI; fragments are not analyzed as standalone apps",
                "scope": "all open app roots and their overlaid import graphs",
                "reanalyze": "all open app roots after any open, change, or close",
            },
            "formatting": {
                "supported": true,
                "source": "ui_lang_core::format_fragment",
                "wholeDocument": true,
            },
            "completion": {
                "supported": true,
                "source": "core.constructs",
                "contextAware": false,
            },
            "definition": {
                "supported": true,
                "symbols": ["component", "handler"],
                "componentLocalHandlers": false,
                "crossFile": true,
                "source": "checked reference spans and imported source origins",
            },
            "rename": {
                "supported": true,
                "prepare": true,
                "symbols": ["component", "handler"],
                "componentLocalHandlers": false,
                "componentRule": "plain names and compound-family roots; a root rename cascades to dotted descendants",
                "definitionOnly": ["dotted component descendants", "mount handler"],
                "completeReferencesOnly": true,
                "declarationCollisionCheck": true,
                "allWorkspaceAppRootsMustCheck": true,
                "workspaceRootRequiredForImportedSymbols": true,
                "openBufferOverlays": true,
            },
        },
        "core": {
            "frozenAt": LANGUAGE_REVISION,
            "generative": true,
            "documentPrelude": {
                "syntax": "app <Name>\ntheme\n  background <color>\n  foreground <color>\n  primary <color>\n  danger <color>",
                "requiredDeclarations": ["app", "theme", "view"],
                "theme": {
                    "required": true,
                    "syntax": "theme",
                    "tokens": [
                        { "name": "background", "type": "color", "required": true },
                        { "name": "foreground", "type": "color", "required": true },
                        { "name": "primary", "type": "color", "required": true },
                        { "name": "danger", "type": "color", "required": true },
                    ],
                    "additionalTokens": true,
                },
            },
            "types": {
                "expression": "statically checked Ice expression",
                "bool-expression": "expression of bool",
                "str-expression": "expression of str",
                "number": "expression checked as a numeric value",
                "color": "#RRGGBB or #RRGGBBAA",
                "length": ["fill", "shrink", "fill(<u16>)", "<number-expression>"],
                "route": "<handler> [<payload-expression>|_]",
                "extern-call": "declared typed extern function call",
                "color-token": "declared theme token or checked color form",
                "background": "color token, color literal, or typed gradient",
                "font": ["default", "mono", "<declared-font>"],
            },
            "constructs": constructs,
            "styleCompatibility": style_compatibility(),
        },
    })
}

pub fn completion_items() -> Vec<Value> {
    COMPLETIONS
        .iter()
        .map(|item| {
            let kind = match item.category {
                "operator" => 24,
                "layout" | "widget" => 15,
                _ => 14,
            };
            json!({
                "label": item.label,
                "kind": kind,
                "detail": format!("Ice Core {}", item.category),
                "insertText": item.insert_text,
                "insertTextFormat": 2,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        ACCESSKIT_WINDOWS_VERSION, COMPLETIONS, ICED_VERSION, ICED_WIDGET_VERSION,
        UI_LANG_RUNTIME_VERSION, completion_items, document,
    };
    use serde_json::json;
    use std::collections::BTreeSet;

    #[test]
    fn schema_drives_completion_and_records_capability_gaps() {
        let schema = document();
        let constructs = schema["core"]["constructs"].as_array().unwrap();
        let completions = completion_items();

        assert_eq!(schema["backend"]["iced"], ICED_VERSION);
        assert_eq!(schema["backend"]["iced_widget"], ICED_WIDGET_VERSION);
        assert_eq!(
            schema["backend"]["runtime"]["version"],
            UI_LANG_RUNTIME_VERSION
        );
        assert_eq!(
            schema["backend"]["runtime"]["accesskit_windows"],
            ACCESSKIT_WINDOWS_VERSION
        );
        assert_eq!(
            schema["backend"]["runtime"]["accesskit_windowsTarget"],
            "windows"
        );
        assert_eq!(constructs.len(), COMPLETIONS.len());
        assert_eq!(completions.len(), COMPLETIONS.len());
        for (construct, completion) in constructs.iter().zip(&completions) {
            assert_eq!(construct["label"], completion["label"]);
            assert_eq!(construct["insertText"], completion["insertText"]);
            assert_eq!(completion["insertTextFormat"], 2);
        }
        assert_eq!(schema["lsp"]["definition"]["supported"], true);
        assert_eq!(schema["lsp"]["definition"]["componentLocalHandlers"], false);
        assert_eq!(schema["lsp"]["rename"]["supported"], true);
        assert_eq!(schema["lsp"]["rename"]["completeReferencesOnly"], true);
        assert_eq!(
            schema["lsp"]["rename"]["definitionOnly"],
            json!(["dotted component descendants", "mount handler"])
        );
    }

    #[test]
    fn generative_core_matches_the_contract_boundary() {
        const CORE_CONTRACT: &[&str] = &[
            "app",
            "use",
            "state",
            "component",
            "slot",
            "on",
            "view",
            "if",
            "match",
            "for",
            "keyed",
            "lazy",
            "row",
            "col",
            "flex",
            "stack",
            "scroll",
            "container",
            "box",
            "text",
            "input",
            "button",
            "checkbox",
            "image",
            "<->",
            "->",
            "_",
            "#id",
            "extern",
            "run",
        ];
        let schema = document();
        let constructs = schema["core"]["constructs"].as_array().unwrap();
        let actual = constructs
            .iter()
            .map(|construct| construct["label"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        let expected = CORE_CONTRACT.iter().copied().collect::<BTreeSet<_>>();

        assert_eq!(schema["core"]["generative"], true);
        assert_eq!(actual, expected);
        for construct in constructs {
            assert!(!construct["contexts"].as_array().unwrap().is_empty());
            assert!(!construct["syntax"].as_str().unwrap().is_empty());
            assert!(construct["children"].is_object());
            for property in construct["properties"].as_array().unwrap() {
                assert!(property["name"].is_string());
                assert!(property["type"].is_string());
                assert!(property["required"].is_boolean());
            }
        }
        let find = |label| {
            constructs
                .iter()
                .find(|construct| construct["label"] == label)
                .unwrap()
        };
        for label in [
            "row",
            "col",
            "stack",
            "scroll",
            "container",
            "text",
            "input",
            "button",
            "checkbox",
            "image",
        ] {
            assert!(!find(label)["properties"].as_array().unwrap().is_empty());
        }
        assert_eq!(find("view")["children"]["min"], 1);
        assert_eq!(find("scroll")["children"]["max"], 1);
        assert_eq!(find("input")["binding"]["operator"], "<->");
        assert!(
            find("input")["insertText"]
                .as_str()
                .unwrap()
                .contains("${1:Label}")
        );
        assert!(
            find("input")["syntax"]
                .as_str()
                .unwrap()
                .contains("\"<label>\"")
        );
        assert_eq!(find("button")["route"]["required"], true);
        assert_eq!(
            find("run")["route"]["failure"]["requiredWhen"],
            "extern declaration has `! <error-type>`"
        );
        assert_eq!(
            find("run")["route"]["failure"]["forbiddenWhen"],
            "extern declaration has no error type"
        );
        assert!(
            find("extern")["syntax"]
                .as_str()
                .unwrap()
                .contains("<type>")
        );
    }

    #[test]
    fn style_compatibility_is_target_scoped() {
        let schema = document();
        let styles = &schema["core"]["styleCompatibility"];
        let mappings = styles["deprecatedDirectBuilderForms"]["mappings"]
            .as_array()
            .unwrap();

        assert!(mappings.iter().any(|mapping| {
            mapping["targets"] == serde_json::json!(["scroll", "container", "box", "flex", "input"])
                && mapping["forms"]["w-full"] == "width=fill"
        }));
        assert!(mappings.iter().any(|mapping| {
            mapping["targets"] == serde_json::json!(["container", "box", "pane", "title"])
                && mapping["forms"]["border"] == "border-width=1.0"
        }));
        assert_eq!(
            styles["intentionalUtilities"]["dualOwnerGeometry"][0]["targets"],
            serde_json::json!(["stack"])
        );
        assert!(
            styles["intentionalUtilities"]["rule"]
                .as_str()
                .unwrap()
                .contains("never deprecate")
        );
    }

    #[test]
    fn prelude_and_accessibility_schema_match_accepted_source() {
        let schema = document();
        let tokens = schema["core"]["documentPrelude"]["theme"]["tokens"]
            .as_array()
            .unwrap();
        assert_eq!(
            tokens
                .iter()
                .map(|token| token["name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["background", "foreground", "primary", "danger"]
        );

        let constructs = schema["core"]["constructs"].as_array().unwrap();
        let find = |label| {
            constructs
                .iter()
                .find(|construct| construct["label"] == label)
                .unwrap()
        };
        for label in ["input", "button", "checkbox", "image"] {
            let names = find(label)["properties"]
                .as_array()
                .unwrap()
                .iter()
                .map(|property| property["name"].as_str().unwrap())
                .collect::<BTreeSet<_>>();
            assert!(names.contains("label"), "{label}");
            assert!(names.contains("description"), "{label}");
        }
        let button_label = find("button")["properties"]
            .as_array()
            .unwrap()
            .iter()
            .find(|property| property["name"] == "label")
            .unwrap();
        assert_eq!(
            button_label["requiredWhen"],
            "button uses child content instead of a string label"
        );
        let image_description = find("image")["properties"]
            .as_array()
            .unwrap()
            .iter()
            .find(|property| property["name"] == "description")
            .unwrap();
        assert_eq!(image_description["forbiddenWhen"], "label is absent");

        let source = r#"app Accessible
theme
  background #000000
  foreground #ffffff
  primary #333333
  danger #ff0000
state
  name = ""
  checked = false
on press
on toggle(value)
view
  col
    input "Name" label="Full name" description="Profile name" <-> name
    button label="Open help" description="Show help" -> press
      text "?"
    checkbox "Ready" label="Ready state" description="Current readiness" checked=checked -> toggle _
    image "photo.ppm" label="Portrait" description="Profile portrait"
"#;
        ui_lang_core::analyze(source).unwrap();
        let error = ui_lang_core::analyze(&source.replace("label=\"Open help\" ", "")).unwrap_err();
        assert_eq!(error.code, "E105");
        assert!(error.message.contains("child content"));
        let error = ui_lang_core::analyze(&source.replace("label=\"Portrait\" ", "")).unwrap_err();
        assert_eq!(error.code, "E105");
        assert!(
            error
                .message
                .contains("requires an accessibility `label=...`")
        );
    }
}
