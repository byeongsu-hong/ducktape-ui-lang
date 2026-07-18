# Ice Language Specification 1.44

Status: implemented reference slice

Ice is a small frontend language with an iced backend. It is not Rust syntax,
JSX, or a token shortcut around a procedural macro. A frontend parses `.ice`
source, resolves names and types, checks UI semantics, and lowers a typed tree
to backend code.

This document describes what the repository implements. A section explicitly
marked “planned” is a design constraint, not accepted 1.44 syntax.

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
  block comments are not part of 1.44.
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
app | daemon
use
extern
theme
qr
state
preset
component
on
subscribe
view
```

An Ice source graph has exactly one `app` or `daemon` root and one `view`, with
at most one `extern` namespace. The root file declares it and normally the view;
imported fragments may hold any other top-level declarations. The graph may
have multiple components and handlers. The view and each component have exactly
one root node.

## 4. Compact grammar

The grammar below uses indentation (`INDENT`) as a block delimiter. `expr` is
defined in section 6.

```text
source_graph   = root_file imported_file*
root_file      = (root_decl | use_decl | declaration)*
imported_file  = (use_decl | declaration)*
use_decl       = "use" string
declaration    = extern_decl | theme_decl | font_decl | qr_decl | state_decl
               | preset_decl | component_decl | handler_decl | subscribe_decl
               | view_decl
document       = root_decl extern_decl? theme_decl qr_decl* state_decl? preset_decl*
                 component_decl* handler_decl* subscribe_decl? view_decl

root_decl      = ("app" | "daemon") PascalName (INDENT app_setting*)?
app_setting    = "title" expr | "theme" expr
               | ("background" | "text-color") expr
               | "id" string | "font" string
               | ("executor" | "renderer") rust_path
               | "default-text-size" number | "scale-factor" expr
               | ("antialiasing" | "vsync") bool
               | window_decl
window_decl    = "window" name? INDENT window_setting*
window_setting = ("size" | "min-size" | "max-size") number number
               | "icon-rgba" string u32 u32
               | "position" ("default" | "centered" | number number)
               | "level" ("normal" | "always-on-bottom" | "always-on-top")
               | ("maximized" | "fullscreen" | "visible" | "resizable"
                 | "closeable" | "minimizable" | "decorations" | "transparent"
                 | "blur" | "exit-on-close-request") bool
               | window_platform
window_platform = "platform" "linux" INDENT
                    (("application-id" string) | ("override-redirect" bool))*
                | "platform" "windows" INDENT
                    (("drag-and-drop" | "skip-taskbar" | "undecorated-shadow") bool
                    | "corner" ("default" | "do-not-round" | "round" | "round-small"))*
                | "platform" "macos" INDENT
                    (("title-hidden" | "titlebar-transparent"
                    | "fullsize-content-view") bool)*
                | "platform" "wasm" INDENT ("target" (string | "none"))?

extern_decl    = "extern" rust_path INDENT extern_item+
extern_item    = struct_sig | function_sig | extern_component_sig
               | extern_selector_sig
               | extern_shader_sig | extern_task_sig | extern_stream_sig
               | extern_sip_sig | extern_recipe_sig | extern_event_filter_sig
               | extern_sync_sig | extern_subscription_sig
               | extern_theme_sig | extern_themer_sig
               | extern_window_sig | extern_markdown_viewer_sig
               | extern_text_style_sig | extern_slider_style_sig
               | extern_progress_style_sig
               | extern_button_style_sig
               | extern_checkbox_style_sig | extern_toggler_style_sig
               | extern_radio_style_sig | extern_container_style_sig
               | extern_svg_style_sig | extern_input_style_sig
               | extern_scroll_style_sig
               | extern_pick_list_style_sig | extern_menu_style_sig
               | extern_pane_grid_style_sig
               | extern_editor_binding_sig
               | extern_editor_highlighter_sig | extern_editor_style_sig
struct_sig     = PascalName "(" field_list? ")"
field_list     = field ("," field)*
field          = name ":" type
extern_component_field_list
               = extern_component_field ("," extern_component_field)*
extern_component_field = name ":" "&"? type
type           = "bool" | "i64" | "f64" | "str" | "bytes" | "image"
               | "image-allocation" | "image-memory" | "image-error"
               | "debug-span"
               | "markdown" | "editor" | "event" | "instant" | "window-id"
               | "key" | "physical-key" | "key-location" | "key-modifiers"
               | "pixels" | "padding" | "degrees" | "radians"
               | "rotation"
               | "content-fit"
               | "color"
               | "length"
               | "alignment" | "horizontal-alignment" | "vertical-alignment"
               | "point" | "point-u32" | "vector" | "size" | "size-u32"
               | "rectangle" | "rectangle-u32"
               | "transformation" | "mouse-button" | "mouse-cursor"
               | "mouse-click" | "touch-finger"
               | "widget-id" | "widget-target"
               | "task-handle" | "unit"
               | PascalName
               | "[" type "]" | type "?" | "result[" type "," type "]"
               | "combo[" type "]"
               | "animation[" ("bool" | "f64" | PascalName) "]"
function_sig   = name "(" field_list? ")" "->" type ("!" type)?
extern_component_sig
               = "component" name "(" extern_component_field_list? ")" "->" type
extern_selector_sig
               = "selector" name "(" field_list? ")" "->" type
extern_shader_sig
               = "shader" name "(" field_list? ")" "->" type
extern_task_sig = "task" name "(" field_list? ")" "->" type ("!" type)?
extern_stream_sig = "stream" name "(" field_list? ")" "->" type ("!" type)?
extern_sip_sig = "sip" name "(" field_list? ")" "progress=" type
                 "->" type ("!" type)?
extern_recipe_sig = "recipe" name "(" field_list? ")" "->" type
extern_event_filter_sig = "event-filter" name "()" "->" type
extern_sync_sig = "sync" name "(" field_list? ")" "->" type
extern_subscription_sig
               = "subscription" name "(" field_list? ")" "->" type
extern_theme_sig = "theme" name "(" field_list? ")"
extern_themer_sig = "themer" name "(" field_list? ")" "->" type
extern_window_sig = "window" name "(" field_list? ")" "->" type
extern_markdown_viewer_sig
               = "markdown-viewer" name "(" field_list? ")" "->" type
extern_editor_binding_sig
               = "editor-binding" name "(" field_list? ")" "->" type
extern_editor_highlighter_sig
               = "editor-highlighter" name "(" field_list? ")"
extern_editor_style_sig
               = "editor-style" name "(" field_list? ")"
extern_text_style_sig
               = "text-style" name "(" field_list? ")"
extern_slider_style_sig
               = "slider-style" name "(" field_list? ")"
extern_progress_style_sig
               = "progress-style" name "(" field_list? ")"
extern_button_style_sig
               = "button-style" name "(" field_list? ")"
extern_checkbox_style_sig
               = "checkbox-style" name "(" field_list? ")"
extern_toggler_style_sig
               = "toggler-style" name "(" field_list? ")"
extern_radio_style_sig
               = "radio-style" name "(" field_list? ")"
extern_container_style_sig
               = "container-style" name "(" field_list? ")"
extern_svg_style_sig
               = "svg-style" name "(" field_list? ")"
extern_input_style_sig
               = "input-style" name "(" field_list? ")"
extern_scroll_style_sig
               = "scroll-style" name "(" field_list? ")"
extern_pick_list_style_sig
               = "pick-list-style" name "(" field_list? ")"
extern_menu_style_sig
               = "menu-style" name "(" field_list? ")"
extern_pane_grid_style_sig
               = "pane-grid-style" name "(" field_list? ")"

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
state_entry    = name (":" type)? "=" expr (INDENT animation_setting*)?
animation_setting = "easing" name
                  | "duration" (duration | "very-quick" | "quick" | "slow" | "very-slow")
                  | "delay" duration
                  | "repeat" (u32 | "forever")
                  | "auto-reverse" bool

preset_decl    = "preset" name (INDENT preset_section*)?
preset_section = preset_state | preset_boot
preset_state   = "state" INDENT preset_override*
preset_override = name "=" expr
preset_boot    = "boot" INDENT statement*

component_decl = "component" component_name "(" field_list? ")"
                 INDENT node

handler_decl   = "on" name ("(" name_list? ")")?
                 INDENT statement*
statement      = name "=" expr ("at" expr)?
               | "markdown" name "append" expr
               | "combo" name "push" expr
               | "return if" expr
               | "exit"
               | task_group
               | abortable_task
               | "abort" name
               | "debug start" expr "->" name
               | "debug finish" name
               | "run" call "->" route ("|" route)?
               | "task" call "->" route ("|" route)?
               | "stream" call "->" route ("|" route)?
               | sip_task
               | task_flow
               | "task time now" "->" route
               | "task system" ("info" | "theme") "->" route
               | "task clipboard" ("read" | "read-primary") "->" route
               | "task clipboard" ("write" | "write-primary") expr
               | "task font load" expr "->" route
               | "task image allocate" expr "->" route "|" route
               | "task widget" widget_operation ("->" route)?
               | "pane" "#" name pane_operation ("->" route)?
               | window_task
task_group     = ("parallel" | "sequential") INDENT task_member+
abortable_task = "abortable" name ("abort-on-drop")? INDENT task_member
sip_task       = "sip" call INDENT sip_route+
sip_route      = sip_progress | sip_done | sip_error
sip_progress   = "progress" "->" route
sip_done       = "done" "->" route
sip_error      = "error" "->" route
task_flow      = "flow" INDENT flow_source flow_item+
flow_source    = "from" task_source
task_source    = ("run" | "task" | "stream") call
               | "done" expr | "none" type
               | "task time now"
               | "task system" ("info" | "theme")
               | "task clipboard" ("read" | "read-primary")
               | "task font load" expr
               | "task image allocate" expr
flow_item      = "map" name "->" expr
               | ("then" | "and-then") name "->" task_source
               | "map-error" name "->" expr
               | "collect" | "discard"
               | ("done" | "error" | "units") "->" route
task_member    = task_group | abortable_task
               | "exit"
               | "run" call "->" route ("|" route)?
               | "task" call "->" route ("|" route)?
               | "stream" call "->" route ("|" route)?
               | sip_task
               | task_flow
               | native_task
native_task    = "task time now" "->" route
               | "task system" ("info" | "theme") "->" route
               | "task clipboard" ("read" | "read-primary") "->" route
               | "task clipboard" ("write" | "write-primary") expr
               | "task font load" expr "->" route
               | "task image allocate" expr "->" route "|" route
               | "task widget" widget_operation ("->" route)?
               | "pane" "#" name pane_operation ("->" route)?
               | window_task
widget_operation = "focus-previous" | "focus-next"
                 | ("focus" | "focused" | "cursor-front" | "cursor-end"
                   | "select-all" | "snap-end") widget_target
                 | "cursor" widget_target expr
                 | "select" widget_target expr expr
                 | ("snap" | "scroll-to" | "scroll-by") widget_target expr expr
                 | ("find" | "find-all") widget_selector
widget_selector = "id" widget_target | "text" expr | "point" expr expr
                | "focused" | call
widget_target  = "#" widget_target_segment
                 ("/" "#"? widget_target_segment)*
widget_target_segment = kebab_name | component_name | name "(" expr ")"
pane_operation = "maximize" name | "restore" | "maximized"
               | "adjacent" name pane_edge
               | "swap" name name | "close" name
               | "move" name pane_edge | "resize" (name expr | expr)
               | "drop" name name ("center" | pane_edge)
               | "split" name name ("horizontal" | "vertical")
                 ("ratio=" expr)?
pane_edge      = "top" | "left" | "right" | "bottom"
window_task    = "task window" window_operation ("target=" expr)? ("->" route)?
window_operation = "open" name? | "oldest" | "latest"
                 | "close" | "drag" | "toggle-maximize" | "toggle-decorations"
                 | "focus" | "system-menu" | "raw-id" | "screenshot"
                 | "drag-resize" direction
                 | ("resize" | "move") expr expr
                 | ("resizable" | "maximize" | "minimize" | "mouse-passthrough"
                   | "automatic-tabbing") expr
                 | ("min-size" | "max-size" | "resize-increments")
                   ("none" | expr expr)
                 | "set-mode" ("windowed" | "fullscreen" | "hidden")
                 | "attention" ("none" | "critical" | "informational")
                 | "level" ("normal" | "always-on-bottom" | "always-on-top")
                 | "size" | "maximized" | "minimized" | "position"
                 | "scale-factor" | "mode" | "monitor-size"
                 | "icon" expr expr expr
                 | call
direction      = "north" | "south" | "east" | "west"
               | "north-east" | "north-west" | "south-east" | "south-west"

subscribe_decl = "subscribe" INDENT subscription_use+
subscription_use = subscription_source ("with=" expr)? ("filter=" name)?
                   ("status=" event_status)? ("when" expr)? "->" route
subscription_source
               = call
               | "every" duration
               | "repeat" call "every" duration
               | "run" call
               | "recipe" call
               | "events" expr "using=" name
               | "event" ("raw")? ("with-id")?
               | "input-method" input_method_event
               | "keyboard" ("press" | "release" | "modifiers")
               | "mouse" mouse_event
               | "touch" touch_event
               | "window" window_event ("with-id")?
               | "system theme"
input_method_event
               = "opened" | "preedit" | "commit" | "closed"
mouse_event    = "entered" | "left" | "moved" | "pressed" | "released"
               | "wheel"
touch_event    = "pressed" | "moved" | "lifted" | "lost"
duration       = positive_integer ("ms" | "s")
event_status   = "any" | "captured" | "ignored"
window_event   = "frame" | "opened" | "closed" | "moved" | "resized"
               | "rescaled" | "close-request" | "focused" | "unfocused"
               | "file-hovered" | "file-dropped" | "files-hovered-left"

view_decl      = "view" INDENT node

node           = layout | text | input | button | checkbox | toggler
               | slider | progress | radio | pick_list | combo_box
               | rule | qr_code | space | float | pin | sensor | responsive
               | media | tooltip | mouse_area | canvas | theme_boundary
               | component_call | slot | extern_component_call | themer_view
               | shader_view
               | if_node | for_node
               | keyed_column | lazy_node | markdown_view | table_view
               | editor_view | container | overlay | rich_text | pane_grid
layout         = "col" id? column_property* styles? INDENT node+
               | "row" id? flex_property* styles? INDENT node+
               | "scroll" id? scroll_property* styles? INDENT node scroll_status*
               | "grid" id? grid_property* styles? INDENT node+
               | "stack" id? stack_property* styles? INDENT node+
container      = "container" id? container_property* styles? INDENT node
container_property = ("width=" | "height=") length
                   | ("max-width=" | "max-height=") expr
                   | ("align-x=" | "align-y=") ("start" | "center" | "end")
                   | "clip=" expr
                   | ("padding=" | "padding-x=" | "padding-y="
                     | "padding-top=" | "padding-right=" | "padding-bottom="
                     | "padding-left=") expr
                   | surface_style_property
overlay        = "overlay" "when=" expr overlay_property*
                 INDENT "content" INDENT node
                 INDENT "layer" INDENT node
overlay_property = "dismiss=" route | "backdrop=" name ("/" u8)?
                 | "padding=" expr
                 | ("align-x=" | "align-y=") ("start" | "center" | "end")
rich_text      = "rich-text" rich_text_property* styles? ("->" route)?
                 INDENT rich_span*
rich_text_property = ("width=" | "height=") length | "size=" expr
                   | ("line-height=" | "line-height-px=") expr
                   | "font=" font_ref | "align-x=" text_alignment
                   | "align-y=" ("top" | "center" | "bottom")
                   | "wrapping=" text_wrapping | "color=" color_ref
                   | "style=" call
rich_span      = "span" expr rich_span_property* styles?
rich_span_property = ("size=" | "line-height=" | "line-height-px=") expr
                   | "font=" font_ref | "color=" color_ref | "link=" expr
                   | "background=" background_value | "border=" color_ref
                   | "border-width=" expr
                   | ("radius=" | "radius-tl=" | "radius-tr="
                     | "radius-br=" | "radius-bl=") expr
                   | ("padding=" | "padding-x=" | "padding-y="
                     | "padding-top=" | "padding-right=" | "padding-bottom="
                     | "padding-left=") expr
                   | "underline" | "underline=" expr
                   | "strike" | "strike=" expr
pane_grid      = "pane-grid" id
                 ("split=" pane_axis ("ratio=" number)? pane_grid_property*
                   INDENT pane_grid_style? pane_view pane_view pane_declaration*
                 | pane_grid_property*
                   INDENT pane_grid_style? pane_configuration pane_declaration*)
pane_grid_property = ("width=" | "height=") length
                   | ("spacing=" | "min-size=" | "resize=") expr
                   | "drag" | "click=" route | "style=" call
pane_grid_style = "style" INDENT pane_grid_style_status+
pane_grid_style_status
               = "hovered-region" pane_region_style_property+
               | ("hovered-split" | "picked-split") pane_line_style_property+
