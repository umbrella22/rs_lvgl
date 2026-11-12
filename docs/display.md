# Configurable displays and RGB888

The 0.8.0 API registers displays with initialized byte buffers, an explicit
native color format and a render mode. Device integrations select buffering and
software periods from their bandwidth, rendering time and memory budget. No fixed
frame rate is guaranteed. Existing `Display::register` remains synchronous RGB888
with a single PARTIAL buffer; `register_raw` follows the native default format.

## Register storage

```rust,no_run
use core::time::Duration;
use rs_lvgl::{ColorFormat, DisplayBuffers, DisplayBuilder, PixelBuffer, RenderMode};

# fn main() -> Result<(), rs_lvgl::DisplayConfigError> {
rs_lvgl::init();
let mut display = DisplayBuilder::new(480, 480)
    .color_format(ColorFormat::Rgb888)
    .render_mode(RenderMode::Direct)
    .invalidation_alignment(4, 1)
    .stride(1440)
    .buffers(DisplayBuffers::Double {
        front: PixelBuffer::Owned(vec![0; 1440 * 480].into_boxed_slice()),
        back: PixelBuffer::Owned(vec![0; 1440 * 480].into_boxed_slice()),
    })
    .build(|flush| {
        // Inspect area, actual stride, format and origin here. Reading native
        // bytes or passing them to a panel driver retains an unsafe boundary.
        let _ = (flush.area, flush.stride(), flush.origin(), flush.is_last());
    })?;
display.set_refresh_period(Duration::from_millis(33)).unwrap();
# Ok(())
# }
```

Owned buffers require `rs_lvgl/alloc` and use the application's Rust allocator.
`PixelBuffer::Static` accepts an initialized `&'static mut [u8]`, including storage
from an application-specific PSRAM allocator. Registration checks address alignment
against the native draw handler. It never adjusts a pointer into the allocation or
assumes that a byte allocation is suitably aligned. DMA suitability, cache maintenance
and access capabilities remain device-driver requirements.

Dimensions and stride fit the native draw-buffer header. DIRECT/FULL need a full
screen in each buffer. PARTIAL uses complete rows from the common capacity of its
buffers, up to the display height. Extra storage can remain unused. LVGL reshapes
PARTIAL buffers for each dirty region and can change its row stride: consult every
`FlushInfo`. DIRECT/FULL retain an explicit configured stride.

The runnable [memory-display example](../rs_lvgl/examples/byte_display.rs) exercises
padded RGB888 rows without a physical display:

```sh
cargo run -p rs_lvgl --example byte_display --features alloc
```

## Invalidation granularity

`DisplayBuilder::invalidation_alignment(columns, rows)` configures logical pixel
granularity, with `(1, 1)` as the default. The device selects the values; a panel
requiring four-column transfer windows can select `(4, 1)`. Both counts must be
nonzero. Zero returns `DisplayConfigError::InvalidInvalidationAlignment` before
native registration. Granularity does not set buffer address alignment, byte
stride, panel registers or content safe areas.

The native `LV_EVENT_INVALIDATE_AREA` hook expands real invalidations outward
on a zero-origin grid and clips at logical display edges. LVGL saves that area
before drawing, joining invalidations and retaining DIRECT synchronization damage.
For example, columns 9..9 on an 11-column display with granularity 4 become 8..10.
The clipped final group can contain fewer pixels than the configured granularity;
the driver must support that boundary or choose compatible display geometry.
FULL mode continues to render the entire display; native LVGL bypasses its
invalidation event in that mode.

PARTIAL buffers must hold at least `min(rows, display_height)` complete full-width
rows at the configured stride. Otherwise registration returns `BufferTooSmall`.
Each native chunk uses complete row groups, with a shorter final group allowed
at the clipped bottom edge. Pinned LVGL also uses INVALIDATE_AREA for synthetic
row-capacity probes during rendering. The event handler fits those probes downward
within available storage, using native RENDER_START/RENDER_READY to distinguish
them from real invalidations. It does not expand a probe beyond its capacity.
Non-power-of-two granularities and counts exceeding the display dimensions are
supported without overflowing coordinate arithmetic.

