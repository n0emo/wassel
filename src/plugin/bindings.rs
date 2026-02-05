wasmtime::component::bindgen!({
    path: "wit",
    world: "exports",
    with: {
        "wasi:http": wasmtime_wasi_http::bindings::http,
    },
    exports: { default: async },
});

