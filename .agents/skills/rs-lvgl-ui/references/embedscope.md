# Optional EmbedScope integration

For rs_lvgl 0.8.0 applications, expose UI inspection through an application-owned
adapter. Default `escope-lvgl` accepts an application `TreeSource` without linking
another LVGL provider. Read tree data from the application's UI thread and copy
actual display pixels into an owned capture. Before using a native SDK adapter,
verify that its resolved dependencies use the application's exact LVGL provider
and source.

Download the matching CLI ZIP from [EmbedScope Releases](https://github.com/umbrella22/EmbedScope/releases/latest), extract it and add its directory to PATH. Cargo installation is optional for a machine that already has Rust.

Enable the application's diagnostic feature and use its configured endpoint.
For a loopback TCP listener on port 9338:

```sh
cargo run --features embedscope
escope -d tcp://127.0.0.1:9338 --json info
escope -d tcp://127.0.0.1:9338 --json ui tree
escope -d tcp://127.0.0.1:9338 input tap 100 140 --hold-ms 60
escope -d tcp://127.0.0.1:9338 display capture NEW_SCREEN.png
```

Use the Inspector and CLI in turn for a single-connection transport. Reuse one
Session for concurrent SDK requests. Capture filenames must be new. Provide
semantic view/text metadata through the application's `TreeSource`; raw native
objects alone do not supply it.

## Feature and profile contract

`embedscope` belongs to the application, not rs_lvgl_ui. Gate the Agent dependencies (optional), module, initialization, listener, capture buffers and log fan-out together. Development and acceptance explicitly enable it. For optimized reproduction use `--release --features embedscope`. A release profile never removes a selected feature.

```sh
cargo build --release
cargo tree --edges normal,build
```

The production normal/build graph must contain no escope-* package. Features are additive; another dependency can enable them. Avoid --all-features for shipping. UI diagnostics, strict-properties and update-metrics have separate purposes: events/sink, stricter validation and update counters. None starts a remote connection by itself. Closing a host session does not remove compiled code. Firmware must be rebuilt without the application feature, installed and booted to remove diagnostic interfaces.

## Platform invariants

Keep all native accesses and Agent dispatch at a UI-thread safe point. Transport
tasks exchange bounded owned requests/responses. Use one LVGL native provider
and configuration, and preserve root/display/input lifetimes.

Advance tree revision after properties, input state or topology change. Copy
final output into a fixed owned capture; free it on End, Cancel or disconnect.
Pair press/release and define disconnect/cancellation behavior so input cannot
remain pressed. Do not invent missing memory or crash measurements. Release
transport/capture/input/tree before UI unmount/display deletion/deinit.

```sh
cargo test --features embedscope -- --test-threads=1
```

Validate the application's actual transport, rendered pixels, tree data, input
and shutdown. Host-native process acceptance does not establish board rendering,
input, USB or heap behavior. See the [debugging guide](https://umbrella22.github.io/rs_lvgl_website/en/guide/embedscope.html)
for the host tools.
