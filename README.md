# Ice

Ice is a small, statically checked frontend language that compiles to
[iced](https://iced.rs/). Humans write the screen and interaction flow in a
compact `.ice` files; Rust keeps domain rules, I/O, and custom platform code.

```text
.ice source -> parser -> typed AST/IR -> semantic checker -> iced Rust backend
```

There is no runtime interpreter. `ui_lang::include_app!` is only the thin Cargo
adapter that includes a file and emits ordinary Rust.

## Taste of the language

```ice
app Tasks

use "backend.ice"
use "theme.ice"

state
  draft = ""
  loading = false

on submit
  return if loading || empty(trim(draft))
  loading = true
  run create_task(trim(draft)) -> created _ | failed _

view
  col @w-full h-full p-6 gap-4 bg-background
    input "New task" #new-task <-> draft @w-full p-3 bg-surface rounded-lg
    button "Add" disabled=loading @p-3 bg-primary text-white rounded-lg -> submit
```

`use` resolves relative to the importing file. Imported declarations share the
same checked app graph, and errors still point to the fragment that caused them.

The punctuation has one job each:

- indentation is the tree;
- `@` starts checked Tailwind-like style utilities;
- `#name` is a scoped component/widget identity;
- `<->` is a two-way local state binding;
- `->` routes a widget or async result to a handler;
- `_` is the payload supplied by that route.

## Run the real iced sample

```bash
cargo run -p iced-app
```

The runnable task app is intentionally small and split by concern:

```text
src/ui/
├── tasks.ice                 app and view
├── backend.ice               typed Rust boundary
├── state.ice                 UI state
├── theme.ice                 color tokens
├── components/panel.ice      component with a structured child slot
├── components/task_row.ice   reusable view
└── handlers/tasks.ice        transitions and effects
```

[`showcase.ice`](examples/iced-app/src/ui/showcase.ice) is the separate
compile-tested kitchen sink. It includes complete wrapping row/column layouts,
grid and fully sized underlay stacks, optional
selection state, native pick and searchable combo lists, native controls and
media, native and extern tooltip/mouse areas, a responsive view, float/pin
positioning, visibility sensing, a clipboard task,
configured scrolling with offset events, pointer movement/wheel events,
formatted text, an extended text input and child-content buttons, plus an
application event subscription, expanded boolean controls, configured rules,
sliders with nested status styles, configured progress bars, configured native
tooltip styles, a styled fixed-version QR code, and nested built-in themes.

Key files:

- [`tasks.ice`](examples/iced-app/src/ui/tasks.ice) is the readable root;
- [`main.rs`](examples/iced-app/src/main.rs) owns the Rust backend;
- [`SPEC.md`](SPEC.md) defines the implemented language.

## Tooling

This repository includes a local Cargo alias, so these work from the repo root:

```bash
cargo ice fmt
cargo ice fmt --check
cargo ice check
cargo ice clippy
cargo ice expand examples/iced-app/src/ui/tasks.ice
```

Normal Cargo commands work too because the proc macro participates in the
standard compilation graph:

```bash
cargo build -p iced-app
cargo check --workspace
cargo clippy --workspace --all-targets --no-deps
cargo fmt --all
```

## Status

This is an executable v0.27 language slice, not yet a complete iced replacement.
It implements typed extern data/actions, state, handlers, async tasks, pure
components with structured child slots, scoped IDs, relative multi-file `use`,
`if`/`for`, five layouts,
twenty-two native widget forms,
checked style utilities, formatting, analysis, and iced code generation. Typed
`Element`, `Task`, and `Subscription` adapters expose advanced iced features
without embedding Rust inside `.ice`. Unsupported syntax is rejected instead
of silently ignored.

[`COVERAGE.md`](COVERAGE.md) is the authoritative iced 0.14 coverage ledger.
