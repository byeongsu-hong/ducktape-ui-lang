# iced coverage ledger

This ledger defines what “Ice covers iced” means. The baseline is the versions
resolved by this workspace: `iced 0.14.0` and `iced_widget 0.14.2`.

- **native**: accepted Ice syntax is parsed, type-checked, lowered, and compiled
  by the reference application or a focused test.
- **partial**: a useful subset exists, but the public iced feature is not fully
  expressible.
- **missing**: there is no accepted Ice representation yet.

An internal use of an iced widget does not count as coverage. For example, the
backend may wrap layouts in `container`, but only explicit accepted Ice syntax
counts toward the row below.

## Typed system reachability

Ice 1.20 has twenty-eight checked Rust boundaries:

| Boundary | Rust ABI | Covers |
| --- | --- | --- |
| `name(args)` | `async fn(...) -> Output` or `Result<Output, Error>` | domain I/O and arbitrary futures through native `Task::perform` |
| `component name(args)` | `fn(...) -> Element<'static, Event>` | any owned default-renderer widget tree, including custom widgets |
| `shader name(args)` | `fn(...) -> impl shader::Program<Event>` | native wgpu primitives, pipeline/storage, state, events, redraw, capture, and mouse interaction |
| `task name(args)` | `fn(...) -> Task<Event>` or `Task<Result<Event, Error>>` | widget/window/clipboard/font/system operations and arbitrary task composition |
| `stream name(args)` | `fn(...) -> impl Stream<Item = Event>` or `Stream<Item = Result<Event, Error>>` | native repeated `Task::run` output and `Subscription::run`/`run_with` workers from channels, iterators, async generators, and other streams |
| `sip name(args)` | `fn(...) -> impl Sipper<Output, Progress>` or `Straw<Output, Progress, Error>` | native repeated progress plus one final output through `Task::sip` |
| `recipe name(args)` | `fn(...) -> impl Recipe<Output = Event>` | custom subscription identity, runtime-event input, streams, cancellation, and arbitrary recipe behavior through native `from_recipe` |
| `event-filter name()` | `fn(subscription::Event) -> Option<Event>` | native raw runtime-event filtering with an explicit hashable identity, including interaction window IDs/status and system-theme changes |
| `sync name(args)` | `fn(...) -> Output` | checked synchronous domain conversions usable in Ice expressions |
| `subscription name(args)` | `fn(...) -> Subscription<Event>` | event, keyboard, mouse, window, system, channel, timer, stream, and custom subscription sources |
| `window name(args)` | `fn(&dyn iced::window::Window, ...) -> Output` | exact typed access to native window/display handles and other callback-only window behavior through `window::run` |
| `markdown-viewer name(args)` | `fn(...) -> impl for<'a> markdown::Viewer<'a, Event>` | native custom rendering of every Markdown item through `view_with` while preserving checked link-event routing |
| `editor-binding name(args)` | `fn(text_editor::KeyPress, ...) -> Option<text_editor::Binding<Event>>` | native custom key mapping across every built-in Binding plus typed custom application routes |
| `editor-highlighter name(args)` | generic adapter from plain `TextEditor` to default `Element` | native `highlight_with` access to arbitrary Highlighter settings, highlights, iterators and Theme-aware formats |
| `editor-style name(args)` | `fn(&Theme, text_editor::Status, ...) -> text_editor::Style` | native theme/status-aware runtime editor style callbacks, equivalent to the default Theme's advanced class representation |
| `text-style name(args)` | `fn(&Theme, ...) -> text::Style` | native theme-aware runtime text and rich-text style callbacks, equivalent to the default Theme's advanced class representation |
| `slider-style name(args)` | `fn(&Theme, slider::Status, ...) -> slider::Style` | native theme/status-aware runtime slider style callbacks, equivalent to the default Theme's advanced class representation |
| `progress-style name(args)` | `fn(&Theme, ...) -> progress_bar::Style` | native theme-aware runtime progress style callbacks, equivalent to the default Theme's advanced class representation |
| `button-style name(args)` | `fn(&Theme, button::Status, ...) -> button::Style` | native status-aware runtime button style callbacks, equivalent to the default Theme's advanced class representation |
| `checkbox-style name(args)` | `fn(&Theme, checkbox::Status, ...) -> checkbox::Style` | native checked/status-aware runtime checkbox style callbacks, equivalent to the default Theme's advanced class representation |
| `toggler-style name(args)` | `fn(&Theme, toggler::Status, ...) -> toggler::Style` | native checked/status-aware runtime toggler style callbacks, equivalent to the default Theme's advanced class representation |
| `radio-style name(args)` | `fn(&Theme, radio::Status, ...) -> radio::Style` | native selection/status-aware runtime radio style callbacks, equivalent to the default Theme's advanced class representation |
| `container-style name(args)` | `fn(&Theme, ...) -> container::Style` | native theme-aware runtime container style callbacks, equivalent to the default Theme's advanced class representation |
| `svg-style name(args)` | `fn(&Theme, svg::Status, ...) -> svg::Style` | native theme/status-aware runtime SVG style callbacks, equivalent to the default Theme's advanced class representation |
| `input-style name(args)` | `fn(&Theme, text_input::Status, ...) -> text_input::Style` | native theme/status-aware runtime text-input style callbacks, equivalent to the default Theme's advanced class representation |
| `scroll-style name(args)` | `fn(&Theme, scrollable::Status, ...) -> scrollable::Style` | native theme/status-aware runtime scrollable style callbacks, equivalent to the default Theme's advanced class representation |
| `pick-list-style name(args)` | `fn(&Theme, pick_list::Status, ...) -> pick_list::Style` | native theme/status-aware runtime pick-list style callbacks, equivalent to the default Theme's advanced class representation |
| `menu-style name(args)` | `fn(&Theme, ...) -> menu::Style` | native theme-aware runtime pick-list/combo overlay menu callbacks, equivalent to the default Theme's advanced class representation |

