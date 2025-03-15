use std::{pin::Pin, sync::OnceLock};
use async_trait::async_trait;
use futures::{stream::select_all, Stream};
use lib::error::Error;
use price_provider::{AssetPriceEvent, PriceProvider};
use providers::defillama::DefiLlamaProvider;
use tracing::{info, warn};
use error_stack::{Report, Result};

use crate::services::{ServiceFactory, ServiceProvider};

use super::Asset;

pub mod price_provider;
pub mod providers;

static SERVICE_INSTANCE: OnceLock<PriceService> = OnceLock::new();

pub struct PriceService {
    providers: Vec<Box<dyn PriceProvider + Sync + Send>>,
    is_running: bool
}

impl PriceService {
    pub async fn new(services: ServiceProvider) -> Self {
        let providers: Vec<Box<dyn PriceProvider + Sync + Send>> = vec![
            Box::new(DefiLlamaProvider::new(services.clone()).await),
        ];
        
        Self { providers, is_running: false }
    }
    
    pub async fn add_asset(&self, asset: Asset) {
        for provider in self.providers.iter() {
            if let Err(e) = provider.add_asset(asset.clone()).await {
                warn!("Failed to add asset to price provider: {e:?}");
            }
        }
    }

    pub async fn remove_asset(&self, asset_address: String) {
        for provider in self.providers.iter() {
            if let Err(e) = provider.remove_asset(asset_address.clone()).await {
                warn!("Failed to remove asset from price provider: {e:?}");
            }
        }
    }
    
    /// Start all price providers
    pub async fn start(&mut self) {
        if self.is_running {
            return;
        }

        // Run price fetcher of every provider
        for provider in self.providers.iter() {
            provider.start();
        }
        
        self.is_running = true;
        info!("Price providers started");
    }
    
    /// Subscribe to all price providers asset price events
    pub async fn subscribe(&self) -> Pin<Box<dyn Stream<Item = AssetPriceEvent> + Send>> {
        let mut streams = Vec::new();

        for provider in self.providers.iter() {
            streams.push(provider.subscribe());
        }

        let streams = select_all(streams);

        Box::pin(streams)
    }
}

/// Get price service instance
pub async fn get_instance(services: ServiceProvider) -> Result<&'static PriceService, Error> {
    if let Some(instance) = SERVICE_INSTANCE.get() {
        return Ok(instance);
    }

    let mut instance = PriceService::new(services).await;
    instance.start().await;

    let _ = SERVICE_INSTANCE.set(instance);
    SERVICE_INSTANCE.get().ok_or(Report::new(Error::Unknown))
}

#[async_trait]
impl ServiceFactory for PriceService {
    async fn factory(services: ServiceProvider) -> Result<Self, Error> {
        Ok(PriceService::new(services).await)
    }
}
