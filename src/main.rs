mod config {
    use std::net::SocketAddr;

    pub use envconfig::Envconfig;

    #[derive(Debug, Envconfig, PartialEq)]
    pub(crate) struct Config {
        #[envconfig(from = "ORDERBOOK_ADDR", default = "127.0.0.1:8080")]
        pub addr: SocketAddr,
    }

    #[cfg(test)]
    mod tests {
        use std::net::Ipv4Addr;

        use super::*;
        use maplit::hashmap;

        #[test]
        fn success_parse() {
            assert_eq!(
                SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 8080),
                Config::init_from_hashmap(&hashmap! {
                    "ORDERBOOK_ADDR".to_owned() => "127.0.0.1:8080".to_owned()
                })
                .unwrap()
                .addr
            );
        }

        #[test]
        fn failed_parse() {
            assert_eq!(
                Config::init_from_hashmap(&hashmap! {
                    "ORDERBOOK_ADDR".to_owned() => "".to_owned()
                }),
                Err(envconfig::Error::ParseError {
                    name: "ORDERBOOK_ADDR"
                }),
            );
        }
    }
}

mod proto {
    tonic::include_proto!("orderbook");
}

mod server {
    use crate::proto::{orderbook_aggregator_server::OrderbookAggregator, Empty, Summary};
    use tonic::{Request, Response, Status};

    #[derive(Debug, thiserror::Error)]
    enum Error {}

    #[derive(Debug, Default)]
    pub struct OrderbookAggregatorService {}

    #[tonic::async_trait]
    impl OrderbookAggregator for OrderbookAggregatorService {
        type BookSummaryStream = tonic::Streaming<Summary>;

        async fn book_summary(
            &self,
            _request: Request<Empty>,
        ) -> Result<Response<Self::BookSummaryStream>, Status> {
            todo!()
        }
    }
}

use config::*;
use std::error;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("While run server: {0:?}")]
    Transport(#[from] tonic::transport::Error),
    #[error("While parse config: {0:?}")]
    Config(#[from] envconfig::Error),
    #[error("While log init: {0}")]
    Log(Box<dyn error::Error + Send + Sync>),
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::try_init().map_err(Error::Log)?;

    let config = Config::init_from_env()?;

    let orderbook_aggregator_service =
        proto::orderbook_aggregator_server::OrderbookAggregatorServer::new(
            server::OrderbookAggregatorService::default(),
        );

    tonic::transport::Server::builder()
        .accept_http1(true)
        .add_service(orderbook_aggregator_service)
        .serve(config.addr)
        .await
        .map_err(Error::from)
}