Generated probes verify the concrete Rust signatures. Reachability is not the
same as native coverage: a row stays partial or missing until its complete
public behavior has direct documented Ice syntax and tests.

## Widgets and layout

| iced surface | Ice status | Current representation / missing work |
| --- | --- | --- |
| `button` | native | native string or arbitrary child content, disabled route, typed size/padding/clip, all eight iced presets, every concrete field across all four statuses including linear backgrounds, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `canvas` | native | declarative rectangle/circle/line/text/path geometry; complete path builder segments, fill rules, solid/linear fill and stroke, caps/joins/dashes, transforms, clips, typed `if`/`for`, complete raster/SVG frame drawing fields, dependency-keyed geometry cache with shared named groups, typed local `Program::State`, all five event families and every variant, state updates, publish/capture/next-frame/timed-redraw actions, pointer routes, and static/state-dependent/out-of-bounds interaction cover the complete public Program behavior |
| `checkbox` | native | native label/value/disabled event, size/width/spacing, text typography/wrapping, complete font descriptors and custom icon; all four presets, every concrete Style field across active/hovered/disabled checked and unchecked statuses, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `column` | native | children, typed spacing/per-side padding, all `Length` bounds, max width, cross-axis alignment, clipping and wrapping column spacing/alignment |
| `combo_box` | native | native typed replaceable and incrementally pushable search state/selection, every builder setter, complete text-input icon, every concrete input Style field across active/hovered/focused/focused-hovered/disabled statuses, complete menu overlay Style fields, typed native input/menu style callbacks, and all events |
| `container` | native | native one-child container with ID, complete concrete layout API, every concrete Style field including linear background, text, per-corner border, shadow and pixel snap, plus typed theme-aware runtime callbacks covering the default Theme's advanced classes |
| `float` | native | one child, positive scale, all original-bounds and viewport geometry exposed as scoped f64 translation inputs, and every concrete Style field through checked shadow color/offset/blur and per-corner shadow radius |
| `grid` | native | dynamic children, pixel spacing/width, fixed or fluid columns, aspect-ratio or all `Length` height modes |
| `image` | native | path, encoded-memory and RGBA handles; all four iced length variants, fit, filter, floating/solid rotation, opacity, scale, expand, per-corner radius and crop cover the complete concrete widget API |
| `image::Viewer` | native | path or memory/RGBA handle, all length and fit modes, both filters, padding, minimum/maximum scale and scale step cover the complete public builder API |
| `keyed` | native | typed list template with bool/i64/f64 identity keys, automatic keyed child scopes, spacing/per-side padding/all `Length` bounds, max width and alignment |
| `lazy` | native | hash-keyed rebuilds with bool/i64/str, `Hash + Clone` extern values, recursive list/optional dependencies, a dependency-only scope and statically enforced owned `Element<'static>` subtrees |
| `markdown` | native | owned parsed/replaced/incrementally appended content, image URI access, syntax highlighting, every `Settings` and `Style` field, str link events, and a typed custom `Viewer` boundary covering every item renderer through native `view_with` |
| `mouse_area` | native | all button/enter/move/scroll/exit events, scroll unit preservation, and all cursor interactions |
| `overlay` | partial | structured content/layer sections, conditional visibility, all three alignments on both axes, padding, checked backdrop color, modal button/scroll blocking and backdrop dismissal lower through native Stack/Float overlay behavior; arbitrary custom Overlay implementations and z-index remain |
| `pane_grid` | partial | recursive initial split trees, closed templates, declared dynamic splits, bounds, click, interactive resize/drag, maximize/query, adjacency, swap, close, move-to-edge, root resize and region drop; native Content/TitleBar, full and responsive compact Controls, per-side title padding and visibility; every concrete PaneGrid Style field including linear hovered backgrounds; every concrete Content/TitleBar container Style field including linear background, per-corner border, shadow and pixel snap; runtime-generated pane templates, named nested-split resize and advanced classes remain |
| `pick_list` | native | native typed choices/optional selection, every builder setter, all arrow/static/dynamic/none handles, every concrete Style field across active/hovered/opened/opened-hovered statuses, complete menu overlay Style fields, and typed native field/menu callbacks covering the default Theme's advanced classes |
| `pin` | native | one child, all `Length` bounds and pixel x/y positioning; x/y is behaviorally identical to iced's `position(Point)` helper |
| `progress_bar` | native | native range/value, all length/girth variants, horizontal/vertical, five presets, checked solid/linear track and bar backgrounds, border and per-corner radius, plus typed theme-aware runtime style callbacks covering the default Theme's advanced classes |
| `qr_code` | native | named UTF-8 or arbitrary byte data, all correction levels and normal/micro versions, cell/total size, and checked cell/background colors |
| `radio` | native | native bool/i64/f64/str/extern payload values, explicit bool selection, complete sizing/typography/font setters, every concrete Style field across active/hovered selected/unselected statuses, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `responsive` | native | arbitrary size-dependent child tree with scoped width/height bindings, breakpoint sugar and all `Length` bounds |
| `row` | native | children, typed spacing/per-side padding, all `Length` bounds, cross-axis alignment, clipping and wrapping row spacing/alignment |
| `rule` | native | axis/thickness, every fill mode, default/weak presets, checked color/opacity, per-corner radius and snap cover all concrete style fields; advanced classes are an alternate extension mechanism |
| `scrollable` | native | native content/ID, every concrete builder setter, all Viewport getters, every Status field through ordered selectors, every concrete Style field for container, rails, scrollers, gap and auto-scroll overlay, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `sensor` | native | show/resize dimensions, hide, comparable owned keys, anticipation and delay; owned keys provide the same continuity behavior as `key_ref` without borrowed lifetimes |
| `shader` | native | typed factory for any concrete native `shader::Program<Event>`, complete width/height builder API, checked message routing, and generated Program/Element probes; the Rust program retains complete State, Primitive, Pipeline/Storage, update/action, draw and mouse-interaction behavior |
| `slider` | native | native f64 or arbitrary typed extern numeric values with Rust-verified iced Slider bounds; complete default/normal+shift step, sizing and change/release behavior; every concrete Style field across active/hovered/dragged including solid/linear rail and handle backgrounds, border/per-corner radius and circle/rectangle handles; typed theme/status-aware runtime callbacks cover advanced classes |
| `space` | native | optional fixed/fill/fill-portion/shrink width and height cover the complete widget API |
| `stack` | native | ordered children, all `Length` widths/heights, clipping and `push_under` base-layer behavior via `under=N` |
| `svg` | native | native path or UTF-8/raw byte memory source, all four iced length variants, fit, rotation, opacity, complete idle/hovered color style, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `table` | native | typed cloned rows, arbitrary header/cell subtrees, automatic row/column identity scopes, all table width/padding/separator setters and all column width/alignment setters |
| `text` | native | native string/numeric text plus structured rich spans; complete Text/Rich bounds, size, relative/absolute line height, font, alignment, wrapping and color, plus Text shaping and Rich str link events; every concrete Span field including solid/linear highlight background, border/per-corner radius/padding/underline/strike; typed theme-aware runtime callbacks cover the default Theme's advanced classes |
| `text_editor` | native | app-owned direct or component-prop content, generated action application, every concrete builder setter, all five built-in themes, typed arbitrary native Highlighter adapters, complete native key bindings with custom routed payloads, every concrete Style field across all statuses, and typed Theme/Status callbacks covering advanced classes |
| `text_input` | native | app-owned direct or component-prop binding, ID, every concrete builder setter, complete custom icon, every concrete Style field across active/hovered/focused/focused-hovered/disabled statuses, and typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `themer` | partial | native default/app/all 22 built-in themes, checked default text color and solid/linear background; arbitrary alternate Theme types missing |
| `toggler` | native | native label/value/disabled event, size/width/spacing, text typography/wrapping/alignment and complete font descriptors; every concrete Style field across active/hovered/disabled checked and unchecked statuses, plus typed theme/status-aware runtime callbacks covering the default Theme's advanced classes |
| `tooltip` | native | native two-child content, all positions, gap, padding, viewport snap, delay, nine container presets, every concrete container Style field, and checked `container-style` callbacks covering the default Theme's advanced classes |

