#![feature(type_alias_impl_trait)]
#![feature(assert_matches)]

mod config;

#[allow(clippy::redundant_async_block)]
mod proto {
    tonic::include_proto!("orderbook");
}

mod binance;
mod bitstamp {}
mod server;

use std::error;
use tokio_stream::Stream;

#[tonic::async_trait]
pub trait GetSummaryStream {
    type Error;
    type SummaryStream: Stream<Item = Result<proto::Summary, Self::Error>>;
    async fn get_summary_stream(
        &self,
        base_currency: &str,
        quote_currency: &str,
    ) -> Result<Self::SummaryStream, Self::Error>;
}

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

    let (_sender, service) = server::OrderbookAggregatorService::new();
    let orderbook_aggregator_service =
        proto::orderbook_aggregator_server::OrderbookAggregatorServer::new(service);

    tonic::transport::Server::builder()
        .accept_http1(true)
        .add_service(orderbook_aggregator_service)
        .serve(config.addr)
        .await
        .map_err(Error::from)
}