pane_region_style_property
               = "background=" background_value | "border=" color_ref
               | ("border-width=" | "radius=" | "radius-tl="
                 | "radius-tr=" | "radius-br=" | "radius-bl=") expr
pane_line_style_property = "color=" name ("/" u8)? | "width=" expr
pane_configuration = pane_view
                   | "split" name? pane_axis ("ratio=" number)?
                     INDENT pane_configuration pane_configuration
pane_view      = "pane" name pane_property* styles?
                 INDENT (node | pane_section+)
closed_pane    = "pane" name "closed" pane_property* styles?
                 INDENT (node | pane_section+)
pane_template  = "pane" name "in" name "by=" expr
                 pane_property* styles? INDENT (node | pane_section+)
pane_declaration = closed_pane | pane_template
pane_property  = surface_style_property | "maximized=" name
pane_section   = "title" pane_title_property* styles? INDENT node
               | "controls" INDENT node
               | "compact-controls" INDENT node
               | "content" INDENT node
pane_title_property
               = ("padding=" | "padding-x=" | "padding-y="
                 | "padding-top=" | "padding-right=" | "padding-bottom="
                 | "padding-left=") expr
               | "always-controls" | surface_style_property
surface_style_property
               = "background=" background_value
               | ("text=" | "border=" | "shadow=") color_ref
               | ("border-width=" | "radius=" | "radius-tl="
                 | "radius-tr=" | "radius-br=" | "radius-bl="
                 | "shadow-x=" | "shadow-y=" | "shadow-blur="
                 | "pixel-snap=") expr
background_value = color_ref
                 | "linear(" expr ("," color_ref "@" expr){0,8} ")"
pane_axis      = "horizontal" | "vertical"
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
                 (INDENT markdown_style)?
markdown_property = ("text-size=" | "h1-size=" | "h2-size="
                  | "h3-size=" | "h4-size=" | "h5-size=" | "h6-size="
                  | "code-size=" | "spacing=") expr
                  | "viewer=" call
markdown_style = "style" markdown_style_property+
markdown_style_property
               = ("font=" | "inline-code-font=" | "code-block-font=") font_ref
               | "inline-code-background=" background_value
               | ("inline-code-color=" | "link=" | "inline-code-border=") color_ref
               | ("inline-code-padding=" | "inline-code-padding-x="
                 | "inline-code-padding-y=" | "inline-code-padding-top="
                 | "inline-code-padding-right=" | "inline-code-padding-bottom="
                 | "inline-code-padding-left=" | "inline-code-border-width="
                 | "inline-code-radius=" | "inline-code-radius-tl="
                 | "inline-code-radius-tr=" | "inline-code-radius-br="
                 | "inline-code-radius-bl=") expr
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
                 (INDENT editor_status*)?
editor_property = "placeholder=" string | "width=" expr | "height=" length
                | ("min-height=" | "max-height=" | "size="
                  | "line-height=" | "line-height-px=" | "padding=") expr
                | "wrapping=" text_wrapping
                | "font=" font_ref
                | "highlight=" string
                | "highlight-theme=" ("solarized-dark" | "base16-mocha"
                  | "base16-ocean" | "base16-eighties" | "inspired-github")
                | "disabled=" expr
editor_status  = ("active" | "hovered" | "focused"
               | "focused-hovered" | "disabled") editor_style_property*
editor_style_property
               = "background=" background_value
               | ("border=" | "placeholder=" | "value=" | "selection=") color_ref
               | ("border-width=" | "radius=" | "radius-tl="
                 | "radius-tr=" | "radius-br=" | "radius-bl=") expr
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
                | "auto=" expr | ("scroll=" | "viewport=") route
                | "style=" call
scroll_status  = ("active" | "hovered" | "dragged")
                 scroll_selector*
                 (INDENT scroll_style_section*)?
scroll_selector = ("horizontal-disabled=" | "vertical-disabled=") bool
                | ("horizontal-hovered=" | "vertical-hovered=") bool
                | ("horizontal-dragged=" | "vertical-dragged=") bool
scroll_bar_surface_property
               = "background=" background_value | "border=" color_ref
               | ("border-width=" | "radius=" | "radius-tl="
                 | "radius-tr=" | "radius-br=" | "radius-bl=") expr
scroll_auto_property
               = scroll_bar_surface_property | "shadow=" color_ref
               | ("shadow-x=" | "shadow-y=" | "shadow-blur=") expr
               | "icon=" color_ref
scroll_style_section
               = "container" surface_style_property*
               | ("horizontal-rail" | "vertical-rail"
                 | "horizontal-scroller" | "vertical-scroller")
                 scroll_bar_surface_property*
               | "gap" "background=" background_value
               | "auto" scroll_auto_property*
text           = "text" expr text_property* styles?
text_property  = ("width=" | "height=") length | "size=" expr
               | ("line-height=" | "line-height-px=") expr
               | "font=" font_ref
               | "align-x=" text_alignment
               | "align-y=" ("top" | "center" | "bottom")
               | "shaping=" ("auto" | "basic" | "advanced")
               | "wrapping=" ("none" | "word" | "glyph" | "word-or-glyph")
               | "style=" call
input          = "input" string id? "<->" name input_property* styles?
                 (INDENT input_child*)?
input_property = "hint=" string | ("disabled=" | "secure=") expr
               | ("submit=" | "paste=") route | "width=" length
               | ("padding=" | "text-size=" | "line-height=") expr
               | "align=" ("left" | "center" | "right")
               | "font=" font_ref | "style=" call
               | "icon=" string | "icon-font=" font_ref
               | "icon-side=" ("left" | "right")
               | ("icon-size=" | "icon-spacing=") expr
input_child    = input_status | input_icon
input_status   = ("active" | "hovered" | "focused"
               | "focused-hovered" | "disabled") input_style_property*
input_style_property
               = "background=" background_value
               | ("border=" | "icon=" | "placeholder="
                 | "value=" | "selection=") color_ref
               | ("border-width=" | "radius=" | "radius-tl="
                 | "radius-tr=" | "radius-br=" | "radius-bl=") expr
input_icon     = "icon" combo_icon_property+
button         = "button" (string | INDENT node) id? button_property*
                 styles? "->" route (INDENT button_status_style*)?
button_property = "disabled=" expr | ("width=" | "height=") length
                | ("padding=" | "clip=") expr
                | "style=" (("primary" | "secondary" | "success" | "warning"
                  | "danger" | "text" | "background" | "subtle") | call)
button_status_style = ("active" | "hovered" | "pressed" | "disabled")
                      surface_style_property*
checkbox       = "checkbox" expr id? "checked=" expr bool_property*
                 checkbox_icon_property* checkbox_style? styles? "->" route
                 (INDENT checkbox_status_style*)?
toggler        = "toggler" expr "checked=" expr bool_property*
                 ("align=" text_alignment)? styles? "->" route
                 (INDENT toggler_status_style*)?
bool_property  = "disabled=" expr | "size=" expr | "width=" length
               | ("spacing=" | "text-size=" | "line-height=") expr
               | "shaping=" ("auto" | "basic" | "advanced")
               | "wrapping=" ("none" | "word" | "glyph" | "word-or-glyph")
               | "font=" font_ref
checkbox_icon_property = "icon=" string
                       | ("icon-size=" | "icon-line-height=") expr
                       | "icon-shaping=" ("auto" | "basic" | "advanced")
checkbox_style = "style=" ("primary" | "secondary" | "success" | "danger")
checkbox_status_style = ("active" | "hovered" | "disabled")
                        ("checked" | "unchecked") checkbox_style_property*
checkbox_style_property = "background=" background_value
                        | ("icon=" | "text=" | "border=") color_ref
                        | ("border-width=" | "radius=" | "radius-tl="
                          | "radius-tr=" | "radius-br=" | "radius-bl=") expr
toggler_status_style = ("active" | "hovered" | "disabled")
                       ("checked" | "unchecked") toggler_style_property*
toggler_style_property = ("background=" | "foreground=") background_value
                       | ("background-border=" | "foreground-border="
                         | "text=") color_ref
                       | ("background-border-width="
                         | "foreground-border-width=" | "radius="
                         | "radius-tl=" | "radius-tr=" | "radius-br="
                         | "radius-bl=" | "padding-ratio=") expr
text_alignment = "default" | "left" | "center" | "right" | "justified"
text_wrapping  = "none" | "word" | "glyph" | "word-or-glyph"
color_ref      = name ("/" u8)?
slider         = "slider" expr "min=" expr "max=" expr slider_property*
                 styles? "->" route (INDENT slider_status+)?
slider_property = ("step=" | "default=" | "shift-step=") expr
                | ("width=" | "height=") length
                | "vertical" | "release=" route | "style=" call
slider_status  = ("active" | "hovered" | "dragged") slider_style_property*
slider_style_property
               = ("rail-start=" | "rail-end=" | "handle-color=")
                 background_value
               | ("rail-border=" | "handle-border=") color_ref
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
               | "style=" (("primary" | "secondary" | "success"
                 | "warning" | "danger") | call)
               | ("background=" | "bar=") background_value
               | "border=" color_ref
               | ("border-width=" | "radius=" | "radius-tl="
                 | "radius-tr=" | "radius-br=" | "radius-bl=") expr
radio          = "radio" expr "value=" expr "selected=" expr
                 radio_property* styles? "->" route
                 (INDENT radio_status_style*)?
radio_property = ("size=" | "spacing=" | "text-size=" | "line-height=") expr
               | "width=" length
               | "shaping=" ("auto" | "basic" | "advanced")
               | "wrapping=" ("none" | "word" | "glyph" | "word-or-glyph")
               | "font=" font_ref
radio_status_style = ("active" | "hovered")
                     ("selected" | "unselected") radio_style_property*
radio_style_property = "background=" background_value
                     | ("dot=" | "border=" | "text=") color_ref
                     | "border-width=" expr
pick_list      = "pick" expr expr pick_property* "->" route
                 (INDENT pick_child*)?
pick_property  = "placeholder=" expr | "width=" length
               | "menu-height=" length | "padding=" expr
               | ("text-size=" | "line-height=") expr
               | "shaping=" ("auto" | "basic" | "advanced")
               | "font=" font_ref | "open=" route | "close=" route
               | ("style=" | "menu-style=") call
pick_child     = pick_status | menu_style | pick_handle
pick_status    = ("active" | "hovered" | "opened" | "opened-hovered")
                 pick_status_property*
pick_status_property
               = "background=" background_value
               | ("text=" | "placeholder=" | "handle=" | "border=") color_ref
               | ("border-width=" | "radius=" | "radius-tl="
                 | "radius-tr=" | "radius-br=" | "radius-bl=") expr
menu_style     = "menu" menu_style_property*
menu_style_property
               = ("background=" | "selected-background=") background_value
               | ("text=" | "selected-text=" | "border=" | "shadow=") color_ref
               | ("border-width=" | "radius=" | "radius-tl="
                 | "radius-tr=" | "radius-br=" | "radius-bl="
                 | "shadow-x=" | "shadow-y=" | "shadow-blur=") expr
pick_handle    = "handle" ("arrow" ("size=" expr)?
               | "static" pick_icon_property+
               | "dynamic" INDENT pick_closed_icon pick_open_icon
               | "none")
pick_closed_icon = "closed" pick_icon_property+
pick_open_icon = "open" pick_icon_property+
pick_icon_property
               = "code=" string | "font=" font_ref
               | ("size=" | "line-height=") expr
               | "shaping=" ("auto" | "basic" | "advanced")
combo_box      = "combo" name expr string combo_property* "->" route
                 (INDENT combo_child*)?
combo_property = "width=" length | "menu-height=" length
               | "padding=" expr | ("text-size=" | "line-height=") expr
               | "shaping=" ("auto" | "basic" | "advanced")
               | "font=" font_ref
               | "input=" route | "hover=" route
               | "open=" route | "close=" route
               | ("style=" | "menu-style=") call
combo_child    = combo_status | menu_style | combo_icon
combo_status   = ("active" | "hovered" | "focused"
               | "focused-hovered" | "disabled") combo_style_property*
combo_style_property
               = "background=" background_value
               | ("border=" | "icon=" | "placeholder="
                 | "value=" | "selection=") color_ref
               | ("border-width=" | "radius=" | "radius-tl="
                 | "radius-tr=" | "radius-br=" | "radius-bl=") expr
combo_icon     = "icon" combo_icon_property+
combo_icon_property
               = "code=" string | "font=" font_ref
               | ("size=" | "spacing=") expr
               | "side=" ("left" | "right")
float          = "float" float_property* INDENT node
float_property = ("scale=" | "x=" | "y=" | "shadow-x="
                 | "shadow-y=" | "shadow-blur=" | "radius="
                 | "radius-tl=" | "radius-tr=" | "radius-br="
                 | "radius-bl=") expr
               | "shadow=" color_ref
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
media          = ("image" | "svg" | "viewer") expr media_property*
media_property = ("width=" | "height=") length
               | "fit=" ("contain" | "cover" | "fill" | "none" | "scale-down")
               | "rotation=" (expr | "solid(" expr ")") | "opacity=" expr
               | "memory" | "color=" color_ref
               | "hover=" (color_ref | "none")
               | "style=" name "(" expr_list? ")"
               | "filter=" ("linear" | "nearest")
               | "scale=" expr | "expand=" expr
               | ("radius=" | "radius-tl=" | "radius-tr="
                 | "radius-br=" | "radius-bl=") expr
               | "crop=(" expr "," expr "," expr "," expr ")"
               | ("padding=" | "min-scale=" | "max-scale="
                 | "scale-step=") expr
length         = "fill" | "fill(" u16 ")" | "shrink" | expr
tooltip        = "tooltip" tooltip_property* INDENT node node
tooltip_property
               = "position=" ("top" | "bottom" | "left" | "right" | "cursor")
               | "gap=" expr | "padding=" expr | "delay=" expr | "snap=" expr
               | "style=" (("transparent" | "rounded" | "bordered" | "dark"
                 | "primary" | "secondary" | "success" | "warning" | "danger")
                 | name "(" expr_list? ")")
               | "background=" background_value
               | ("text=" | "border=" | "shadow=") color_ref
               | ("border-width=" | "radius=" | "radius-tl="
                 | "radius-tr=" | "radius-br=" | "radius-bl="
                 | "shadow-x=" | "shadow-y=" | "shadow-blur=") expr
               | "pixel-snap=" expr
mouse_area     = "mouse" mouse_property+ INDENT node
mouse_property = ("press=" | "release=" | "double=" | "right_press="
               | "right_release=" | "middle_press=" | "middle_release="
               | "enter=" | "move=" | "scroll=" | "exit=") route
               | "cursor=" mouse_cursor
canvas         = "canvas" canvas_property* INDENT canvas_item*
canvas_property = ("width=" | "height=") length
                | ("cache=" | "capture=") expr
                | ("press=" | "release=" | "right_press=" | "right_release="
                  | "middle_press=" | "middle_release=" | "enter=" | "move="
                  | "scroll=" | "exit=") route
                | "cursor=" (mouse_cursor | "(" expr ")")
                | "cursor-outside=" expr
canvas_item    = canvas_state | canvas_event | canvas_command
canvas_state   = "state" INDENT state+
canvas_event   = "event" canvas_event_source "->" route
               | "event" canvas_event_source ("as" name_list)?
                 INDENT canvas_event_action+
               | "capture" canvas_event_source
               | "redraw" canvas_event_source ("after=" duration)?
canvas_event_source
               = "input-method" input_method_event
               | "keyboard" ("press" | "release" | "modifiers")
               | "mouse" mouse_event | "touch" touch_event
               | "window" window_event
canvas_event_action
               = "set" name "=" expr | "emit" route | "capture"
               | "redraw" ("after=" duration)?
canvas_command = canvas_rect | canvas_circle | canvas_line | canvas_text
               | canvas_path | canvas_group | canvas_if | canvas_for
canvas_rect    = "rect" point size canvas_radius* canvas_paint+
canvas_circle  = "circle" point "radius=" expr canvas_paint+
canvas_line    = "line" "x1=" expr "y1=" expr "x2=" expr "y2=" expr
                 canvas_stroke
canvas_text    = "text" expr "x=" expr "y=" expr canvas_text_property*
canvas_text_property = ("max-width=" | "size=" | "line-height="
                       | "line-height-px=") expr
                     | "color=" color_ref | "font=" name
                     | "align-x=" ("default" | "left" | "center" | "right"
                       | "justified")
                     | "align-y=" ("top" | "center" | "bottom")
                     | "shaping=" ("auto" | "basic" | "advanced")
canvas_path    = "path" canvas_paint+ INDENT canvas_path_segment+
canvas_group   = "group" canvas_transform* INDENT canvas_command*
canvas_if      = "if" expr INDENT canvas_command*
canvas_for     = "for" name "in" expr INDENT canvas_command*
point          = "x=" expr "y=" expr
size           = "width=" expr "height=" expr
canvas_radius  = ("radius=" | "radius-tl=" | "radius-tr="
                 | "radius-br=" | "radius-bl=") expr
