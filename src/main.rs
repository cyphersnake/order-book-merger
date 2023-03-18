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
    use tokio::sync::broadcast;
    use tokio_stream::{
        wrappers::{errors::BroadcastStreamRecvError, BroadcastStream},
        Stream, StreamExt,
    };
    use tonic::{Request, Response, Status};
    use tracing::*;

    use crate::proto::{orderbook_aggregator_server::OrderbookAggregator, Empty, Summary};

    #[derive(Debug, thiserror::Error, Clone)]
    pub enum Error {}
    impl From<Error> for tonic::Status {
        fn from(_value: Error) -> Self {
            todo!()
        }
    }

    pub type SourceOfSummary = broadcast::Sender<Result<Summary, Error>>;

    pub struct OrderbookAggregatorService {
        source_of_summary: SourceOfSummary,
    }
    impl OrderbookAggregatorService {
        pub fn new() -> (SourceOfSummary, Self) {
            let sender = broadcast::channel(10).0;
            (
                sender.clone(),
                Self {
                    source_of_summary: sender,
                },
            )
        }
    }

    #[tonic::async_trait]
    impl OrderbookAggregator for OrderbookAggregatorService {
        type BookSummaryStream = impl Stream<Item = Result<Summary, tonic::Status>>;

        async fn book_summary(
            &self,
            _: Request<Empty>,
        ) -> Result<Response<Self::BookSummaryStream>, Status> {
            Ok(Response::new(
                BroadcastStream::new(self.source_of_summary.subscribe()).filter_map(
                    |result_with_summary| match result_with_summary {
                        Ok(result_with_summary) => {
                            trace!("Send {result_with_summary:?} via stream");
                            Some(result_with_summary.map_err(tonic::Status::from))
                        }
                        Err(BroadcastStreamRecvError::Lagged(lagged)) => {
                            warn!("Lagged {lagged} messages");
                            None
                        }
                    },
                ),
            ))
        }
    }
}

use config::*;
use std::error;
use tokio_stream::Stream;

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
