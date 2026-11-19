# Optional EmbedScope diagnostics

This standalone application connects published `rs_lvgl` / `rs_lvgl_ui` 0.8 to
EmbedScope 0.6.0. It renders a counter and controlled brightness slider into a
320 × 240 LVGL 9.6 memory display. Inspector reads the RGB888 flush output;
there is no simulator window. Native pointer input sends typed `App::Action`
messages through the ordinary UI loop.

The app supplies a `TreeSource` for named controls and uses the default
`escope-lvgl` feature set. Applications that use `escope-lvgl 0.6/native`
resolve the same `rs_lvgl_sys` 0.8 provider, source and configuration as rs_lvgl.

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
escope -d tcp://127.0.0.1:9338 display capture settings.png
```

Use new capture filenames. The live tree gives the position of `settings.increment`
and `action:settings.brightness`. `escope input tap X Y` accepts display-pixel
coordinates; the reply confirms admission, not an application change. The listener
accepts loopback addresses only and serves one connection at a time. Inspector and
CLI take turns; an SDK client can subscribe and tap on the same session.

`--listen 127.0.0.1:0` chooses an available port; the application prints its endpoint.
`--run-for-ms 1000` limits execution. For automated runs with a piped stdin,
`--exit-on-stdin-close` starts the same ordered native teardown when the parent
closes the pipe.

## Reproduce slider outcomes

Connect with EmbedScope Inspector 0.6.0 or later. Enable **Send taps**, click near the middle of the brightness
slider, and inspect **Last observed action**. Refresh if the displayed screenshot
predates the frame shown in the evidence. Tree and screenshot are separate reads.
Restart the example and reconnect for each independent case:

| `--brightness-case` | Tap position | Expected result |
| --- | --- | --- |
| `normal` (default) | About 50% across the slider | The preview swatch changes; a newer completed RGB888 frame contains the expected preview pixel |
| `normal` | About 88% across | The application rejects values over 65 and restores the controlled slider; the swatch is unchanged |
| `ignore-input` | About 50% across | The request is admitted but no brightness action reaches the application |
| `hold-preview` | About 50% across | The application changes brightness state, but the swatch does not show the expected color |

Example: append `-- --brightness-case hold-preview` to the `cargo run` command.
The device reports this bounded observation over the `escope.ui.evidence.v1`
log target. It does not export the input value. The panel shows the last available
observation for that tagged control in the current session, not a prediction or
an acknowledgement of the latest tap. A short-lived CLI tap can change the UI but
cannot collect an outcome from the closed session.

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
- `diagnostics/tree.rs` projects this page's fixed native hierarchy on the UI
  thread. Public names are explicit; no label text or object pointers are exported.
  View keys are not used as native IDs. Input or layout updates advance the revision.
- Captures copy the full rendered display into a fixed owned frame. New captures
  return Busy until End; disconnect and Cancel release the frame. There is no
  panel readback or synthetic reference image.
- Native taps pair press/release. Disconnect completes an accepted tap early,
  including a short-lived CLI connection; explicit Cancel suppresses the pending
  click or slider edit. Neither carries pending evidence into a later session.
  Unsupported crash and unmeasured memory statistics are not invented.
- The application registers a diagnostic sink and forwards error/ignored-property
  metadata through a bounded queue. Counter updates produce structured log events.
  It does not replace the application's global logger.
- Shutdown stops transport, releases input/capture and drops the tree adapter,
  then unmounts UI, deletes the display and deinitializes LVGL.

For a board, keep transport in its existing executor and queue requests to the UI
thread. Mirror the platform's actual final display path and route injected input
through the system's input arbitration. Keep exactly one native provider: Cargo
cannot unify path and registry sources by version number alone. The [S31 procedure](s31-acceptance.md)
separates firmware identity, tree evidence, final display capture and panel checks.

## Validate

```sh
bash scripts/check-embedscope-example.sh
```

The gate verifies the provider, runs the native process tests, and checks the
feature-off dependency graph and executable. Tests cover fixed captures, stale
revisions, malformed/fragmented frames, reset and teardown, and compare the
preview pixel before and after the four input cases. These are host-native
checks, not physical display or USB acceptance.
