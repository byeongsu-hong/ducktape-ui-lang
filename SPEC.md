# Ice Language Specification 0.2

Status: implemented reference slice

Ice is a small frontend language with an iced backend. It is not Rust syntax,
JSX, or a token shortcut around a procedural macro. A frontend parses `.ice`
source, resolves names and types, checks UI semantics, and lowers a typed tree
to backend code.

This document describes what the repository implements. A section explicitly
marked “planned” is a design constraint, not accepted 0.2 syntax.

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
UTF-8 .ice source
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

The macro emits `include_str!` so Cargo rebuilds after a `.ice` change. It also
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
  block comments are not part of 0.2.
- Identifiers use ASCII letters, digits, and `_`, and cannot begin with a digit.
- App, extern-struct, and component names conventionally use `PascalCase`.
- State, field, function, handler, and parameter names conventionally use
  `snake_case`.
- Static IDs use kebab case after `#`, for example `#task-list`.
- Strings use double quotes and support `\n`, `\r`, `\t`, `\"`, and `\\`.

Top-level declarations are order-independent, but canonical source uses:

```text
app
extern
theme
state
component
on
view
```

A file has exactly one `app` and one `view`, with at most one `extern`
namespace. It may have multiple components and handlers. The view and each
component have exactly one root node.

## 4. Compact grammar

The grammar below uses indentation (`INDENT`) as a block delimiter. `expr` is
defined in section 6.

```text
document       = app_decl extern_decl? theme_decl state_decl?
                 component_decl* handler_decl* view_decl

app_decl       = "app" PascalName

extern_decl    = "extern" rust_path INDENT extern_item+
extern_item    = struct_sig | function_sig
struct_sig     = PascalName "(" field_list? ")"
field_list     = field ("," field)*
field          = name ":" type
function_sig   = name "(" field_list? ")" "->" type ("!" type)?

theme_decl     = "theme" INDENT color_entry+
color_entry    = name color

state_decl     = "state" INDENT state_entry+
state_entry    = name (":" type)? "=" expr

component_decl = "component" PascalName "(" field_list? ")"
                 INDENT node

handler_decl   = "on" name ("(" name_list? ")")?
                 INDENT statement*
statement      = name "=" expr
               | "return if" expr
               | "run" call "->" route ("|" route)?

view_decl      = "view" INDENT node

node           = layout | text | input | button | checkbox | toggler
               | slider | progress | radio | rule | space
               | component_call | if_node | for_node
layout         = ("col" | "row" | "scroll") id? styles? INDENT node+
               | "grid" id? ("columns=" expr)? styles? INDENT node+
               | "stack" id? ("clip=" expr)? styles? INDENT node+
text           = "text" expr styles?
input          = "input" string id? "<->" name property* styles?
button         = "button" string id? property* styles? "->" route
checkbox       = "checkbox" expr id? property* styles? "->" route
toggler        = "toggler" expr "checked=" expr ("disabled=" expr)?
                 styles? "->" route
slider         = "slider" expr "min=" expr "max=" expr "step=" expr
                 "vertical"? ("release=" route)? styles? "->" route
progress       = "progress" expr ("min=" expr)? ("max=" expr)?
                 "vertical"? styles?
radio          = "radio" expr "value=" expr "selected=" expr
                 styles? "->" route
rule           = "rule" ("horizontal" | "vertical")
                 ("thickness=" expr)? styles?
space          = "space" ("width=" expr)? ("height=" expr)? styles?
component_call = PascalName "(" expr_list? ")" id?
if_node        = "if" expr INDENT node+
for_node       = "for" name "in" expr INDENT node+

property       = "hint=" string | "disabled=" expr | "checked=" expr
styles         = "@" utility+
id             = "#" kebab_name | "#" name "(" expr ")"
route          = name | name "(" route_arg_list? ")"
route_arg      = expr | "_"
```

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

Extern functions are asynchronous. `A -> B` means `async fn(...) -> B`.
`A -> B ! E` means `async fn(...) -> Result<B, E>`. Values crossing into iced
messages must satisfy the traits required by generated iced code, notably
`Clone` for 0.2 message payloads.

Struct declarations are read-only views of Rust data. Ice may read a declared
field (`task.title`) but cannot construct or mutate the struct. Declaring a
field or function does not create it; the generated Rust probes verify the
actual item and type.

## 6. State and expressions

