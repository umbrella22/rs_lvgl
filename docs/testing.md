# Host validation

The [WebAssembly acceptance script](../website/scripts/check-wasm.mjs) executes the compiled Rust/LVGL module, checks all nine categories, input, animation, independent instances and teardown/reinitialization. Build first with `npm --prefix website run wasm:build`, then run `npm --prefix website run check-wasm -- --three-passes`. These checks complement visible browser acceptance; they do not validate hardware.


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
cargo test -p rs_lvgl_ui --no-default-features --features alloc,components -- --test-threads=1
python3 scripts/check-ui-api.py --artifacts target/ui-api-audit
```

`Cargo.lock` is ignored. Local receipts record its digest; `--locked --offline`
requires an existing resolved lockfile and cached dependencies. CI first resolves
dependencies, then uses `--locked` for subsequent checks. `--all-features` is not a
supported replacement for the matrix: some features require nightly or external
application hooks.

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

On macOS, the native linker may need the SDL2 library search path explicitly:

```sh
LIBRARY_PATH="$(pkg-config --variable=libdir sdl2)" cargo test -- --test-threads=1
```

## Raw configuration and packaging

```sh
cargo test -p rs_lvgl_sys --test build_script -- --test-threads=1
python3 scripts/check-native-inputs.py --artifacts target/native-input-audit
```

The runner builds a raw-only application and checks C/Rust configuration agreement,
custom functions and a custom font descriptor. It changes a transitive configuration
header, a private C header and font source in place. Negative fixtures require the
intended capability/path/header failure. It also verifies unchanged-input cache reuse,
configuration switching, independent output directories, failed-build recovery,
interrupted C compilation and recovery, disabled/enabled generated widget APIs, and
an executable built against an unpacked `.crate`.

The interruption fixture needs POSIX process groups and terminates only its own
compiler process group. Windows interruption acceptance is not implemented. Font
symbol linking is tested; the fixture font descriptor is not a glyph-rendering test.

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
- Keypad registration, focus groups and modal restoration.
- Canvas resizing and snapshots that survive UI unmount.
- QR, barcode, GIF, Lottie and Pinyin individually, plus sequential replacement.
- Completed/interrupted screen transitions and unmount.
- Validation failures, failed Lottie setup and recovery.

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

`npm --prefix website run check-interactive -- --passes 3` compiles complete Rust
`App` examples to WebAssembly, verifies switch messages and text changes, edited
pixels, source diagnostics, invalid views, cleanup/reinitialization, superseding
compilation and concurrent consumer isolation. `playground:wasm:check` binds
bundled interactive presets to source and artifact hashes. The browser Worker
path additionally requires real input, blocked-app timeout, editor recovery,
locale/subpath and navigation checks. Native snapshot checks remain separate.
