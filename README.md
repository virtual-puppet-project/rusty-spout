# Rusty Spout

[Spout](https://spout.zeal.co/) bindings to Rust. Initially created for usage with [Godot](https://github.com/godotengine/godot) and [gdext](https://github.com/godot-rust/gdext).

Uses a [fork of Spout2](https://github.com/virtual-puppet-project/Spout2-lean.git) with all the precompiled `dll`s, `lib`s, and `exe`s removed.

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

## Examples

Before building and running any example, make sure the library itself has been built using the
instructions in [Building](#building).

### Send/Receive

1. Run `cargo build --example send && cargo build --example receive`
2. Copy the examples from `target/debug/examples` to `target/debug`, since they need to be next to the
`SpoutLibrary.dll` to work. Alternatively, copy the `SpoutLibrary.dll` into `target/debug/examples` so that
`cargo run --example [send|receive]` just works


## License

MPL-2.0
