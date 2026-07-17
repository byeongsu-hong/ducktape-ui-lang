# Ice Language Specification 0.37

Status: implemented reference slice

Ice is a small frontend language with an iced backend. It is not Rust syntax,
JSX, or a token shortcut around a procedural macro. A frontend parses `.ice`
source, resolves names and types, checks UI semantics, and lowers a typed tree
to backend code.

This document describes what the repository implements. A section explicitly
marked “planned” is a design constraint, not accepted 0.37 syntax.

## 1. Design contract

Ice optimizes for two readers:

- a person should understand the screen, state, and effects by scanning it;
- an agent should see one canonical construct for each operation and receive a
  local error instead of guessing framework conventions.

The language therefore follows these rules:

1. Structure is indentation, with no closing delimiters.
2. UI state and transitions are explicit; generated messages and borrows are
   not.
3. Expressions are a small closed language, not embedded Rust.
4. Style utilities are a checked vocabulary. Unknown or ineffective utilities
   are errors.
5. Domain work crosses a typed `extern` boundary.
6. The compiler has one parser and checker shared by every frontend.

Ice owns transient/display state, layout, style, event routing, and calls to
actions. Rust owns validation, invariants, persistence, networking, security,
observability, and platform-specific behavior.

```text
interaction -> handler -> extern async Rust fn -> result handler -> state -> view
```

UI validation such as disabling an empty submit button is only a convenience.
The Rust action must still validate its input.

## 2. Compiler model

```text
UTF-8 .ice source graph
  -> relative `use` resolution + source map
  -> indentation-aware parser
  -> AST
  -> name resolution + type inference + semantic checks
  -> typed AST/IR
  -> iced Rust backend
  -> rustc
```

`ui-lang-core` owns the parser, AST, checker, formatter, and backend. The
`ui-lang` proc macro and `cargo-ice` command are thin frontends over that core.
There is no runtime parser and no `build.rs` generator.

The Rust adapter is one manifest-relative include:

```rust
ui_lang::include_app!("src/ui/tasks.ice");

fn main() -> iced::Result {
    Tasks::run()
}
```

The macro emits `include_str!` for the root and every imported file so Cargo
rebuilds after any `.ice` change. It also
emits probes for every declared extern struct field and async function. Rustc
therefore rejects missing, private, or shape-incompatible Rust items even when
an extern declaration is not reached at runtime.

## 3. Source rules

- Files are UTF-8 and use the `.ice` extension.
- Tabs are errors. `cargo ice fmt` prints two spaces per indentation level.
- A deeper indentation level makes the following lines children of the prior
  line. Indentation may only return to an existing level.
- Empty lines are ignored by the parser and normalized by the formatter.
- A line whose first non-space characters are `//` is a comment. Inline and
  block comments are not part of 0.37.
- Identifiers use ASCII letters, digits, and `_`, and cannot begin with a digit.
- App, extern-struct, and component names conventionally use `PascalCase`.
- State, field, function, handler, and parameter names conventionally use
  `snake_case`.
- Static IDs use kebab case after `#`, for example `#task-list`.
- Strings use double quotes and support `\n`, `\r`, `\t`, `\"`, and `\\`.
- A top-level `use "relative/file.ice"` includes declarations relative to the
  importing file. Paths must end in `.ice`, use `/`, and cannot be absolute.
- Imports may be nested. Re-importing the same canonical file is idempotent;
  import cycles and missing files are errors.

Top-level declarations are order-independent, but canonical source uses:

```text
app
use
extern
theme
qr
state
component
on
subscribe
view
```

An app source graph has exactly one `app` and one `view`, with at most one
`extern` namespace. The root file declares the app and normally the view;
imported fragments may hold any other top-level declarations. The graph may
have multiple components and handlers. The view and each component have exactly
one root node.

## 4. Compact grammar

The grammar below uses indentation (`INDENT`) as a block delimiter. `expr` is
defined in section 6.