Alignment state and its event hook follow native display lifetime. Dropping a
handle preserves the setting; native deletion/deinit retires the hook and releases
state once. The setting is immutable after registration. Raw native calls must
preserve its invalidation/render event handling and dimensions; additional native
event handlers must not invalidate during render-start/render-ready processing or
change an already aligned region outside its grid/bounds. Quiesce and recreate
the registration when changing granularity.

An existing PARTIAL port can continue copying into an initialized full shadow and
expanding transfer damage there: the extra pixels come from persistent shadow
storage protected by its DMA lease. That integration does not require this new
setting or a migration to DIRECT. A future DIRECT integration must handle any
required expansion before native invalidation/drawing/synchronization, and transfer
only rendered dirty rows. Expanding a flush-only view does not establish those
bytes' rendering/completion guarantees.

## Pixel order and dirty regions

`ColorFormat::Rgb888` maps to LVGL's 24-bit B,G,R memory layout. Red is `00 00 FF`;
blue is `FF 00 00`. RGB565 uses native little-endian 16-bit storage on current
little-endian targets; RGB565_SWAPPED uses big-endian byte storage. These layouts
are independent of a panel's RGB/BGR register configuration.

`FlushInfo` describes the native pointer, initialized capacity, actual stride,
dirty area, buffer origin and final-chunk marker. PARTIAL data starts at the chunk
origin; DIRECT/FULL data starts at the full framebuffer origin. A dirty rectangle
at (20,30) in DIRECT therefore does not start at byte zero. `rows()` yields only
dirty pixels, excluding padding, but requires an unsafe call. No safe native slice
is provided by `Display::active_buffer`; it returns `BufferInfo` metadata.

Native byte views are valid only while the allocation remains registered and no
renderer, writer or teardown can run. They must not survive synchronous callback
return or async completion. Do not reenter LVGL while holding them. Pointers stored
outside the callback do not extend validity.

## Asynchronous backends and deletion

`build` automatically signals flush-ready after its synchronous callback returns.
`unsafe build_async(flush, wait)` installs the native wait hook. `flush` can start a
device transfer and return; `wait` must finish or cancel the prior transfer before
returning. LVGL then permits reuse. The wait must not depend on a task blocked by
the same UI thread. A timeout/error still has to quiesce hardware before return;
the library cannot infer whether DMA is done.

```text
render A -> submit A -> render B -> wait A -> submit B -> ...
                    A remains immutable until completion
```

`Display::flush_ready` remains unsafe because early completion permits native
reuse of a buffer still read by hardware. Interrupt/executor synchronization is
specific to the platform. All callbacks run on the serialized native UI context;
they and their destructors must not unwind, reenter LVGL or delete/reconfigure the
display. Raw native APIs must preserve registered state and buffer descriptors.
Changing builder-managed dimensions, rotation, format, user data, buffers or
callbacks requires quiescing and recreating the registration; mutating those
fields through raw APIs is outside this builder contract.

Dropping a Display handle preserves registration, buffers, backend captures,
aliases and implicitly created widgets. Native deletion/deinit releases backend
state and buffers once. Finish all render, GPU/DMA and flush work and retire
aliases/widgets/inputs before unsafe deletion. Runtime buffer replacement is not
part of the builder interface; quiesce and recreate when changing layouts.

## Partial updates and frame ownership

Each flush describes its own native layout. Buffer capacity is not the number of
pixel bytes to send: it can include padding, unused rows and pixels outside the
dirty rectangle. For RGB888, transfer only `area_width * 3` bytes per dirty row,
advancing by that flush's actual stride between source rows.

| Mode | Data pointer and origin | Stride | Completion protects |
| --- | --- | --- | --- |
| PARTIAL | chunk buffer; origin is the dirty chunk's top-left | recalculated by native LVGL for each chunk width | the submitted chunk until transfer completion |
| DIRECT | full framebuffer; origin is its display offset | configured full-row stride | the submitted dirty rows; the whole frame is complete only at the last region |
| FULL | full framebuffer; origin is its display offset | configured full-row stride | the submitted full frame |

For example, a display initially configured with 40 bytes per row can produce a
three-pixel-wide PARTIAL RGB888 flush with 9-byte rows, or 16-byte rows when the
native stride alignment is 8. Its pointer starts at that chunk, even if its screen
coordinates are `(1, 1)`. Copying into a full shadow uses the shadow's own stride:

```text
source row = flush.data + row * flush.stride
shadow row = shadow.data + (flush.area.y1 + row) * shadow.stride
             + flush.area.x1 * 3
copy flush.area.width * 3 bytes
```

This example assumes a zero-offset logical display; apply the same coordinate
transform used by the device when the display has a nonzero physical offset.
`unsafe FlushInfo::rows()` performs the native source-row addressing. Preserve
each reported dirty rectangle for the downstream plane. Native LVGL can already
join invalidations, so a backend cannot recover finer damage from the pixel data.
`is_last()` marks the final region of the native render batch, not completion of
panel DMA or optical presentation.

A PARTIAL-to-shadow integration can use synchronous `build`: copy every dirty row
into a device-owned shadow, then let the callback return and release the native
chunk. Once the batch is complete, lease that shadow to the display plane for all
its DMA blocks. Prevent further flushes into the leased shadow until the plane
releases it. This includes explicit `lv_refr_now` calls as well as timer-handler
rendering. Native chunk completion and the shadow's DMA lease are separate
lifetimes. The library does not manage the downstream shadow, screen generation,
damage queue, dropped-job recovery or its lease.

The asynchronous registration accepts ordinary blocking callbacks, not Rust
`async` closures or a Future. A wait callback must complete or cancel the pending
transfer independently of the blocked UI call. A DMA task that needs the same
cooperative executor to make progress cannot be waited on by spinning or blocking
inside that executor. Retain the existing device-owned shadow lease/guard, or use
a backend whose completion can progress independently. There is no safe generic
`FrameLease` or Future-to-LVGL adapter in this API.

Completion can also be signalled explicitly with `unsafe Display::flush_ready`
after the device has retired the current transfer. Dispatch completion onto the
serialized UI context unless the platform supplies a valid stronger contract.
After explicit ready, native LVGL skips the wait callback for that transfer. The
last submission can remain pending after a refresh call returns; neither a final
callback nor handle drop automatically waits for it. Quiesce it before deletion.
`active_buffer()` can already describe the next draw buffer after a swap; use the
pointer and metadata from the submitted `FlushInfo` for the in-flight transfer.

### DIRECT double-buffer synchronization

Native LVGL retains the preceding frame's dirty regions. Before rendering the next
frame, it waits for the previous transfer and copies the portions needed to make
the new draw buffer current, excluding areas that will be redrawn. This native
synchronization provides pixel preservation; it adds copying and waiting costs.
Two independent downstream shadow buffers do not receive this synchronization.

Within a DIRECT frame, LVGL flushes individual dirty regions using the same buffer
pointer and swaps buffers only after the last region. It can draw the next dirty
region before waiting for the preceding region's transfer. An intermediate flush
therefore protects its dirty rows, not every byte in the full framebuffer. DMA
must read those rows only. If a backend needs a complete-frame submission, it can
collect damage without starting DMA on intermediate callbacks and submit after
`is_last()`, preserving the native wait/completion contract for any actual transfer.
PARTIAL chunks must be copied before reuse when collecting a whole batch.

Do not hold a Rust `&[u8]` over native rendering or queue `&FlushInfo` for another
task. Raw metadata does not extend allocation validity or establish cross-core
ownership, cache coherency or synchronization. The backend owns those proofs.

## Images and native Canvas

`ImageSource::rgb888` accepts static packed B,G,R bytes;
`rgb888_with_stride` accepts padded rows. Data is exactly `stride * height` bytes.
The mounted tree retains each generated descriptor through native use.
`ImageResource::rgb888(width, height, stride, Rc<[u8]>)` accepts shared loaded data,
retaining its existing synchronous-software/no-image-cache and alignment rules.

With the native canvas capability, `rs_lvgl::CanvasBuffer::new` validates static or
owned RGB888/RGB565 data and `rs_lvgl::widgets::Canvas::attach_buffer` retains its
draw descriptor and pixels. Attachment requires synchronous software drawing and
disabled image caches; unsupported configurations return an error. Replacement
switches native descriptors before releasing the prior allocation. Native deletion
callbacks must not draw or reuse a Canvas being deleted. Editable swapped Canvas
is rejected because native set-pixel/fill operations lack parity. Display output
can still use RGB565_SWAPPED.

