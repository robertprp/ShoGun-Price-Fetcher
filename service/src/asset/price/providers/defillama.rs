use crate::asset::price::price_provider::{AssetPriceEvent, AssetPriceProvider, PriceProvider};
use crate::asset::Chain;
use crate::services::ServiceProvider;
use crate::telemetry;
use crate::{asset::Asset, config::ConfigService};
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use error_stack::{Result, ResultExt};
use futures_util::StreamExt;
use lib::error::Error;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, pin::Pin, sync::Arc};
use tokio::sync::broadcast::{self as broadcast, Sender};
use tokio::sync::RwLock;
use tokio_stream::{wrappers::BroadcastStream, Stream};
use tracing::{info, info_span, instrument, Instrument};

pub const DEFILLAMA_PRICE_FETCHER_URL: &str = "https://coins.llama.fi/prices/current";

#[derive(Clone, Debug)]
pub struct DefiLlamaProvider {
    assets: Arc<RwLock<HashMap<String, Asset>>>,
    sender: Sender<AssetPriceEvent>,
    fetch_interval: u64,
}

impl DefiLlamaProvider {
    pub async fn new(services: ServiceProvider) -> Self {
        let config = services.get_service_unchecked::<ConfigService>().await;

        let interval = config.tasks.fetcher.interval;
        // Create unbounded channel
        let (sender, _) = broadcast::channel::<AssetPriceEvent>(100);

        Self {
            sender,
            fetch_interval: interval,
            assets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[instrument(name = "fetch_asset_prices")]
    pub async fn fetch_asset_prices(&self) -> Result<Vec<AssetPriceEvent>, Error> {
        let assets = self.assets.read().await.clone();

        if assets.is_empty() {
            return Ok(vec![]);
        }

        info!("Fetching DefiLlama prices for {:?} assets", assets.len());

        let request_params: String = assets
            .clone()
            .into_iter()
            .map(|(_, asset)| AssetIdentifier::from(asset).to_string())
            .collect::<Vec<String>>()
            .join(",");

        let url = format!(
            "{api_url}/{request_params}",
            api_url = DEFILLAMA_PRICE_FETCHER_URL
        );
        let response = reqwest::get(&url).await.change_context(Error::Unknown)?;

        let feeds = response
            .json::<PriceResponse>()
            .await
            .change_context(Error::Unknown)?;

        let asset_price_events = feeds
            .coins
            .into_iter()
            .filter_map(|(asset_id, coin_info)| {
                let address = match asset_id.split(':').last() {
                    Some(addr) => addr.to_string(),
                    None => {
                        tracing::error!("Failed to parse asset id: {asset_id}");
                        return None;
                    }
                };

                let asset = match assets.get(&address) {
                    Some(asset) => asset.clone(),
                    None => {
                        tracing::error!("Failed to find asset with address: {address}");
                        return None;
                    }
                };

                let price = Decimal::from_f64(coin_info.price).unwrap_or(Decimal::ZERO);
                let fetched_at = Utc.timestamp_opt(coin_info.timestamp as i64, 0).unwrap(); // Should be safe to unwrap

                Some(AssetPriceEvent {
                    provider: AssetPriceProvider::DeFiLlama,
                    asset,
                    price,
                    fetched_at,
                })
            })
            .collect::<Vec<AssetPriceEvent>>();

        Ok(asset_price_events)
    }
}

#[async_trait]
impl PriceProvider for DefiLlamaProvider {
    async fn add_asset(&self, asset: Asset) -> Result<(), Error> {
        let mut assets = self.assets.write().await;
        assets.insert(asset.address.clone(), asset.clone());
        info!("Added asset to DefiLlamaProvider: {:?}", asset);
        Ok(())
    }

    async fn remove_asset(&self, asset_address: String) -> Result<(), Error> {
        let mut assets = self.assets.write().await;
        assets.remove(&asset_address);
        info!("Removed asset from DefiLlamaProvider: {:?}", asset_address);
        Ok(())
    }

    fn subscribe(&self) -> Pin<Box<dyn Stream<Item = AssetPriceEvent> + Send>> {
        let stream = BroadcastStream::new(self.sender.subscribe()).filter_map(|event| async move {
            match event {
                Ok(event) => Some(event),
                Err(_) => None,
            }
        });

        stream.boxed()
    }

    fn start(&self) -> tokio::task::JoinHandle<Result<(), Error>> {
        let provider_clone = self.clone();
        let sender = self.sender.clone();
        let span = info_span!("price_provider", price_provider = "defillama").or_current();

        tokio::spawn({
            async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                    provider_clone.fetch_interval,
                ));

                loop {
                    interval.tick().await;

                    let times_fetched_counter = telemetry::get_meter_provider()
                        .meter("shogun")
                        .u64_counter("times_fetched_counter")
                        .with_description("Number of times DefiLlama has been fetched")
                        .build();

                    times_fetched_counter.add(1, &[]);

                    match provider_clone.fetch_asset_prices().await {
                        Ok(price_events) => {
                            info!("Fetched {} price events from DefiLlama", price_events.len());

                            for event in price_events {
                                if let Err(e) = sender.send(event) {
                                    tracing::error!("Failed to broadcast price event: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to fetch asset prices from DefiLlama: {}", e);
                            break;
                        }
                    }
                }

                Ok(())
            }
            .instrument(span)
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PriceResponse {
    coins: HashMap<String, CoinInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CoinInfo {
    decimals: u8,
    symbol: String,
    price: f64,
    timestamp: u64,
    confidence: f64,
}

struct AssetIdentifier(String);

impl ToString for AssetIdentifier {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl From<Asset> for AssetIdentifier {
    fn from(asset: Asset) -> Self {
        let chain = match asset.chain {
            Chain::Svm(_) => "solana",
            Chain::Evm(_) => "ethereum", // TODO: Just added ethereum here for use case purposes, extra logic needed to handle other chains
        };

        Self(format!("{}:{}", chain, asset.address))
    }
}
