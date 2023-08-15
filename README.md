# Rusty Spout

[Spout](https://spout.zeal.co/) bindings to Rust. Initially created for usage with [Godot](https://github.com/godotengine/godot) and [gdext](https://github.com/godot-rust/gdext).

Uses a [fork of Spout2](https://github.com/virtual-puppet-project/Spout2-lean.git) with all the precompiled `dll`s and `lib`s removed.

A Godot GDExtension-compatible library can be built with the `godot` feature enabled.

## Building

Build using the included `build.sh` utility. Because of the dependency on building Spout2,
building the crate is more complicated than just running `cargo build`.

### Script build steps

Run: `build.sh [debug|release|package|clean]`

If using `package`, a new directory will be created in the project root containing the
compiled library along with `SpoutLibrary.dll`.

### Manual build steps

1. `cargo build` or `cargo build --release`
2. Find the build `$OUT_DIR/bin` directory
3. Copy the compiled `SpoutLibrary.dll` to be next to your binary

## License

MPL-2.0
