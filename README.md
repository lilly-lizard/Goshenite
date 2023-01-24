# Goshenite

SDF rendering engine thingy.

![Goshenite](/assets/gosh.webp)

If debugging, run with environment variable `RUST_BACKTRACE=1` to see [anyhow](https://github.com/dtolnay/anyhow) error backtrace.

## Cargo features

- __colored-term__ (default): enables colored terminal log messages.
- __shader-compile__: enables a build script to compile spirv binaries when shader source is changed. _NOTE: will add a notable increase to build times if you don't have shaderc libraries installed on your system._

## Render Stages

```
        ┆
  ┌───┐ ┆ ┌───┐   ┌───┐   ┌───┐
  │ G │──>│ L │──>│ O │──>│ E │
  └───┘ ┆ └───┘   └───┘   └───┘
        ┆        ╰------┬------╯
subpass ┆ subpass       ┆
   0    ┆    1         gui
```

1. __G__ - Geometry pass
	- vert shader - full screen quad
	- frag shader - signed distance field ray marching
2. __L__ - Lighting pass
	- vert shader - full screen quad
	- frag shader - shading
	- reads input attachment g-buffers
3. __O__ - Overlay pass - misc ui elements e.g. coordinates
4. __E__ - Egui pass - egui menus

Subpass outputs:
1. Subpass 0 - g-buffers:
	- rgba8 [normal.xyz, 0]
	- u32 [primitive-id]
2. Subpass 1 - swapchain image
