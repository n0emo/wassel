wasmtime::component::bindgen!({
    path: "adapters/wit",
    async: true,
    with: {
        "wasi:http": wasmtime_wasi_http::bindings::http,
    },
});