canvas_paint   = "fill=" background_value | "fill-rule=" ("non-zero" | "even-odd")
               | canvas_stroke
canvas_stroke  = "stroke=" background_value ("stroke-width=" expr)?
                 ("cap=" ("butt" | "square" | "round"))?
                 ("join=" ("miter" | "round" | "bevel"))?
                 ("dash=" "(" expr_list ")")? ("dash-offset=" expr)?
canvas_transform = ("x=" | "y=" | "rotate=" | "scale="
                   | "scale-x=" | "scale-y=") expr
                 | "clip=(" expr "," expr "," expr "," expr ")"
canvas_path_segment = "move" point | "line" point
                    | "arc" point "radius=" expr "start=" expr "end=" expr
                    | "arc-to" "ax=" expr "ay=" expr "bx=" expr "by=" expr
                      "radius=" expr
                    | "ellipse" point "radius-x=" expr "radius-y=" expr
                      "rotation=" expr "start=" expr "end=" expr
                    | "bezier" "ax=" expr "ay=" expr "bx=" expr "by=" expr point
                    | "quadratic" "cx=" expr "cy=" expr point
                    | "rect" point size
                    | "rounded" point size canvas_radius+
                    | "circle" point "radius=" expr | "close"
theme_boundary = "theme" theme_preset? theme_property* INDENT node
theme_preset   = "default" | "app" | built_in_iced_theme | theme_call
theme_call     = name "(" expr_list? ")"
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
component_name = PascalName ("." PascalName)*
component_call = component_name ("(" expr_list? ")" id? | component_item*)
                 (INDENT (node | named_slot+ | component_call+))?
component_item = named_prop | id
named_prop     = name "=" expr
named_slot     = name ":" INDENT node
slot           = "slot" name?
extern_component_call
               = "extern" name "(" expr_list? ")" ("->" route)?
themer_view    = "themer" name "(" expr_list? ")" ("->" route)?
shader_view    = "shader" name "(" expr_list? ")"
                 (("width=" | "height=") length)* ("->" route)?
if_node        = "if" expr INDENT node+
for_node       = "for" name "in" expr INDENT node+

property       = "hint=" string | "disabled=" expr | "checked=" expr
styles         = "@" utility+
id             = "#" kebab_name | "#" name "(" expr ")"
route          = name | name "(" route_arg_list? ")"
route_arg      = expr | "_"
```

Application configuration lives under the app declaration. The four iced
callbacks accept state expressions directly:

```ice
app Tasks
  title window_title
  theme app_theme
  background app_background
  text-color app_text
  id "dev.ducktape.ice.tasks"
  executor iced::executor::Default
  renderer crate::backend::AppRenderer
  font "assets/Inter-Regular.ttf"
  font "assets/Inter-Bold.ttf"
  default-text-size 16
  antialiasing true
  vsync true
  scale-factor ui_scale
  window
    icon-rgba "assets/app.rgba" 32 32
    size 960 720
    min-size 480 360
    max-size 1920 1080
    position centered
    level normal
    platform linux
      application-id "dev.ducktape.ice.tasks"
      override-redirect false
    platform windows
      drag-and-drop true
      skip-taskbar false
      undecorated-shadow true
      corner round-small
    platform macos
      title-hidden false
      titlebar-transparent true
      fullsize-content-view true
    platform wasm
      target "iced"
  window child
    size 640 480
    min-size 320 240
    position centered

state
  window_title = "Ice Tasks"
  app_theme = "app"
  app_background = "#0f172a"
  app_text = "#f8fafc"
  ui_scale = 1.0
```

`title`, `theme`, `background`, `text-color`, and `scale-factor` are recomputed
from current state through iced's native callbacks. Title/theme/style values are
typed `str`; scale is `f64`. Theme accepts `app`, `default`, or any of iced's 22
kebab-case built-ins. Application colors accept 3/4/6/8 digit hexadecimal
strings. Invalid dynamic theme/color values safely retain the generated app
theme or selected theme base style, and a non-positive dynamic scale is clamped
to `f32::EPSILON`. Literal mistakes are rejected during analysis.

The remaining application values lower to iced `Settings` and builder
configuration.
`executor` is a Rust type path passed to iced's typed `Application::executor`;
rustc reports a local generated-code error when the type is missing or does not
implement `iced::Executor`.
`renderer` selects the app's concrete `iced::program::Renderer` type. It
defaults to `iced::Renderer`; generated view and extern `Element` signatures
use the selected type, so rustc checks its renderer, compositor, text, and
headless contracts at the generated boundary.
Each `font` path is relative to the root `.ice` file, must name an existing
file during `cargo ice check`, and lowers to iced's startup
`.font(include_bytes!(...))` builder. Repeating the same path is rejected;
different files may be loaded in declaration order.
The unnamed `window` block configures the initial window. A named block such
as `window child` declares a checked settings template that `task window open`
can instantiate; names must be unique. Both forms cover every cross-platform
`window::Settings` field:
initial/minimum/maximum size, maximized/fullscreen state,
default/centered/fixed position, visibility, resizability, close/minimize
buttons, decorations, transparency, blur, level, and close-request behavior.
Nested `platform linux`, `platform windows`, `platform macos`, and
`platform wasm` blocks expose every field of iced's target-specific settings.
All four may coexist in one source; generated `cfg` blocks select only the
current compilation target. Wasm `target none` appends to the document body.
Sizes, text size, and scale factor must be positive; minimum size cannot exceed
maximum size. `icon-rgba` embeds a relative raw RGBA file without an image
codec; width and height are positive integers, and generated Rust rejects a
byte length other than `width × height × 4`. `cargo ice check` reports a
mismatch at the icon declaration, and generated Rust repeats the check at
compile time. Encoded icon formats remain outside 1.44.

Use `daemon Name` instead of `app Name` for an iced daemon that starts without
an initial window and remains alive after all windows close. A daemon rejects
the unnamed `window` block; declare named window templates and open them from
`on mount` or another handler. The read-only `window:window-id` binding names
the window currently being rendered and is available to the root view, title,
theme, and scale-factor expressions. Pure components receive it explicitly as
a typed prop. Standalone `exit` is a native `iced::exit()` task and must be the
final statement in a handler (or a task-group member):

```ice
daemon BackgroundAgent
  title daemon_title(window)
  window dashboard

on mount
  task window open dashboard -> opened _

on quit
  exit

view
  AgentWindow id=window
```

Application boot presets are structured top-level declarations:

```ice
preset pristine

preset seeded
  state
    draft = "Preset task"
    loading = true
  boot
    run list_tasks() -> loaded _ | failed _
```

Each preset starts from declared state and internal widget layout state without
running `on mount`. The optional `state` section applies checked assignments in
order. The optional `boot` section accepts the same checked statements, task
composition, and routes as a handler. With no task it returns `Task::none`.
Generated code passes each strategy to iced `Preset::new`; an empty preset is a
side-effect-free default-state fixture.

Media fixed lengths, opacity, scale, and radius are `f64`. `fit=` accepts its
compact contain/cover/fill/none/scale-down names or a first-class `content-fit`
expression. Rotation accepts
legacy f64 radians (floating by default), `solid(angle)`, or a first-class
`rotation` expression. Opacity is `0.0..=1.0`, scale is positive, and
sizes/radius are non-negative. `filter`, `scale`, `expand`, `radius`, and `crop` are image-only.
Crop is `(x, y, width, height)` in non-negative `i64` source-pixel coordinates.
`memory`, `color`, and `hover` are SVG-only. `memory` accepts UTF-8 SVG text or
raw `bytes`; `color` filters both statuses and `hover` overrides the
hovered status with a checked theme color or `none`.
`viewer` wraps an image path or handle in iced's stateful zoom/pan widget. It
accepts width, height, fit, filter, non-negative padding, positive minimum and
maximum scales, and a positive scale step; the minimum cannot exceed the
maximum. The widget owns gesture state in iced's tree, so no app state or event
handler is needed:

```ice
viewer memory_image width=fill height=240.0 fit=contain filter=nearest padding=8.0 min-scale=0.5 max-scale=8.0 scale-step=0.1
```
Every `length` position accepts fixed `f64`, `fill`, `fill(N)` portions with a
decimal `u16`, `shrink`, or a checked first-class `length` expression;
out-of-range compact portions fail during parsing. Grid width and the short
axis of a horizontal/vertical slider remain fixed pixels because those iced
builders accept `Pixels`, not `Length`.
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
x/y as four f64 payloads. `viewport=` is the complete alternative and receives
14 f64 values in this order: absolute x/y, reversed absolute x/y, relative x/y,
viewport x/y/width/height, then content x/y/width/height. The two routes are
mutually exclusive. Bare handler names receive every payload automatically.

`style=task_scroll(loading)` may call a declared `scroll-style`. Its Rust
function receives `&iced::Theme`, the current `scrollable::Status`, then its
owned arguments and returns `scrollable::Style`.

Optional `active`, `hovered`, and `dragged` lines expose every concrete
scrollable Style field: its container, both rails and scrollers, corner gap,
and auto-scroll overlay. Bool selectors match iced's horizontal/vertical
hovered, dragged, and disabled status fields. Omitted selectors are wildcards;
matching lines apply in source order after the typed callback, so a later
specific line can refine its base style:

```ice
scroll direction=both viewport=viewport_changed style=task_scroll(loading)
  col
    text "Scrollable"
  active
    container background=background
    horizontal-scroller background=primary
    vertical-scroller background=primary
    auto background=surface icon=foreground
  hovered horizontal-hovered=true
    horizontal-scroller background=foreground
  dragged vertical-dragged=true
    vertical-scroller background=danger
```

`text` accepts str, i64, and f64 values plus typed width/height, positive size,
relative `line-height=` or absolute `line-height-px=`, horizontal and vertical
alignment, shaping, wrapping, and declared or built-in fonts. An explicit `size=`
overrides a `@text-*` utility; `font=mono @font-bold` preserves both choices.

`input` keeps its required `str` binding and additionally supports bool secure
mode, submit routes, str-payload paste routes, typed width/padding/text size,
relative line height, horizontal alignment, complete font descriptors, and a
complete text-input icon. Its five optional status lines expose every concrete
iced text-input Style field. A disabled input suppresses typing, submit, and
paste messages together. The old inline `icon=`, `icon-font=`, `icon-size=`,
`icon-spacing=`, and `icon-side=` properties remain accepted as compact syntax.

```ice
input "Search" #query <-> query hint="Find anything" font=ui
  active background=surface border=border icon=primary placeholder=muted value=foreground selection=primary
  focused-hovered background=surface border=primary border-width=2.0 radius=8.0
  disabled background=background border=border value=muted
  icon code="⌕" font=ui size=14.0 spacing=6.0 side=left
```

`style=form_input(disabled)` may call a declared `input-style`. Its Rust
function receives `&iced::Theme`, the current `text_input::Status`, then its
owned arguments and returns `text_input::Style`. Checked utilities apply next,
and status lines are the final overrides.

`button` accepts either its compact string label or exactly one arbitrary child
node. It also supports typed width/height, non-negative padding, bool clipping,
disabled routing, all eight iced presets, and checked style utilities. Optional
`active`, `hovered`, `pressed`, and `disabled` child lines override every
concrete button style field with solid/linear backgrounds, text, per-corner
border, shadow, and pixel snapping. A structured content node may appear beside
these status lines.

`style=action_button(loading)` may instead call a declared `button-style`.
Its Rust function receives `&iced::Theme`, the current `button::Status`, then
its declared owned arguments and returns `button::Style`. Ice installs it as
the native runtime style callback; utilities and status lines still override
that returned base style.

`checkbox` and `toggler` share typed control size/width/spacing, text size and
relative line height, shaping, wrapping, and default/mono font properties.
Togglers add full text alignment. Checkboxes add a single-character icon with
size, relative line height, and shaping. A checkbox may start from any of iced's
primary, secondary, success, or danger presets and override each checked and
unchecked form of its active, hovered, and disabled statuses independently:

```ice
checkbox "Complete" checked=done style=success -> changed _
  active checked background=linear(1.57, primary@0.0, surface@1.0) icon=foreground text=foreground border=primary border-width=1.0 radius=4.0
  active unchecked background=surface icon=primary border=border
  hovered checked background=primary icon=foreground border=foreground
  hovered unchecked background=background icon=primary border=primary
  disabled checked background=surface icon=muted text=muted border=border
  disabled unchecked background=background icon=muted text=muted border=border
```

`style=task_checkbox(loading)` may instead call a declared `checkbox-style`.
Its Rust function receives `&iced::Theme`, the current `checkbox::Status`, then
its declared owned arguments and returns `checkbox::Style`. Status lines still
override the returned base style.

Each line starts from the selected preset for that exact status and overrides
any listed solid/linear background, icon/text color, or border color, width, and
uniform/per-corner radius. Metrics are checked non-negative f64 expressions.

A toggler uses the same six checked-aware status selectors. Each starts from
iced's default style and may override every concrete field:

```ice
toggler "Notifications" checked=enabled -> changed _
  active checked background=linear(1.57, primary@0.0, surface@1.0) background-border=primary background-border-width=1.0 foreground=foreground foreground-border=border foreground-border-width=1.0 text=foreground radius=8.0 padding-ratio=0.125
  active unchecked background=surface foreground=foreground text=muted
  hovered checked background=primary foreground=foreground text=foreground
  hovered unchecked background=background foreground=primary text=foreground
  disabled checked background=surface foreground=muted text=muted
  disabled unchecked background=background foreground=muted text=muted
```

Background and foreground accept checked solid or linear values. Both borders,
optional uniform/per-corner radius, and text color map directly to
`toggler::Style`; widths and radii are non-negative, while `padding-ratio=` is
checked in `0.0..=0.5` to keep the foreground dimensions non-negative.
`style=notification_toggler(loading)` may call a declared `toggler-style` whose
Rust function receives `&iced::Theme`, `toggler::Status`, then its owned
arguments and returns `toggler::Style`. Status lines override that base.

`slider` accepts `f64` or one extern named numeric type consistently across its
value, range, step, optional default, and optional shift step. The route carries
that exact type. `f64` steps are statically positive and literal defaults stay
inside literal ranges. A generic slider supplies an explicit same-type step;
generated Rust verifies iced's native `Copy`, `From<u8>`, `PartialOrd`,
`Into<f64>`, and `FromPrimitive` bounds. A zero-field extern declaration can
name a Rust numeric alias without exposing fake fields:

```ice
extern crate::backend
  SliderNumber()
  sync slider_number(value:f64) -> SliderNumber

state
  precise:SliderNumber = slider_number(50.0)

on precise_changed(next)
  precise = next

view
  slider precise min=slider_number(0.0) max=slider_number(100.0) step=slider_number(0.5) -> precise_changed _
```

The optional default handles command/control-click reset and an optional release
route reports interaction completion. Horizontal
sliders accept any length for width and fixed height; vertical sliders accept
fixed width and any length for height. For `f64`, literal reversed ranges and
invalid defaults are rejected before code generation; fluid cross-axis sizes
are rejected for every slider type.

A slider may own one nested `active`, `hovered`, and `dragged` style block.
`style=volume_slider(loading)` may call a declared `slider-style` whose Rust
function receives `&iced::Theme`, `slider::Status`, then its owned arguments
and returns `slider::Style`. Each block starts from that callback result, or
iced's default style, and overrides any listed rail backgrounds/width/border/
radius or handle shape/background/border.
Rail and handle backgrounds accept checked solid or linear values; borders stay
checked theme colors. Rectangle widths are `u16`; every other metric is a
non-negative f64. Handle corner radii require a rectangle handle in the same
status block.

`progress` supports all iced length variants for its main `length` and cross-axis
`girth`, horizontal or vertical direction, and primary/secondary/success/warning/
danger presets. Checked solid or linear backgrounds can override the track and
filled bar; a checked theme color overrides the border. Border width and
uniform/per-corner radii are non-negative f64 values.
Literal reversed ranges are rejected before generation.

A declared progress style call may replace the preset while keeping those
field overrides:

```ice
extern crate::backend
  progress-style loading_progress(active:bool)

progress amount style=loading_progress(loading) bar=primary
```

Its Rust function receives the current `&iced::Theme` before its declared
owned arguments and returns `iced::widget::progress_bar::Style`. Ice installs
it as the native runtime style callback, which is also the default Theme's
advanced class representation.

`radio` accepts bool, i64, f64, str, or extern values and sends that typed value
to its route. `selected=` remains an explicit bool expression, so groups can use
any selection model without requiring a second optional state. The backend uses
a private bool as iced's `Eq + Copy` radio identity and puts the original owned
Ice value in the generated message; string and extern values therefore keep the
same click semantics without pretending they are Rust `Copy` types.

Size, every width `Length`, spacing, text size, relative line height, shaping,
wrapping, and complete font descriptors map to the corresponding radio setters.
Four optional `active|hovered × selected|unselected` child lines start from
iced's default style and override every concrete field:

```ice
radio "Summary" value="summary" selected=(mode == "summary") size=18.0 width=fill font=ui -> mode_changed _
  active selected background=linear(1.57, primary@0.0, surface@1.0) dot=foreground border=primary border-width=2.0 text=foreground
  active unselected background=surface dot=primary border=border text=muted
  hovered selected background=primary dot=foreground border=foreground text=foreground
  hovered unselected background=background dot=primary border=primary text=foreground