```text
source_graph   = root_file imported_file*
root_file      = (app_decl | use_decl | declaration)*
imported_file  = (use_decl | declaration)*
use_decl       = "use" string
declaration    = extern_decl | theme_decl | font_decl | qr_decl | state_decl | component_decl
               | handler_decl | subscribe_decl | view_decl
document       = app_decl extern_decl? theme_decl qr_decl* state_decl?
                 component_decl* handler_decl* subscribe_decl? view_decl

app_decl       = "app" PascalName

extern_decl    = "extern" rust_path INDENT extern_item+
extern_item    = struct_sig | function_sig | extern_component_sig
               | extern_task_sig | extern_subscription_sig
struct_sig     = PascalName "(" field_list? ")"
field_list     = field ("," field)*
field          = name ":" type
function_sig   = name "(" field_list? ")" "->" type ("!" type)?
extern_component_sig
               = "component" name "(" field_list? ")" "->" type
extern_task_sig = "task" name "(" field_list? ")" "->" type ("!" type)?
extern_subscription_sig
               = "subscription" name "(" field_list? ")" "->" type

theme_decl     = "theme" INDENT color_entry+
color_entry    = name color

font_decl      = "font" name font_property*
font_property  = "family=" (string | "serif" | "sans" | "cursive" | "fantasy" | "mono")
               | "weight=" ("thin" | "extra-light" | "light" | "normal"
                 | "medium" | "semibold" | "bold" | "extra-bold" | "black")
               | "stretch=" ("ultra-condensed" | "extra-condensed" | "condensed"
                 | "semi-condensed" | "normal" | "semi-expanded" | "expanded"
                 | "extra-expanded" | "ultra-expanded")
               | "style=" ("normal" | "italic" | "oblique")
               | "default=" bool
font_ref       = "default" | "mono" | name

qr_decl        = "qr" name qr_payload qr_data_property*
qr_payload     = string | "bytes(" hex_byte* ")"
qr_data_property = "correction=" ("low" | "medium" | "quartile" | "high")
                 | "version=" ("normal(" u8 ")" | "micro(" u8 ")")

state_decl     = "state" INDENT state_entry+
state_entry    = name (":" type)? "=" expr

component_decl = "component" PascalName "(" field_list? ")"
                 INDENT node

handler_decl   = "on" name ("(" name_list? ")")?
                 INDENT statement*
statement      = name "=" expr
               | "return if" expr
               | "run" call "->" route ("|" route)?
               | "task" call "->" route ("|" route)?
               | "task system" ("info" | "theme") "->" route
               | "task clipboard" ("read" | "read-primary") "->" route
               | "task clipboard" ("write" | "write-primary") expr
               | "task widget" widget_operation ("->" route)?
widget_operation = "focus-previous" | "focus-next"
                 | ("focus" | "focused" | "cursor-front" | "cursor-end"
                   | "select-all" | "snap-end") id
                 | "cursor" id expr
                 | "select" id expr expr
                 | ("snap" | "scroll-to" | "scroll-by") id expr expr

subscribe_decl = "subscribe" INDENT subscription_use+
subscription_use
               = call "->" route
               | "keyboard" ("press" | "release" | "modifiers") "->" route
               | "system theme" "->" route

view_decl      = "view" INDENT node

node           = layout | text | input | button | checkbox | toggler
               | slider | progress | radio | pick_list | combo_box
               | rule | qr_code | space | float | pin | sensor | responsive
               | media | tooltip | mouse_area | theme_boundary
               | component_call | slot | extern_component_call | if_node | for_node
               | keyed_column | lazy_node | markdown_view | table_view
               | editor_view
layout         = "col" id? column_property* styles? INDENT node+
               | "row" id? flex_property* styles? INDENT node+
               | "scroll" id? scroll_property* styles? INDENT node
               | "grid" id? grid_property* styles? INDENT node+
               | "stack" id? stack_property* styles? INDENT node+
keyed_column   = "keyed" name "in" expr "by=" expr keyed_property*
                 INDENT node
keyed_property = ("width=" | "height=") length | "spacing=" expr
               | ("padding=" | "padding-x=" | "padding-y="
                 | "padding-top=" | "padding-right=" | "padding-bottom="
                 | "padding-left=") expr
               | "max-width=" expr
               | "align=" ("start" | "center" | "end")
lazy_node      = "lazy" expr "as" name INDENT node
markdown_view  = "markdown" name markdown_property* "->" route
markdown_property = ("text-size=" | "h1-size=" | "h2-size="
                  | "h3-size=" | "h4-size=" | "h5-size=" | "h6-size="
                  | "code-size=" | "spacing=") expr
table_view     = "table" name "in" expr table_property* INDENT table_column+
table_property = "width=" length
               | ("padding=" | "padding-x=" | "padding-y="
                 | "separator=" | "separator-x=" | "separator-y=") expr
table_column   = "column" table_column_property* INDENT
                 "header" INDENT node
                 "cell" INDENT node
table_column_property = "width=" length
                      | "align-x=" ("left" | "center" | "right")
                      | "align-y=" ("top" | "center" | "bottom")
editor_view    = "editor" id? "<->" name editor_property*
editor_property = "placeholder=" string | "width=" expr | "height=" length
                | ("min-height=" | "max-height=" | "size="
                  | "line-height=" | "line-height-px=" | "padding=") expr
                | "wrapping=" text_wrapping
                | "font=" font_ref
                | "highlight=" string
                | "highlight-theme=" ("solarized-dark" | "base16-mocha"
                  | "base16-ocean" | "base16-eighties" | "inspired-github")
                | "disabled=" expr
column_property = flex_property | "max-width=" expr
flex_property  = ("width=" | "height=") length | "spacing=" expr
               | ("padding=" | "padding-x=" | "padding-y="
                 | "padding-top=" | "padding-right=" | "padding-bottom="
                 | "padding-left=") expr
               | "align=" ("start" | "center" | "end") | "clip=" expr
               | "wrap" | "wrap-spacing=" expr
               | "wrap-align=" ("start" | "center" | "end")
stack_property = ("width=" | "height=") length | "clip=" expr
               | "under=" u16
grid_property  = "columns=" expr | "fluid=" expr | "width=" expr
               | "spacing=" expr | "height=" grid_sizing
grid_sizing    = length | "aspect(" expr "," expr ")"
scroll_property = "direction=" ("vertical" | "horizontal" | "both")
                | ("width=" | "height=") length
                | "bar=" ("visible" | "hidden")
                | ("bar-width=" | "bar-margin=" | "scroller-width="
                  | "bar-spacing=") expr
                | ("anchor-x=" | "anchor-y=") ("start" | "end")
                | "auto=" expr | "scroll=" route
text           = "text" expr text_property* styles?
text_property  = ("width=" | "height=") length | "size=" expr
               | ("line-height=" | "line-height-px=") expr
               | "font=" font_ref
               | "align-x=" text_alignment
               | "align-y=" ("top" | "center" | "bottom")
               | "shaping=" ("auto" | "basic" | "advanced")
               | "wrapping=" ("none" | "word" | "glyph" | "word-or-glyph")
input          = "input" string id? "<->" name input_property* styles?
input_property = "hint=" string | ("disabled=" | "secure=") expr
               | ("submit=" | "paste=") route | "width=" length
               | ("padding=" | "text-size=" | "line-height=") expr
               | "align=" ("left" | "center" | "right")
               | "font=" font_ref | "icon=" string
               | "icon-side=" ("left" | "right")
               | ("icon-size=" | "icon-spacing=") expr
button         = "button" (string | INDENT node) id? button_property*
                 styles? "->" route
button_property = "disabled=" expr | ("width=" | "height=") length
                | ("padding=" | "clip=") expr
checkbox       = "checkbox" expr id? "checked=" expr bool_property*
                 checkbox_icon_property* styles? "->" route
toggler        = "toggler" expr "checked=" expr bool_property*
                 ("align=" text_alignment)? styles? "->" route
bool_property  = "disabled=" expr | "size=" expr | "width=" length
               | ("spacing=" | "text-size=" | "line-height=") expr
               | "shaping=" ("auto" | "basic" | "advanced")
               | "wrapping=" ("none" | "word" | "glyph" | "word-or-glyph")
               | "font=" font_ref
checkbox_icon_property = "icon=" string
                       | ("icon-size=" | "icon-line-height=") expr
                       | "icon-shaping=" ("auto" | "basic" | "advanced")
text_alignment = "default" | "left" | "center" | "right" | "justified"
slider         = "slider" expr "min=" expr "max=" expr slider_property*
                 styles? "->" route (INDENT slider_status+)?
slider_property = ("step=" | "default=" | "shift-step=") expr
                | ("width=" | "height=") length
                | "vertical" | "release=" route
slider_status  = ("active" | "hovered" | "dragged") slider_style_property*
slider_style_property
               = ("rail-start=" | "rail-end=" | "rail-border="
                 | "handle-color=" | "handle-border=") name ("/" u8)?
               | ("rail-width=" | "rail-border-width="
                 | "handle-border-width=") expr
               | ("rail-radius=" | "rail-radius-tl=" | "rail-radius-tr="
                 | "rail-radius-br=" | "rail-radius-bl=") expr
               | "handle=" ("circle(" expr ")" | "rect(" u16 ")")
               | ("handle-radius=" | "handle-radius-tl="
                 | "handle-radius-tr=" | "handle-radius-br="
                 | "handle-radius-bl=") expr
progress       = "progress" expr progress_property* styles?
progress_property
               = ("min=" | "max=") expr
               | ("length=" | "girth=") length | "vertical"
               | "style=" ("primary" | "secondary" | "success"
                 | "warning" | "danger")
               | ("background=" | "bar=" | "border=") name ("/" u8)?
               | ("border-width=" | "radius=" | "radius-tl="
                 | "radius-tr=" | "radius-br=" | "radius-bl=") expr
radio          = "radio" expr "value=" expr "selected=" expr
                 styles? "->" route
pick_list      = "pick" expr expr pick_property* "->" route
pick_property  = "placeholder=" expr | "width=" length
               | "menu-height=" length | "padding=" expr
               | "text-size=" expr | "open=" route | "close=" route
combo_box      = "combo" name expr string combo_property* "->" route
combo_property = "width=" length | "menu-height=" length
               | "padding=" expr | "text-size=" expr
               | "input=" route | "hover=" route
               | "open=" route | "close=" route
float          = "float" ("scale=" expr)? ("x=" expr)? ("y=" expr)?
                 INDENT node
pin            = "pin" (("width=" | "height=") length)?
                 ("x=" expr)? ("y=" expr)? INDENT node
sensor         = "sensor" sensor_property+ INDENT node
sensor_property = ("show=" | "resize=" | "hide=") route
                | "key=" expr | "anticipate=" expr | "delay=" expr
responsive     = "responsive" responsive_mode
                 (("width=" | "height=") length)? INDENT node+
responsive_mode = "at=" expr | "size=(" name "," name ")"
rule           = "rule" ("horizontal" | "vertical") rule_property* styles?
rule_property  = "thickness=" expr | "style=" ("default" | "weak")
               | "fill=" rule_fill | "color=" name ("/" u8)?
               | ("radius=" | "radius-tl=" | "radius-tr="
                 | "radius-br=" | "radius-bl=") expr
               | "snap=" expr
rule_fill      = "full" | "percent(" expr ")" | "pad(" u16 ")"
               | "pad(" u16 "," u16 ")"
qr_code        = "qr" name qr_property*
qr_property    = ("cell-size=" | "total-size=") expr
               | ("cell=" | "background=") name ("/" u8)?
space          = "space" ("width=" length)? ("height=" length)? styles?
media          = ("image" | "svg") expr media_property*
media_property = ("width=" | "height=") length
               | "fit=" ("contain" | "cover" | "fill" | "none" | "scale-down")
               | "rotation=" expr | "opacity=" expr
               | "filter=" ("linear" | "nearest")
               | "scale=" expr | "expand=" expr | "radius=" expr
length         = "fill" | "fill(" u16 ")" | "shrink" | expr
tooltip        = "tooltip" tooltip_property* INDENT node node
tooltip_property
               = "position=" ("top" | "bottom" | "left" | "right" | "cursor")
               | "gap=" expr | "padding=" expr | "delay=" expr | "snap=" expr
               | "style=" ("transparent" | "rounded" | "bordered" | "dark"
                 | "primary" | "secondary" | "success" | "warning" | "danger")
               | ("background=" | "text=" | "border=" | "shadow=")
                 name ("/" u8)?
               | ("border-width=" | "radius=" | "radius-tl="
                 | "radius-tr=" | "radius-br=" | "radius-bl="
                 | "shadow-x=" | "shadow-y=" | "shadow-blur=") expr
               | "pixel-snap=" expr
mouse_area     = "mouse" mouse_property+ INDENT node
mouse_property = ("press=" | "release=" | "double=" | "right_press="
               | "right_release=" | "middle_press=" | "middle_release="
               | "enter=" | "move=" | "scroll=" | "exit=") route
               | "cursor=" mouse_cursor
theme_boundary = "theme" theme_preset? theme_property* INDENT node
theme_preset   = "default" | "app" | built_in_iced_theme
built_in_iced_theme
               = "light" | "dark" | "dracula" | "nord"
               | "solarized-light" | "solarized-dark"
               | "gruvbox-light" | "gruvbox-dark"
               | "catppuccin-latte" | "catppuccin-frappe"
               | "catppuccin-macchiato" | "catppuccin-mocha"
               | "tokyo-night" | "tokyo-night-storm" | "tokyo-night-light"
               | "kanagawa-wave" | "kanagawa-dragon" | "kanagawa-lotus"
               | "moonfly" | "nightfly" | "oxocarbon" | "ferra"
theme_property = ("text=" | "background=") name ("/" u8)?
component_call = PascalName "(" expr_list? ")" id? (INDENT node)?
slot           = "slot"
extern_component_call
               = "extern" name "(" expr_list? ")" ("->" route)?
if_node        = "if" expr INDENT node+
for_node       = "for" name "in" expr INDENT node+

property       = "hint=" string | "disabled=" expr | "checked=" expr
styles         = "@" utility+
id             = "#" kebab_name | "#" name "(" expr ")"
route          = name | name "(" route_arg_list? ")"
route_arg      = expr | "_"
```

