# Optional EmbedScope diagnostics

This standalone application connects published `rs_lvgl` / `rs_lvgl_ui` 0.3.1 to
EmbedScope 0.1.0. It renders a real LVGL 9.6 counter into a 320 × 240 memory display.
There is no visible simulator window. The Inspector displays the actual RGB888
flush output; its input requests use a native LVGL pointer driver and update the
Rust application through the ordinary message loop.

The four library crates do not start a server or depend on EmbedScope. This
application owns the `embedscope` feature, which is **off by default**. All its
diagnostics modules, optional dependencies, framebuffer mirror, pointer driver and
listener are selected only by that feature. Cargo's release profile controls
optimization and does not disable an explicitly selected feature.

## Run and inspect

Download the CLI ZIP for your platform from [EmbedScope Releases](https://github.com/umbrella22/EmbedScope/releases/latest), extract it and add its directory to `PATH`. Running the example also requires Rust/Cargo, a native C compiler and libclang. SDL is not required. From the rs_lvgl repository root:

```sh
cargo run --manifest-path examples/embedscope/Cargo.toml --features embedscope
```

In another terminal:

```sh
escope -d tcp://127.0.0.1:9338 --json info
escope -d tcp://127.0.0.1:9338 --json ui tree
escope -d tcp://127.0.0.1:9338 display capture counter-before.png
escope -d tcp://127.0.0.1:9338 input tap 100 140 --hold-ms 60
escope -d tcp://127.0.0.1:9338 display capture counter-after.png
```

Use new capture filenames. The button spans 224 × 56 pixels; its exact position
comes from `ui tree`. The default layout includes `(100, 140)`. In the desktop
Inspector, connect to `tcp://127.0.0.1:9338`. Enable sending taps when reproducing
input. The listener accepts loopback addresses only and serves one connection at
a time. Use one Session for concurrent SDK operations.

`--listen 127.0.0.1:0` chooses an available port; the application prints its endpoint.
`--run-for-ms 1000` limits execution and performs ordered native teardown.

## Enable and disable

```sh
# Optimized reproduction with diagnostics retained
cargo run --manifest-path examples/embedscope/Cargo.toml --release --features embedscope

# Production configuration: no diagnostics feature selected
cargo build --manifest-path examples/embedscope/Cargo.toml --release

# Inspect normal/build dependencies; no escope-* package should appear
cargo tree --manifest-path examples/embedscope/Cargo.toml --edges normal,build
```

Default builds still run the counter's memory display, with no diagnostic network
endpoint. Closing a connection resets capture/input state but leaves diagnostics
compiled into an enabled binary. Rebuild without the feature to remove it. Cargo
features are additive; avoid `--all-features` for a production build and inspect
the final application's unified feature graph.

## Platform contract

- `diagnostics/wire.rs` keeps one pending frame, preserves partial TX, bounds RX
  work per UI tick, and drops the connection if TX stalls for three seconds.
- `diagnostics/device.rs` reads the existing native screen on the UI thread. The
  general `NativeTree<128>` supplies generic `lv_obj` kinds, geometry and state;
  semantic View names and text need an application-owned `TreeSource`.
- Captures copy the full rendered display into a fixed owned frame. New captures
  return Busy until End; disconnect and Cancel release the frame. There is no
  panel readback or synthetic reference image.
- Native taps pair press/release. Disconnect completes an accepted tap early,
  including a short-lived CLI connection; explicit Cancel suppresses its click.
  Unsupported crash and unmeasured memory statistics are not invented.
- The application registers a diagnostic sink and forwards error/ignored-property
  metadata through a bounded queue. Counter updates produce structured log events.
  It does not replace the application's global logger.
- Shutdown stops transport, releases input/capture and drops the tree adapter,
  then unmounts UI, deletes the display and deinitializes LVGL.

For a board, keep transport in its existing executor and queue requests to the UI
thread. Mirror the platform's actual final display path and route injected input
through the system's input arbitration. Keep exactly one native provider: this
example uses registry dependencies throughout so `escope-lvgl/native` resolves to
the same `rs_lvgl`. Mixing a path provider with a registry provider needs explicit
source unification or an application `TreeSource` without the `native` feature.

## Validate

```sh
cargo test --manifest-path examples/embedscope/Cargo.toml --features embedscope -- --test-threads=1
```

The process acceptance test exercises real native tree/pixels, input-driven state
changes, structured logs, fixed captures, stale revisions, malformed/fragmented
frames, disconnect/reconnect cleanup and ordered process teardown. This establishes
host-native behavior; it does not certify a physical display or device transport.