```

Background accepts checked solid or linear values; dot, border, and text are
checked colors, and border width is a non-negative f64 expression.
`style=view_radio(loading)` may call a declared `radio-style` whose Rust
function receives `&iced::Theme`, `radio::Status`, then its owned arguments and
returns `radio::Style`. Status lines override that returned base.

`tooltip` styles start from transparent, rounded, bordered, dark, primary,
secondary, success, warning, or danger iced container presets. A checked solid
or linear background plus theme colors can override the preset's background,
text, border, and shadow. Border width, shadow
blur, and uniform/per-corner radii are non-negative f64 values; shadow x/y may
be negative. `pixel-snap=` controls the container style's pixel-grid snap and is
separate from the tooltip overlay's viewport `snap=` behavior. A declared
`container-style` call may replace the preset because iced uses the same
`container::Style` callback for tooltip surfaces; concrete tooltip properties
override the callback result.

`pick` requires a homogeneous `[T]` options expression and a matching optional
`T?` selection. Its main route carries `T`; `open=` and `close=` routes carry no
payload. Pick values may be bool, i64, f64, str, or an extern type. Fixed
width/menu height, padding, text size, relative line height, complete font
descriptors, and shaping map directly to iced's setters. All concrete field
styles are structured children: `active`, `hovered`, `opened`, and
`opened-hovered` cover the field statuses, while `menu` covers its overlay.
Each starts from iced's status default and accepts checked solid/linear
backgrounds, colors, border/per-corner radius, and menu shadow fields.
`style=view_picker(loading)` and `menu-style=view_menu(loading)` may instead
start from declared native callbacks; the structured status and menu fields
remain final overrides.

```ice
pick modes mode placeholder="Choose" font=ui shaping=advanced style=view_picker(loading) menu-style=view_menu(loading) -> changed _
  active text=foreground placeholder=muted handle=primary background=surface border=border radius=6.0
  opened-hovered text=foreground background=background border=primary
  menu text=foreground selected-text=foreground selected-background=primary background=surface shadow=black/50 shadow-y=4.0
  handle dynamic
    closed code="⌄" font=ui size=12.0
    open code="⌃" font=ui size=12.0 shaping=advanced
```

Handles support iced's arrow with optional size, one static icon, distinct
closed/open dynamic icons, or no handle. Icon code points contain exactly one
Unicode scalar; icon size and relative line height are non-negative `f64`.

`combo` requires a `combo[T]` search state and matching `T?` selection. Its
main and `hover=` routes carry `T`; `input=` carries str; `open=` and `close=`
carry no payload. A bare input/hover handler name receives the payload
automatically. Width/menu height, padding, text size, relative line height,
shaping, and complete fonts map to every native builder setter. A structured
`icon` line covers the complete text-input icon: one Unicode scalar, font,
size, spacing, and side.

The five `active`, `hovered`, `focused`, `focused-hovered`, and `disabled`
lines start from iced's status default and expose every concrete input Style
field. The shared `menu` line exposes every menu overlay Style field:
`style=form_input(loading)` reuses the native `input-style` ABI, and
`menu-style=view_menu(loading)` reuses the same menu callback as `pick`.
Structured lines override both callback results.

```ice
combo modes mode "Search views" font=ui shaping=advanced style=form_input(loading) menu-style=view_menu(loading) -> changed _
  active background=surface border=border icon=primary placeholder=muted value=foreground selection=primary
  focused-hovered background=background border=primary border-width=2.0 radius=6.0
  menu text=foreground selected-text=foreground selected-background=primary background=surface shadow=black/50
  icon code="⌕" font=ui size=14.0 spacing=6.0 side=right
```

Assigning a matching `[T]` to `combo[T]` state replaces its searchable options
with a freshly indexed native `combo_box::State<T>`. `combo modes push value`
calls native `State::push` to incrementally add one matching option and rebuild
its search index. Mismatched lists or pushed values fail before Rust generation.

`float` applies positive scale and x/y translation to one child. Its x/y
expressions can use the scoped `f64` names `original_x`, `original_y`,
`original_width`, `original_height`, `viewport_x`, `viewport_y`,
`viewport_width`, and `viewport_height` from iced's translation callback:

```ice
float scale=1.02 x=(viewport_width - original_width) y=-1.0 shadow=black/50 shadow-y=2.0 shadow-blur=4.0 radius=4.0
  text "Floating label"
```

The shadow color, offset, blur, and uniform/per-corner radius properties cover
every concrete `float::Style` field. The scoped geometry names exist only in
`x=` and `y=`; style expressions and the child use the surrounding scope.
`pin` places one child at x/y coordinates inside optional typed width/height
bounds; x/y is the direct decomposition of iced's `position(Point)` helper.
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
Checked `text=` and solid or linear `background=` values override the subtree
defaults.

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
the view. A literal initializes it directly, `markdown(source)` replaces it,
`markdown state append source` incrementally extends it, and
`markdown_images(state)` returns every referenced image URI as `[str]`:

```ice
state
  help:markdown = "# Help [docs](https://iced.rs)"
  images:[str] = []

on open_link(url)
on extend
  markdown help append "\n\n![Ice](asset://ice)"
  images = markdown_images(help)

view
  markdown help text-size=16.0 spacing=12.0 -> open_link _
    style font=ui inline-code-background=surface inline-code-color=foreground inline-code-font=mono code-block-font=mono link=primary inline-code-padding=3.0 inline-code-border=border inline-code-border-width=1.0 inline-code-radius=4.0
```

The route receives the clicked URI as str. `text-size`, every h1-h6 size,
`code-size`, and `spacing` map directly to iced Markdown `Settings`; sizes must
be positive and spacing non-negative. The nested `style` line covers every
field in iced Markdown `Style`: base, inline-code, and code-block fonts;
inline-code highlight background, padding, text color, and full border; and
link color. Uniform, axis, per-side padding and uniform/per-corner radius use
the most specific supplied value. The reference app enables iced's Markdown
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
    active background=surface border=border placeholder=muted value=foreground selection=primary
    focused-hovered background=surface border=primary border-width=2.0 radius=8.0
    disabled background=background border=border value=muted
```

The compiler owns iced's `Action` message variant and calls `Content::perform`
automatically, so ordinary editor actions never leak into application handlers.
Width is fixed pixels, height accepts every iced `Length`, metrics are
range-checked, and all four wrapping modes, declared or built-in fonts,
relative/absolute line height, and all five iced highlighter themes are
accepted. Optional status lines cover every concrete Style field for active,
hovered, focused, focused-hovered, and disabled editors. A disabled editor is
rendered without `on_action`.

The remaining native extension points are typed:

```ice
extern crate::backend
  EditorCommand(save:bool)
  editor-binding editor_keys(readonly:bool) -> EditorCommand
  editor-highlighter editor_highlight(token:str)
  editor-style editor_surface(readonly:bool)

component EditorPanel(content:editor, readonly:bool)
  editor <-> content highlighter=editor_highlight("fn") key-binding=editor_keys(readonly) style=editor_surface(readonly) -> editor_command _
```

`editor-binding` receives iced's `KeyPress` implicitly and returns an optional
native `Binding<EditorCommand>`; built-in edit bindings stay native while
`Binding::Custom` is mapped through the checked route. `editor-highlighter`
receives the fully configured plain-text `TextEditor` and returns a value
convertible to the same default `Element`, so Rust can call `highlight_with`
with any `Highlighter`, settings, highlight type, and format function.
`editor-style` receives Theme and editor Status implicitly and returns native
`text_editor::Style`, covering the advanced catalog class. An editor or input
inside a pure component may bind an `editor` or `str` prop when every call
passes a direct app state; the checker rejects computed temporary bindings.

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
| `result[T,E]` | `Result<T, E>` |
| `combo[T]` | `iced::widget::combo_box::State<T>` |
| `animation[bool]` | `iced::Animation<bool>` |
| `animation[f64]` | `iced::Animation<f32>`; expressions convert at the Ice numeric boundary |
| `animation[Name]` | `iced::Animation<crate::...::Name>`; rustc verifies `Copy + PartialEq + iced::animation::Float` |
| `image` | `iced::widget::image::Handle` |
| `image-allocation` | `iced::widget::image::Allocation` |
| `image-memory` | `Weak<iced::advanced::image::Memory>` |
| `image-error` | `iced::widget::image::Error` |
| `size-u32` | `iced::Size<u32>` |
| `debug-span` | `iced::debug::Span`; only valid as optional owned state |
| `rotation` | `iced::Rotation` |
| `content-fit` | `iced::ContentFit` |
| `color` | `iced::Color` |
| `length` | `iced::Length` |
| `alignment` | `iced::Alignment` |
| `horizontal-alignment` | `iced::alignment::Horizontal` |
| `vertical-alignment` | `iced::alignment::Vertical` |
| `instant` | `iced::time::Instant` |
| `window-id` | `iced::window::Id` |
| `markdown` | `iced::widget::markdown::Content` |
| `editor` | `iced::widget::text_editor::Content` |
| `event` | `iced::Event` |
| `task-handle` | `iced::task::Handle` |
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
`Clone` for 1.44 message payloads.

Declared `sync` functions are checked, synchronous Rust calls available in
Ice expressions. They are the small escape hatch for pure domain conversions
that do not justify a language builtin:

```ice
extern crate::backend
  NetworkError(message:str)
  AppError(message:str)
  sync normalize_error(error:NetworkError) -> AppError
```

This declaration requires
`fn normalize_error(NetworkError) -> AppError`; generated probes verify the
actual Rust signature. A sync function cannot declare `! Error` because it
returns its value directly.

Thirty-two typed iced adapters expose framework capabilities without embedding Rust
expressions in Ice:

```ice
extern crate::backend
  component native_help(active:bool) -> bool
  component borrowed_help(label:&str, active:&bool) -> bool
  selector by_kind(kind:str) -> str
  shader status_shader(speed:f64) -> bool
  task copy_text(text:str) -> unit
  stream task_steps(count:i64) -> i64
  sip download(url:str) progress=f64 -> bytes ! AppError
  recipe events(channel:i64) -> str
  event-filter runtime_event() -> str
  subscription app_events() -> bool
  theme app_theme(dark:bool)
  themer alternate_panel(active:bool) -> bool
  markdown-viewer docs_viewer(prefix:str) -> str
  editor-binding editor_keys(readonly:bool) -> EditorCommand
  editor-highlighter editor_highlight(token:str)
  editor-style editor_surface(readonly:bool)
  text-style summary_text(busy:bool)
  slider-style volume_slider(busy:bool)
  progress-style loading_progress(active:bool)
  button-style action_button(busy:bool)
  checkbox-style task_checkbox(busy:bool)
  toggler-style notification_toggler(busy:bool)
  radio-style view_radio(busy:bool)
  container-style summary_container(busy:bool)
  svg-style status_svg(active:bool)
  input-style form_input(disabled:bool)
  scroll-style task_scroll(active:bool)
  pick-list-style view_picker(active:bool)
  menu-style view_menu(active:bool)
  pane-grid-style workspace_panes(active:bool)
```

Their Rust signatures are:

```rust
fn native_help(active: bool) -> iced::Element<'static, bool>;
fn borrowed_help<'a>(label: &'a str, active: &'a bool)
    -> iced::Element<'a, bool, iced::Theme, AppRenderer>;
fn by_kind(kind: String) -> impl iced::widget::selector::Selector<Output = String>;
fn status_shader(speed: f64) -> impl iced::widget::shader::Program<bool>;
fn copy_text(text: String) -> iced::Task<()>;
fn task_steps(count: i64) -> impl iced::futures::Stream<Item = i64> + Send + 'static;
fn download(url: String) -> impl iced::task::Straw<Vec<u8>, f64, AppError> + Send + 'static;
fn events(channel: i64) -> impl iced::advanced::subscription::Recipe<Output = String>;
fn runtime_event(event: iced::advanced::subscription::Event) -> Option<String>;
fn app_events() -> iced::Subscription<bool>;
fn app_theme(dark: bool) -> iced::Theme;
fn alternate_panel(active: bool) -> (
    Option<AlternateTheme>,
    iced::Element<'static, bool, AlternateTheme>,
    Option<fn(&AlternateTheme) -> iced::Color>,
    Option<fn(&AlternateTheme) -> iced::Background>,
);
fn docs_viewer(prefix: String) -> impl for<'a> iced::widget::markdown::Viewer<'a, String>;
fn editor_keys(event: iced::widget::text_editor::KeyPress, readonly: bool) -> Option<iced::widget::text_editor::Binding<EditorCommand>>;
fn editor_highlight<'a, Message: 'a>(editor: iced::widget::text_editor::TextEditor<'a, iced::advanced::text::highlighter::PlainText, Message>, token: String) -> impl Into<iced::Element<'a, Message>>;
fn editor_surface(theme: &iced::Theme, status: iced::widget::text_editor::Status, readonly: bool) -> iced::widget::text_editor::Style;
fn summary_text(theme: &iced::Theme, busy: bool) -> iced::widget::text::Style;
fn volume_slider(theme: &iced::Theme, status: iced::widget::slider::Status, busy: bool) -> iced::widget::slider::Style;
fn loading_progress(theme: &iced::Theme, active: bool) -> iced::widget::progress_bar::Style;
fn action_button(theme: &iced::Theme, status: iced::widget::button::Status, busy: bool) -> iced::widget::button::Style;
fn task_checkbox(theme: &iced::Theme, status: iced::widget::checkbox::Status, busy: bool) -> iced::widget::checkbox::Style;
fn notification_toggler(theme: &iced::Theme, status: iced::widget::toggler::Status, busy: bool) -> iced::widget::toggler::Style;
fn view_radio(theme: &iced::Theme, status: iced::widget::radio::Status, busy: bool) -> iced::widget::radio::Style;
fn summary_container(theme: &iced::Theme, busy: bool) -> iced::widget::container::Style;
fn status_svg(theme: &iced::Theme, status: iced::widget::svg::Status, active: bool) -> iced::widget::svg::Style;
fn form_input(theme: &iced::Theme, status: iced::widget::text_input::Status, disabled: bool) -> iced::widget::text_input::Style;
fn task_scroll(theme: &iced::Theme, status: iced::widget::scrollable::Status, active: bool) -> iced::widget::scrollable::Style;
fn view_picker(theme: &iced::Theme, status: iced::widget::pick_list::Status, active: bool) -> iced::widget::pick_list::Style;
fn view_menu(theme: &iced::Theme, active: bool) -> iced::overlay::menu::Style;
fn workspace_panes(theme: &iced::Theme, active: bool) -> iced::widget::pane_grid::Style;
```

An extern component parameter without `&` is owned. `&str`, `&bytes`, and
`&[T]` lower to borrowed slices; any other `&T` parameter lowers to a shared
Rust reference. A component may therefore return `Element<'a, Event, Theme,
Renderer>` borrowing app state, while owned-only components may return
`Element<'static, Event, Theme, Renderer>`. Both use the app's configured
renderer. A shader factory returns any concrete
`shader::Program<Event>`; Ice constructs the native `Shader`, exposes its full
width/height builder API, and maps the program's published event through a
checked route:

```ice
shader status_shader(1.0) width=fill height=32.0 -> shader_hovered _
```

A selector factory returns any concrete `widget::selector::Selector`. Ice passes
its declared arguments, preserves its declared output type, and uses the result
with native `find` or `find_all`. The consumer must enable iced's `selector`
feature. Built-in selectors produce Ice's normalized `widget-target` value;
custom selectors should use ordinary declared output types.

A task returns `Task<Event>` or `Task<Result<Event, Error>>`. A stream returns
any static `Stream<Item = Event>` or `Stream<Item = Result<Event, Error>>` that
meets iced's platform send bound. A sip returns a static
`Sipper<Output, Progress>` or `Straw<Output, Progress, Error>`. A subscription
recipe factory returns a concrete `advanced::subscription::Recipe`. An event
filter receives iced's implicit runtime `Event` and optionally returns its
declared payload. A subscription adapter returns `Subscription<Event>`.
`theme` returns the concrete default-renderer `iced::Theme`; unlike value
externs, its return type is implicit and no `->` appears in Ice. Its checked
arguments may come from app state or the local component scope. The same
factory is valid in the app `theme` setting and around one nested subtree, so
Rust can construct `Theme::custom`, `Theme::custom_with_fn`, or any built-in
theme while retaining full `Palette` and `Extended` palette logic.
`themer` applies any Rust `Theme: iced::theme::Base` to a Rust-owned subtree
while the surrounding Ice app keeps its normal Theme. Its factory returns the
optional alternate Theme, an `Element` using that exact Theme type, and
optional Theme-dependent text-color and background function pointers. The
generated probe verifies all four tuple fields use the same Theme type; the
view lowers through native `widget::themer`, applies both callbacks when
present, and maps the declared event through an ordinary checked route.

