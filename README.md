# Ice

Ice is a small, statically checked frontend language that compiles to
[iced](https://iced.rs/). Humans write the screen and interaction flow in a
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

[`showcase.ice`](examples/iced-app/src/ui/showcase.ice) is the separate
compile-tested kitchen sink. It includes complete wrapping row/column layouts,
grid, keyed columns and fully sized underlay stacks, optional
selection state, native pick and searchable combo lists, native controls and
media, native and extern tooltip/mouse areas, a responsive view, float/pin
positioning, visibility sensing, a clipboard task,
configured scrolling with offset events, pointer movement/wheel events,
formatted text, an extended text input and child-content buttons, plus an
application event subscription, expanded boolean controls, configured rules,
sliders with nested status styles, configured progress bars, configured native
tooltip styles, a styled fixed-version QR code, nested built-in themes, and
dependency-keyed lazy subtrees, plus incrementally parsed Markdown with complete
native styling, image URI access, link events, and typed custom viewers.
The showcase also contains a typed structured table with arbitrary header and
cell subtrees, an automatically bound syntax-highlighted text editor, and
structured keyboard press/release/modifier subscriptions.
It also compiles the full tree through an explicit app renderer type and a Rust
component that borrows `str` and `bool` props into its returned `Element`.
It also exercises native system information/theme tasks and theme-change
subscriptions, standard/primary clipboard reads and writes, and every checked
window effect/query task. Compile fixtures cover every native window,
mouse, touch, input-method, and timer subscription without adding continuous
event loops to the demo, plus structured parallel and sequential native task
composition, abortable task handles, typed native task streams, synchronous
typed Rust calls, result-preserving task flows with error mapping, and the
complete native iced time API.
The native Tasks app separately exercises fully styled checked-aware checkboxes,
togglers, and generic-value radios.

Key files:

- [`tasks.ice`](examples/iced-app/src/ui/tasks.ice) is the readable root;
- [`accessibility.ice`](examples/iced-app/src/ui/accessibility.ice) is the
  compile- and headless-tested Core accessibility fixture;
- [`pointer_values.ice`](examples/iced-app/src/ui/pointer_values.ice) preserves
  native pointer values through state, events, and Rust externs;
- [`mouse_interaction.ice`](examples/iced-app/src/ui/mouse_interaction.ice)
  preserves every native mouse interaction and passes typed values to widgets;
- [`scroll_delta.ice`](examples/iced-app/src/ui/scroll_delta.ice) preserves both
  native scroll delta variants, coordinates, and Rust extern passage;
- [`event_status.ice`](examples/iced-app/src/ui/event_status.ice) preserves both
  native event statuses, merge precedence, and Rust extern passage;
- [`redraw_request.ice`](examples/iced-app/src/ui/redraw_request.ice) preserves
  every native redraw request, scheduled instant, and Rust extern passage;
- [`window_id.ice`](examples/iced-app/src/ui/window_id.ice) preserves native
  unique IDs, display, comparison, lazy identity, and Rust extern passage;
- [`window_screenshot.ice`](examples/iced-app/src/ui/window_screenshot.ice)
  preserves native captures, construction, cropping, fields, and byte access;
- [`window_values.ice`](examples/iced-app/src/ui/window_values.ice) preserves
  native direction, level, mode, and user-attention enums;
- [`window_position.ice`](examples/iced-app/src/ui/window_position.ice)
  preserves every native position variant, including callback positioning;
- [`transformation_values.ice`](examples/iced-app/src/ui/transformation_values.ice)
  composes and applies native iced geometry transformations;
- [`geometry_values.ice`](examples/iced-app/src/ui/geometry_values.ice)
  exercises complete default geometry values, arithmetic, queries, and exact
  unsigned snapping;
- [`padding_angles.ice`](examples/iced-app/src/ui/padding_angles.ice) preserves
  native pixels, padding, degrees, and radians through operations and externs;
- [`rotation.ice`](examples/iced-app/src/ui/rotation.ice) preserves the native
  rotation enum, mutation/query API, geometry application, and widget passage;
- [`content_fit.ice`](examples/iced-app/src/ui/content_fit.ice) preserves every
  native fit strategy, geometry calculation, extern, and media-widget passage;
- [`color.ice`](examples/iced-app/src/ui/color.ice) preserves native color
  construction, conversion, contrast, mutation, parsing, and extern passage;
- [`length.ice`](examples/iced-app/src/ui/length.ice) preserves native length
  variants, conversions, queries, externs, and direct layout/widget passage;
- [`alignment.ice`](examples/iced-app/src/ui/alignment.ice) preserves all native
  axis-alignment variants, conversions, lazy identity, and extern passage;
- [`shadow.ice`](examples/iced-app/src/ui/shadow.ice) preserves the complete
  native shadow value, field projections, equality, and extern passage;
- [`border_radius.ice`](examples/iced-app/src/ui/border_radius.ice) preserves
  native borders and every radius constructor, builder, conversion, and field;
- [`background_gradient.ice`](examples/iced-app/src/ui/background_gradient.ice)
  preserves native backgrounds, gradients, linear fills, and color stops;
- [`font_values.ice`](examples/iced-app/src/ui/font_values.ice) preserves native
  fonts and every family, weight, stretch, and style descriptor;
- [`theme_mode.ice`](examples/iced-app/src/ui/theme_mode.ice) preserves every
  native theme mode, default, equality, and Rust extern passage;
- [`text_values.ice`](examples/iced-app/src/ui/text_values.ice) preserves native
  text alignment, shaping, wrapping, line-height values, and conversions;
- [`animation.ice`](examples/iced-app/src/ui/animation.ice) declares native
  boolean/numeric animation state with checked easing, timing, projection, and
  active-frame driving;
- [`image_allocation.ice`](examples/iced-app/src/ui/image_allocation.ice)
  preallocates image handles and retains exact native allocation/error values;
- [`debug_timing.ice`](examples/iced-app/src/ui/debug_timing.ice) owns native
  timing spans and measures typed expressions;
- [`task_map.ice`](examples/iced-app/src/ui/task_map.ice) executes native task
  output/optional mapping and fallible error preservation;
- [`theme_factory.ice`](examples/iced-app/src/ui/theme_factory.ice) executes a
  typed native Theme factory for app and nested custom themes;
- [`alternate_theme.ice`](examples/iced-app/src/ui/alternate_theme.ice) embeds a
  subtree using a different Rust Theme type;
- [`native_overlay.ice`](examples/iced-app/src/ui/native_overlay.ice) proves the
  typed Element escape hatch with a custom indexed Overlay;
- [`main.rs`](examples/iced-app/src/main.rs) only wires the app and re-exports
  fixture backends; every new harness stays in a dedicated module such as
  [`rotation.rs`](examples/iced-app/src/rotation.rs) and
  [`content_fit.rs`](examples/iced-app/src/content_fit.rs), with new surfaces
  such as [`color.rs`](examples/iced-app/src/color.rs) and
  [`length.rs`](examples/iced-app/src/length.rs), plus
  [`alignment.rs`](examples/iced-app/src/alignment.rs) and
  [`shadow.rs`](examples/iced-app/src/shadow.rs), with larger surfaces such as
  [`border_radius.rs`](examples/iced-app/src/border_radius.rs) and
  [`background_gradient.rs`](examples/iced-app/src/background_gradient.rs), or
  [`font_values.rs`](examples/iced-app/src/font_values.rs) and
  [`mouse_interaction.rs`](examples/iced-app/src/mouse_interaction.rs), plus
  [`scroll_delta.rs`](examples/iced-app/src/scroll_delta.rs) and
  [`window_values.rs`](examples/iced-app/src/window_values.rs), plus
  [`window_position.rs`](examples/iced-app/src/window_position.rs) and
  [`event_status.rs`](examples/iced-app/src/event_status.rs), plus
  [`redraw_request.rs`](examples/iced-app/src/redraw_request.rs) and
  [`window_id.rs`](examples/iced-app/src/window_id.rs), plus
  [`theme_mode.rs`](examples/iced-app/src/theme_mode.rs) and
  [`text_values.rs`](examples/iced-app/src/text_values.rs), plus
  [`window_screenshot.rs`](examples/iced-app/src/window_screenshot.rs), instead
  of growing a monolithic entry point;
- [`SPEC.md`](SPEC.md) defines the implemented language.

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
existing app file it overlays the open root buffer on the disk-backed import
graph and publishes imported errors at the imported URI. It does not overlay
unsaved import buffers or advertise definition and rename.

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
