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