```ice
view
  themer alternate_panel(active) -> alternate_changed _
```

`markdown-viewer` returns one concrete viewer implementing iced's default-theme,
default-renderer `Viewer` for every item lifetime. `viewer=docs_viewer(args)`
switches the Markdown node to native `view_with`; its declared output type is
the checked route payload. The viewer owns customization of images, headings,
paragraphs, code blocks, lists, quotes, rules, and tables. `progress-style`
receives the current Theme implicitly and returns one native progress Style;
generated code uses it directly as the widget's runtime style callback.

`editor-binding` receives native `KeyPress` before its declared arguments and
returns `Option<Binding<Output>>`; `Output` is the custom route payload.
`editor-highlighter` receives a fully configured plain `TextEditor` before its
declared arguments and returns any value convertible to the same default
`Element`. `editor-style` receives Theme and native editor Status implicitly.

`text-style` receives the current Theme implicitly and returns native
`text::Style`. Both `text ... style=summary_text(args)` and
`rich-text style=summary_text(args)` use it as a runtime callback. An explicit
rich-text `color=` or trailing text-color utility overrides the callback color.
`button-style` also receives the current button Status and returns its native
Style. `checkbox-style`, `toggler-style`, and `radio-style` do the same for
their selection-aware widget Status values. `container-style` receives Theme
without a Status and returns its native surface Style. `svg-style` receives
Theme and the idle/hovered SVG Status and returns the native SVG Style.
`input-style` receives Theme and the current text-input Status and returns its
native Style. `scroll-style` receives Theme and the complete scrollable Status
and returns its native Style. `pick-list-style` does the same for pick-list
Status. `menu-style` receives Theme without a Status and returns the shared
pick-list/combo overlay menu Style. `pane-grid-style` receives Theme without a
Status and returns the native pane-grid Style; checked structured style fields
remain available as explicit overrides.

Generated probes type-check every declaration
against the actual Rust item. Extern component, shader, recipe, event-filter,
sync, selector, subscription, theme, themer, window, Markdown viewer, editor extension, and widget style declarations are
infallible; errors are ordinary event payloads when an adapter needs them.
Shader programs retain native control of `State`, `Primitive`, GPU
pipeline/storage, event actions, redraws, capture, and mouse interaction. The
consumer must enable iced's `wgpu` feature.

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

Native animation state keeps motion structured while iced remains in charge of
time and interpolation:

```ice
extern crate::backend
  sync elastic(value:f64) -> f64

state
  expanded:animation[bool] = false
    easing ease-in-out
    duration 400ms
    delay 50ms
    repeat 1
    auto-reverse true
  progress:animation[f64] = 0.0
    easing elastic
    duration quick

on open
  expanded = true
  progress = 1.0
```

The built-in state types are `bool` and `f64`; a named extern type is also
accepted when its Rust type implements iced's animation bounds. Every native
easing variant is accepted in kebab case. A different easing name
must resolve to `sync name(value:f64) -> f64`. Durations and delays accept whole
`ms`/`s` values, including zero; duration presets are `very-quick`, `quick`,
`slow`, and `very-slow`. `repeat N` preserves iced's meaning (one repetition
plays twice), while `repeat forever` and `auto-reverse true` map directly to the
native builders. Ice subscribes to native window frames only while at least one
animation is active.

The expression language contains:

- literals: strings, booleans, `i64`, `f64`, `none`, list literals such as
  `[]` and `["List", "Board"]`, and hexadecimal `bytes(00 ff ...)`;
- paths: `state_name`, `parameter`, `item.field`;
- unary operators: `!`, `-`;
- arithmetic: `*`, `/`, `%`, `+`, `-`;
- comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`;
- boolean operators: `&&`, `||`;
- parentheses;
- built-ins: `len(list_or_str_or_bytes) -> i64`,
  `empty(list_or_str_or_bytes) -> bool`, `trim(str) -> str`, `some(T) -> T?`,
  `encoded(bytes) -> image`, `rgba(i64, i64, bytes) -> image`, and
  `aborted(task-handle?) -> bool`;
- namespaced keyboard built-ins such as `key.named("Enter")`,
  `key.code("KeyA")`, and `key.latin(logical, physical)`;
- native pointer built-ins such as `point(x, y)`, `mouse.button("left")`,
  `mouse.cursor(point)`, `mouse.click(point, button, previous)`, and
  `touch.finger("42")`;
- native geometry transformations such as `vector(x, y)`, `size(width, height)`,
  `point.distance(from, to)`, `rectangle.intersection(left, right)`,
  `transform.translate(x, y)`, `transform.compose(left, right)`, and
  `transform.point(point, transformation)`;
- native units such as `pixels(value)`, `padding.all(value)`, `degrees(value)`,
  and `radians(value)`;
- native rotation values with `rotation.default`, `floating`, `solid`, `from`,
  `with_radians`, and `apply`;
- native content fitting with all `fit.*` variants and `fit.apply`;
- native color values with `color.*` construction, conversion, parsing,
  mutation, luminance, contrast, and readability operations;
- native layout lengths with `length.*` variants, conversions, queries, and
  composition;
- native axis alignments with `alignment.*`, `horizontal.*`, and `vertical.*`;
- image allocation retention with `image.downgrade(allocation) -> image-memory`
  and `image.upgrade(memory) -> image-allocation?`;
- debug timing with `debug.active(span_state) -> bool` and
  `debug.time_with(name, value)`, preserving the value's checked type;
- animation queries `animation.value(state)`,
  `animation.animating(state[, at])`,
  `animation.interpolate(bool_state, start, end[, at])` for matching `f64` or
  `f64?` endpoints, and
  `animation.remaining(bool_state[, at])`; remaining time is returned in
  milliseconds;
- checked projection
  `animation.project(state, value, expression[, at])`, where the expression
  sees the current inner value as `value` and returns `f64` or `f64?`;
- `markdown(str) -> markdown` and `markdown_images(markdown) -> [str]`;
- calls to declared typed `sync` extern functions.

Store `encoded` and `rgba` handles in state so they are created when state
changes instead of on every view pass. Literal RGBA data is checked to contain
exactly `width × height × 4` bytes. Image widgets accept either a path string or
an `image` handle:

```ice
state
  logo = encoded(bytes(50 36 0a 31 20 31 0a 32 35 35 0a ff 00 ff))
  pixel = rgba(1, 1, bytes(ff 00 ff ff))

view
  image pixel crop=(0, 0, 1, 1)
```

Explicit allocation prevents the first-frame delay of lazily uploaded image
handles. Hold the returned allocation for as long as the guarantee is needed:

```ice
state
  handle:image = rgba(1, 1, bytes(ff 00 ff ff))
  allocation:image-allocation? = none
  failure:image-error? = none

on prepare
  task image allocate handle -> ready _ | failed _

on ready(value)
  allocation = some(value)

on failed(error)
  failure = some(error)
```

`image-allocation` exposes `.handle` and exact `.size:size-u32`; `size-u32`
exposes integer `.width` and `.height`. `image-error` preserves the native
value and exposes `.kind` (`invalid`, `inaccessible`, `unsupported`, `empty`,
or `out-of-memory`) plus its display `.message`. Downgrade/upgrade expose the
native weak-memory lifecycle. This task requires iced's `image` Cargo feature,
not only `image-without-codecs`.

Native debug spans have an explicit, ownership-safe state lifecycle:

```ice
state
  timer:debug-span? = none
  measured = 0

on begin
  debug start "interaction" -> timer

on finish
  debug finish timer

on compute
  measured = debug.time_with("compute", measured + 1)
```

`debug start` stores the exact `iced::debug::Span`; if the target already owns
a span, it is finished before replacement. `debug finish` takes and finishes
the span and is harmless when state is `none`. Because native spans are not
cloneable, `debug-span` is accepted only as `debug-span?` state and cannot cross
an extern, component, handler-message, collection, or ordinary assignment
boundary. Use `debug.active(state)` to read whether a span is present.
`debug.time_with` accepts a string name plus any non-span expression and returns
the expression's exact type. These calls always compile; iced's `debug` Cargo
feature activates reporting, while builds without it use iced's native no-op
spans.

There is no arbitrary Rust expression, method call, closure, general allocation
API, or implicit truthiness. New operations either belong in a small universal builtin
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
- assigning the inner value of `animation[T]` starts its native transition at
  the current monotonic instant; `state = value at instant` supplies an exact
  `instant` instead;
- `combo state push value` requires a `combo[T]` state and a `T` value;
- `return if` requires `bool`;
- `run`, `task`, `sip`, `flow`, or a task group must be the final statement because each
  returns one iced `Task`;
- fallible externs require both success and error routes;
- infallible externs permit only the success route;
- handler parameter types are inferred from every incoming route;
- incompatible incoming payloads are a type error;
- `_` means the payload produced by the current widget or action route.

`run` wraps an async Rust function with `Task::perform`. `task` directly maps a
Rust function that already returns an iced `Task`, which exposes clipboard,
window, focus, scroll, font, system, cancellation, batching, and other runtime
operations without duplicating their implementation in Ice.

Multiple tasks can be composed as one structured final statement:

```ice
on refresh
  parallel
    run load_tasks() -> tasks_loaded _ | failed _
    run load_profile() -> profile_loaded _ | failed _

on save_then_refresh
  sequential
    run save_draft() -> saved _ | failed _
    run load_tasks() -> tasks_loaded _ | failed _
```

`parallel` lowers to `Task::batch`; `sequential` lowers to repeated
`Task::chain`. Groups may nest and accept only task-producing statements,
including native clipboard, system, font, widget, window, and pane-query tasks.
Sequential construction reads handler inputs and state before either task
runs; it orders runtime task actions, not the later processing of their routed
messages. Use a result handler when the next task needs state produced by the
previous result.

Native task cancellation stores iced's own handle in optional UI state:

```ice
state
  request:task-handle? = none

on start
  abortable request abort-on-drop
    run load_tasks() -> tasks_loaded _ | failed _

on cancel
  abort request

view
  col
    if aborted(request)
      text "Canceled"
```

`abortable` accepts exactly one task-producing child, including a nested task
group, and must be the final handler statement. It lowers to `Task::abortable`
and stores the returned handle. Optional `abort-on-drop` applies iced's
`Handle::abort_on_drop`; replacing the state handle or assigning `none` then
cancels unfinished work when the last clone drops. `abort handle` calls
`Handle::abort` when present and intentionally keeps the handle so
`aborted(handle)` can report its status. A missing handle reports `false`.
Task handles are opaque and cannot be compared or used as lazy keys.

Native task streams route every yielded item through `Task::run`:

```ice
extern crate::backend
  AppError(message:str)
  stream progress(total:i64) -> i64
  stream checked_progress() -> i64 ! AppError

on start
  parallel
    stream progress(100) -> progressed _
    stream checked_progress() -> progressed _ | failed _
```

An infallible stream item becomes the success-route payload. A fallible stream
must yield `Result<T, E>` items and requires both success and error routes.
Stream statements are task-producing, so they work inside `parallel`,
`sequential`, and `abortable` blocks. Because the mapping closure runs once per
item, stream routes may pass one `_` or discard the item with a parameterless
route; they cannot capture other expressions. Read current UI state inside the
destination handler.

Native sippers keep repeated progress distinct from their single final output
and lower through `Task::sip`:

```ice
extern crate::backend
  AppError(message:str)
  sip download(url:str) progress=f64 -> bytes ! AppError

on start
  sip download(url)
    progress -> downloading _
    done -> downloaded _
    error -> failed _
```

The Rust factory returns
`impl iced::task::Straw<Vec<u8>, f64, AppError> + Send + 'static` (or
`Sipper<Output, Progress>` when infallible). `progress` and `done` are required;
`error` is required only for a fallible declaration. Each route may pass one
`_` or discard its payload, and may not capture other expressions. A sip is a
task-producing statement and can be nested in `parallel`, `sequential`, and
`abortable`. Consumers must enable iced's `sipper` Cargo feature.

Typed task flows keep domain output unwrapped until the final UI route, so
native task combinators can depend on earlier output:

```ice
on start
  flow
    from stream page_ids(4)
    then id -> task load_page(id)
    collect
    done -> pages_loaded _
    units -> work_planned _
```

`from` accepts an extern `run`, `task`, or `stream` source and the built-in
system, clipboard-read, font-load, and image-allocation tasks. It also accepts `done expr` and
`none Type`, which lower directly to `Task::done` and `Task::none`:

```ice
flow
  from done 7
  then value -> done value + 1
  done -> finished _

flow
  from none i64
  done -> finished _
```

`then name -> source` lowers to `Task::then` and binds each output only inside
the next source call. Use
`and-then` for `T?` output or a fallible task; fallible steps must keep the same
error type required by iced's `Result` overload. A transform cannot capture UI
state because the native closure is static; pass stable input to the first
source or read current state in the destination handler.

`map name -> expr` lowers to `Task::map` and replaces each output with the
expression value. It may read only its binding. On a fallible flow it maps the
successful value and preserves the error type; on an optional flow the binding
is the whole optional value, matching native `Task::map` exactly:

```ice
flow
  from task load_count()
  map count -> count + 1
  done -> loaded _
  error -> failed _
```

`map-error error -> expr` lowers to `Task::map_err`, may read only its error
binding, and replaces the flow's error type with the expression type. A sync
extern is the normal way to translate one domain error into another:

```ice
flow
  from task request()
  map-error reason -> normalize_error(reason)
  collect
  done -> collected _
```

`collect` lowers to `Task::collect`. It changes an infallible `T` into `[T]`
and a fallible `T ! E` into `[result[T,E]]`, preserving each failure as data
and making the collected flow itself infallible.
`discard` must be last, suppresses both output routes, and lowers to
`Task::discard`. `units -> handler _` reads native `Task::units` during flow
construction and emits an `i64` notification alongside the task. Non-discarded
flows require `done`; fallible flows also require `error`. All three routes may
pass one `_` or discard their payload. Flows are task-producing and work inside
task groups and `abortable`.

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
| `scroll` | one content child; complete direction/scrollbar/builders, every viewport getter and status selector, every concrete Style field, and typed native runtime style callbacks |
| `grid` | responsive children with pixel width/spacing, fixed columns or fluid max-cell width, and aspect-ratio or evenly distributed `Length` height |
| `stack` | overlays children with typed width/height, optional clipping and `under=N` intrinsic-base control |
| `container` | exactly one child with ID, all length bounds, max bounds, per-axis alignment, clipping, per-side padding, every concrete surface style field including linear backgrounds, and typed native runtime style callbacks |
| `overlay` | named `content` and `layer` trees with checked visibility, alignment, padding, backdrop and optional dismissal |
| `text` | one `str`, `i64`, or `f64` expression with bounds, size/line-height, font, alignment, shaping, wrapping and checked color/weight styles |
| `rich-text` | zero or more structured spans with rich defaults, complete span highlights and optional string link events |
| `pane-grid` | named pane trees backed by recursive persistent split state, structured title/full/compact controls, complete concrete state and surface styles with linear backgrounds, closed panes, list-keyed runtime templates, typed dynamic references, click, resize and drag/drop behavior |
| `input` | required `str` binding; ID, hint, disabled/secure, submit/paste, every concrete builder setter, complete icon, all concrete status style fields, and typed native runtime style callbacks |
| `button` | string label or one child; optional ID/disabled, typed size/padding/clip, eight presets, complete status styles, typed native runtime style callbacks and required route |
| `checkbox` | string label, bool value/route, disabled, sizing/typography/wrapping/font, custom icon, four presets and complete checked-aware status styles |
| `toggler` | string label, bool value/route, disabled, sizing/typography/wrapping/font/alignment and complete checked-aware status styles |
| `slider` | `f64` or typed extern numeric value/range/default/normal+shift steps, direction-aware sizing, change/release routes and nested status styles |
| `progress` | `f64` value/range, all length/girth variants, vertical axis, five presets, complete concrete style overrides and typed native runtime style callbacks |
| `radio` | string label, bool/i64/f64/str/extern value route, bool selection, complete sizing/typography/font and selected-aware status styles |
| `pick` | `[T]` options, `T?` selection, complete typography/handle/status/menu configuration, typed native field/menu style callbacks and `T`-payload route |
| `combo` | searchable/replaced/incrementally pushed `combo[T]` state, `T?` selection, complete typography/icon/input/menu styles, typed native input/menu style callbacks and all routes |
| `float` | one child with positive scale, bounds/viewport-aware x/y translation, shadow and per-corner shadow radius |
| `pin` | one child with typed width/height and fixed x/y position |
| `sensor` | one child with show/resize `(width, height)`, hide, key, anticipation and delay |
| `responsive` | breakpoint sugar or one arbitrary size-dependent child tree with scoped width/height bindings and typed bounds |
| `rule` | horizontal/vertical separator with non-negative thickness, all fill modes, default/weak preset, color, corner radii and snap |
| `qr` | named text/binary QR data with correction/version, cell/total sizing and checked colors |
| `space` | optional fixed/fill/fill-portion/shrink width and height |
| `image` | raster path or encoded/RGBA handle with every concrete sizing/fit/filter/floating-or-solid rotation/opacity/scale/expand/per-corner-radius/crop property |
| `viewer` | interactive image zoom/pan with path/handle sources and complete sizing/fit/filter/padding/scale configuration |
| `svg` | SVG path or UTF-8/raw-byte memory expression with typed layout, idle/hover color properties, and a typed native runtime style callback |
| `tooltip` | exactly two children (content then tip), full positioning/timing, every concrete container style field, and typed native runtime style callbacks |
| `mouse` | one child; all button/enter/move/scroll/exit events and every iced cursor interaction |
| `canvas` | declarative native geometry, raster/SVG drawing, path building, transforms, clipping, typed control flow, grouped dependency caches and pointer events |
| `theme` | one child with default/app/all built-in iced themes and checked text color plus solid/linear background |
| `if` | includes its children when a bool expression is true |
| `for` | iterates a list and adds one typed item binding |
| `keyed` | repeats one child template with a bool/i64/f64 identity key and native column sizing/alignment |
| `lazy` | caches one owned static child subtree by a checked hashable dependency |
| `markdown` | renders owned parsed/replaced/appended content, exposes image URIs, all Settings and Style fields, str link events, and typed custom Viewer factories |
| `table` | maps typed rows into arbitrary structured headers/cells with complete sizing, padding, separator and alignment options |
| `editor` | binds owned multi-line content to generated iced actions with sizing, typography, wrapping, built-in highlighting and every concrete status style field |