Media fixed lengths, rotation, opacity, scale, and radius are `f64`; rotation
is radians, opacity is `0.0..=1.0`, scale is positive, and sizes/radius are
non-negative. `filter`, `scale`, `expand`, and `radius` are image-only.
Every `length` position accepts fixed `f64`, `fill`, `fill(N)` portions with a
decimal `u16`, or `shrink`; out-of-range portions fail during parsing.
`rule` exposes all four iced fill modes. Percent is checked in `0.0..=100.0`;
padding is `u16`. Its default/weak preset can be overridden by a checked theme
color token (including `/0..100` opacity), uniform or per-corner non-negative
radius, and bool pixel snapping.
QR declarations accept UTF-8 strings or arbitrary hexadecimal bytes. Normal
versions are `1..=40`, micro versions are `1..=4`, and omitted correction uses
iced's medium default. A QR view accepts one of `cell-size=` or `total-size=`
plus checked cell/background colors. Its encoded data is built once at app
startup and borrowed by each view.
Tooltip gap/padding are non-negative `f64`, delay is non-negative `i64`
milliseconds, and snap is bool.

The consuming Rust crate must enable iced's `image-without-codecs` or `image`
feature for `image`, `svg` for `svg`, and `qr_code` for QR declarations. Raster
decoder features remain a Cargo choice; the reference app enables only the PNM
decoder used by its tiny checked-in sample.