## Application and runtime

| iced surface | Ice status | Current representation / missing work |
| --- | --- | --- |
| application settings | native | state-dependent title, all built-in/custom theme selection, base background/text style and guarded scale-factor callbacks; application ID, custom typed executor, ordered checked font byte preloads, default text size/font, antialiasing, vsync, codec-free checked RGBA icons, complete initial/named window settings including structured Linux, Windows, macOS, and Wasm fields, structured state/task boot presets and run |
| `Theme` and styles | partial | checked color tokens and a Tailwind-like subset; native theme/style catalogs and custom closures missing |
| `Task` | partial | async/sync externs, typed arbitrary iced `Task` adapters, direct system/clipboard/font/widget/window tasks, nested batch/chain groups, complete abortable handles, repeated `run` streams, typed `sip`, and native typed flows with direct `done`/`none`, output-dependent `then`, optional-or-result `and_then`, `map_err`, result-preserving `collect`, `discard`, and `units`; low-level task-module `oneshot`/`channel`/blocking/effect constructors remain adapter-only |
| `Subscription` | partial | typed arbitrary iced `Subscription` adapters, batching, checked conditional activation/status filters, direct every/repeat timers, native `listen`/`listen_with`/`listen_raw` generic events, input-method/keyboard/mouse/touch/window sources (with optional typed IDs on all eleven discrete window events) and system theme changes, native typed `run`/`run_with` workers, custom `Recipe` factories through `from_recipe`, raw `EventStream` filters with hashable identity, plus `with` identity context and noncapturing typed `filter_map` transforms on every source; direct recipe extraction remains runtime-only |
| widget operations | partial | all 13 core focus/cursor/selection/scroll operations with checked static IDs, direct repeated `i64`/`str` dynamic IDs, and typed focus query; nested component/keyed/table/pane scopes and feature-gated selectors remain |
| clipboard | native | standard and primary read/write tasks; reads preserve iced's optional string payload and writes are checked fire-and-forget effects |
| fonts | native | ordered app-level relative font files are checked and embedded into iced's startup loader; runtime bytes lower to native `font::load`; every family/weight/stretch/style descriptor, checked named reference, application default and all widget font setters are covered |
| system | native | current theme task, theme-change subscription, and every information field with optionality preserved; information requires iced's `sysinfo` feature |
| time | native | `instant` maps to iced's native monotonic value; `task time now`, payload-producing `every`, and typed async `repeat` cover the complete enabled `iced::time` task/subscription API with checked positive `ms`/`s` durations (`repeat` requires iced's `tokio` feature) |
| window | native | every initial and named-open setting, including codec-free RGBA icons and structured Linux/Windows/macOS/Wasm fields; typed `window-id`, open/oldest/latest, direct targeting for every per-window close/drag/resize/constraints/state/move/mode/focus/level/menu/attention/passthrough/monitor/raw-ID/screenshot/icon task, automatic tabbing, lossless RGBA screenshot payloads, all 12 event forms with optional IDs on all 11 discrete events, and an exact typed `window::run` callback boundary for raw window/display handles |
| event routing | native | all five structured families plus first-class generic `event` values through native `listen`/`listen_with`/`listen_raw`, optional window IDs, status filters, transforms, handler routing, and typed extern passage; system-theme runtime events remain a separate native source because iced does not represent them as `iced::Event` |
| keyboard | partial | direct subscriptions cover press, release and modifier-change events with logical/modified/physical key, location, text, repeat and every modifier query; typed key enums, constructors, matching and latin translation remain |
| mouse/touch | partial | every mouse and touch event has a direct typed subscription; native mouse-area events and all cursor interactions are covered; low-level `Cursor` and `Click` construction remain custom-widget concerns |
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
