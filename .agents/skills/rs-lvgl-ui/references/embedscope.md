# Optional EmbedScope integration

The rs_lvgl repository provides `examples/embedscope/`, an independent application using published rs_lvgl/rs_lvgl_ui 0.3.1 and EmbedScope 0.1.0. It renders a real LVGL memory display and exposes its tree, actual flush pixels, native pointer input, logs and available metrics through loopback TCP. It has no visible SDL window and is not a physical-device fixture.

Download the matching CLI ZIP from [EmbedScope Releases](https://github.com/umbrella22/EmbedScope/releases/latest), extract it and add its directory to PATH. Cargo installation is optional for a machine that already has Rust.

```sh
cargo run --manifest-path examples/embedscope/Cargo.toml --features embedscope
escope -d tcp://127.0.0.1:9338 --json info
escope -d tcp://127.0.0.1:9338 --json ui tree
escope -d tcp://127.0.0.1:9338 input tap 100 140 --hold-ms 60
escope -d tcp://127.0.0.1:9338 display capture NEW_SCREEN.png
```

Use the Inspector and CLI in turn: one connection is served at a time. Reuse one Session for concurrent SDK requests. Capture filenames must be new. The general NativeTree exports generic lv_obj kinds, empty names, geometry and native state; semantic View/text metadata requires an application TreeSource.

## Feature and profile contract

`embedscope` belongs to the application, not rs_lvgl_ui. Gate the Agent dependencies (optional), module, initialization, listener, capture buffers and log fan-out together. Development and acceptance explicitly enable it. For optimized reproduction use `--release --features embedscope`. A release profile never removes a selected feature.

```sh
cargo build --manifest-path examples/embedscope/Cargo.toml --release
cargo tree --manifest-path examples/embedscope/Cargo.toml --edges normal,build
```

The production normal/build graph must contain no escope-* package. Features are additive; another dependency can enable them. Avoid --all-features for shipping. UI diagnostics, strict-properties and update-metrics have separate purposes: events/sink, stricter validation and update counters. None starts a remote connection by itself. Closing a host session does not remove compiled code. Firmware must be rebuilt without the application feature, installed and booted to remove diagnostic interfaces.

Dango OS uses the same explicit policy: `cargo fw --features embedscope` enables its second diagnostic CDC in a release image; plain `cargo fw` omits it. Official v0.4.0 has no Agent. Select the actual second interface and --dtr on, preserving the original console/OTA.

## Platform invariants

Keep all native accesses and Agent dispatch at a UI-thread safe point. Transport tasks exchange bounded owned requests/responses. Use one LVGL native provider/configuration and preserve root/display/input lifetimes. The standalone example uses registry dependencies throughout; mixing local path and registry providers needs explicit source unification, or a custom TreeSource without escope-lvgl/native.

Advance tree revision after properties, input state or topology change. Copy final output into a fixed owned capture; free it on End, Cancel or disconnect. Pair press/release. The example completes accepted taps early on disconnect (including short-lived CLI use); explicit Cancel suppresses the click. Do not invent missing memory or crash measurements. Release transport/capture/input/tree before UI unmount/display deletion/deinit.

```sh
cargo test --manifest-path examples/embedscope/Cargo.toml --features embedscope -- --test-threads=1
```

This is real host-native process acceptance. Board rendering/input/USB and heap behavior need the target's own checks. See the [bilingual debugging guide](https://umbrella22.github.io/rs_lvgl_website/en/guide/embedscope.html).
