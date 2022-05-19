# Goshenite

SDF rendering engine thingy maybe.

Building for Linux requires enabling either the x11 or wayland features otherwise winit won't compile e.g.
```shell
cargo build --features x11
```

You can avoid building shaderc by having it installed on your system (used to compile shader SPIR-V binaries in build.rs). See https://github.com/google/shaderc-rs#setup for more details.