Mouse routes do not carry a payload. `cursor=` accepts the iced interaction
names in kebab case: `none`, `hidden`, `idle`, `context-menu`, `help`,
`pointer`, `progress`, `wait`, `cell`, `crosshair`, `text`, `alias`, `copy`,
`move`, `no-drop`, `not-allowed`, `grab`, `grabbing`, `resize-horizontal`,
`resize-vertical`, `resize-diagonal-up`, `resize-diagonal-down`,
`resize-column`, `resize-row`, `all-scroll`, `zoom-in`, and `zoom-out`.

The mouse `move=` route is the exception and receives `(x:f64, y:f64)` in
local widget coordinates. `scroll=` receives `(x:f64, y:f64, pixels:bool)`;
`pixels=false` identifies iced line units. Bare handler names receive these
payloads automatically.

`scroll` accepts every native direction, all four iced length variants, visible
or hidden scrollbars, scrollbar dimensions/spacing, axis anchors, and bool
auto-scroll. Its `scroll=` handler receives absolute x/y followed by relative
x/y as four f64 payloads. Bare handler names receive all four automatically.

`text` accepts str, i64, and f64 values plus typed width/height, positive size,
relative `line-height=` or absolute `line-height-px=`, horizontal and vertical
alignment, shaping, wrapping, and declared or built-in fonts. An explicit `size=`
overrides a `@text-*` utility; `font=mono @font-bold` preserves both choices.

`input` keeps its required `str` binding and additionally supports bool secure
mode, submit routes, str-payload paste routes, typed width/padding/text size,
relative line height, horizontal alignment, default/monospace font, and a
single-character icon with side/size/spacing. A disabled input suppresses
typing, submit, and paste messages together.

`button` accepts either its compact string label or exactly one arbitrary child
node. It also supports typed width/height, non-negative padding, bool clipping,
disabled routing, and the checked button style utilities.

`checkbox` and `toggler` share typed control size/width/spacing, text size and
relative line height, shaping, wrapping, and default/mono font properties.
Togglers add full text alignment. Checkboxes add a single-character icon with
size, relative line height, and shaping.

`slider` uses positive f64 normal and shift steps, an optional in-range default
for command/control-click reset, and an optional release route. Horizontal
sliders accept any length for width and fixed height; vertical sliders accept
fixed width and any length for height. Literal reversed ranges, invalid defaults,
and fluid cross-axis sizes are rejected before code generation.

A slider may own one nested `active`, `hovered`, and `dragged` style block.
Each block starts from iced's default style for that status and overrides any
listed rail colors/width/border/radius or handle shape/color/border/radius.
Colors are checked theme tokens with optional opacity. Rectangle widths are
`u16`; every other metric is a non-negative f64. Handle corner radii require a
rectangle handle in the same status block.

`progress` supports all iced length variants for its main `length` and cross-axis
`girth`, horizontal or vertical direction, and primary/secondary/success/warning/
danger presets. Checked theme colors can override background, filled bar, and
border; border width and uniform/per-corner radii are non-negative f64 values.
Literal reversed ranges are rejected before generation.

`tooltip` styles start from transparent, rounded, bordered, dark, primary,
secondary, success, warning, or danger iced container presets. Checked theme
colors can override background, text, border, and shadow. Border width, shadow
blur, and uniform/per-corner radii are non-negative f64 values; shadow x/y may
be negative. `pixel-snap=` controls the container style's pixel-grid snap and is
separate from the tooltip overlay's viewport `snap=` behavior.

`pick` requires a homogeneous `[T]` options expression and a matching optional
`T?` selection. Its main route carries `T`; `open=` and `close=` routes carry no
payload. Pick values may be bool, i64, f64, str, or an extern type. Fixed
width/menu height, padding, and text size are non-negative `f64` values.

`combo` requires a `combo[T]` search state and matching `T?` selection. Its
main and `hover=` routes carry `T`; `input=` carries str; `open=` and `close=`
carry no payload. A bare input/hover handler name receives the payload
automatically. Combo search state owns its initial options and cannot be
assigned after initialization.

`float` applies positive scale and x/y translation to one child. `pin` places
one child at x/y coordinates inside optional typed width/height bounds; x/y is
the direct decomposition of iced's `position(Point)` helper.
`sensor` observes one child: show/resize handlers receive `(width:f64,
height:f64)`, while hide has no payload; anticipation is non-negative f64 and
delay is non-negative i64 milliseconds. `key=` owns a comparable Ice value and
provides the same continuity behavior as iced's borrowed `key_ref` form.
`responsive at=N` chooses its first child below width N and its second child
otherwise. The general `responsive size=(width, height)` form binds the current
iced `Size` as two scoped `f64` names and accepts one arbitrary child tree, so
conditions and component inputs can depend on either dimension.

