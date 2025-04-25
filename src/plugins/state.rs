use wasmtime_wasi::{p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView}, ResourceTable};

pub struct State {
    pub ctx: WasiCtx,
    pub table: ResourceTable,
}

impl Default for State {
    fn default() -> Self {
        let mut builder = WasiCtxBuilder::new();
        builder.inherit_stdout();
        builder.inherit_stderr();

        Self {
            ctx: builder.build(),
            table: ResourceTable::new(),
        }
    }
}

impl IoView for State {
    fn table(&mut self) -> &mut ResourceTable { &mut self.table }
}

impl WasiView for State {
    fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
}
