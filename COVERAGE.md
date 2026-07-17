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

Ice 0.32 has three checked Rust boundaries:

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
| `button` | partial | native string or arbitrary child content, disabled route, typed size/padding/clip and basic styles; full style catalog missing |
| `canvas` | missing | drawing program, geometry, cache, events |
| `checkbox` | partial | native label/value/disabled event, size/width/spacing, text typography/wrapping/default-mono font and custom icon; arbitrary fonts and full style catalog missing |
| `column` | native | children, typed spacing/per-side padding, all `Length` bounds, max width, cross-axis alignment, clipping and wrapping column spacing/alignment |
| `combo_box` | partial | native typed search state/selection, input, hover, open/close, sizing, padding and text size; dynamic option replacement, icon/font/shaping and style catalogs missing |
| `container` | partial | generated around layouts; explicit alignment, clipping, sizing, style API missing |
| `float` | partial | native scale and fixed x/y translation; viewport-aware translation closure and style API missing |
| `grid` | native | dynamic children, pixel spacing/width, fixed or fluid columns, aspect-ratio or all `Length` height modes |
| `image` | partial | native path, all four iced length variants, fit, filter, rotation, opacity, scale, expand and radius; memory handles and crop missing |
| `keyed` | native | typed list template with bool/i64/f64 identity keys, automatic keyed child scopes, spacing/per-side padding/all `Length` bounds, max width and alignment |
| `lazy` | native | hash-keyed rebuilds with bool/i64/str, `Hash + Clone` extern values, recursive list/optional dependencies, a dependency-only scope and statically enforced owned `Element<'static>` subtrees |
| `markdown` | partial | owned parsed/replaced content, syntax highlighting, every `Settings` size/spacing field and str link events; incremental append, image URI access, full `Style` and custom `Viewer` remain |
| `mouse_area` | native | all button/enter/move/scroll/exit events, scroll unit preservation, and all cursor interactions |
| `overlay` | missing | modal/overlay positioning and dismissal |
| `pane_grid` | missing | pane state, resizing, dragging, focus |
| `pick_list` | partial | native typed choices/optional selection, placeholder, sizing, padding, text size, open/close events; font, shaping, handle and style catalogs missing |
| `pin` | native | one child, all `Length` bounds and pixel x/y positioning; x/y is behaviorally identical to iced's `position(Point)` helper |
| `progress_bar` | partial | native range/value, all length/girth variants, horizontal/vertical, five presets, checked color backgrounds, border and per-corner radius; gradients, arbitrary runtime closures and advanced classes missing |
| `qr_code` | native | named UTF-8 or arbitrary byte data, all correction levels and normal/micro versions, cell/total size, and checked cell/background colors |
| `radio` | partial | native bool/i64 values and selection event; generic values/style API missing |
| `responsive` | native | arbitrary size-dependent child tree with scoped width/height bindings, breakpoint sugar and all `Length` bounds |
| `row` | native | children, typed spacing/per-side padding, all `Length` bounds, cross-axis alignment, clipping and wrapping row spacing/alignment |
| `rule` | native | axis/thickness, every fill mode, default/weak presets, checked color/opacity, per-corner radius and snap cover all concrete style fields; advanced classes are an alternate extension mechanism |
| `scrollable` | partial | native content/ID, all directions, bounds, scrollbar geometry/visibility/spacing, anchors, auto-scroll and absolute/relative offset events; viewport bounds/reversed offsets and style catalog missing |
| `sensor` | native | show/resize dimensions, hide, comparable owned keys, anticipation and delay; owned keys provide the same continuity behavior as `key_ref` without borrowed lifetimes |
| `shader` | missing | custom GPU primitive/program |
| `slider` | partial | native f64 behavior/sizing plus nested active/hovered/dragged styles covering color rail/background/border/radius and circle/rectangle handles; generic numeric values, gradient backgrounds, arbitrary runtime closures and advanced classes missing |
| `space` | native | optional fixed/fill/fill-portion/shrink width and height cover the complete widget API |
| `stack` | native | ordered children, all `Length` widths/heights, clipping and `push_under` base-layer behavior via `under=N` |
| `svg` | partial | native path, all four iced length variants, fit, rotation and opacity; memory handles and status style missing |
| `table` | native | typed cloned rows, arbitrary header/cell subtrees, automatic row/column identity scopes, all table width/padding/separator setters and all column width/alignment setters |
| `text` | partial | native string/numeric value, bounds, relative/absolute line height, alignment, shaping, wrapping, default/mono font, color and bold; arbitrary font attributes, rich spans and full style catalog missing |
| `text_editor` | partial | owned/replaced app state, generated action application, ID, placeholder, width/height/min/max, typography, padding, wrapping, disabled mode and all five built-in highlight themes; component-owned bindings, custom key binding/highlighter and full status styles remain |
| `text_input` | partial | native binding, hint, disabled/secure, ID, submit/paste, typed width/padding/size/line-height, alignment, default/mono font, icon and basic style; arbitrary fonts and full style catalog missing |
| `themer` | partial | native default/app/all 22 built-in themes, checked default text color and solid background; gradient backgrounds and arbitrary alternate Theme types missing |
| `toggler` | partial | native label/value/disabled event, size/width/spacing, text typography/wrapping/alignment and default-mono font; arbitrary fonts and full style catalog missing |
| `tooltip` | partial | native two-child content, all positions, gap, padding, viewport snap, delay, nine container presets, checked color/background/border/per-corner radius/shadow/pixel-snap styles; gradient backgrounds, arbitrary runtime closures and advanced classes missing |

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
| mouse/touch | partial | native mouse-area events and all cursor interactions; raw mouse subscriptions and touch types missing |
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
