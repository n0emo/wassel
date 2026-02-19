use std::path::PathBuf;

use anyhow::Context as _;
use tokio::sync::Mutex;
use wasmtime::{
    Engine,
    component::{Component, HasSelf, InstancePre},
};
use wasmtime_wasi_config::WasiConfig;
use wassel_world::wassel::foundation;

use crate::{instance::PluginInstance, meta::PluginMeta, state::PluginState};

pub struct PluginImage {
    pre: InstancePre<PluginState>,
    meta: PluginMeta,
    data_dir: PathBuf,
}

impl PluginImage {
    pub async fn load(
        engine: &Engine,
        bytes: &[u8],
        meta: PluginMeta,
        data_dir: impl Into<PathBuf>,
    ) -> anyhow::Result<Self> {
        let component = Component::new(engine, bytes).context("Creating WASM component")?;

        let mut linker = wasmtime::component::Linker::<PluginState>::new(engine);

        foundation::http_client::add_to_linker::<_, HasSelf<PluginState>>(&mut linker, |s| s)
            .context("Could not add wassel:foundation/http-client to linker")?;

        wasmtime_wasi::p2::add_to_linker_async(&mut linker)
            .context("Adding WASIp2 exports to linker")?;
        wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)
            .context("Adding WASI HTTP tp linker")?;
        wasmtime_wasi_config::add_to_linker(&mut linker, |c| WasiConfig::from(c.config_vars()))
            .context("Adding WASI config to linker")?;

        let export = "wassel:foundation/http-handler";
        if component.get_export(None, export).is_none() {
            anyhow::bail!("There is no '{export}' export");
        }

        let pre = linker
            .instantiate_pre(&component)
            .context("Pre-instantiating plugin")?;

        let image = Self {
            pre,
            meta,
            data_dir: data_dir.into(),
        };

        Ok(image)
    }

    pub async fn instantiate(&self, engine: &Engine) -> anyhow::Result<PluginInstance> {
        let mut store = wasmtime::Store::new(engine, PluginState::new(&self.data_dir)?);
        let instance = self.pre.instantiate_async(&mut store).await?;
        Ok(PluginInstance::new(
            instance,
            Mutex::new(store),
            self.meta.endpoint.clone(),
        ))
    }

    pub fn id(&self) -> &str {
        &self.meta.id
    }

    pub fn meta(&self) -> &PluginMeta {
        &self.meta
    }
}
