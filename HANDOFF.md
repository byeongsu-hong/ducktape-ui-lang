# Ice project handoff

Updated: 2026-07-19
Base: `origin/main` at `ff78ac7` (`Ice 1.58`, split Core modules)

## Product decision

Ice is a small, statically checked frontend language for authoring Ducktape UI.
It compiles to ordinary Rust/Iced code; it is not a runtime interpreter and it
is not intended to mirror every public Iced API.

Optimize for these two readers:

- a person scanning screen structure, state, and effects;
- an agent generating one canonical form and receiving local, useful errors.

The former goal of complete native Iced coverage is stopped. Existing advanced
syntax remains supported for compatibility, but the coverage ledger is now an
inventory, not a roadmap.

## Preserve these contracts

- `.ice` remains an independent language, not embedded Rust or Rust-like JSX.
- UI/display state, transitions, layout, and event routing live in Ice.
- validation, invariants, persistence, networking, security, and unusual
  platform behavior live in Rust.
- Rust is reached through checked `extern` declarations. Missing or
  shape-incompatible functions, structs, and components must produce useful
  source-located errors.
- `use` keeps apps split by concern; do not return to a single large app file.
- Components keep structured children, named slots, typed props, and scoped
  widget IDs.
- The parser, checker, formatter, and code generator share one language model.
- `cargo build`, `cargo fmt`, and `cargo clippy` remain compatible; `cargo ice`
  adds Ice-aware formatting, checking, linting, and expansion.

## Language boundary

### Ice Core

Core is the stable authoring surface:

- `app`, `use`, `state`, `component`, `slot`, `on`, and `view`;
- `if`, `for`, `keyed`, and `lazy`;
- `row`, `col`, `stack`, `scroll`, and `container`;
- `text`, `input`, `button`, `checkbox`, and `image`;
- `<->` binding, `->` routing, `_` event payloads, and scoped `#id`;
- typed extern calls and basic async success/failure routing.

A feature belongs in Core only when it is common UI authoring, has one clear
canonical representation, and cannot be handled cleanly by an existing typed
boundary.

### Extension and compatibility surface

Keep advanced existing support working, but do not grow it merely to match
Iced:

- Canvas path and event DSLs;
- complete PaneGrid mutation;
- raw window values, screenshots, and platform handles;
- shaders, custom renderers, overlays, and custom widgets;
- stream, sip, abortable task, and task-composition variants;
- one-to-one mirrors of Iced geometry/value types;
- exhaustive per-widget status style fields.

Prefer an existing typed `component`, `task`, `subscription`, `stream`, or
other Rust boundary for these cases. Do not delete accepted syntax without a
separate compatibility decision and migration path.

## Style system reset

The current language breaks its “one canonical construct” rule because a
single property may be expressed through a preset, a Tailwind-like utility,
and a native property/status field.

Target ownership is:

| Concern | Canonical form |
| --- | --- |
| layout and geometry | typed properties such as `width=`, `padding=`, `spacing=`, and `radius=` |
| colors, font emphasis, and design tokens | checked semantic `@` utilities such as `@bg-surface`, `@text-muted`, and `@font-bold` |
| built-in widget appearance | fixed native `style=` presets |
| reusable or complex stateful appearance | typed Rust style or component callbacks |
| unusual native styling | a typed Rust style/component boundary |

The audit found that top-level `preset` configures boot and state; it is not a
visual reuse mechanism. Do not add a new `recipe` or `variant` declaration yet:
use the existing typed callback boundary until concrete repetition proves a
language construct is needed. Do not keep geometry utilities such as
`@w-full`, `p-4`, or `rounded-lg` as a second permanent form.

The migration must be compatible:

1. Inventory which semantic property every existing utility, property, preset,
   and status field writes.
2. Make duplicate ownership deterministic, then have the checker reject a
   widget that writes the same property through competing forms.
3. Let the formatter emit the canonical form where rewriting is unambiguous.
4. Deprecate redundant forms before removing them; do not rewrite all examples
   and grammar in one change.

## Current state

- `README.md`, `SPEC.md`, and `COVERAGE.md` describe language version 1.58.
- Language revision 1.58, Cargo package SemVer `0.1.0`, and the resolved iced
  baseline intentionally use separate schemes, as documented in `SPEC.md`.
- The workspace resolves `iced 0.14.0` and `iced_widget 0.14.2`.
- The Core parser, checker, and code generator are split into responsibility-
  focused modules. Preserve those module boundaries instead of recreating the
  former monolithic files.
- Core end-to-end `format`, `diagnostic`, and `compile` fixtures under
  `crates/ui-lang-core/tests/cases` are auto-discovered by the Rust test runner.
- Thirty-three typed Rust boundaries already provide broad escape hatches.
- The style ownership audit is recorded beside `check_styles`. `E045` rejects
  every currently proven same-builder overlap: direct container/scroll/input
  geometry, row/column/grid spacing and alignment, text size and rich-text
  color, container and pane surfaces, and input/button status surfaces.
- Callback and fixed-preset styles remain valid base layers. Row, column, grid,
  and stack geometry utilities that style an outer wrapper remain valid
  composition rather than being rejected by a generic width rule.
