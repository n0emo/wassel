use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_config::WasiConfigVariables;
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

pub struct State {
    pub ctx: WasiCtx,
    pub http_ctx: WasiHttpCtx,
    pub config_vars: WasiConfigVariables,
    pub table: ResourceTable,
}

impl Default for State {
    fn default() -> Self {
        let ctx = {
            let mut builder = WasiCtxBuilder::new();
            builder.inherit_stdout();
            builder.inherit_stderr();
            builder.build()
        };
        let http_ctx = WasiHttpCtx::new();
        let table = ResourceTable::new();
        let config_vars = WasiConfigVariables::new();

        Self {
            ctx,
            http_ctx,
            config_vars,
            table,
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
//
impl WasiHttpView for State {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http_ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}
