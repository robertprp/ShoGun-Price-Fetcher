use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::Result;
use lib::error::Error;
use rust_decimal::Decimal;
use std::pin::Pin;
use tokio::task::JoinHandle;
use tokio_stream::Stream;
use crate::asset::Asset;

#[derive(Clone, Debug)]
pub struct AssetPriceEvent {
    pub provider: AssetPriceProvider,
    pub asset: Asset,
    pub price: Decimal,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AssetPriceProvider {
    DeFiLlama
}

#[async_trait]
pub trait PriceProvider: Send + Sync {
    async fn add_asset(&self, asset: Asset) -> Result<(), Error>;
    async fn remove_asset(&self, asset_address: String) -> Result<(), Error>;
    fn subscribe(&self) -> Pin<Box<dyn Stream<Item = AssetPriceEvent> + Send>>;
    fn start(&self) -> JoinHandle<Result<(), Error>>;
}
