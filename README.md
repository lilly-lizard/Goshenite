# Goshenite

SDF rendering engine thingy.

![Goshenite](/assets/Goshenite.webp)

If debugging, run with environment variable `RUST_BACKTRACE=1` to see [anyhow](https://github.com/dtolnay/anyhow) error backtrace.

### Cargo features:

- __colored-term__ (default): enables colored terminal log messages.
- __shader-compile__: enables a build script to compile spirv binaries when shader source is changed. _NOTE: will add a notable increase to build times if you don't have shaderc libraries installed on your system._