# S31 diagnostics acceptance

Dango OS reads native rs_lvgl 0.8 objects through its own `TreeSource`. Its
firmware uses EmbedScope 0.2 without `escope-lvgl/native`. The S31 settings app
does not emit the host reference's `escope.ui.evidence.v1` records. Its
brightness control changes the settings model, the displayed readout, and the
panel brightness through separate paths.

## Identify the pair

From the Dango OS repository, with its pinned RISC-V toolchain and libclang:

```sh
cargo fw --locked --features display-metrics,embedscope --target-dir target/s31/escope-on
cargo fw --locked --features display-metrics --target-dir target/s31/escope-off
```

Both commands build only. `cargo fw-run` uses a flashing runner. Stage images
and a known-good fallback with the Dango OS tools; install only approved images.
Match revision, locked dependencies, display flags, profile, clock, assets and
device. Record both image SHA-256s, build IDs, toolchain, and a redacted board ID.
Only the candidate has the binary diagnostic CDC. Keep using the primary CDC
for console and display probe output.

## Repeat the same action

1. Enumerate CDC devices with `escope --json devices` and select the candidate's
   diagnostic interface. Use `escope -d "$ESCOPE_PORT" --dtr on --json info` to
   verify the expected build ID and boot ID before collecting evidence. The
   feature-off image must not expose that interface. Inspector and CLI use the
   port in turn.
2. Open the native Display Settings page. Record the initial brightness setting,
   tree revision, slider bounds and capture frame ID. Choose two safe positions
   from the current slider bounds. In one Inspector or SDK session submit the
   same low-to-high and high-to-low taps. Save the new tree and an independent
   fixed capture after state and rendering settle. Compare the slider and value
   readout pixels with the before capture; an input reply is not a state change.
3. Perform the two transitions with real touch on both candidates. Record
   whether the physical backlight changes and check for stuck presses, artifacts
   or tearing. The final-plane RGB565 capture cannot show emitted light, scanout
   tearing or RGB888 low bits. Restore the original brightness after the run.
4. Repeat disconnect/reconnect three times on the unchanged diagnostic image.
   Check a new session ID, unchanged build identity, stale capture rejection,
   no stuck press and continuing access to the ordinary console. Stop for an
   unexpected diagnostic interface, image identity or display/input failure.

## Compare cost

Use identical resources, input coordinates and repetitions for the two builds.
The Dango OS `firmware/os/DISPLAY_PIPELINE.md` and `tools/device/display_probe.py`
define the ordinary-console measurements. Record software touch/enqueue to
submit and pre-DMA timestamps with p50/p95 and missing-sample counts. Record
separate periods with no client, a connected client and an active capture.
Software frame intervals are not optical FPS or touch-to-light latency.

Compare heap snapshots only when both images have the same sampler, pool and
sampling point. Record internal free bytes, largest block and PSRAM where
available. The enabled image can additionally report these via `metrics`, plus
queue depth and dropped logs. Do not subtract its per-pool values from the
disabled image's aggregated free memory. Record missing samples as unavailable;
an allocator low-water mark cannot be inferred from sampled free bytes.

Keep board identity, screenshots, console measurements and physical observations
separate. Record `not run` without a board or verified image; no software fixture
substitutes for those results.
