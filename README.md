# Goshenite

SDF rendering engine thingy.

![Goshenite](/assets/gosh.webp)

If debugging, run with environment variable `RUST_BACKTRACE=1` to see [anyhow](https://github.com/dtolnay/anyhow) error backtrace.

## dependencies

If running in debug mode you need vulkan validation layers installed (I haven't got around to writing code that can detect the presence of layers on different platorms...)

## Cargo features

- __colored-term__: (default) enables colored terminal log messages.
- __include-spirv-bytes__: (default) load spirv bytecode at compile-time. if disabled, the engine will attempt to read spirv files from the `./assets` directory at runtime (relative to the executable location).
- __shader-compile__: enables a build script to compile spirv binaries whenever shader source is changed. _NOTE: will add a notable increase to the build time if you don't already have shaderc libraries installed on your system._

## Design objectives

Source split into three directories:
1. user interface - intuitive, responsive and clear feedback.
2. renderer - optimized and portable.
3. engine - idk tbh. extensible? low-coupling? connecting glue between user interface and backend.

## Render Stages

```
        ┆
  ┌───┐ ┆ ┌───┐   ┌───┐   ┌───┐
  │ G │──>│ L │──>│ O │──>│ E │
  └───┘ ┆ └───┘   └───┘   └───┘
        ┆        ╰------┬------╯
subpass ┆ subpass      gui
   0    ┆    1
```

1. __G__ = Geometry pass
	- vert shader - bounding boxes
	- frag shader - signed distance field sphere tracing
2. __L__ = Lighting pass
	- vert shader - full screen triangle
	- frag shader - shading
	- reads input attachment g-buffers
3. __O__ = Overlay pass - rendered ui elements e.g. coordinate indicators
4. __E__ = Egui pass - egui stuff

Subpass outputs:
1. Subpass 0 - g-buffers:
	- rgba8 - normal.xyz, 0
	- u32 - object-id, primitive-id
2. Subpass 1 - swapchain image
