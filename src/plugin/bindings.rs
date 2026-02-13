wasmtime::component::bindgen!({
    path: "wit",
    world: "http-plugin",
    with: {
        "wasi:http": wasmtime_wasi_http::bindings::http,
    },
    exports: { default: async },
    imports: { default: async },
});
