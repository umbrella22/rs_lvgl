# Host validation

The [WebAssembly acceptance script](https://github.com/umbrella22/rs_lvgl_website/blob/main/scripts/check-wasm.mjs) executes the compiled Rust/LVGL module, checks the categories in its pinned source, input, animation, independent instances and teardown/reinitialization. Run these website checks from a recursively initialized [rs_lvgl_website](https://github.com/umbrella22/rs_lvgl_website) checkout: build with `npm run wasm:build`, then run `npm run check-wasm -- --three-passes`. The [website build guide](https://github.com/umbrella22/rs_lvgl_website#build-from-a-clean-checkout) lists toolchains and dependency preparation. These checks complement visible browser acceptance; they do not validate hardware.


The workspace compiles vendored LVGL 9.6. Tests sharing its native global state run
serially. `cargo test-lvgl` aliases `cargo test -- --test-threads=1`.
Commands run from the repository root. C/C++, libclang and Cargo are prerequisites;
SDL paths additionally require SDL2. The native-input runner uses Python 3.12+.

## Supported checks

```sh
cargo build
cargo test -- --test-threads=1
cargo check -p rs_lvgl_ui --examples --features showcase,rust_timer
cargo check -p rs_lvgl --no-default-features --lib
cargo test -p rs_lvgl --no-default-features --features lvgl_alloc,rust_timer -- --test-threads=1
cargo test -p rs_lvgl_ui --features showcase,font-cjk-14,rust_timer -- --test-threads=1
cargo test -p rs_lvgl_ui --features diagnostics -- --test-threads=1
cargo test -p rs_lvgl_ui --features diagnostics,strict-properties -- --test-threads=1
cargo test -p rs_lvgl_ui --features showcase,diagnostics,strict-properties,update-metrics --test control_depth --test chart_depth --test custom_props -- --test-threads=1
cargo check -p rs_lvgl_ui --no-default-features --features alloc,diagnostics,strict-properties --lib
cargo test -p rs_lvgl_ui --no-default-features --features alloc,components -- --test-threads=1
python3 scripts/check-ui-api.py --artifacts target/ui-api-audit
python3 scripts/check-display-contracts.py --locked --artifacts target/display-contracts
# Development acceptance on unchanged inputs:
python3 scripts/check-display-contracts.py --locked --repeat-lifecycle 3 --artifacts target/display-contracts
```

`Cargo.lock` is ignored. Local receipts record its digest; `--locked --offline`
requires an existing resolved lockfile and cached dependencies. CI first resolves
dependencies, then uses `--locked` for subsequent checks. `--all-features` is not a
supported replacement for the matrix: some features require nightly or external
application hooks.

The [display checks](display.md) execute RGB888/RGB565/swapped pixel oracles across
all three render modes and both buffer counts, asynchronous completion, deletion,
logical invalidation granularity, clipped edge groups, native DIRECT sync regions,
period readback, RGB888 images and preserved ARGB Canvas behavior. They repeat
resource lifecycles when requested, exercise an 8-byte native stride/alignment
configuration, and compile positive/negative external `no_std` consumers. Calls
to native-byte access and flush completion without `unsafe` are rejected.
The dual-heap runner also covers the new retained display, Canvas and RGB888 image
resources. These are host contracts; physical DMA, FPS, panel VRR and safe areas
require separate device or UI acceptance.

The `rs_lvgl` workflow accepts `main` and `codex/**` pushes, PRs targeting
`main`, and manual dispatch. Workspace builds run on Ubuntu 24.04 and macOS 15. UI,
native-input and heap jobs run separately on Linux. Each complete validation
script runs once in CI; relevant development acceptance retains the repository
guide's repeated lifecycle/recovery checks. Artifact uploads preserve logs and studio
renders on failure as well as success. Local invocation does not establish that
a remote workflow has executed.

The complete studio has its own native interaction/class coverage and three-cycle
lifecycle test. `python3 scripts/render-studio.py` renders all categories without
a window server. `scripts/check-ui-examples.py` also runs its actual SDL window;
see [studio coverage](component-studio.md).

The public API runner compiles a separate `no_std` consumer using ordinary
constructors and `View`, including arrays, iterators and message mapping. It
checks root/prelude feature gates, keeps transitive native widgets out of the
typed API, and verifies that old associated constructors and safe calls to
`raw` fail for their intended Rust diagnostics.
It also checks safe `NativeObject` implementation rejection, an external adapter
delegating to a borrowed `Obj`, loaded-font unsafe transfer, loaded-image feature
gates, static/short-borrowed text compatibility, typed capability preservation
through styles/map, explicit heterogeneous conversion, binding input/mapper types
and the new `no_std` APIs. Development controls include range binding/handler types,
textarea input payload migration, chart axes/history/cursors and custom Props,
plus intentional failure of the removed chart/custom signatures.

On macOS, the native linker may need the SDL2 library search path explicitly:

```sh
LIBRARY_PATH="$(pkg-config --variable=libdir sdl2)" cargo test -- --test-threads=1
```

## Raw configuration and packaging

```sh
cargo test -p rs_lvgl_sys --test build_script -- --test-threads=1
python3 scripts/check-native-inputs.py --artifacts target/native-input-audit
python3 scripts/check-layout-config.py --artifacts target/layout-config-audit
```

The layout runner compiles and runs real wrapper consumers with neither layout,
Flex only, Grid only and both layouts, using C integer expressions in headers.
Disabled APIs must fail for their intended missing symbol. It disables native
widgets that themselves require Flex; this does not make the declarative UI a
layout-free runtime. Version 0.5.0 includes the target layout projection fix.

The native-input runner builds a raw-only application and checks C/Rust configuration agreement,
custom functions and a custom font descriptor. It changes a transitive configuration
header, a private C header and font source in place. Negative fixtures require the
intended capability/path/header failure. It also verifies unchanged-input cache reuse,
configuration switching, independent output directories, failed-build recovery,
interrupted C compilation and recovery, disabled/enabled generated widget APIs, and
an executable built against an unpacked `.crate`.
The loaded-resource consumer checks supported admission, rejects images and font
mounts with image/header caches enabled in an application header, then restores
the supported header in the same output directory.
It also disables RGB565 input drawing, checks explicit image rejection with fonts
still admitted, and restores the format in the same output directory.

The interruption fixture needs POSIX process groups and terminates only its own
compiler process group. Windows interruption acceptance is not implemented. Font
symbol linking is tested; the fixture font descriptor is not a glyph-rendering test.

The UI package runner separately packages and extracts all four workspace crates:

```sh
python3 scripts/check-ui-package.py --locked --offline --artifacts target/ui-package-audit
```

An isolated consumer resolves exact versions through patches solely to those
extracted archives, checks Cargo metadata against every extracted manifest and
builds fresh native outputs. Native readback verifies range values, authoritative
UTF-8 textarea text, chart history and same-instance custom Props with exactly-once
deletion. Archive/content, lock, executable and native artifact hashes bind the
receipt. `cargo package --no-verify` creates the archives; the isolated build/run
provides consumer evidence. No registry publication occurs. Existing UI manifest
example targets located outside that crate are omitted by Cargo's packaging rules;
the representative consumer verifies the packaged library path.

## Strict heap accounting

```sh
python3 scripts/check-ui-leaks.py --artifacts target/leak-audit
```

The runner selects [an isolated configuration](../rs_lvgl_ui/tests/leak-check/lv_conf.h)
with the LVGL BUILTIN allocator and an 8 MiB pool; it does not change production's
CLIB default. A Rust `GlobalAlloc` wrapper records successful allocations,
reallocations and frees. `lv_mem_monitor` supplies LVGL live bytes and block count.
CLIB's monitor cannot observe native allocations, so a separate negative run must
fail with `LEAK_UNOBSERVABLE_NATIVE`.

Each workload runs in a new subprocess. After one fixed priming cycle, every one
of 64 cycles must return to exactly the same live byte and block counts in both
domains. No tolerance, growing baseline or repeated warm-up is allowed. Workloads
cover:

- Display buffers and capturing flush callbacks.
- Keyed reconciliation, queued click messages and reorder/update/drain.
- Retained styles, native state transitions and explicit animations.
- Lazy conditional removal, typed/bound controls, mapped capturing handlers and
  ListItem/Panel/Dialog slots with body replacement and keyed reorders.
- Keypad registration, focus groups and modal restoration.
- Canvas resizing and snapshots that survive UI unmount.
- QR, barcode, GIF, Lottie and Pinyin individually, plus sequential replacement.
- Completed/interrupted screen transitions and unmount.
- Validation failures, failed Lottie setup and recovery.
- Loaded image sharing, static replacement, inherited fonts, native destructor
  before backing-data destruction, later-sibling creation/update failure and retry.
- Native event observer capture replacement and release.
- Editing filters/cursor/password updates and complete text correction.
- Chart history append/recovery, cursor retirement and native allocation failure.
- Custom Props replacement, equal-value correction, failed partial/initial applies,
  later-sibling failure, backing-data deletion order and remount.

The software renderer retains up to four radius-mask cache entries across displays.
Replacing entries can change their allocated size without increasing block count.
After every display is deleted and synchronous drawing is complete, the test calls
the pinned renderer's `lv_draw_sw_mask_cleanup` to release this known global cache.
It exercises cached drawing before cleanup; it does not disable rendering or forgive
a byte difference. The call is internal LVGL 9.6 test instrumentation and must be
reviewed if the native dependency changes.

The declared priming cycle includes library and test-runtime first-use allocations.
On the recorded macOS run, the Lottie and mixed-media cases retain an additional
184 native bytes / 4 blocks at that baseline; the other cases share the smaller
baseline. All later cycles must return exactly to their baseline. This test detects
accumulating lifecycle leaks after priming; it does not classify every allocation
retained by the first cycle as required or prove the absence of one-time retention.

Two sensitivity controls deliberately retain a one-byte Rust allocation and a
minimum LVGL allocation. Both must fail with the matching heap diagnostic.
`--skip-allocator-control` is available for local diagnosis and is not full acceptance.

The measured domains are the Rust global allocator and LVGL's BUILTIN pool.
Third-party libc/C++ allocations, including allocations made directly inside
ThorVG, are outside those counters. RSS, allocator fragmentation, sanitizers and
physical memory budgets are separate evidence. Three consecutive unchanged-input
passes are required for the repository readiness acceptance, not a universal proof
of leak freedom for every application or native configuration.

## Update costs

```sh
python3 scripts/check-ui-costs.py --artifacts target/update-costs/owned
python3 scripts/check-ui-costs.py --static-text --artifacts target/update-costs/static
# Record the same mode/configuration before and after a style API change:
python3 scripts/check-ui-costs.py --artifacts target/update-costs/after --compare target/update-costs/before/results.json
```

The test uses the same isolated BUILTIN configuration as heap accounting. A
64-node memory-display page runs 16 iterations of idle drain, unchanged explicit
update and one changed number. The static variant replaces 63 fixed owned label
strings with `label_static`; the changing number remains dynamic. A deliberate
257-byte Rust allocation verifies counter and peak sensitivity. Receipts include
source identities before/after the command; drift makes the measurement fail.

A second workload uses 24 keyed, styled label/button rows. After priming both
orders and themes, it measures idle, unchanged, style changes, keyed reorders and
theme changes, with 16 updates each. `STYLE_SIZE_JSON` records `Style`, `View<()>`
and native `lv_style_t` sizes; `STYLE_COST_JSON` includes Rust allocation metrics
and native live bytes/blocks. A separate control requires style/descriptor
construction without children or owned text to allocate nothing. `--compare`
requires matching target/profile/toolchain/configuration/lock identities,
unchanged descriptor sizes and no allocation/live-storage metric increases.
The baseline must exist and come from the pre-change equivalent workload;
recording the post-change source twice is not a valid before/after comparison.
See [style semantics](ui-styles.md) for the public contract.

A form-construction workload compares equivalent erased explicit handlers,
typed explicit handlers and typed bindings over 32 constructions each. It uses
the same child append shape, text and nonzero captures, requires equal Rust
allocation/requested/peak/retained metrics, and checks release after each form.
`FORM_COST_JSON` records these measurements separately from native update costs.
Typed descriptor size/alignment is checked against the erased form.

The Rust meter reports cumulative allocation calls/requested bytes, live-byte
delta and maximum extra live bytes above the per-update baseline. Native metrics
report live bytes before/after and high-water **since LVGL initialization**, not
per-update native peak or native allocation-call counts. `update-metrics` counts
native label-text writes and chart sample append/full-write/full-sync operations.
One XY pair counts as one sample; draws, allocator calls and other setters are
unavailable. The original representative workload exposes only label writes; the
separate history fixture asserts chart counters. Host test-profile elapsed time is recorded
without a device-performance threshold. Neither leak balance nor a reduced
allocation count establishes physical memory-budget or board-render acceptance.

The full keyed matching/reorder tests remain required alongside the stable-order
path: matching key sequences cannot assume native order after raw/custom moves.
See [runtime contracts](ui-runtime.md) for render-skip obligations and resource
configuration limits.

## Representative updates and bounded history

```sh
# Development statistical acceptance; one native process at a time, no concurrent builds.
python3 scripts/check-ui-workloads.py --locked --offline --artifacts target/ui-workloads
python3 scripts/check-ui-history.py --locked --offline --artifacts target/ui-history
# Routine correctness checks retain all scenarios and corruption/clock controls.
python3 scripts/check-ui-workloads.py --locked --offline --correctness-only --artifacts target/ui-workloads-check
python3 scripts/check-ui-history.py --locked --offline --correctness-only --artifacts target/ui-history-check
```

The workload runner verifies 53 settings/list/chart scenarios. The history runner
compares full vectors and immutable history snapshots over 16 cases per strategy:
1/4 series, capacity 128/1024, batch 1/16 and Shift/Circular. Both use independent
native sample/pixel oracles and intentional corruption controls. Snapshot copying
is included in construction. Debug allocator instrumentation is separate from
release timing; phase clocks have explicit on/off controls. Full measurements use
20 isolated AB/BA pairs and report raw samples, medians, p95 and paired bootstrap
intervals. Comparison requires matching fixture/provider/configuration/tool/lock
identities; an explicitly audited API-only fixture port is the sole exception.
`--compare` uses `--fixture-port` only for that source migration.

`--correctness-only` runs every case and negative control, both release strategies
and all clock checks with one pair. It reports no statistical summaries or paired
confidence intervals and cannot serve as a comparison baseline. CI selects this
mode once per complete runner. The default remains the full 20-pair experiment.
Host memory-display rendering excludes device transfer; these distributions do
not certify board timing, frame rate or a deployed RAM budget.

## Example and native display checks

```sh
python3 scripts/check-ui-examples.py --artifacts target/ui-audit
# Headless Linux:
xvfb-run -a python3 scripts/check-ui-examples.py --artifacts target/ui-audit
```

The runner tests actual example views with memory displays and records diagnostic
images, then executes twenty bounded scenarios across the focused examples and
galleries. Each focused example has three mount/update/unmount cycles covering
light and dark themes, native pointer actions, content bounds and button shadows. SDL smoke proves
that native windows can run and close; it does not emulate OS input. Pixel and
interaction assertions over memory displays provide separate evidence.

## Evidence boundaries

Local fixture logs belong under ignored artifact directories; CI uploads
diagnostics, not full build trees. Required environment failures are explicit
`not-run` results.

Host compilation/execution does not certify cross-toolchain builds, ESP-IDF
integration, board rendering, pending DMA/GPU teardown, other operating systems,
external-provider ABI or sanitizer coverage. Inspect the configured target runner
before a cross-target run: the checked-in Xtensa runner flashes hardware.

## Interactive documentation playground

From the website checkout, `npm run check-interactive -- --passes 3` compiles complete Rust
`App` examples to WebAssembly, verifies switch messages and text changes, edited
pixels, source diagnostics, invalid views, cleanup/reinitialization, superseding
compilation and concurrent consumer isolation. `playground:wasm:check` binds
bundled interactive presets to source and artifact hashes. The browser Worker
path additionally requires real input, blocked-app timeout, editor recovery,
locale/subpath and navigation checks. Native snapshot checks remain separate.
