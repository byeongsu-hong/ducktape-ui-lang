# UI Lang v0.1

Status: design draft

UI Lang is a small, statically checked UI language for `iced`. Its source files
use the `.rsx` extension. The `.rsx` file contains only UI Lang; one procedural
macro call in Rust loads it into the normal Cargo build. There is no runtime
parser or `build.rs` code generator.

```rust
// src/main.rs
ui_lang::include_app!("src/ui/tasks.rsx");

fn main() -> iced::Result {
    Tasks::run()
}
```

```rust
// src/ui/tasks.rsx
app Tasks;
// extern declarations, state, handlers, and one view
```

`include_app!` resolves a manifest-relative literal path, checks UI semantics,
and emits ordinary Rust. It also emits a hidden
[`include_str!`](https://doc.rust-lang.org/std/macro.include_str.html) dependency
so stable Cargo rebuilds when the `.rsx` file changes. `rustc` then checks
referenced Rust items and `iced` types.

## 1. Goals

- Remove ownership, lifetime, message-enum, update-match, and `iced::Task`
  boilerplate from ordinary UI work.
- Keep state transitions and asynchronous data flow explicit.
- Keep business rules in Rust behind typed `extern` declarations.
- Statically validate names, types, widget properties, handler routes, theme
  tokens, and Tailwind-style utilities.
- Preserve `.rsx` locations through explicit source maps and diagnostics.
- Provide one canonical formatter and machine-readable diagnostics for agents.
- Work with normal `cargo build`, `cargo check`, and `cargo clippy`.

## 2. Non-goals

Version 0.1 does not provide:

- React, JSX, hooks, reconciliation, or DOM compatibility;
- general Rust expressions inside the DSL;
- CSS selectors, cascade, inheritance, or arbitrary CSS values;
- runtime interpretation or hot-loaded source;
- business-rule, persistence, networking, or validation primitives;
- reusable components, animation, subscriptions, or custom widgets.

## 3. Architecture

There is one parser, one syntax tree, and one semantic checker. They are shared
by three thin front ends:

```text
include_app! proc macro -> Rust expansion for cargo build/check/clippy
cargo ui fmt            -> canonical .rsx printer
cargo ui check          -> fast DSL checks + remapped rustc diagnostics
```

The macro and CLI must never implement separate grammars.

UI Lang checks what is knowable from `.rsx`: DSL syntax, state and handler
types, declared extern shapes, widget contracts, and utilities. It cannot query
rustc's symbol table. Instead, it emits a small Rust probe and source-map entry
for every `extern`; `rustc` checks whether the item exists, is visible, and has
the declared shape.

## 4. UI and business boundary

UI Lang owns:

- input, selection, loading, and displayed-error state;
- event routing and simple state assignments;
- action invocation and result routing;
- layout and appearance.

Rust owns:

- domain validation and invariants;
- filesystem, database, network, and operating-system access;
- authorization and security decisions;
- shared calculations, logging, and observability.

Data moves in one direction:

```text
view interaction -> handler -> extern Rust fn -> result handler -> state -> view
```

UI checks such as disabling an empty submit button are conveniences. The Rust
function must validate the input again.

## 5. Source and lexical rules

- File extension: `.rsx`.
- A file starts with one `app Name;` declaration and contains no Rust wrapper.
- The grammar uses Rust-like tokens; whitespace and indentation have no meaning.
- Declarations and leaf widgets end in `;`.
- Blocks use `{ ... }`; struct and theme entries end in `,`.
- Identifiers use `snake_case`; app and struct aliases use `PascalCase`.
- Strings use Rust string-literal escaping. Interpolation is not supported.
- Comments use normal Rust `//` and `/* ... */` syntax.
- Source must be UTF-8.

Declaration order is fixed:

```rust
app AppName;

extern struct ViewType = crate::path::Type { /* fields */ }
extern fn action_name(/* args */) -> Success ! Error = crate::path::function;
theme { /* colors */ }
state state_name: Type = initial_value;

on mount { /* statements */ }
on event_name(/* args */) { /* statements */ }

view { /* exactly one root node */ }
```

Keywords are:

```text
app extern struct fn theme state on mount view
if else for in run return true false
```

## 6. Types

| UI Lang | Rust at an extern boundary |
| --- | --- |
| `bool` | `bool` |
| `i64` | `i64` |
| `f64` | `f64` |
| `str` | `String` |
| `[T]` | `Vec<T>` |
| extern struct alias | declared Rust struct path |
| `unit` | `()` |

Optionals, maps, enums, tuples, generics, and user-defined methods are deferred.
An application needing a richer domain type should expose a small view-model
struct at the UI boundary.

UI Lang has value semantics. Numbers and Booleans are copied. Strings, lists,
and structs are cloned only when they cross into a message or action. Generated
views borrow where possible. `.rsx` source never spells Rust borrows, moves,
lifetimes, or `.clone()`.

## 7. Extern declarations

Externs are declarations, not generated implementations.

### 7.1 Structs

```rust
extern struct Task = crate::backend::Task {
    id: i64,
    title: str,
    done: bool,
}
```

The alias `Task` is used inside UI Lang. The right-hand Rust type must exist and
be visible. Each declared field must be public and have the mapped Rust type.
Extra Rust fields are allowed because the UI does not construct extern structs.

The generated checker performs field-access probes equivalent to:

```rust
fn check(value: &crate::backend::Task) {
    let _: &i64 = &value.id;
    let _: &String = &value.title;
    let _: &bool = &value.done;
}
```

Extern structs used in state or messages must implement `Clone`. Comparison
requires `PartialEq`; it is checked only when the DSL compares the value.

### 7.2 Functions

```rust
extern fn create_task(title: str) -> [Task] ! AppError = crate::backend::create;
```

This declaration expects a function equivalent to:

```rust
async fn create(title: String) -> Result<Vec<Task>, AppError>
```

The function may return any future with that output; it does not need to use
`async fn` syntax. Arguments and results are owned values so the future can be
`'static` for `iced::Task::perform`.

An infallible function omits `! Error`:

```rust
extern fn close_window() -> unit = crate::system::close;
```

Extern functions may only be called by `run`; ordinary expression calls are not
allowed.

## 8. State and expressions

Every state declaration has an initial literal:

```rust
state tasks: [Task] = [];
state draft: str = "";
state loading: bool = false;
```

Handlers replace whole state values:

```rust
draft = value;
loading = true;
```

Nested mutation, indexing assignment, and collection mutation are absent.

Supported expressions are:

- string, integer, float, Boolean, and empty-list literals;
- state, handler parameter, loop variable, and extern-struct field references;
- parentheses;
- unary `!` and numeric `-`;
- numeric `+`, `-`, `*`, and `/`;
- `==`, `!=`, `<`, `<=`, `>`, and `>=`;
- Boolean `&&` and `||`;
- `len(value)`, `empty(value)`, and `trim(value)`.

`len` accepts a list or string and returns `i64`. `empty` accepts a list or
string. `trim` accepts and returns `str`.

There are no arbitrary calls, closures, assignment expressions, references,
casts, `await`, or nested macros.

## 9. Handlers and async actions

`mount` runs once after initial state creation:

```rust
on mount {
    loading = true;
    run list_tasks() -> loaded(_) | failed(_);
}
```

Named handlers declare parameter types:

```rust
on edit(value: str) {
    draft = value;
}
```

Handler statements are limited to state assignment, `if`/`else`, `run`, and
`return;`. There are no handler-local variables or loops.

`run` schedules an extern function and routes its output:

```rust
run set_task_done(id, checked) -> updated(_) | failed(_);
```

`_` is a payload placeholder. The success route replaces it with the success
value; the error route replaces it with the error value. Route arguments may
also forward in-scope values:

```rust
run load_page(page) -> loaded(page, _) | failed(_);
```

`run` must be the final statement in its control-flow branch. Infallible externs
have one route:

```rust
run close_window() -> closed(_);
```

Cancellation, progress, batching, and subscriptions are deferred.

The generated update function uses the pinned target version's `iced::Task`
model. See the official [`iced::Task` documentation](https://docs.iced.rs/iced/task/struct.Task.html).

## 10. View language

`view` is pure and contains exactly one root node.

### 10.1 Structural nodes

```rust
col class="w-full gap-4" {
    // zero or more children
}

row class="w-full items-center gap-2" {
    // zero or more children
}

scroll class="w-full h-full" {
    // exactly one child
}
```

The compiler may insert iced containers when padding, backgrounds, borders, or
maximum sizes cannot be applied directly to a structural widget.

### 10.2 Leaf widgets

```rust
text "Tasks" class="text-2xl font-bold text-foreground";
text len(tasks) class="text-sm text-muted";

button "Add"
    disabled=(loading || empty(trim(draft)))
    class="px-4 py-2 bg-primary text-white rounded-lg"
    -> submit;

input
    id="new-task"
    label="New task"
    value=draft
    placeholder="What needs doing?"
    disabled=loading
    class="w-full px-4 py-2 bg-surface text-foreground border border-border rounded-lg"
    -> edit(_);

checkbox
    label=task.title
    checked=task.done
    disabled=loading
    class="w-full text-foreground"
    -> toggle(task.id, _);
```

Properties containing operators must be parenthesized. `class` and `id` require
string literals. The route payloads are:

| Widget | Payload |
| --- | --- |
| `button` | none; `_` is forbidden |
| `input` | new `str`; `_` is required once |
| `checkbox` | new `bool`; `_` is required once |

### 10.3 Conditional and repeated nodes

```rust
if loading {
    text "Loading..." class="text-sm text-muted";
} else {
    text "Ready" class="text-sm text-foreground";
}

for task in tasks {
    checkbox label=task.title checked=task.done -> toggle(task.id, _);
}
```

There is no React-style `key`: v0.1 has no component-local state or
reconciliation, so a key would have no semantics.

## 11. Accessibility

- `button` requires a non-empty textual label.
- `input` requires a non-empty literal `label`.
- `checkbox` requires a `str` label expression.
- `id`, when present, must be a unique string literal in the view.
- Disabled controls expose iced's disabled semantics.
- Keyboard focus and activation use native iced widget behavior.

Images are deferred until the language has an asset model and can require
alternative text.

## 12. Theme and Tailwind-style utilities

Theme values are static color tokens:

```rust
theme {
    background: "#0f172a",
    surface: "#111827",
    foreground: "#f8fafc",
    muted: "#94a3b8",
    primary: "#7c3aed",
    danger: "#dc2626",
    border: "#334155",
}
```

Values must be `#RRGGBB` or `#RRGGBBAA`. `white`, `black`, and `transparent`
are built in. Unknown, unused, shadowing, and duplicate tokens are errors.

`class` is a string literal containing a checked Tailwind-style subset. It is
not CSS and has no runtime engine. Class concatenation and computed classes are
errors.

Spacing number `N` means `N * 4` iced logical pixels. Supported numbers are:

```text
0 1 2 3 4 5 6 8 10 12 16 20 24
```

| Group | Utilities |
| --- | --- |
| Size | `w-full`, `w-fit`, `h-full`, `h-fit`, `w-N`, `h-N` |
| Maximum width | `max-w-sm`, `max-w-md`, `max-w-lg`, `max-w-xl`, `max-w-2xl` |
| Spacing | `p-N`, `px-N`, `py-N`, `pt-N`, `pr-N`, `pb-N`, `pl-N`, `gap-N` |
| Alignment | `items-start`, `items-center`, `items-end`, `justify-start`, `justify-center`, `justify-end`, `justify-between`, `self-start`, `self-center`, `self-end` |
| Text size | `text-xs`, `text-sm`, `text-base`, `text-lg`, `text-xl`, `text-2xl` |
| Weight | `font-normal`, `font-medium`, `font-semibold`, `font-bold` |
| Text alignment | `text-left`, `text-center`, `text-right` |
| Color | `bg-TOKEN`, `text-TOKEN`, `border-TOKEN` |
| Border | `border`, `border-0`, `border-2` |
| Radius | `rounded-none`, `rounded-sm`, `rounded`, `rounded-md`, `rounded-lg`, `rounded-full` |
| Opacity | `opacity-0`, `opacity-25`, `opacity-50`, `opacity-75`, `opacity-100` |
| Decoration | `line-through` |

Maximum widths are 384, 448, 512, 576, and 672 logical pixels. Text sizes are
12, 14, 16, 18, 20, and 24 logical pixels.

Colors accept opacity suffixes in multiples of ten, such as `bg-primary/90`.
State variants are `hover:`, `pressed:`, `focus:`, and `disabled:`.

The checker rejects unknown utilities, conflicting utilities in one variant,
unsupported widget variants, arbitrary values such as `w-[13px]`, selectors,
responsive prefixes, grid, positioning, transforms, animation, and margins.
Parents use `gap`; containers use padding.

## 13. Formatter

`cargo ui fmt` is the authoritative `.rsx` formatter:

```text
cargo ui fmt
cargo ui fmt --check
```

Formatting is an AST print, not token whitespace cleanup. It is idempotent and:

- uses four-space indentation;
- writes one declaration or statement per line;
- expands multi-property widgets vertically when they exceed 100 columns;
- orders declarations by the required source order;
- orders utilities by size, spacing, alignment, typography, color, border,
  opacity, then state variant;
- preserves comments by attaching each comment to the next syntax node;
- never changes expression or child order.

`cargo fmt` continues to format Rust targets and leaves `.rsx` files alone. It
cannot discover a foreign source format and is therefore not the `.rsx`
formatter. `cargo ui fmt` first runs `cargo fmt`, then formats all referenced
`.rsx` files, giving the project one command for both kinds of source.

Cargo custom subcommands are ordinary `cargo-NAME` executables; `cargo ui` is
implemented by a `cargo-ui` binary, following Cargo's
[`custom subcommand` contract](https://doc.rust-lang.org/cargo/reference/external-tools.html#custom-subcommands).

## 14. Analyzer and Cargo compatibility

Commands are:

```text
cargo ui check             # DSL checks, then cargo check with diagnostic mapping
cargo ui clippy            # DSL checks, then cargo clippy with diagnostic mapping
cargo ui fmt [--check]     # cargo fmt, then .rsx formatting
```

The analyzer reports syntax, type, flow, accessibility, theme, and utility
errors without compiling Rust. Output supports human text and Cargo-compatible
JSON.

Raw Cargo remains valid:

| Command | Contract |
| --- | --- |
| `cargo build` | expands `.rsx`, resolves externs, builds the app |
| `cargo check` | same static checks without code generation output |
| `cargo clippy` | lints handwritten Rust and the generated expansion |
| `cargo fmt` | formats Rust and leaves `.rsx` files untouched |

`cargo ui check` and `cargo ui clippy` invoke Cargo with
`--message-format=json`, then map generated probes back to extern declarations.
They use Cargo's CLI and `cargo metadata`; they do not link Cargo as a library.

Generated code should be idiomatic. Narrow `#[allow(clippy::...)]` attributes
may cover unavoidable expansion artifacts, but a blanket Clippy allow is
forbidden.

## 15. Friendly extern diagnostics

The analyzer never scans Rust source text to guess whether an extern exists.
Rust macros, re-exports, features, and generated modules make that unreliable.
`rustc` is the authority; source maps and diagnostic codes make its result easy
to understand.

Stable procedural macros cannot create rustc spans inside a separately read
file. Raw `cargo build`, `check`, and `clippy` therefore point at the
`include_app!` call while naming the failing extern probe. `cargo ui` uses the
generated source map to point at the exact `.rsx` declaration and rewrites known
rustc failures:

```text
E201 src/ui/tasks.rsx:3:26
extern struct `crate::backend::Task` was not found
hint: define `pub struct Task` or correct the extern path
```

```text
E202 src/ui/tasks.rsx:3:26
extern struct `crate::backend::Task` has no public field `title: String`
hint: expose that field or declare the view-model struct actually returned by the backend
```

```text
E203 src/ui/tasks.rsx:14:69
extern fn `crate::backend::create` has the wrong signature
expected: async fn(String) -> Result<Vec<Task>, AppError>
hint: change the function or its extern declaration
```

```text
E041 src/ui/tasks.rsx:74:16
unsupported utility `backdrop-blur-xl`
hint: remove it; iced has no backdrop-blur utility
```

Stable UI Lang codes are used for mapped errors. The original rustc diagnostic
is retained as structured `cause` data in JSON, not duplicated in human output.

## 16. Generated Rust contract

The macro generates:

- a private state struct;
- a private message enum from handlers and continuations;
- `update` and `view` functions;
- `iced::Task::perform` calls for `run`;
- field and function probes for externs;
- a public app type with `run() -> iced::Result`.

Generated Rust is not written into `src/`, committed, or edited. The consuming
crate pins one iced version; UI Lang does not carry several iced adapters in
v0.1.

## 17. Complete example

- [`examples/tasks.rsx`](examples/tasks.rsx) is the complete UI source.
- [`examples/main.rs`](examples/main.rs) loads it with `include_app!` and implements
  the extern Rust structs and functions.

The example uses only the standard library for its in-memory business layer.
The compiler, formatter, analyzer, and Cargo subcommand are specified here but
are intentionally not scaffolded by this design-only draft.
