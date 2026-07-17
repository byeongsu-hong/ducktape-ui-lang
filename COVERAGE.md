# iced coverage ledger

This ledger defines what “Ice covers iced” means. The baseline is the versions
resolved by this workspace: `iced 0.14.0` and `iced_widget 0.14.2`.

- **native**: accepted Ice syntax is parsed, type-checked, lowered, and compiled
  by the reference application or a focused test.
- **partial**: a useful subset exists, but the public iced feature is not fully
  expressible.
- **missing**: there is no accepted Ice representation yet.

An internal use of an iced widget does not count as coverage. For example, the
backend wraps layouts in `container`, but Ice cannot yet express the full
container API, so container remains partial.

## Typed system reachability

Ice 0.5 has three checked Rust boundaries:

| Boundary | Rust ABI | Covers |
| --- | --- | --- |
| `extern name(args)` | `fn(...) -> Element<'static, Event>` | any owned default-renderer widget tree, including custom widgets |
| `task name(args)` | `fn(...) -> Task<Event>` or `Task<Result<Event, Error>>` | widget/window/clipboard/font/system operations and arbitrary task composition |
| `subscribe` | `fn(...) -> Subscription<Event>` | event, keyboard, mouse, window, system, channel, timer, stream, and custom recipe sources |

Generated probes verify the concrete Rust signatures. Reachability is not the
same as native coverage: a row stays partial or missing until its complete
public behavior has direct documented Ice syntax and tests.

## Widgets and layout

| iced surface | Ice status | Current representation / missing work |
| --- | --- | --- |
| `button` | partial | native label, disabled route, basic styles; icon/content and full style API missing |
| `canvas` | missing | drawing program, geometry, cache, events |
| `checkbox` | partial | native label/value/disabled event; size/spacing/icon/style API missing |
| `column` | partial | native children, spacing, padding, size/alignment/basic decoration; full sizing API missing |
| `combo_box` | missing | state, selection, input events |
| `container` | partial | generated around layouts; explicit alignment, clipping, sizing, style API missing |
| `float` | missing | floating element and translation |
| `grid` | partial | native children, fixed column count, spacing; fluid columns and height sizing missing |
| `image` | partial | native path, fixed/fill/shrink length, fit, filter, rotation, opacity, scale, expand and radius; memory handles and crop missing |
| `keyed` | partial | scoped IDs exist; keyed column diffing is not exposed |
| `lazy` | missing | lazy/cache boundary |
| `markdown` | missing | parsing/settings/link events |
| `mouse_area` | partial | native button/enter/exit routes and all cursor interactions; move and scroll payload routes missing |
| `overlay` | missing | modal/overlay positioning and dismissal |
| `pane_grid` | missing | pane state, resizing, dragging, focus |
| `pick_list` | partial | native typed choices/optional selection, placeholder, sizing, padding, text size, open/close events; font, shaping, handle and style catalogs missing |
| `pin` | missing | absolute pinning |
| `progress_bar` | partial | native range/value and horizontal/vertical; style API missing |
| `qr_code` | missing | data, cell size, theme |
| `radio` | partial | native bool/i64 values and selection event; generic values/style API missing |
| `responsive` | missing | size-dependent view function |
| `row` | partial | native children, spacing, padding, size/alignment/basic decoration; full sizing API missing |
| `rule` | partial | native axis/thickness; fill/style API missing |
| `scrollable` | partial | native one-child scroll and ID; direction, scrollbar, snap and scroll events missing |
| `sensor` | missing | layout resize event |
| `shader` | missing | custom GPU primitive/program |
| `slider` | partial | native f64 range/value/step/change/release and horizontal/vertical; shift-step/style API missing |
| `space` | partial | native fixed width/height; fill and shrink lengths missing |
| `stack` | partial | native children, clip, size/basic decoration; alignment and full sizing API missing |
| `svg` | partial | native path, fixed/fill/shrink length, fit, rotation and opacity; memory handles and status style missing |
| `table` | missing | columns, headers, rows, sizing |
| `text` | partial | native string/numeric value, size/color/bold; wrapping, shaping, alignment, font and rich spans missing |
| `text_editor` | missing | content state, actions, highlight, key bindings |
| `text_input` | partial | native string binding, hint, disabled, ID and basic style; submit/paste/secure/icon/alignment missing |
| `themer` | missing | nested theme boundary |
| `toggler` | partial | native label/value/disabled event; size/spacing/style API missing |
| `tooltip` | partial | native two-child content, all positions, gap, padding, snap and delay; custom tooltip style missing |

## Application and runtime

| iced surface | Ice status | Current representation / missing work |
| --- | --- | --- |
| application settings | partial | generated title/theme/run; window, fonts, antialiasing, executor, scale and presets missing |
| `Theme` and styles | partial | checked color tokens and a Tailwind-like subset; native theme/style catalogs and custom closures missing |
| `Task` | partial | async externs and typed arbitrary iced `Task` adapters; direct batch, chain, stream, cancellation and progress syntax missing |
| `Subscription` | partial | typed arbitrary iced `Subscription` adapters and batching; direct source/combinator syntax missing |
| widget operations | missing | focus, cursor selection, scroll and selector operations |
| clipboard | partial | typed Task adapter with write exercised; direct read/write syntax and `Option` payload missing |
| fonts | missing | font loading and discovery |
| system | missing | system information query |
| window | missing | settings, open/close, multiple windows, resize/move/mode/focus/screenshot/monitor operations |
| event routing | partial | raw event subscription adapter exercised; native event/status types missing |
| keyboard | missing | key/modifier events and subscriptions |
| mouse/touch | partial | native mouse button/enter/exit and all cursor interactions; move/scroll payloads, raw mouse and touch types missing |
| custom widget | partial | typed owned `Element<'static, Event>` adapter; borrowed elements and custom Theme/Renderer missing |
| custom renderer | missing | renderer/graphics backend escape hatch |

## Evidence rule

A row moves to **native** only when every public behavior in the pinned iced
surface has:

1. documented Ice syntax and static types;
2. parser and semantic-checker coverage, including invalid input;
3. generated Rust that compiles against the pinned iced release;
4. a reference or focused runtime example when behavior is interactive.

The repository does not claim complete iced coverage while any row is partial
or missing.
