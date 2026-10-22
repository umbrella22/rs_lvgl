# rs_lvgl_codegen

Build-time generation of Rust widget wrappers for `rs_lvgl`. The generator reads
bindgen source from the selected `rs_lvgl_sys` configuration and emits wrappers
for enabled widgets. Generated pointer-taking methods retain their unsafe API
boundary. This crate does not render UI or provide a second native LVGL provider.

The `rs_lvgl` build script uses target `DEP_LVGL_BINDINGS` metadata when present;
its host build-dependency can have a different feature set. Edit the generator or
native inputs, never `OUT_DIR/generated.rs`.

```sh
cargo test -p rs_lvgl_codegen
```

See the [workspace README](../README.md) for package roles and setup.