- The formatter shares the parser's string- and delimiter-aware tokenization.
  It canonicalizes only proven same-builder properties, including text size,
  while preserving routes, quoted markers, semantic tokens, wrapper-only
  operations, and legacy forms whose typed spelling would change Iced defaults.
- File-backed diagnostics include the resolved source line and caret, including
  errors remapped into imported fragments.
- Semantic analysis returns the nominal, backend-neutral `CheckedDocument`;
  only the checker constructs it, and code generation accepts no unchecked
  `Document`.
- `cargo ice schema` emits a generative Core model with contexts, syntax, child
  shapes, typed properties, bindings, and routes. LSP completion derives from
  the same construct table.
- `cargo ice lsp` provides full-document stdio synchronization, UTF-16
  diagnostics over an unsaved root plus disk imports, imported-file diagnostic
  routing, whole-document formatting, and Core completion. Imported-buffer
  overlays, definition, and rename are explicitly unsupported.
- `cargo ice compat` analyzes app graphs, checks exact lockfile and manifest
  pins, and runs the reference app tests. Generated applications declare the
  exact `ui-lang-runtime = "=0.1.0"` dependency directly.
- Accepted `text-transparent` and `text-TOKEN/NN` utilities lower instead of
  being silently dropped by code generation.
- The task demo is split into app, state, backend, theme, handler, and component
  files. Keep it as the human-readable example; keep exhaustive features in
  focused fixtures or the showcase.

## Accessibility boundary

The delivered Core contract is intentionally narrow:

- `text`, `input`, `button`, `checkbox`, and labeled `image` lower to AccessKit
  `Label`, `TextInput`/`PasswordInput`, `Button`, `CheckBox`, and `Image` roles;
- `label=` and `description=` are checked `str` expressions. Positional input,
  compact button, and checkbox text provide default names; child-content
  buttons require `label=`, unlabeled images are decorative, and media
  descriptions require a label;
- password inputs never export their value;
- source/view-tree order controls reading and Tab/Shift+Tab focus. Disabled
  controls are skipped, buttons activate with Enter or Space, checkboxes with
  Space, and focused wrappers draw a visible outline. Do not add numeric focus
  order.

`ui-lang-runtime` builds the same deterministic tree and action map on every
target. Its native screen-reader adapter is Linux-only, uses AccessKit/AT-SPI,
and supports one window. Stock Iced does not expose enough window-scoped state
for daemon or multi-window adapters, nor the desktop transform for exact
screen-coordinate bounds. Non-Linux builds keep deterministic semantics but do
not export them to a native screen reader. Do not imply semantics for rich text
or advanced widgets until each receives its own checked contract and evidence.

`examples/iced-app/src/ui/accessibility.ice` and its headless test cover the Core
roles and state. `scripts/a11y-smoke.sh` is the Linux native gate: it discovers
the AT-SPI tree and delivers an invoked action to the Iced bridge.

## Remaining compatibility boundaries

Do not add another Iced value type. The current lowerer's proven same-builder
collisions are covered, and the parser-aware formatter migrates only exact
ownership-equivalent forms. Keep checker hints for conflicting source and do
not broaden rewriting beyond mappings whose generated owners are proven equal.

The remaining geometry utilities are deliberate compatibility boundaries:

| Utilities | Why they remain accepted |
| --- | --- |
| `w-full`, `h-full` | `w-full` is compatibility-only on direct scroll/container/input builders and `h-full` on scroll/container when used alone; row/column/grid use them on an outer wrapper, while stack intentionally applies them to both its stack and wrapper. |
| `max-w-sm` through `max-w-2xl` | Compatibility-only on a direct container when used alone; on layouts they constrain the generated outer wrapper, including when a column also has an inner `max-width=`. |
| `gap-*` | Compatibility-only spelling for row/column/grid `spacing=` when used alone. |
| `p-*`, `px-*`, `py-*` | Compatibility-only where row/column/container scalar or sided padding can express the same value; grid/stack padding belongs to the outer wrapper, and axis-specific input/button padding has no equivalent typed property. |
| `items-center` | Compatibility-only spelling for row/column `align=center`. |
| `self-center` | Intentional outer-wrapper operation; there is no typed form. |
| `border`, `border-2`; `rounded-sm`, `rounded`, `rounded-md`, `rounded-lg`, `rounded-full` | Compatibility-only when a direct container/pane or typed input/button status owns the same field; border remains available for layout wrappers and input surfaces, while radius remains available for layout wrappers and input/button surfaces with no top-level typed radius field. |

Do not reject these families more broadly without first proving that both forms
write the same generated builder. In particular, preserve callback/preset base
layers, `font=` plus `@font-bold`, and semantic color/token utilities.

Do not create a second backend to justify the checked boundary. Core remains
backend-neutral without adding Iced-specific mirror types.

## Verification

Run the gates already documented by the project:

```bash
cargo ice fmt --check
cargo ice check
cargo ice clippy
cargo test --workspace
cargo run -p iced-app
scripts/a11y-smoke.sh # Linux native adapter
```

For a focused parser/checker change, run the smallest relevant crate test first,
then the full gates before delivery.

## Definition of success

Ice succeeds when ordinary Ducktape screens are shorter and clearer than their
Rust/Iced equivalent, agents reliably emit the same source form, errors point
to the responsible `.ice` code, and uncommon UI behavior can escape to typed
Rust without expanding Core.