Literal state types are inferred:

```ice
state
  draft = ""
  loading = false
  retries = 0
```

These infer to `str`, `bool`, and `i64`, respectively.

Empty lists need an annotation because their element type is unknowable:

```ice
tasks:[Task] = []
```

The expression language contains:

- literals: strings, booleans, `i64`, `f64`, and `[]`;
- paths: `state_name`, `parameter`, `item.field`;
- unary operators: `!`, `-`;
- arithmetic: `*`, `/`, `+`, `-`;
- comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`;
- boolean operators: `&&`, `||`;
- parentheses;
- built-ins: `len(list_or_str) -> i64`, `empty(list_or_str) -> bool`, and
  `trim(str) -> str`.

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
- `run` must be the final statement because it returns one iced `Task`;
- fallible externs require both success and error routes;
- infallible externs permit only the success route;
- handler parameter types are inferred from every incoming route;
- incompatible incoming payloads are a type error;
- `_` means the payload produced by the current widget or action route.

Examples of payload flow:

```ice
checkbox task.title checked=task.done -> toggle(task.id, _)
run list_tasks() -> loaded _ | failed _

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
| `col` | vertical children |
| `row` | horizontal children |
| `scroll` | exactly one child |
| `grid` | responsive grid; optional positive `i64` `columns` (default 3) |
| `stack` | overlays children; optional bool `clip` |
| `text` | one `str`, `i64`, or `f64` expression |
| `input` | string label, optional ID/hint/disabled, required `str` binding |
| `button` | literal label, optional ID/disabled, required route |
| `checkbox` | string label expression, required bool `checked`, optional disabled, bool-payload route |
| `toggler` | string label, bool `checked`, optional disabled, bool-payload route |
| `slider` | `f64` value/range/step, optional vertical axis and release route, `f64`-payload route |
| `progress` | `f64` value/range, optional vertical axis |
| `radio` | string label, `i64` or bool value, bool `selected`, value-payload route |
| `rule` | horizontal or vertical separator with `f64` thickness |
| `space` | optional fixed `f64` width and height |
| `if` | includes its children when a bool expression is true |
| `for` | iterates a list and adds one typed item binding |

`if` and `for` are child control-flow nodes inside a layout. There is no virtual
DOM or runtime reconciliation layer; the iced backend constructs the current
element tree from state.

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

`cargo ice check` first reports these language errors directly, then invokes
`cargo check` so rustc verifies extern items and generated iced types. A missing
Rust item is named by its `crate::module::item` path in rustc's diagnostic. A
future source-map layer may remap those rustc spans into the precise extern line;
0.2 does not claim that remapping.

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

`cargo-ice` discovers `.ice` files recursively below the current directory and
skips `.git` and `target`.

## 12. Current coverage and planned escape hatch

The 0.2 native backend is enough for CRUD/settings-style screens, not all of
iced. It still lacks pick lists, combo boxes, image, SVG, canvas, tooltip,
overlay/modal, rich text and text editors, subscriptions/streams,
keyboard/mouse/clipboard events, widget operations, multiple windows, and
custom widgets. [`COVERAGE.md`](COVERAGE.md) is the exact versioned ledger.

The language must not grow one ad-hoc syntax form for every iced API. The next
extension is a typed extern-widget boundary: Ice declares typed properties and
emitted events, while Rust supplies an iced `Element` adapter. That boundary
will make advanced widgets available without admitting arbitrary Rust into
expressions or duplicating iced in the core grammar. Its exact syntax and ABI
are planned, not accepted by the 0.2 parser.

Native language coverage and system coverage are therefore separate:

```text
common screen structure -> small checked Ice vocabulary
advanced/custom widget  -> typed Rust widget adapter (planned)
domain and I/O           -> typed Rust async externs (implemented)
```

## 13. Reference application

The authoritative full example is
[`examples/iced-app/src/ui/tasks.ice`](examples/iced-app/src/ui/tasks.ice), with
its Rust boundary in
[`examples/iced-app/src/main.rs`](examples/iced-app/src/main.rs). It exercises
state inference, typed extern structs/functions, mount and result handlers,
direct input binding, `if`, `for`, a pure component, dynamic component IDs,
theme utilities, disabled controls, fallible asynchronous tasks, grid and stack
layouts, toggles, sliders, progress, radio controls, rules, and fixed spacing.
