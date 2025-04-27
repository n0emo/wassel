# Wassel

An extensible server for modern web powered by WebAssembly plugins

## Quick start

Build the server using `cargo`

```console
cargo build --release
```

## Writing plugins

Please refer to `adapters/README.md` to get more detailed overview on writing
your own plugins.

## Roadmap

- [x] Support handling HTTP requests
- [x] Automatically spin up additional plugin instances for incoming requests
- [ ] Add wasi:config support
- [ ] Add ability to configure instantiated plugin pool
- [ ] Support for various middlewares
- [ ] Plugin bindings for popular languages. Support for more languages will
      come later
    - [ ] Rust
    - [ ] Python
    - [ ] JavaScript
    - [ ] Go
    - [ ] C#
- [ ] Hot-reload plugins as they are modified
- [ ] Support for WASIp3 and concurrent instance execution

## Notice

This project includes [Wasmtime](https://github.com/bytecodealliance/wasmtime),
which is licensed under Apache License 2.0 (with LLVM exceptions)