`theme` applies an iced theme to exactly one child subtree. With no preset or
`default`, iced chooses the default theme for the outer light/dark mode; `app`
reuses the app's generated custom palette. Every iced built-in theme is accepted
in kebab case, such as `dark`, `catppuccin-mocha`, or `tokyo-night-storm`.
Checked `text=` and solid `background=` colors override the subtree defaults.

`stack` accepts every iced `Length` for width and height. Its first rendered
child normally determines intrinsic size. `under=N` places the first N rendered
children beneath that base without letting them determine intrinsic size,
matching iced's `push_under`; values larger than the rendered child count simply
leave the stack without an intrinsic base layer.

`row` and `col` accept typed spacing, every iced `Length` for width/height,
cross-axis `start`/`center`/`end` alignment, and clipping. Columns additionally
accept `max-width=`. Padding can be uniform, axis-specific, or per-side; the
more specific value wins regardless of property order. Bare `wrap` switches to
iced's wrapping layout. `wrap-spacing=` controls spacing between wrapped rows or
columns and `wrap-align=` controls their main-axis placement; both require
`wrap`.

`keyed item in items by=key` is iced's identity-preserving column. `items` must
be a list, `key` is checked in the item scope and must be bool, i64, or f64,
and the indented node is the single repeated child template. Each child also
receives an automatic `key(...)` identity scope. Keyed columns accept every
native keyed-column setter: spacing, uniform/axis/per-side padding, every
`Length` for width and height, max width, and cross-axis alignment.

`lazy dependency as cached` rebuilds its one child subtree only when the
dependency hash changes. The dependency may be bool, i64, str, an extern type
implementing Rust `Hash + Clone`, or a recursive list/optional of those. Only
the owned `cached` alias is visible inside the subtree, which statically enforces
iced's `Element<'static>` contract. Input, combo, named QR data, and a slot from
an enclosing component are rejected because those forms borrow app-owned data.
Components and structured children remain usable when their complete expanded
tree satisfies the same static rule.

Markdown content is parsed into owned iced state instead of being reparsed by
the view. A literal initializes it directly, and `markdown(source)` replaces it
from a runtime str:

```ice
state
  help:markdown = "# Help [docs](https://iced.rs)"

on open_link(url)

view
  markdown help text-size=16.0 spacing=12.0 -> open_link _
```

The route receives the clicked URI as str. `text-size`, every h1-h6 size,
`code-size`, and `spacing` map directly to iced Markdown `Settings`; sizes must
be positive and spacing non-negative. The reference app enables iced's Markdown
parser and syntax highlighter features.

A table iterates a typed list and gives every cell its row binding. Headers and
cells are arbitrary one-root Ice subtrees:

```ice
table task in tasks width=fill padding-x=8.0 separator=1.0
  column width=fill align-x=left align-y=center
    header
      text "Task" @font-bold
    cell
      text task.title
```

Table width accepts every iced `Length`. Uniform/horizontal/vertical padding
and separator thickness are non-negative pixels. Each column accepts every
`Length` width plus all horizontal and vertical alignments. Row and column
identity scopes are generated automatically, so IDs inside repeated cells do
not collide. Rust row values must be `Clone`, matching iced's table contract.

Text editor content is another owned UI state type. A literal initializes it,
and `editor(source)` replaces it from a runtime str:

```ice
state
  notes:editor = "fn main() {}"

view
  editor #notes <-> notes placeholder="Write notes" width=640.0 height=fill min-height=80.0 max-height=240.0 size=14.0 line-height=1.3 padding=8.0 wrapping=word font=mono highlight="rs" highlight-theme=base16-ocean disabled=loading
```

The compiler owns iced's `Action` message variant and calls `Content::perform`
automatically, so editor actions never leak into application handlers. Width is
fixed pixels, height accepts every iced `Length`, metrics are range-checked,
and all four wrapping modes, declared or built-in fonts, relative/absolute line
height, and all five iced highlighter themes are accepted. A disabled editor is
rendered without `on_action`. An editor must live in the app view or in slot
content supplied by the app; the checker rejects editor bindings declared
inside a pure component because their generated actions must mutate app-owned
state.

Spaces inside a compound expression should be wrapped in parentheses when the
expression shares a line with widget properties:

```ice
button "Add" disabled=(loading || empty(trim(draft))) -> submit
```

## 5. Types and externs

| Ice | Rust extern type |
| --- | --- |
| `bool` | `bool` |
| `i64` | `i64` |
| `f64` | `f64` |
| `str` | `String` |
| `[T]` | `Vec<T>` |
| `T?` | `Option<T>` |
| `combo[T]` | `iced::widget::combo_box::State<T>` |
| `markdown` | `iced::widget::markdown::Content` |
| `editor` | `iced::widget::text_editor::Content` |
| `Name` | the named struct in the extern namespace |
| `unit` | `()` |

One namespace keeps declarations short:

```ice
extern crate::backend
  Task(id:i64, title:str, done:bool)
  AppError(message:str)
  list_tasks() -> [Task] ! AppError
  create_task(title:str) -> [Task] ! AppError
```

This means:

```rust
crate::backend::Task
crate::backend::AppError
crate::backend::list_tasks
crate::backend::create_task
```

Bare extern functions are asynchronous. `A -> B` means `async fn(...) -> B`.
`A -> B ! E` means `async fn(...) -> Result<B, E>`. Values crossing into iced
messages must satisfy the traits required by generated iced code, notably
`Clone` for 0.37 message payloads.

Three typed iced adapters expose framework capabilities without embedding Rust
expressions in Ice:

```ice
extern crate::backend
  component native_help(active:bool) -> bool
  task copy_text(text:str) -> unit
  subscription app_events() -> bool
```

Their Rust signatures are:

```rust
fn native_help(active: bool) -> iced::Element<'static, bool>;
fn copy_text(text: String) -> iced::Task<()>;
fn app_events() -> iced::Subscription<bool>;
```

An extern component receives owned props and returns a default-renderer
`Element<'static, Event>`. A task returns `Task<Event>` or
`Task<Result<Event, Error>>`. A subscription returns `Subscription<Event>`.
Generated probes type-check every declaration against the actual Rust item.
Extern component and subscription declarations are infallible; errors are
ordinary event payloads when an adapter needs them.