`if` and `for` are child control-flow nodes inside a layout. There is no virtual
DOM or runtime reconciliation layer; the iced backend constructs the current
element tree from state.

Grid `columns=` and `fluid=` are mutually exclusive. `columns=` is a positive
`i64`; `fluid=` and both dimensions of `height=aspect(W,H)` are positive `f64`
values. `width=` and `spacing=` are non-negative `f64` pixels. A non-aspect
`height=` accepts `fill`, `fill(N)`, `shrink`, or a non-negative `f64` pixel
expression and maps to iced's evenly distributed sizing.

`container` is the explicit one-child wrapper used to size, align, clip, pad,
and style an arbitrary structured child tree. It accepts the shared surface
properties used by pane content and title bars: solid or linear background,
text, border with per-corner radius, shadow offset/blur, and pixel snapping.
Typed properties override any equivalent `@` utility on the same node:

```ice
container #card width=fill max-width=640.0 align-x=center padding=12.0 background=linear(1.57, surface@0.0, background@1.0) shadow=black/50 shadow-y=2.0 shadow-blur=8.0 pixel-snap=true @bg-surface rounded-lg
  TaskRow task=task loading=loading
```

`style=summary_container(loading)` may call a declared `container-style`. Its
Rust function receives `&iced::Theme`, then its owned arguments, and returns
`container::Style`. Utilities and typed properties override that returned base.

An SVG accepts `style=status_svg(loading)` after a matching `svg-style`
declaration. The Rust function receives `&iced::Theme`, `svg::Status`, then its
owned arguments and returns `svg::Style`. Explicit `color=` and `hover=`
properties override the callback result for their respective statuses.

An `overlay` keeps the two trees explicit instead of relying on child order.
When its bool condition is true, `layer` floats over `content`; the backdrop
blocks button and scroll input and an optional `dismiss=` route handles a left
click outside the layer. Pointer events inside the layer do not dismiss it:

```ice
overlay when=about_open dismiss=close_about backdrop=black/60 padding=24.0
  content
    Dashboard
  layer
    AboutDialog
```

Advanced overlays stay behind the existing typed component boundary instead
of duplicating the `Overlay` trait in Ice:

```ice
extern crate::backend
  component native_overlay(index:f64) -> unit

view
  extern native_overlay(42.0)
```

The Rust `Element` may contain a custom `Widget::overlay` implementation. Rust
therefore retains the complete native `Overlay` contract: layout, draw,
operate, update, mouse interaction, nested overlays, and `index()` ordering.
The generated component probe verifies the owned Element ABI; a non-unit
overlay event uses the same checked `-> handler _` mapping as any extern
component.

Rich text uses structured `span` children with `str`, `i64`, `f64`, or bool
expressions, so mixed formatting and links remain
readable without embedding markup in a string. A route is required exactly when
at least one span has a string `link=`:

```ice
rich-text width=fill wrapping=word @text-sm text-muted -> open_link _
  span "Read the "
  span "Ice guide" link="https://example.com" underline @font-bold text-primary
  span "."
```

Rich defaults cover size, relative or absolute line height, font, bounds,
alignment, wrapping and color. A span can override size, line height, font and
color; attach a string link; use a solid or linear highlight background with
complete border/radius/padding; and toggle underline or strikethrough.

A pane grid owns persistent iced layout state generated from its required static
ID. Static names and checked `template(key)` references are the identities
exposed to Ice; native pane/split IDs stay inside generated Rust. `resize=` is
grab leeway and enables automatic ratio
updates, while `drag` automatically applies successful drop targets. The old
two-pane shorthand remains valid:

```ice
pane-grid #workspace split=vertical ratio=0.7 width=fill height=fill spacing=8.0 min-size=120.0 resize=6.0 drag click=pane_clicked(_)
  pane files
    FileList
  pane editor
    Editor
```

For an arbitrary initial layout, nest binary split nodes. A root-level
`pane name closed` declares checked content without opening it:

```ice
pane-grid #workspace width=fill height=fill
  split workspace_root vertical ratio=0.7
    pane files
      FileList
    split editor_stack horizontal ratio=0.6
      pane editor
        Editor
      pane terminal
        Terminal
  pane preview closed
    Preview
```

A runtime pane template repeats one checked pane body over list state. It is
initially closed; handlers open and target any item through the template's
bool, i64, f64, or str key:

```ice
state
  documents:[Document] = []
  selected_id = 42

on open_document
  pane #workspace split editor document(selected_id) horizontal ratio=0.4

on close_document(id)
  pane #workspace close document(id)

view
  pane-grid #workspace
    pane editor
      EditorHome
    pane document in documents by=document.id maximized=is_maximized
      title
        text document.title
      controls
        button "Close" -> close_document document.id
      content
        col
          if is_maximized
            text "Focused editor"
          DocumentEditor document=document
```

The list must be app state so the generated pane body can safely borrow its
current item. If an open key is no longer present, the pane renders a readable
missing-data placeholder until it is closed or the item returns. Opening the
same `template(key)` twice is a no-op. Optional `maximized=name` binds iced's
per-pane maximized callback flag as a checked bool inside that pane's title,
controls, content, styles, and scoped IDs; it works on static, closed, and
runtime panes.

A pane may expose iced's native `Content`, `TitleBar`, and `Controls`
structure directly. `compact-controls` is the fallback used when the full
controls would overlap the title. `always-controls` disables the default
hover-only visibility, and title padding accepts the same per-side precedence
as containers:

```ice
pane-grid #workspace split=vertical resize=8.0 drag
  style
    hovered-region background=linear(0.785, primary/10@0.0, primary/40@1.0) border=primary border-width=2.0 radius=8.0
    hovered-split color=primary width=3.0
    picked-split color=foreground width=3.0
  pane files background=linear(1.57, surface@0.0, background@1.0) shadow=black/50 shadow-y=2.0 shadow-blur=8.0 pixel-snap=true @bg-surface border border-border rounded-lg
    title padding=8.0 padding-x=12.0 always-controls background=background border=border border-width=1.0 radius-tl=8.0 radius-tr=8.0 @bg-background
      text "Files" @font-bold
    controls
      row @gap-2
        button "Refresh" -> refresh
        button "Close" -> close_files
    compact-controls
      button "…" -> open_file_menu
    content
      FileList
  pane editor
    Editor
```

The legacy one-child form remains identical. Structured panes require exactly
one `content` section; `controls` require `title`, and `compact-controls`
require full `controls`. Pane and title typed surface properties cover every
concrete `container::Style` field: solid or linear background, text, border
with per-corner radius, shadow offset/blur, and pixel snapping. Their checked
`@` utilities remain a concise base and typed properties override them; layout
stays explicit in child nodes. Linear angles are radians, offsets are checked
in `0.0..=1.0`, and iced's maximum of eight color stops is enforced.

The optional first `style` child maps directly to iced's complete concrete
`pane_grid::Style`: hovered region solid or linear background and border
(including every corner radius), plus hovered and picked split line colors and
widths. Omitted fields retain `pane_grid::default(theme)`. Background parsing
is shared with pane surfaces instead of being a pane-grid-only special case.
A declared `pane-grid-style` call can provide the native runtime base instead;
the structured child still applies checked field overrides after that callback:

```ice
pane-grid #workspace split=vertical style=workspace_panes(loading)
  style
    picked-split width=4.0
  pane files
    FileList
  pane editor
    Editor
```

Pane grids may only live in the app view because component/repeated instances
need separately keyed persistent state. Click routes receive a stable `str`:
the static name or `template(key)` for a runtime pane.

Canvas is a checked declarative layer over iced's native `Canvas`, `Program`,
`Frame`, `Path`, and `Cache`. Its body is drawing code, not a widget subtree:

```ice
canvas width=fill height=220.0 cache=chart_version cache-group=charts capture=true cursor=(cursor_state) cursor-outside=true
  state
    cursor_state = "crosshair"
    drag_count = 0
    drag_x = 0.0
    drag_y = 0.0
  event mouse pressed as button
    set cursor_state = "grabbing"
    set drag_count = drag_count + 1
    emit chart_button button
    capture
  event mouse released as button
    set cursor_state = "crosshair"
    redraw
    capture
  event keyboard press -> chart_key _
  capture touch lost
  redraw window frame after=16ms
  rect x=0.0 y=0.0 width=canvas_width height=canvas_height fill=background
  circle x=64.0 y=64.0 radius=28.0 fill=primary stroke=foreground stroke-width=2.0
  path fill=primary/25 stroke=primary stroke-width=2.0 cap=round join=round
    move x=96.0 y=160.0
    bezier ax=140.0 ay=20.0 bx=180.0 by=200.0 x=240.0 y=80.0
    line x=240.0 y=160.0
    close
  text "Drag me" x=16.0 y=196.0 color=foreground size=14.0
  image logo x=264.0 y=16.0 width=48.0 height=48.0 filter=nearest opacity=0.9 snap=true radius=6.0
  svg "icon.svg" x=320.0 y=16.0 width=48.0 height=48.0 color=primary rotation=0.1 opacity=0.9
```

`canvas_width` and `canvas_height` are scoped `f64` bindings containing the
current frame dimensions. A single optional `state` block declares typed,
per-canvas `Program::State`; its initializers are self-contained and cannot
capture app state or component parameters. Commands accept app state, canvas
state, and frame dimensions in expressions. Nested `if` and `for` commands draw
conditional or repeated geometry. `group` applies translation, rotation,
uniform/non-uniform scale and an optional `(x, y, width, height)` clip while
restoring the previous transform after its body.

`rect`, `circle`, and `path` accept a checked solid or `linear(...)` fill,
`non-zero` or `even-odd` fill rule, and an optional stroke. Strokes expose
width, butt/square/round caps, miter/round/bevel joins, dash segments and dash
offset. Path bodies map directly to move, line, arc, arc-to, ellipse, cubic
Bézier, quadratic, rectangle, per-corner rounded rectangle, circle, and close
builder calls. Canvas text accepts string/numeric content, position, maximum
width, color, size, relative/absolute line height, font, alignment and shaping.

`image source` draws either a path string or an `image` handle produced by
`encoded(...)`/`rgba(...)`. It requires `x`, `y`, `width`, and `height`, and
exposes every concrete `iced::Image` field: linear/nearest filtering, rotation,
opacity, pixel snapping, and per-corner radius. `svg source` has the same bounds,
rotation, and opacity contract plus an optional checked color. Add the bare
`memory` flag to accept UTF-8 SVG text or raw bytes instead of a path. Both
commands draw inside the current frame transform and clip.

`cache=dependency` uses iced's geometry cache and clears it when the checked
hashable app dependency changes, the bounds change, or a canvas event updates
local state. Include every app state value that affects drawing in that
dependency; omit `cache=` for always-fresh geometry. `cache-group=name`
requires `cache=` and gives every canvas carrying
the same static name a shared native `canvas::Group`; this maps directly to
iced's grouped cache storage without changing invalidation keys. Pointer
press/release variants and move emit local `(x, y)` values;
scroll emits `(x, y, pixels)`. `enter`/`exit` have no payload. `capture=true`
marks emitted pointer events captured. Native consumers must enable iced's
`canvas` Cargo feature.

Canvas event directives expose the complete native `Program::update` event and
action surface without turning drawing commands into handlers:

```ice
event keyboard press -> chart_key _
event input-method preedit -> composing _ _ _
event mouse wheel -> wheel _ _ _
event touch moved -> touched _ _ _
event window resized -> resized _ _
capture window close-request
redraw window frame
redraw window frame after=16ms

event mouse moved as x, y
  set drag_x = x
  set drag_y = y
  redraw
  capture

event mouse released as button
  set cursor_state = "grab"
  emit released button
  capture
```

`event` accepts every input-method, keyboard, mouse, touch, and window variant
listed by `subscribe` and uses the same checked payload types. Mouse event
coordinates are raw window coordinates; the compact `move=` canvas property
continues to emit local coordinates. `capture source` returns iced's
capture-only action. `redraw source` requests the next frame, while `after=ms`
or `after=s` calls `request_redraw_at` relative to the current instant. A routed
event publishes a message and therefore already redraws. A structured event
uses `as` to name its typed payload, then may update local state with `set`,
choose one explicit `emit` or immediate/timed `redraw`, and optionally
`capture`. Publishing already requests a redraw. `emit` uses the named values
instead of `_`. `cursor=(expression)`
derives the complete iced mouse interaction from canvas state, while
`cursor-outside=true` lets that interaction remain active outside the bounds;
unknown runtime strings safely use the default interaction. `capture=true`
also marks routed and redraw actions captured. Event sources must be unique
within a canvas and these directives are allowed only at its root, not inside
drawing groups or control flow.

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

A component may declare required slots. Bare `slot` is the conventional
`children` slot and receives one structured child tree at its call site:

```ice
component Panel(title:str)
  col @p-4 bg-surface rounded-lg
    text title @font-bold
    slot

Panel title="Tasks" #tasks-panel
  scroll height=fill
    col
      for task in tasks
        TaskRow task=task loading=loading #task(task.id)
```

A component call may use checked named props in any order, as above. The older
positional form `Panel("Tasks")` remains accepted for compatibility. Unknown,
missing, duplicate, and incorrectly typed props are compile-time errors.

For React-like compound structure, name the slots in the component and fill
them with readable `name:` blocks at the call site:

```ice
component Dialog()
  col @p-6 bg-surface rounded-lg
    slot header
    slot body
    slot actions

Dialog
  header:
    text "Delete task?" @font-bold
  body:
    text "This cannot be undone."
  actions:
    row @gap-2
      button "Cancel" -> cancel
      button "Delete" -> delete
```

Qualified component names provide a React-style compound form without the
extra `name:` layer. A direct `Dialog.Name` child fills the matching `Name`
slot while remaining a normal checked component call:

```ice
component Dialog()
  col @p-6 bg-surface rounded-lg
    slot Header
    slot Body
    slot Actions

component Dialog.Header()
  row
    slot

component Dialog.Body()
  container width=fill
    slot

component Dialog.Actions()
  row @gap-2
    slot

Dialog
  Dialog.Header
    text "Delete task?" @font-bold
  Dialog.Body
    text "This cannot be undone."
  Dialog.Actions
    row @gap-2
      button "Cancel" -> cancel
      button "Delete" -> delete
```

All direct children in compound form must be immediate qualified children of
the parent call. Mixing a `Dialog.Header` child with an unrelated direct child
is a compile-time error. Explicit `header:` blocks remain useful when the slot
content should not have its own component styling or behavior.

Every declared slot is required and accepts exactly one root. Wrap sibling
nodes in `row`, `col`, `grid`, or `stack`. Unknown, missing, and duplicate slot
names are compile-time errors. A component can forward a named slot through
another component by placing `slot name` inside the corresponding `name:`
block.

A component without slots rejects child content. Slot content keeps the
caller's state, loop bindings, handlers, and IDs while rendering under the
component instance scope.

### Extern components and subscriptions

An extern component is a typed Rust `Element` adapter with owned or borrowed
parameters:

```ice
extern native_help(external_hover) -> external_hover_changed _
extern borrowed_help(draft, external_hover) -> external_hover_changed _
```

Its arguments and emitted payload are checked against the declaration. A
non-`unit` output requires a route. A `unit` component may omit the route; its
messages are mapped to an internal no-op. Extern components own their styling,
so `@` utilities and `#` IDs are not accepted on the call.

Subscriptions are declared separately from activation:

