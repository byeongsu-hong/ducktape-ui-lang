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
| `image` | missing | raster handle, fit, filter, rotation, opacity |
| `keyed` | partial | scoped IDs exist; keyed column diffing is not exposed |
| `lazy` | missing | lazy/cache boundary |
| `markdown` | missing | parsing/settings/link events |
| `mouse_area` | missing | pointer/button/enter/exit/scroll events |
| `overlay` | missing | modal/overlay positioning and dismissal |
| `pane_grid` | missing | pane state, resizing, dragging, focus |
| `pick_list` | missing | choices, selection, open/close events |
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
| `svg` | missing | handle, fit, rotation, opacity, style |
| `table` | missing | columns, headers, rows, sizing |
| `text` | partial | native string/numeric value, size/color/bold; wrapping, shaping, alignment, font and rich spans missing |
| `text_editor` | missing | content state, actions, highlight, key bindings |
| `text_input` | partial | native string binding, hint, disabled, ID and basic style; submit/paste/secure/icon/alignment missing |
| `themer` | missing | nested theme boundary |
| `toggler` | partial | native label/value/disabled event; size/spacing/style API missing |
| `tooltip` | missing | position, gap, padding, snap and delay |

## Application and runtime

| iced surface | Ice status | Current representation / missing work |
| --- | --- | --- |
| application settings | partial | generated title/theme/run; window, fonts, antialiasing, executor, scale and presets missing |
| `Theme` and styles | partial | checked color tokens and a Tailwind-like subset; native theme/style catalogs and custom closures missing |
| `Task` | partial | one async extern call per handler; batch, chain, stream, cancellation, progress and arbitrary task adapters missing |
| `Subscription` | missing | time, event, keyboard, mouse, window, system, channel and custom recipe subscriptions |
| widget operations | missing | focus, cursor selection, scroll and selector operations |
| clipboard | missing | read/write and primary clipboard |
| fonts | missing | font loading and discovery |
| system | missing | system information query |
| window | missing | settings, open/close, multiple windows, resize/move/mode/focus/screenshot/monitor operations |
| event routing | missing | raw iced events and event status |
| keyboard | missing | key/modifier events and subscriptions |
| mouse/touch | missing | pointer, wheel, touch and interaction APIs |
| custom widget | missing | typed Rust `Element`/advanced `Widget` escape hatch |
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
