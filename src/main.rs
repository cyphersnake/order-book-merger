#![allow(dead_code)]
#![feature(type_alias_impl_trait)]
#![feature(assert_matches)]
#![feature(result_option_inspect)]
#![feature(is_sorted)]

mod config;

mod exchanges;
mod merge_iter;
#[allow(clippy::redundant_async_block)]
mod proto;
mod server;

use std::error;

use tracing::*;

mod order_book;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("While run server: {0:?}")]
    Transport(#[from] tonic::transport::Error),
    #[error("While parse config: {0:?}")]
    Config(#[from] envconfig::Error),
    #[error("While log init: {0}")]
    Log(Box<dyn error::Error + Send + Sync>),
    #[error(transparent)]
    ServerError(#[from] server::Error),
}

use config::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::try_init().map_err(Error::Log)?;

    let config = Config::init_from_env()?;

    let mut service = server::OrderbookAggregatorService::new(
        &config.base_currency,
        &config.quote_currency,
        config.summary_size,
    );

    service
        .add_orderbook_source(
            "binance".to_owned(),
            exchanges::binance::Binance {
                ws_url: config.binance_websocket_addr,
                depth: exchanges::binance::Depth::_10,
            },
        )
        .instrument(span!(Level::TRACE, "Process binance orderbook"))
        .await?;

    service
        .add_orderbook_source(
            "bitstamp".to_owned(),
            exchanges::bitstamp::Bitstamp::new(config.bitstamp_websocket_addr),
        )
        .instrument(span!(Level::TRACE, "Process bitstamp orderbook"))
        .await?;

    let orderbook_aggregator_service =
        proto::orderbook_aggregator_server::OrderbookAggregatorServer::new(service);

    tonic::transport::Server::builder()
        .accept_http1(true)
        .add_service(orderbook_aggregator_service)
        .serve(config.addr)
        .instrument(span!(Level::TRACE, "Handle grpc service"))
        .await
        .map_err(Error::from)
}