Struct declarations are read-only views of Rust data. Ice may read a declared
field (`task.title`) but cannot construct or mutate the struct. Declaring a
field or function does not create it; the generated Rust probes verify the
actual item and type.

## 6. State and expressions

Literal state types are inferred, including non-empty homogeneous lists:

```ice
state
  draft = ""
  loading = false
  retries = 0
  modes = ["List", "Board"]
```

These infer to `str`, `bool`, `i64`, and `[str]`, respectively.

Empty lists need an annotation because their element type is unknowable:

```ice
tasks:[Task] = []
selection:str? = none
search_modes:combo[str] = ["List", "Board"]
```

The expression language contains:

- literals: strings, booleans, `i64`, `f64`, `none`, and list literals such as
  `[]` and `["List", "Board"]`;
- paths: `state_name`, `parameter`, `item.field`;
- unary operators: `!`, `-`;
- arithmetic: `*`, `/`, `+`, `-`;
- comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`;
- boolean operators: `&&`, `||`;
- parentheses;
- built-ins: `len(list_or_str) -> i64`, `empty(list_or_str) -> bool`,
  `trim(str) -> str`, and `some(T) -> T?`.

There is no arbitrary Rust expression, method call, closure, allocation API, or
implicit truthiness. New operations either belong in a small universal builtin
set or behind a typed extern function.

## 7. Handlers and effects

Handlers are the only place state changes:

```ice
on submit
  return if loading || empty(trim(draft))
  loading = true
  run create_task(trim(draft)) -> created _ | failed _
```

Rules:

- assignment targets must be declared state;
- assigned expressions must have the state type;
- `return if` requires `bool`;
- `run` and `task` must be the final statement because each returns one iced
  `Task`;
- fallible externs require both success and error routes;
- infallible externs permit only the success route;
- handler parameter types are inferred from every incoming route;
- incompatible incoming payloads are a type error;
- `_` means the payload produced by the current widget or action route.

`run` wraps an async Rust function with `Task::perform`. `task` directly maps a
Rust function that already returns an iced `Task`, which exposes clipboard,
window, focus, scroll, font, system, cancellation, batching, and other runtime
operations without duplicating their implementation in Ice.

Examples of payload flow:

```ice
checkbox task.title checked=task.done -> toggle(task.id, _)
run list_tasks() -> loaded _ | failed _
task copy_text(draft) -> copied

on toggle(id, checked)
  run set_task_done(id, checked) -> updated _ | failed _
```

`on mount` runs once during app initialization and has no parameters. Generated
message enums, update matching, owned clones, lifetimes, and `iced::Task::perform`
calls are backend details.

## 8. View language

The implemented native nodes are:

| Node | Contract |
| --- | --- |
| `col` | vertical children with full sizing, padding, spacing, alignment, clipping and wrapping behavior |
| `row` | horizontal children with full sizing, padding, spacing, alignment, clipping and wrapping behavior |
| `scroll` | one child; direction, bounds, scrollbar, anchors, auto-scroll and absolute/relative offset route |
| `grid` | responsive children with pixel width/spacing, fixed columns or fluid max-cell width, and aspect-ratio or evenly distributed `Length` height |
| `stack` | overlays children with typed width/height, optional clipping and `under=N` intrinsic-base control |
| `text` | one `str`, `i64`, or `f64` expression with bounds, size/line-height, font, alignment, shaping, wrapping and checked color/weight styles |
| `input` | required `str` binding; ID, hint, disabled/secure, submit/paste, sizing, alignment, default/mono font and icon properties |
| `button` | string label or one child; optional ID/disabled, typed size/padding/clip, required route |
| `checkbox` | string label, bool value/route, disabled, sizing/typography/wrapping/font and custom icon properties |
| `toggler` | string label, bool value/route, disabled, sizing/typography/wrapping/font/alignment properties |
| `slider` | `f64` value/range/default/normal+shift steps, direction-aware sizing, change/release routes and nested status styles |
| `progress` | `f64` value/range, all length/girth variants, vertical axis, five presets and color/border/radius style overrides |
| `radio` | string label, `i64` or bool value, bool `selected`, value-payload route |
| `pick` | `[T]` options, `T?` selection, placeholder/size/open/close properties, `T`-payload route |
| `combo` | searchable `combo[T]` state, `T?` selection, input/hover/open/close routes and sizing |
| `float` | one child with positive scale and x/y translation |
| `pin` | one child with typed width/height and fixed x/y position |
| `sensor` | one child with show/resize `(width, height)`, hide, key, anticipation and delay |
| `responsive` | breakpoint sugar or one arbitrary size-dependent child tree with scoped width/height bindings and typed bounds |
| `rule` | horizontal/vertical separator with non-negative thickness, all fill modes, default/weak preset, color, corner radii and snap |
| `qr` | named text/binary QR data with correction/version, cell/total sizing and checked colors |
| `space` | optional fixed/fill/fill-portion/shrink width and height |
| `image` | raster path expression, typed length/fit/filter/rotation/opacity/scale/expand/radius properties |
| `svg` | SVG path expression with typed length/fit/rotation/opacity properties |
| `tooltip` | exactly two children (content then tip), full positioning/timing plus preset, color, border, radius, shadow and pixel-snap styles |
| `mouse` | one child; all button/enter/move/scroll/exit events and every iced cursor interaction |
| `theme` | one child with default/app/all built-in iced themes and checked text/background colors |
| `if` | includes its children when a bool expression is true |
| `for` | iterates a list and adds one typed item binding |
| `keyed` | repeats one child template with a bool/i64/f64 identity key and native column sizing/alignment |
| `lazy` | caches one owned static child subtree by a checked hashable dependency |
| `markdown` | renders owned parsed content with all text/heading/code sizes, spacing and str link events |
| `table` | maps typed rows into arbitrary structured headers/cells with complete sizing, padding, separator and alignment options |
| `editor` | binds owned multi-line content to generated iced actions with sizing, typography, wrapping and built-in highlighting |

`if` and `for` are child control-flow nodes inside a layout. There is no virtual
DOM or runtime reconciliation layer; the iced backend constructs the current
element tree from state.

Grid `columns=` and `fluid=` are mutually exclusive. `columns=` is a positive
`i64`; `fluid=` and both dimensions of `height=aspect(W,H)` are positive `f64`
values. `width=` and `spacing=` are non-negative `f64` pixels. A non-aspect
`height=` accepts `fill`, `fill(N)`, `shrink`, or a non-negative `f64` pixel
expression and maps to iced's evenly distributed sizing.

### Components

Components are pure typed view templates:

```ice
component TaskRow(task:Task, loading:bool)
  row #root @w-full items-center p-4 bg-surface rounded-lg
    checkbox task.title checked=task.done disabled=loading -> toggle(task.id, _)
