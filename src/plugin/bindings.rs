wasmtime::component::bindgen!({
    path: "wit",
    world: "exports",
    async: true,
    with: {
        "wasi:http": wasmtime_wasi_http::bindings::http,
    },
});
