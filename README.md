# Ice

Ice is a small, statically checked frontend language that compiles to
[iced](https://iced.rs/). Humans write the screen and interaction flow in
compact `.ice` files; Rust keeps domain rules, I/O, and custom platform code.

```text
.ice source -> parser -> semantic checker -> checked AST -> iced Rust backend
```

There is no runtime interpreter. `ui_lang::include_app!` is only the thin Cargo
adapter that includes a file and emits ordinary Rust.

Successful analysis produces a nominal `CheckedDocument`; only the checker can
construct it, and the Iced backend has no unchecked `Document` entry point.
Generated applications also declare `ui-lang-runtime = "=0.1.0"` directly
because generated Rust refers to its public crate path.

## Taste of the language

```ice
app Tasks
  title "Ice Tasks"
  window
    size 960 720
    min-size 480 360
    position centered

use "backend.ice"
use "theme.ice"
use "components/panel.ice"

state
  draft = ""
  loading = false

on submit
  return if loading || empty(trim(draft))
  loading = true
  run create_task(trim(draft)) -> created _ | failed _

view
  col width=fill height=fill padding=24.0 spacing=16.0 @bg-background
    Panel title="Create task" #create-task
      row width=fill spacing=12.0
        input "New task" #new-task <-> draft width=fill padding=12.0 @bg-surface
        button "Add" disabled=loading padding=12.0 @bg-primary text-white -> submit
```

`use` resolves relative to the importing file. Imported declarations share the
same checked app graph. File-backed errors point to the fragment that caused
them and include the offending source line and caret.

The punctuation has one job each:

- indentation is the tree;
- `@` starts checked semantic color, font-emphasis, and design-token utilities;
- `#name` is a scoped component/widget identity;
- `<->` is a two-way local state binding;
- `->` routes a widget or async result to a handler;
- `_` is the payload supplied by that route.

## Accessibility

Ice lowers a small Core surface into a deterministic AccessKit tree:

| Ice node | AccessKit role | Exported state |
| --- | --- | --- |
| `text` | `Label` | visible text value |
| `input` | `TextInput`, or `PasswordInput` when `secure=true` | current value for non-secure input; passwords never export their value |
| `button` | `Button` | name, description, disabled state, focus/click actions |
| `checkbox` | `CheckBox` | name, description, checked/disabled state, focus/click actions |
| labeled `image` | `Image` | name and description |

`label=` and `description=` accept checked `str` expressions. An input's first
string, a compact button's string, and a checkbox's visible label are their
default accessible names; explicit `label=` overrides that default. A button
with child content must declare `label=`, and an image enters the semantic tree
only when it has `label=`. Unlabeled images are decorative, and
`description=` without `label=` is rejected for media.

```ice
input "Name" label="Full name" description="Profile name" <-> name
button #help label="Open help" description="Keyboard help" -> show_help
  text "?"
image "help.ppm" label="Help diagram" description="The keyboard flow"
```

Enabled controls use source/view-tree order for Tab and Shift+Tab; disabled
controls are skipped. Enter or Space activates a focused button, Space
activates a focused checkbox, and wrapper-focused controls draw a visible
two-pixel outline. There is no numeric focus-order syntax.

The tree, focus, and action mapping are deterministic on every target. Native
screen-reader export is currently limited to single-window Linux applications
through AccessKit's AT-SPI adapter. Non-Linux native export, daemon and
multi-window adapters, and exact desktop screen-coordinate bounds are not
available through stock Iced 0.14.0. Rich text and advanced widgets do not gain
accessibility claims from this Core contract.

## Run the real iced sample

```bash
cargo run -p iced-app
```

The runnable task app is intentionally small and split by concern:

```text
src/
├── main.rs                   app entry point
├── backend/                  production Rust boundary
├── tests/                    example behavior tests by feature
└── ui/
    ├── tasks.ice             app and view
    ├── backend.ice           typed Rust boundary declarations
    ├── state.ice             UI state
    ├── theme.ice             color tokens
    ├── components/           reusable views
    └── handlers/tasks.ice    transitions and effects
```

[`showcase.ice`](examples/iced-app/src/ui/showcase.ice) is the compile-tested
compatibility fixture; focused `.ice` and Rust modules exercise individual
native surfaces without bloating the readable task app.

## Tooling

This repository includes a local Cargo alias, so these work from the repo root:

```bash
cargo ice fmt
cargo ice fmt --check
cargo ice check
cargo ice clippy
cargo ice compat
cargo ice expand examples/iced-app/src/ui/tasks.ice
cargo ice schema
cargo ice lsp
scripts/a11y-smoke.sh
```

`cargo ice schema` prints a generative JSON description of each Core
construct's context, syntax, child shape, typed properties, binding, and route,
plus the language revision and backend contract. LSP completion is derived from
the same construct table.

`cargo ice lsp` is a minimal stdio server with full-document synchronization,
UTF-16 diagnostics, whole-document formatting, and Core completion. For an
existing app file it overlays every open buffer in the import graph, reanalyzes
all open app roots after buffer changes, and publishes imported errors at the
imported URI. Closing a buffer falls back to disk. Definition and rename are
not advertised.

`cargo ice compat` analyzes every app graph, checks the exact `iced 0.14.0`,
`iced_widget 0.14.2`, `ui-lang-runtime`, and AccessKit lockfile baseline,
verifies the direct reference-app and runtime manifest pins, and runs the app
tests.

On Linux, `scripts/a11y-smoke.sh` creates an isolated D-Bus/AT-SPI session and
checks that the native tree is discoverable and an AT-SPI action reaches the
Iced bridge. Headless tests cover dispatch from that bridge to the app message.

`cargo ice fmt` uses the parser's string- and delimiter-aware tokenization to
migrate deprecated utilities, including text size, when the typed property
targets the same generated builder. It preserves routes, quoted markers,
semantic token utilities, and wrapper-only or dual-owner geometry whose
behavior differs.

Normal Cargo commands work too because the proc macro participates in the
standard compilation graph:

```bash
cargo build -p iced-app
cargo check --workspace
cargo clippy --workspace --all-targets --no-deps
cargo fmt --all
```

Core end-to-end cases use the built-in Rust test runner and paired fixture
files under `crates/ui-lang-core/tests/cases`:

```text
cases/<suite>/<case>/
├── as-is.ice   input
└── to-be.*     exact formatted output or expected diagnostic/Rust fragments
```

The `format`, `diagnostic`, and `compile` suites are auto-discovered, so a new
case needs no Rust test function. Focused AST and edge-case assertions remain
next to their parser, checker, or code generator module.

## Status

Ice 1.58 is an executable language revision, not an attempt to replace iced.
Its stable authoring Core is app/state/component/handler/view structure, common
layout and widgets, checked event routing, and typed Rust effects. Existing
advanced syntax remains available as a compatibility surface, while typed
`Element`, `Task`, `Subscription`, style, and component boundaries cover unusual
native behavior without growing Core merely for API parity.

Language revisions and Cargo package versions are intentionally separate. The
specification is revision 1.58; the workspace packages currently use pre-1.0
SemVer `0.1.0`.

[`SPEC.md`](SPEC.md) defines the Core and compatibility boundary.
[`COVERAGE.md`](COVERAGE.md) inventories the existing iced 0.14 surface; it is
not a roadmap for adding missing native syntax.