```

They have one root, typed inputs, no local mutable state, no lifecycle, and no
implicit capture of app state. They may route events to app handlers. The
compiler expands them into the typed view IR; they are not runtime component
objects.

A component may declare one required `slot` and receive one structured child
tree at its call site:

```ice
component Panel(title:str)
  col @p-4 bg-surface rounded-lg
    text title @font-bold
    slot

Panel("Tasks") #tasks-panel
  scroll height=fill
    col
      for task in tasks
        TaskRow(task, loading) #task(task.id)
```

A call to a slotted component must provide exactly one child root; siblings can
be wrapped in `row`, `col`, `grid`, or `stack`. A component without `slot`
rejects child content. Slot content keeps the caller's state, loop bindings,
handlers, and IDs while rendering under the component instance scope. A wrapper
component can forward its own `slot` as another component's child.

### Extern components and subscriptions

An extern component is an owned Rust `Element` adapter:

```ice
extern native_help(external_hover) -> external_hover_changed _
```

Its arguments and emitted payload are checked against the declaration. A
non-`unit` output requires a route. A `unit` component may omit the route; its
messages are mapped to an internal no-op. Extern components own their styling,
so `@` utilities and `#` IDs are not accepted on the call.

Subscriptions are declared separately from activation:

```ice
subscribe
  app_events() -> external_event _
  keyboard press -> key_pressed _
  keyboard release -> key_released _
  keyboard modifiers -> key_modifiers_changed _
```

The compiler batches active subscriptions and wires the application builder.
Subscription routes accept only `_`; handlers can read current state after the
event arrives. This prevents generated `'static` subscription closures from
capturing a borrowed application state.

Native keyboard subscriptions infer structured payloads. Press events expose
`key`, `modified_key`, `physical_key`, `location`, `modifiers`, optional `text`,
and `repeat`; release events expose the same fields except `text` and `repeat`.
Modifier payloads expose `shift`, `control`, `alt`, `logo`, `command`, `jump`,
and `macos_command`. The nested `event.modifiers` value has that same shape.
Logical character keys keep their text, named and physical keys use their iced
names, and locations are `standard`, `left`, `right`, or `numpad`. Like
`iced::keyboard::listen`, these subscriptions receive keyboard events that no
widget captured.

System queries and theme changes use the same task/subscription model:

```ice
on inspect
  task system info -> inspected _

on read_theme
  task system theme -> theme_changed _

subscribe
  system theme -> theme_changed _
```

`system theme` produces `"none"`, `"light"`, or `"dark"`. The inferred
`system-info` payload exposes `system_name`, `system_kernel`, `system_version`,
`system_short_version`, `cpu_brand`, `cpu_cores`, `memory_total`, `memory_used`,
`graphics_backend`, and `graphics_adapter`. Optional iced fields remain
optional; core and byte counts use `i64` and saturate at `i64::MAX` instead of
wrapping. `task system info` requires iced's `sysinfo` Cargo feature. Both
system tasks are infallible and reject an error route.

Clipboard effects cover both platform targets:

```ice
task clipboard read -> clipboard_read _
task clipboard read-primary -> primary_read _
task clipboard write draft
task clipboard write-primary draft
```

Reads are infallible tasks with a `str?` payload because the target may not
contain text. Writes require `str`, produce no message in iced, accept no route,
and must be the handler's final statement.

Font declarations map directly to iced's complete `Font` descriptor:

```ice
font brand family="Inter" weight=semibold stretch=normal style=italic default=true

view
  text "Brand" font=brand
```

The family may be a named family or any of iced's five generic families. Every
weight, stretch, and style variant is accepted. At most one declaration may be
the application default. `font=default` and `font=mono` remain built-ins;
declared fonts also work on text, input, editor, checkbox, and toggler. Font
byte loading is not part of 0.37.

Widget operation tasks target checked static IDs in the app view:

```ice
task widget focus #search
task widget focused #search -> focus_checked _
task widget cursor #search 3
task widget select #search 0 5
task widget snap #results 0.0 1.0
task widget scroll-by #results 0.0 24.0
```

Ice exposes all 13 functions in `iced::widget::operation`: previous/next/direct
focus and focus query; cursor front/end/position; select all/range; relative
snap/end; and absolute scroll-to/scroll-by. Effects have no route and
`focused` requires a `bool` route. Cursor and selection positions are
non-negative `i64`; relative offsets are `f64` in `0.0..=1.0`; absolute
offsets are unrestricted `f64`. Targets must be real static IDs in the app
scope. Repeated/component scopes and the feature-gated selector API remain
outside 0.37.

### IDs

IDs are identities, not CSS selectors. Static IDs must be unique in their local
view/component scope. Repeated instances use a stable typed key:

```ice
for task in tasks
  TaskRow(task, loading) #task(task.id)
```

The logical identity is hierarchical:

```text
App / component-instance / local-node
Tasks/task(42)/root
```

A component call without an explicit ID receives its component name as the
instance segment. Repeated component calls should therefore provide a dynamic
ID. The iced backend lowers identities to native iced IDs on widgets that
support them (currently input and scroll) and still uses layout/component IDs
to build descendant scopes.

## 9. Theme and style

Theme colors are named tokens with `#RRGGBB` or `#RRGGBBAA` values:

```ice
theme
  background #0f172a
  foreground #f8fafc
  primary    #7c3aed
  danger     #dc2626
```

