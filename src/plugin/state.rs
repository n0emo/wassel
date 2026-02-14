use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_config::WasiConfigVariables;
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

pub struct State {
    pub ctx: WasiCtx,
    pub config_vars: WasiConfigVariables,
    pub table: ResourceTable,
    pub http_ctx: WasiHttpCtx,
    pub http_client: reqwest::Client,
}

impl Default for State {
    fn default() -> Self {
        let ctx = {
            let mut builder = WasiCtxBuilder::new();
            builder.inherit_stdout();
            builder.inherit_stderr();
            builder.build()
        };

        Self {
            ctx,
            config_vars: WasiConfigVariables::new(),
            table: ResourceTable::new(),
            http_ctx: WasiHttpCtx::new(),
            http_client: reqwest::Client::new(),
        }
    }
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for State {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http_ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}