```ice
subscribe
  every 500ms when auto_refresh -> tick
  input-method preedit -> composing _ _ _
  app_events() -> external_event _
  keyboard press -> key_pressed _
  keyboard release -> key_released _
  keyboard modifiers -> key_modifiers_changed _
  mouse moved -> pointer_moved _ _
  mouse wheel -> wheel_scrolled _ _ _
  touch pressed -> finger_pressed _ _ _
```

The compiler batches active subscriptions and wires the application builder.
Subscription routes accept only `_`; handlers can read current state after the
event arrives. This prevents generated `'static` subscription closures from
capturing a borrowed application state.

Any source may have a boolean activation condition between the source and
route:

```ice
subscribe
  every 2s when auto_refresh && online -> refresh
  keyboard press when shortcuts_enabled -> key_pressed _
```

The condition is type-checked and evaluated from current app state whenever
iced rebuilds subscriptions. False returns `Subscription::none()`, so an
inactive timer or external stream is actually stopped instead of merely
dropping its messages.

Runtime event sources may also filter iced's dispatch status:

```ice
subscribe
  event status=any -> runtime_event _
  event with-id status=ignored -> window_event _ _
  mouse moved status=captured -> drag_moved _ _
  keyboard press status=ignored when shortcuts_enabled -> key_pressed _
  window close-request status=any -> close_requested
```

`captured` means a widget handled the event; `ignored` means none did; `any`
accepts both. The modifier is available on generic, input-method, keyboard,
mouse, touch, and non-frame window events. Timers, system/extern subscriptions,
and raw window frames have no iced event status and reject it. For compatibility,
an omitted `event` or keyboard status keeps iced's ignored-only listener while
the other direct input sources keep their previous any-status behavior.

`event` carries the complete native `iced::Event` value across handlers and
typed extern functions. Plain `event` lowers to `event::listen`; adding
`status=` or `with-id` lowers to `event::listen_with`. `event raw` lowers to
`event::listen_raw`, includes redraw requests, and therefore must be filtered
or routed without causing another redraw; an unfiltered raw listener can loop
forever. `with-id` prepends the originating `window-id`. The raw source defaults
to any status, while a non-raw `with-id` source without `status=` preserves
`listen` semantics by accepting ignored events only. Runtime system-theme
changes are not `iced::Event` values and remain available through
`system theme`.

Generic events support the same transforms as every native source:

```ice
extern crate::backend
  sync label_event(value:event) -> str?

subscribe
  event filter=label_event status=any -> labeled _
  event raw with-id status=captured -> captured _ _
```

Every native or extern source also supports iced's identity and output
transforms:

```ice
extern crate::backend
  sync visible_pointer(x:f64, y:f64) -> str?

state
  generation = 7

on pointer_moved(generation, position)

subscribe
  mouse moved with=generation filter=visible_pointer -> pointer_moved _ _
```

`filter=` names a declared `sync` function and lowers to
`Subscription::filter_map`. Its parameters exactly match the source payloads:
no parameters for a payload-free event, one for a scalar source, and one per
field for a multi-payload native event. It must return `T?`; after filtering,
the route has one `T` payload. The generated closure captures nothing.

`with=` lowers to `Subscription::with`, participates in the subscription's
iced identity, and becomes the first route payload. It accepts Ice values with
a statically hashable type; extern value types must additionally implement
Rust's `Hash + Clone + Send + Sync + 'static` contract. Filtering happens
before context is attached. Both modifiers are optional and routes may omit
all `_` placeholders when their values are intentionally discarded.

Declared `stream` externs can also become long-lived subscriptions:

```ice
extern crate::backend
  stream worker() -> str
  stream room_events(room:i64, generation:i64) -> str

subscribe
  run worker() -> received _
  run room_events(room, generation) -> received _
```

A zero-argument `run` passes the Rust function item to
`Subscription::run`. One or more arguments lower to `Subscription::run_with`;
their value, or ordered tuple of values, is the subscription identity data and
the generated noncapturing builder clones it into the declared stream
function. Every data argument must be hashable. A fallible declaration
`stream ... -> T ! E` emits one `result[T,E]` payload so failures remain stream
values. These sources may use the same `with=`, `filter=`, and `when` modifiers
as every other subscription.

Custom iced recipes use the same checked source shape:

```ice
extern crate::backend
  recipe counter(id:i64) -> i64

subscribe
  recipe counter(generation) -> counted _
```

The factory arguments are checked against Rust, its concrete return type must
implement `iced::advanced::subscription::Recipe<Output = T>`, and the source
lowers directly to `advanced::subscription::from_recipe`. Ice owns only the
route and optional transforms; Rust retains the recipe's identity hashing,
runtime-event input, stream, and cancellation behavior.

Raw runtime events can be narrowed by a typed Rust filter without exposing
iced's large platform event enum in Ice:

```ice
extern crate::backend
  event-filter runtime_event() -> str

state
  event_identity = 1

subscribe
  events event_identity using=runtime_event -> received _
```

The identity expression must be hashable. The declared function takes no Ice
parameters because Rust receives one implicit
`iced::advanced::subscription::Event` and returns `Option<T>`. Generated code
uses a native `Recipe` over iced's `EventStream`, hashes both the filter type
and identity, and forwards only `Some(T)`. This exposes interaction window IDs,
dispatch status, all runtime event variants, and system-theme changes while
preserving native cancellation.

Together, the declarative sources and typed adapters cover the complete
application-facing `iced::Subscription` construction surface: `none`, `batch`,
`run`, `run_with`, `from_recipe`, `with`, `map`, `filter_map`, and `units`.
`iced::advanced::subscription::into_recipes` is the runtime consumer that
extracts boxed recipes after an application has built a subscription; it does
not create or transform an application subscription, so it intentionally has
no source-language form.

Ice covers all three public iced time operations with its native monotonic
`instant` type:

```ice
extern crate::backend
  refresh_status() -> i64

state
  last:instant? = none

on read_time
  task time now -> tick _

on tick(now)
  last = some(now)

subscribe
  every 500ms -> tick _
  repeat refresh_status() every 2s -> refreshed _
```

`task time now` lowers to `iced::time::now`. `every` forwards each native
`Instant`; the route may omit `_` when it does not need the tick value.
`repeat` accepts a declared zero-argument async extern and lowers its function
item directly to `iced::time::repeat`. A fallible extern produces
`result[T,E]` values instead of splitting the subscription into success/error
routes. Durations must be positive whole numbers using `ms` or `s`. `every`
requires iced's `tokio` or `smol` Cargo feature; `repeat` requires `tokio`,
which the reference app uses.

Native keyboard subscriptions infer structured payloads. Press events expose
`key:key`, `modified_key:key`, `physical_key:physical-key`,
`location:key-location`, `modifiers:key-modifiers`, optional `text`, and
`repeat`; release events expose the same fields except `text` and `repeat`.
These four public types are the exact native iced values, so state and typed
extern functions can preserve them without string conversion.

```ice
state
  shortcut:key = key.named("Enter")
  scan:physical-key = key.code("Enter")
  location:key-location = key.location("standard")
  modifiers:key-modifiers = key.modifiers(false, true, false, false)
  latin:str? = none

on pressed(event)
  latin = key.latin(event.key, event.physical_key)
  shortcut = event.key
```

`key.named("Variant")` and `key.code("Variant")` accept exact iced Rust enum
variant names and lower directly, covering every current named and physical
code without a second alias catalog; rustc reports an unknown variant during
`cargo ice check`. `key.character(str)` and `key.unidentified()` construct the
other logical variants. `key.native_unidentified()` and literal
`key.native("android" | "macos" | "windows" | "xkb", code)` construct every
native physical variant with checked integer ranges. For runtime integers,
`key.try_native(platform, code) -> physical-key?` returns none on overflow.

`key.location("standard" | "left" | "right" | "numpad")` covers every
location. `key.modifiers(shift, control, alt, logo)` constructs any flag set;
`key.command_modifiers()` preserves iced's platform-dependent `COMMAND`
constant. Modifier values expose `shift`, `control`, `alt`, `logo`, `command`,
`jump`, and `macos_command` booleans. Logical keys expose `kind`, optional
`named`, and optional `character`; physical keys expose `kind`, optional
`code`, `native_platform`, and `native_code`; locations expose `name`.
Equality compares the native typed values, and `key.latin` delegates to iced's
native locale-aware physical-key translation. Like `iced::keyboard::listen`,
these subscriptions receive keyboard events that no widget captured.

Pointer values also preserve iced's native types across state, handlers, and
typed extern functions:

```ice
state
  position:point = point(12.0, 24.0)
  offset:vector = vector(10.0, 20.0)
  extent:size = size(100.0, 80.0)
  bounds:rectangle = rectangle(0.0, 0.0, 100.0, 80.0)
  transform:transformation = transform.compose(transform.translate(10.0, 20.0), transform.scale(2.0))
  button:mouse-button = mouse.button("left")
  cursor:mouse-cursor = mouse.cursor(point(12.0, 24.0))
  click:mouse-click = mouse.click(point(12.0, 24.0), mouse.button("left"), none)
  finger:touch-finger = touch.finger("18446744073709551615")
```

`mouse.button` accepts `left`, `right`, `middle`, `back`, and `forward`.
`mouse.other_button` accepts a checked literal `u16` value;
`mouse.try_other_button(i64) -> mouse-button?` safely handles runtime values.
`touch.finger` accepts a checked decimal `u64` string, preserving the full
native identifier without unsigned Ice arithmetic, while
`touch.try_finger(str) -> touch-finger?` parses runtime input safely.

`mouse.cursor(point)`, `mouse.levitating(point)`, and `mouse.unavailable()`
construct all cursor variants. `mouse.cursor_position`, `cursor_over`,
`cursor_in`, and `cursor_from` expose iced's optional coordinate queries;
`cursor_is_over`, `cursor_is_levitating`, `cursor_levitate`, `cursor_land`, and
`cursor_translate` expose its variant and vector-translation behavior.
`mouse.click` creates a native click from a point, button, and optional previous
click. Point, vector, size, and rectangle coordinates are `f64` in Ice and
lower to iced's `f32` geometry.

Fields are checked: points and vectors expose `x/y` plus lossless two-value
`values`; points also expose native `display`; sizes expose `width/height` plus
`values`; rectangles expose `x/y/width/height`, `center`, `center_x`,
`center_y`, `position`, `size`, and `area`; buttons expose `kind`
and optional `number`; cursors expose `kind`, optional `position`, and
`levitating`; clicks expose `kind` and `position`; fingers expose their
lossless decimal `id`.

Native units remain first-class iced values instead of becoming untyped
numbers:

```ice
state
  gap:pixels = pixels(8.0)
  inset:padding = padding(4.0, 8.0, 12.0, 16.0)
  quarter:degrees = degrees(45.0) * 2.0
  rotation:radians = radians.from_degrees(quarter)

on inspect
  inset = padding.fit(inset, size(80.0, 40.0), size(96.0, 56.0))
  rotation = (rotation + radians.pi()) % radians(6.0)
```

`pixels(value)` and `pixels.zero()` construct native `Pixels`;
`pixels.from_u32(literal)` checks the full native range and
`pixels.try_from_u32(i64) -> pixels?` safely converts runtime integers. Pixels
expose their `value`, native equality/order, and every native `+`, `*`, and `/`
combination with another pixels value or an Ice `f64`. The native `u32`
division form is represented by the same checked f64 scalar operation.

`padding(top, right, bottom, left)`, `padding.zero`, `all`, `top`, `right`,
`bottom`, `left`, `horizontal`, `vertical`, and `axes(vertical, horizontal)`
cover every native constructor and scalar/axis conversion.
`padding.from_pixels` preserves the exact Pixels conversion. `with_top`,
`with_right`, `with_bottom`, `with_left`, `with_horizontal`, and
`with_vertical` call the native builder methods and accept either f64 or
pixels. Padding exposes each side plus computed `x/y`; `padding.fit` delegates
to iced's size-constrained fit. `size.from_padding`,
`rectangle.expand_padding`, and `rectangle.shrink_padding` preserve the native
conversion and geometry behavior. Padding values support equality and typed
extern passage.

`degrees(value)` and `radians(value)` narrow numeric construction to native
f32. Both expose their lossless Ice-f64 `value`, equality/order against their
own type, and iced's angle-left comparison against f64. Degrees support native
f64 multiplication. Radians support same-type `+`, `-`, `*`, `/`, `%`, f64
scaling in either native direction, and native addition of Degrees.
`radians.from_degrees` performs iced's exact conversion; `radians.pi` exposes
the native constant and `display` uses native formatting.

`degrees.range_start/end/in_range` and `radians.range_start/end/in_range`
expose the full native `RangeInclusive` behavior without adding a speculative
generic range type. `radians.distance_start/end` expose both points returned by
native `to_distance`. Size and rectangle rotation accept either the existing
f64 radians or a first-class radians value; `rectangle.vertices_angle` keeps
the exact native radians result alongside the compatible f64
`vertices_rotation` projection.

`rotation.default()` and `rotation.from(f64)` preserve iced's floating default
and scalar conversion. `rotation.floating(radians)` and
`rotation.solid(radians)` construct both native variants;
`rotation.with_radians(value, radians)` updates the angle through the native
`radians_mut` method and returns the value. A rotation exposes checked
`.radians`, `.degrees`, and `.kind` (`floating` or `solid`) projections, supports
native equality and typed extern passage, and `rotation.apply(value, size)`
returns iced's exact minimum layout size. Image and SVG `rotation=` properties
accept this first-class value directly alongside the compact numeric syntax.

`fit.default()` and `fit.contain()` produce iced's default `Contain` strategy;
`fit.cover`, `fit.fill`, `fit.none`, and `fit.scale_down` construct every other
native variant. A content-fit value exposes `.kind` with the compact kebab name
and `.display` through iced's native formatter, supports equality, lazy hashing,
and typed extern passage, and `fit.apply(value, content_size, bounds_size)` calls
the exact native sizing algorithm. Image, SVG, and Viewer `fit=` properties
accept the first-class value directly; their existing compact names remain
equivalent sugar.

`color.default()`, `color.black()`, `color.white()`, and `color.transparent()`
produce the native default and constants. `color.rgb`, `color.rgba`,
`color.rgb8`, `color.rgba8`, and `color.linear_rgba` call the corresponding
native constructors; the three 8-bit channels are checked integer literals in
`0..=255`. `color.try_rgb8(i64, i64, i64)` and
`color.try_rgba8(i64, i64, i64, f64)` accept dynamic channels and return `none`
instead of wrapping an out-of-range value. `color.from3` and `color.from4`
preserve iced's array conversions.
`color.parse(str) -> color?` accepts every native 3/4/6/8-digit RGB hexadecimal
form and maps its native parse error to `none`.

A color exposes `.r`, `.g`, `.b`, `.a`, `.rgba8`, `.linear`, `.luminance`, and
`.display`. `color.inverse`, `color.invert`, and `color.scale_alpha` preserve the
native value and in-place APIs while returning the resulting color;
`color.luminance`, `color.contrast`, and `color.readable(foreground, background)`
call iced's exact WCAG calculations. Colors support equality and typed extern
passage. They are deliberately rejected as lazy identities because native
`Color` contains floating-point channels and does not implement `Hash`.

`length.fill()`, `length.fill_portion(u16 literal)`, `length.shrink()`, and
`length.fixed(f64)` construct every native variant. Dynamic `i64` portions use
`length.try_fill_portion(value) -> length?`, which returns `none` outside the
native `u16` range. `length.from_f64`, `length.from_pixels`, and
`length.from_u32` call all three native conversions; dynamic unsigned units use
`length.try_from_u32(value) -> length?` without wrapping.

A length exposes `.fill_factor`, `.is_fill`, `.kind`, optional `.portion`, and
optional `.fixed` projections. `length.fluid(value)` and
`length.enclose(value, other)` call the exact native layout methods. Lengths
support equality and typed extern passage, and may be used directly in every
view property whose iced builder accepts `Length`; the compact `fill`,
`fill(N)`, `shrink`, and numeric spellings remain equivalent sugar. Native
floating fixed lengths do not implement `Hash`, so lengths are rejected as lazy
identities.

`alignment.start/center/end`, `horizontal.left/center/right`, and
`vertical.top/center/bottom` construct every variant of iced's three alignment
enums. `alignment.from_horizontal`, `alignment.from_vertical`,
`horizontal.from_alignment`, and `vertical.from_alignment` preserve all native
conversions. Each value exposes a compact `.kind`, supports equality, hashable
lazy identity, and typed extern passage. Existing view properties keep their
short `start/center/end`, `left/center/right`, and `top/center/bottom` sugar.

The default iced `f32` geometry API has direct checked expressions:

```ice
state
  origin:point = point.origin()
  snapped:point-u32 = point.snap(point(3.25, 4.75))
  unit:size = size.unit()
  bounds:rectangle = rectangle.with_size(size(640.0, 480.0))

on inspect
  distance = point.distance(origin, point(3.0, 4.0))
  moved = (bounds + vector(4.0, 8.0)) * 2.0
  overlap = rectangle.intersection(bounds, moved)