`background`, `foreground`, `primary`, and `danger` are required. Other names
are app-defined. `white`, `black`, and `transparent` are built in. A color may
carry opacity, such as `bg-primary/70`.

`@` switches the remainder of a node to style utilities. Utilities are resolved
at compile time; there is no CSS engine, selector matching, cascade, or runtime
string parser.

The implemented utility surface is:

| Family | Values | Effective on |
| --- | --- | --- |
| size | `w-full`, `h-full` | layouts; `w-full` also input |
| max width | `max-w-sm` through `max-w-2xl` | row, col, grid, stack |
| alignment | `items-center`, `self-center` | row, col |
| spacing | `p-*`, `px-*`, `py-*`, `gap-*` | row/col/grid/stack; padding also input/button; grid supports gap |
| text | `text-xs` through `text-2xl`, `font-bold` | text |
| color | `bg-TOKEN`, `text-TOKEN`, `border-TOKEN` | checked per widget |
| border | `border`, `border-2` | row, col, grid, stack, input |
| radius | `rounded-sm`, `rounded`, `rounded-md`, `rounded-lg`, `rounded-full` | row, col, grid, stack, input, button |
| states | `hover:bg-*`, `pressed:bg-*`, `disabled:opacity-*` | button |
| focus | `focus:border-*` | input |

Spacing values are `0 1 2 3 4 5 6 8 10 12 16 20 24` and map to four iced
logical pixels per unit. Opacity values are `0 25 50 75 100`; color opacity may
be any integer from 0 through 100.

`border-TOKEN` and `focus:border-TOKEN` require `border` or `border-2` on the
same node. A rounded row, column, grid, or stack requires a background or
border, because iced would otherwise have nothing to round.

The checker rejects both an unknown utility (`E041`) and a known utility on a
node where the iced backend would ignore it (`E042`/`E044`). Silent CSS-like
no-ops are not allowed.

## 10. Diagnostics

Language errors have stable codes and source coordinates:

```text
E132 src/ui/tasks.ice:26:1: unknown handler `save`
E041 src/ui/tasks.ice:61:1: unsupported utility `grid-cols-3`
E042 src/ui/tasks.ice:61:1: utility `gap-4` has no effect on `text`
```

The implemented families are:

| Range | Meaning |
| --- | --- |
| `E000-E019` | document, indentation, theme |
| `E020-E039` | extern, type, and state syntax |
| `E040-E079` | component, statement, view, expression, and style rules |
| `E100-E119` | duplicate declarations and theme semantics |
| `E120-E139` | view, action, and route resolution |
| `E140-E159` | handler and expression types |
| `E160-E179` | IDs and backend lowering constraints |
| `E180-E199` | file imports and source loading |

`cargo ice check` first reports these language errors directly, then invokes
`cargo check` so rustc verifies extern items and generated iced types. A missing
Rust item is named by its `crate::module::item` path in rustc's diagnostic.
Imported-language diagnostics already point to the original fragment and line.
A future generated-Rust source-map layer may remap rustc spans into the precise
extern line; 0.37 does not claim that remapping.

## 11. Cargo commands

| Command | Behavior |
| --- | --- |
| `cargo build` / `cargo check` | expands each included `.ice` file and checks generated Rust |
| `cargo fmt` | formats Rust; foreign `.ice` files are unchanged |
| `cargo clippy` | lints generated Rust as part of the normal crate |
| `cargo ice fmt` | runs Rust formatting and formats all discovered `.ice` files |
| `cargo ice fmt --check` | checks both Rust and Ice formatting without changing `.ice` files |
| `cargo ice check` | language analysis followed by workspace `cargo check` |
| `cargo ice clippy` | language analysis followed by workspace clippy |
| `cargo ice expand FILE` | prints generated Rust for debugging |

`cargo-ice` discovers `.ice` files recursively below the current directory,
skips `.git` and `target`, analyzes files with a top-level `app` as roots, and
formats both roots and imported fragments.

## 12. Current coverage and escape hatches

The 0.37 native backend is enough for CRUD/settings-style screens, selection,
media, hover
overlays, and common pointer events, not all of iced. It still lacks direct
syntax for canvas, general overlays/modals, rich text, multiple
windows, and custom widgets. [`COVERAGE.md`](COVERAGE.md) is the exact versioned
ledger.

The language must not grow one ad-hoc syntax form for every iced API. The next
layer is therefore implemented as three typed Rust adapters: component, task,
and subscription. They make advanced widgets and runtime operations reachable
without admitting arbitrary Rust into expressions or duplicating iced in the
core grammar. Direct native syntax remains preferable for common UI concepts.

Native language coverage and system coverage are therefore separate:

```text
common screen structure -> checked native Ice vocabulary
advanced/custom widget  -> typed Rust Element adapter
iced runtime operation  -> typed Rust Task adapter
event/stream source      -> typed Rust Subscription adapter
domain and I/O           -> typed Rust async extern
```

## 13. Reference application

The runnable multi-file task app starts at
[`examples/iced-app/src/ui/tasks.ice`](examples/iced-app/src/ui/tasks.ice), with
its Rust boundary in
[`examples/iced-app/src/main.rs`](examples/iced-app/src/main.rs). The exhaustive
compile-tested widget example is
[`examples/iced-app/src/ui/showcase.ice`](examples/iced-app/src/ui/showcase.ice).
Together they exercise
state inference, typed extern structs/functions, mount and result handlers,
direct input/editor binding, typed keyboard/system subscriptions, system tasks, clipboard effects, `if`, `for`, native keyed columns and lazy subtrees, parsed Markdown, structured tables, pure components, structured slot composition,
dynamic component IDs,
theme utilities, disabled controls, fallible asynchronous tasks, complete
wrapping row/column layouts, grids and fully sized underlay stacks, toggles,
sliders, progress, radio controls,
rules, fixed spacing, an
optional selection value, pick list and searchable combo box, extern and native
tooltip/mouse-area components including pointer movement and wheel payloads,
raster and SVG media, configured scrolling with offset events,
responsive/positioned content, visibility sensing, formatted text, extended
text input, child-content buttons, configured boolean controls, rules and
status-styled sliders, configured progress bars and tooltips, a clipboard task,
and a raw-event subscription.
