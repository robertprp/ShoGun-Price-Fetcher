use std::path::Path;
use clap::Parser;
use cli::Cli;
use service::{asset::{price::PriceService, Asset, Chain}, config::ConfigService, services::ServiceProvider, telemetry};
use futures::StreamExt;
use tracing::info;

mod cli;

#[tokio::main]
#[tracing::instrument]
async fn main() {
    info!("Starting grafana shogun");
    let args = Cli::parse();

    let config =
        ConfigService::read_file(Path::new(&args.config)).expect("Failed to read config file");

    let service_name = String::from("shogun");
    
    telemetry::init(
        &config,
        service_name.clone(),
        args.log_level.into(),
    ).expect("Failed to initialize telemetry");
    
    info!("Starting service: {}", service_name);
    
    let meter = telemetry::get_meter_provider().meter("shogun");
    let counter = meter.u64_counter("service_startups").with_description("Number of times the service has started").build();
    
    counter.add(1, &[]);
    
    let services = ServiceProvider::new();
    services.add_service(config.clone()).await;
    
    let mut price_service = PriceService::new(services).await;
    
    // hardcoded assets to fetch ))
    let trump_solana_address = "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN";
    let weth_ethreum_address = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
    
    let trump_coin = Asset::builder()
        .address(trump_solana_address.to_string())
        .symbol("TRUMP".to_string())
        .chain(Chain::Svm(1))
        .name(String::from("Trump Coin"))
        .decimals(6)
        .build();
    
    let weth_coin = Asset::builder()
        .address(weth_ethreum_address.to_string())
        .symbol("WETH".to_string())
        .chain(Chain::Evm(1))
        .name(String::from("Wrapped Ether"))
        .decimals(18)
        .build();
    
    let _ = price_service.add_asset(weth_coin).await;
    let _ = price_service.add_asset(trump_coin).await;
    
    price_service.start().await;
    
    let mut stream_handler = price_service.subscribe().await;
    
    while let Some(event) = stream_handler.next().await {
        info!("Received a new asset event: {:?}", event);
    }
    
    telemetry::shutdown().await.expect("Failed to shutdown telemetry");
}