```

Points support `+/- vector` and point subtraction produces a vector. Vectors
support negation, vector `+/-`, and `*` or `/ f64`. Sizes support size `+/-`,
`*` or `/ f64`, and component multiplication by a vector. Rectangles support
`+/- vector` and `* f64`. Codegen narrows Ice scalars to the native `f32`
operand only at these typed operations.

`point.origin`, `point.distance`, and `point.snap` cover point constants and
queries; `vector.zero` is the native zero value. Sizes provide `zero`, `unit`,
`infinite`, `min`, `max`, `expand`, `rotate`, `ratio`, `from_vector`, and
`vector.from_size`. `size.from_u32` accepts two checked literal `u32` values;
`size.try_from_u32(i64, i64) -> size?` returns none for runtime overflow.

Rectangles provide `zero`, `infinite`, `with_size`, `with_radius`,
`with_vertices`, `vertices_rotation`, `contains`, `distance`, `offset`,
`is_within`, `intersection`, `intersects`, `union`, `snap`, `expand`, `shrink`,
`rotate`, `zoom`, `anchor`, and `from_u32`. Expand and shrink take exact
`top, right, bottom, left` values. Rotate and vertex rotation use radians.
Anchor accepts checked `left|center|right` and `top|center|bottom` literals.

Snapping preserves iced's exact unsigned results: `point.snap` returns
`point-u32`, while `rectangle.snap` returns `rectangle-u32?`. Their coordinates
and dimensions project to lossless Ice `i64` values and both types can cross a
typed extern boundary. `rectangle.from_u32` converts an exact snapped rectangle
back to the default native `f32` rectangle.

`transform.identity()`, `transform.translate(x, y)`, and
`transform.scale(factor)` construct native transformations.
`transform.orthographic(width, height)` accepts two literal values in the full
native `u32` range; `transform.try_orthographic(i64, i64) -> transformation?`
safely handles runtime dimensions. `transform.inverse(value)` and
`transform.compose(left, right)` preserve iced's exact matrix behavior and
composition order. Transformations expose `scale_factor`, `translation`, and a
lossless 16-value `matrix` projection.

`transform.point`, `transform.vector`, `transform.size`,
`transform.rectangle`, `transform.cursor`, and `transform.click` apply the
native matrix to every iced value that implements transformation
multiplication. All geometry and transformation values can cross typed extern
boundaries unchanged. `mouse-click` uses iced's advanced mouse API and
therefore requires the `advanced` Cargo feature. Native clicks do not implement
equality; compare their checked `kind` or `position` fields instead.

Input-method composition events use a separate readable source:

```ice
subscribe
  input-method opened -> composition_opened
  input-method preedit -> composition_changed _ _ _
  input-method commit -> composition_committed _
  input-method closed -> composition_closed
```

Preedit emits `(text:str, start:i64?, end:i64?)`; both byte offsets are absent
when iced hides the composition cursor. Commit emits the final `str`, while
opened and closed have no payload. These subscriptions observe captured and
ignored events and require iced's `advanced` Cargo feature.

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
declared fonts also work on text, rich text and spans, input, editor, checkbox,
toggler, radio, pick, combo, and their custom icons. App-level `font "path"`
settings embed and preload the corresponding bytes before iced starts; a
descriptor's named family selects the family exposed by those bytes.

Runtime bytes use iced's native font Task directly:

```ice
on load_font
  task font load downloaded_font_bytes -> font_loaded _
```

The expression must be `bytes`, the success payload is `unit`, and the task is
treated as infallible because iced's current `font::Error` has no variants.

Image preallocation is a fallible native task:

```ice
task image allocate handle -> allocated _ | allocation_failed _
```

Success carries `image-allocation`; failure carries the exact `image-error`.
Both routes are required, and the task composes inside task groups, abortable
tasks, and typed task flows.

Widget operation tasks target checked IDs in the app view:

```ice
task widget focus #search
task widget focused #search -> focus_checked _
task widget cursor #search 3
task widget select #search 0 5
task widget snap #results 0.0 1.0
task widget scroll-by #results 0.0 24.0
```

Targets use the same hierarchy as the rendered component tree. Write the
outer scope first, then each nested scope separated by `/`:

```ice
state
  selected = 42
  draft = ""

component Dialog()
  col
    slot

component TaskField(value:str)
  input "Title" #title <-> value

on edit_selected
  task widget focus #dialog(selected)/TaskField/title

view
  Dialog #dialog(selected)
    TaskField value=draft
```

Explicit component IDs create a scope; a component without one uses its name.
Layout and container IDs create descendant scopes, slot content inherits its
slot position's scope, keyed rows add `key(value)`, table headers/cells add
`header(index)` or `row(index)/column(index)`, and panes add their name.
Declared dynamic IDs use `i64` or `str`; keyed rows use bool/i64/f64 and table
indices use i64. Every segment name, key presence, order, and key type must
match a real input, editor, or scroll ID. Static paths lower to
`widget::Id::new`; a path with any dynamic segment lowers to
`widget::Id::from(String)`.

Ice exposes all 13 functions in `iced::widget::operation`: previous/next/direct
focus and focus query; cursor front/end/position; select all/range; relative
snap/end; and absolute scroll-to/scroll-by. Effects have no route and
`focused` requires a `bool` route. Cursor and selection positions are
non-negative `i64`; relative offsets are `f64` in `0.0..=1.0`; absolute
offsets are unrestricted `f64`.

Feature-gated native widget selectors use the same checked paths and ordinary
Ice expressions:

```ice
state
  found:widget-target? = none
  matches:[widget-target] = []

on inspect
  task widget find id #dialog(selected)/TaskField/title -> found_one _

on inspect_text
  task widget find-all text "Title" -> found_many _

on inspect_point
  task widget find point 12.0 24.0 -> found_one _

on inspect_focus
  task widget find focused -> found_one _
```

`find` emits an optional result and `find-all` emits a list. Built-in `id`,
`text`, `point`, and `focused` selectors return `widget-target`, including its
kind, optional native `widget-id`, bounds, visible bounds, text content,
scrollable content bounds and translation when the selected target provides
them. Fields unavailable for a target kind are optional. A custom selector
call such as `find-all by_kind("text")` emits the selector declaration's output
type instead. Reading a field through `widget-target?` lifts that field into an
optional result. The consumer must enable iced's `selector` feature.

Persistent pane grids expose their native layout-state operations directly in
handlers:

```ice
pane #workspace maximize details
pane #workspace restore
pane #workspace swap tasks details
pane #workspace move details left
pane #workspace resize 0.6
pane #workspace resize editor_stack 0.55
pane #workspace drop details tasks center
pane #workspace split details preview horizontal ratio=0.4
pane #workspace close details
pane #workspace maximized -> pane_observed _
pane #workspace adjacent tasks right -> pane_observed _
```

Grid names, static pane names, template names, and dynamic key types are checked
against the app view. Effects mutate the compiler-owned `pane_grid::State`
synchronously and do not accept routes. Any pane argument may use
`template(key)`; `split` stores that key and the template resolves the current
item from its declared list state.
`maximized` and `adjacent` are final handler queries and emit `str?`, because
there may be no maximized or adjacent pane. `resize ratio` targets the root
split. A nested `split name axis` declaration gives that native split a stable
checked identity, and `resize name ratio` keeps targeting that split while it
remains in the layout. Ratios are checked `f64` values in `0.0..=1.0`. `drop`
accepts `center` or an edge region. `split` opens a declared closed pane beside
an open target with the requested axis and ratio; asking to open an already-open
pane is a no-op.

Window tasks can open named templates and retain iced's typed window ID in
ordinary Ice state:

```ice
state
  child:window-id? = none

on open_child
  task window open child -> child_opened _

on child_opened(id)
  child = some(id)
  task window size target=id -> window_size _ _

on find_oldest
  task window oldest -> oldest_found _

on find_latest
  task window latest -> latest_found _

on inspect_raw_id
  task window raw-id -> raw_id_read _

on capture_window
  task window screenshot -> window_captured _ _ _ _

on window_captured(pixels, width, height, scale)
  snapshot = rgba(width, height, pixels)

on change_icon
  task window icon bytes(ff 00 00 ff 00 ff 00 ff) 2 1
```

`open` emits `window-id`; `oldest` and `latest` emit `window-id?`. They require
routes and do not accept `target=`. All per-window effects and queries accept an
optional `target=window-id`; without it they retain the convenient behavior of
resolving iced's oldest initial window. Automatic tabbing is application-wide
and does not accept a target.

Other effects have no route and queries require one. `size` emits two `f64`
values; `maximized` emits `bool`; `minimized` emits `bool?`; `position` and
`monitor-size` each emit two `f64?` values; `scale-factor` emits `f64`; and
`mode` emits `str`. `raw-id` emits the opaque platform `u64` identifier as a
lossless `str`. `screenshot` emits RGBA `bytes`, physical `i64` width and
height, then its `f64` scale factor; the bytes can feed directly into
`rgba(width, height, pixels)`. `icon` accepts RGBA `bytes` followed by positive
`i64` width and height. Literal byte counts are checked as
`width × height × 4`; dynamic invalid data safely produces no task.

Callback-only iced window behavior crosses one exact typed boundary:

```ice
extern crate::backend
  window describe_window(prefix:str) -> str

on inspect_window
  task window describe_window("main") -> window_described _
```

The Rust item has the ABI
`fn(&dyn iced::window::Window, String) -> String`. The implicit first argument
provides iced's native `HasWindowHandle` and `HasDisplayHandle` access without
putting Rust syntax in Ice. Parameters, output, route, and optional
`target=window-id` are statically checked; generated probes make a missing item
or wrong Rust signature a local rustc error.

Ice covers close, drag and all resize directions, resize
and constraints, resizability, maximize/minimize state, position and movement,
all modes, decorations, user attention, focus, level, system menu, mouse
passthrough, monitor size, runtime RGBA icon changes, and automatic tabbing.
Positive sizes, bool arguments, icon payloads, callback arguments, and target
IDs are checked before Rust generation. Together with the structured platform
blocks and typed callback boundary, this covers iced 0.14's public window
surface.

Every iced window event has a direct subscription form:

```ice
subscribe
  window frame -> frame
  window opened with-id -> opened _ _ _ _ _
  window moved with-id -> moved _ _ _
  window resized with-id -> resized _ _ _
  window close-request with-id -> close_requested _
  window file-dropped with-id -> file_dropped _ _
```

`opened` emits optional x/y followed by width/height; moved and resized emit
two `f64` values; rescaled emits `f64`; file paths emit `str`; and frame,
closed, close-request, focused, unfocused, and files-hovered-left have no
payload. Add `with-id` to any of the eleven non-frame forms to prepend the
originating typed `window-id`; the frame stream is application-wide and has no
window identity. Routes accept only the exact number of `_` payloads. The
modifier also works with `status=`, while Canvas window events reject it because
Canvas receives only the event value.

Every iced mouse event also has a direct subscription form:

```ice
subscribe
  mouse entered -> entered
  mouse left -> left
  mouse moved -> moved _ _
  mouse pressed -> pressed _
  mouse released -> released _
  mouse wheel -> wheel _ _ _
```

Moved emits window x/y as two `f64` values. Pressed and released emit the exact
native `mouse-button`; its `kind` field is `left`, `right`, `middle`, `back`,
`forward`, or `other`, and `number` preserves the optional native `u16` value
as `i64?`.
Wheel emits x/y as `f64` followed by `pixels:bool`; false means iced line
units. These subscriptions observe captured and ignored runtime events. As
with window subscriptions, routes accept only the exact number of `_`
payloads.

All four iced touch events are direct subscriptions:

```ice
subscribe
  touch pressed -> pressed _ _ _
  touch moved -> moved _ _ _
  touch lifted -> lifted _ _ _
  touch lost -> lost _ _ _
```

Each emits `(finger:touch-finger, x:f64, y:f64)`. The typed finger preserves
iced's full native `u64` identity; its `id` field exposes a lossless decimal
string when text is needed. Routes
accept exactly three `_` payloads and observe both captured and ignored touch
events.

### IDs

IDs are identities, not CSS selectors. Static IDs must be unique in their local
view/component scope. Repeated instances use a stable typed key:

```ice
for task in tasks
  TaskRow task=task loading=loading #task(task.id)
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

Apps and nested subtrees may use `default`, `app`, or any of iced's 22 built-in
default-renderer themes. A typed Rust factory covers arbitrary native
`iced::Theme` values without embedding Rust expressions in Ice:

```ice
extern crate::backend
  theme native_theme(dark:bool)

app NativeTheme
  theme native_theme(dark)

view
  theme native_theme(!dark)
    text "Native nested theme"
```

The Rust function has signature `fn(bool) -> iced::Theme`. It may use
`Theme::custom_with_fn` to derive the complete extended palette; generated
probes reject a missing function, wrong arguments, or a different return type.

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
extern line; 1.44 does not claim that remapping.

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
skips `.git` and `target`, analyzes files with a top-level `app` or `daemon` as roots, and
formats both roots and imported fragments.

## 12. Current coverage and escape hatches

The 1.44 native backend covers both windowed applications and windowless
daemons alongside CRUD/settings-style screens, selection, media, hover
overlays, declarative canvas geometry, and pointer events. Borrowed custom
widgets and an application-wide renderer type remain the escape hatch for
specialized native behavior. [`COVERAGE.md`](COVERAGE.md) is the exact
versioned ledger.

The language must not grow one ad-hoc syntax form for every iced API. Thirty-three
typed Rust boundaries cover domain work, native elements and programs, runtime
tasks and subscriptions, Markdown viewers, and native style callbacks without
admitting arbitrary Rust into expressions or duplicating iced in the core
grammar. Direct native syntax remains preferable for common UI concepts.

Native language coverage and system coverage are therefore separate:

```text
common screen structure -> checked native Ice vocabulary
advanced/custom widget  -> typed Rust Element adapter
custom GPU program      -> typed Rust Shader Program adapter
iced runtime operation  -> typed Rust Task adapter
repeated task output     -> typed Rust Stream/Sipper adapter
event/stream source      -> typed Rust Subscription adapter
native default theme     -> typed Rust Theme factory
alternate themed subtree -> typed Rust Themer adapter
domain and I/O           -> typed Rust async extern
pure domain conversion   -> typed Rust sync extern
native window handle     -> typed Rust window callback
```

## 13. Reference application

The runnable multi-file task app starts at
[`examples/iced-app/src/ui/tasks.ice`](examples/iced-app/src/ui/tasks.ice), with
its Rust boundary in
[`examples/iced-app/src/main.rs`](examples/iced-app/src/main.rs). The exhaustive
compile-tested widget example is
[`examples/iced-app/src/ui/showcase.ice`](examples/iced-app/src/ui/showcase.ice).
Native pointer constructors, subscription payloads, projections, and Rust
extern round trips are exercised by
[`examples/iced-app/src/ui/pointer_values.ice`](examples/iced-app/src/ui/pointer_values.ice).
Complete native geometry construction, fields, constants, conversions,
arithmetic, queries, exact unsigned snapping, and extern passage are exercised
by
[`examples/iced-app/src/ui/geometry_values.ice`](examples/iced-app/src/ui/geometry_values.ice).
First-class pixels, padding, degrees, radians, range behavior, mixed native
operators, distance conversion, geometry integration, and extern passage are
exercised by
[`examples/iced-app/src/ui/padding_angles.ice`](examples/iced-app/src/ui/padding_angles.ice).
Native `Task::map` output/optional conversion and fallible error preservation
are executed by
[`examples/iced-app/src/ui/task_map.ice`](examples/iced-app/src/ui/task_map.ice).
Native app and nested custom Theme construction with extended-palette logic is
executed by
[`examples/iced-app/src/ui/theme_factory.ice`](examples/iced-app/src/ui/theme_factory.ice).
An alternate Rust Theme type, its Theme-dependent base callbacks, and the
native Themer bridge are executed by
[`examples/iced-app/src/ui/alternate_theme.ice`](examples/iced-app/src/ui/alternate_theme.ice).
The typed Element escape hatch is compiled with a custom advanced Overlay and
non-default `index()` by
[`examples/iced-app/src/ui/native_overlay.ice`](examples/iced-app/src/ui/native_overlay.ice).
Native transformation construction, matrix inspection, application, and extern
passage are exercised by
[`examples/iced-app/src/ui/transformation_values.ice`](examples/iced-app/src/ui/transformation_values.ice).
Together they exercise
state inference, typed extern structs/functions, mount and result handlers,
direct and component-prop input/editor binding, complete typed time tasks/subscriptions, typed generic/native-keyboard/mouse/touch/input-method/system subscriptions with status filters, exact keyboard, pointer, geometry, and transformation value operations with native extern passage, static, repeated, and hierarchically scoped widget operations, built-in and custom widget selectors, system tasks, clipboard effects, `if`, `for`, native keyed columns and lazy subtrees, parsed Markdown, structured tables, pure components, structured and compound component composition,
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