`rs_lvgl_ui::Canvas` continues to use ARGB8888 for declarative drawing and opacity.
RGB888 has no alpha channel; opaque buffers do not replace that contract.

## Periods and device policy

`Display::{set_refresh_period,refresh_period}` and concrete
`Pointer`/`Encoder`/`Keypad` `set_read_period`/`read_period` use nonzero whole
milliseconds within u32. Fractional milliseconds, zero, overflow and absent native
timers return errors; invalid input preserves the old period. The native provider's
small read-only timer shim supplies actual period readback.

The refresh period controls software scheduling, not physical panel scan-out.
LVGL already pauses refresh when no dirty work remains (unless performance
monitoring requires it). A resumed 1000 ms timer can delay the next paint; lowering
input/global-handler cadence to 1 Hz also delays wakeup. Device runtimes should
evaluate event-driven wake and actual content deadlines before adding a governor.
Automatic pacing, panel VRR and irregular-screen safe areas are not implemented.

For 480×480 RGB888, one full frame is 691,200 bytes. Four-line SDR QSPI at 40 MHz
has a 34.56 ms pixel-transfer floor; at 80 MHz it has a 17.28 ms floor. Commands,
synchronization and rendering add costs. Double buffering overlaps work without
raising bus capacity. Partial-update and full-screen workloads need separate budgets.

## Validation and compatibility

Run `python3 scripts/check-display-contracts.py --locked` for native layout/lifecycle
tests, a padded-stride configuration and external no_std API controls. Development
acceptance uses `--repeat-lifecycle 3` on unchanged inputs. Run the existing dual-heap
suite for retained display/Canvas/image resources. These checks do not certify a
physical panel, DMA engine, actual frame rate, scan-rate control or safe-area UX.

New `ImageSource::Rgb888` requires external exhaustive matches to handle the new
variant when upgrading to 0.8.0. Oversized static-image strides that cannot
fit the native header are rejected before mutation. The default native configuration and existing constructor/destructor semantics
are preserved.

### Released dependencies

The typed display, timing, RGB888 image and Canvas interfaces are available in
0.8.0. Select matching crates from crates.io:

```toml
[dependencies]
rs_lvgl = { version = "0.8.0", default-features = false, features = ["alloc"] }
rs_lvgl_ui = "0.8.0"
```

Update any direct `rs_lvgl_sys` or `rs_lvgl_codegen` dependencies to 0.8.0 as well.
Resolve the consumer lockfile deliberately and inspect normal/build dependency
sources, effective application `lv_conf.h`, target bindgen and the final linked
provider. Keep one vendored LVGL provider. `rs_lvgl_sys` is both a normal and build
dependency of `rs_lvgl`; a matching version string alone does not establish ABI
compatibility with an application-supplied binary.

An existing 0.7.0 raw port already has native RGB888 rendering, stride-aware
buffers, display/input timers and `LV_EVENT_INVALIDATE_AREA`. The typed builder,
period readback, `lv_rs_timer_get_period` shim and RGB888 image constructors in
this guide require 0.8.0. Do not call the new shim against an older native provider.
Existing `Display::register`/`register_raw` integrations can adopt the builder
separately from changing their device's pixel-transfer path.

### Timer-loop migration

`timer_handler()` runs ready native timers and returns milliseconds until the
next timer is due. `NO_TIMER_READY` means all native timers are paused. A platform
loop must also wake for input or work that readies a timer. The return value is a
software scheduling hint; it does not change the physical panel's scan rate.
`task_handler()` preserves the same behavior while discarding that hint.

`tick_inc` carries fractional milliseconds across calls and requires 32-bit atomic
load/store plus a single tick source. Do not feed a custom tick callback and
`tick_inc` as competing clocks. With `rust_timer`, `update_clock` wraps at the
native `u32` millisecond boundary and always returns `Ok(())`; applications must
track absolute uptime themselves. See the [migration guide](ui-api-migration.md)
for compatibility details. Automatic pacing and safe-area layout remain research.
