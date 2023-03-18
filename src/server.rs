use std::sync::Arc;

use binary_heap_plus::BinaryHeap;
use tokio::sync::{broadcast, RwLock};
use tokio_stream::{
    wrappers::{errors::BroadcastStreamRecvError, BroadcastStream},
    Stream, StreamExt,
};
use tonic::{Request, Response, Status};
use tracing::*;

use crate::proto::{orderbook_aggregator_server::OrderbookAggregator, Empty, Level, Summary};

#[derive(Debug, thiserror::Error, Clone)]
pub enum Error {
    #[error("")]
    SummaryStreamError(Arc<dyn std::error::Error + Send + Sync>),
}

impl From<Error> for tonic::Status {
    fn from(_value: Error) -> Self {
        todo!()
    }
}

#[derive(Default)]
struct MergedSummary {
    bids: BinaryHeap<Level>,
    asks: BinaryHeap<Level>,
}
impl MergedSummary {
    fn merge(&mut self, _summary: Summary) {
        todo!()
    }
    fn get_summary(&self) -> Summary {
        todo!()
    }
}

pub type SummarySender = broadcast::Sender<Result<Summary, Error>>;

pub struct OrderbookAggregatorService<'l> {
    summary_sender: SummarySender,
    base_currency: &'l str,
    quote_currency: &'l str,
    merged_summary: Arc<RwLock<MergedSummary>>,
    producer: tokio::task::JoinSet<()>,
}
impl<'l> OrderbookAggregatorService<'l> {
    pub fn new(base_currency: &'l str, quote_currency: &'l str) -> Self {
        Self {
            summary_sender: broadcast::channel(10).0,
            base_currency,
            quote_currency,
            merged_summary: Arc::new(RwLock::new(MergedSummary::default())),
            producer: tokio::task::JoinSet::default(),
        }
    }

    pub async fn add_summary_source<G: crate::GetSummaryStream>(
        &mut self,
        summary_stream_getter: G,
    ) -> Result<(), Error>
    where
        G::Error: std::error::Error + Send + Sync + 'static,
        G::SummaryStream: Send + Sync + 'static,
    {
        use futures_util::StreamExt;

        let merged_summary = self.merged_summary.clone();
        let summary_sender = self.summary_sender.clone();

        let stream = summary_stream_getter
            .get_summary_stream(self.base_currency, self.quote_currency)
            .await
            .map_err(|err| Error::SummaryStreamError(Arc::new(err)))?;

        self.producer.spawn(async move {
            stream
                .for_each(move |order_book| {
                    let merged_summary = merged_summary.clone();
                    let summary_sender = summary_sender.clone();

                    async move {
                        let mut merged_summary = merged_summary.write().await;
                        merged_summary.merge(order_book.unwrap());
                        summary_sender
                            .send(Ok(merged_summary.get_summary()))
                            .unwrap();
                    }
                })
                .await;
        });

        todo!()
    }
}

#[tonic::async_trait]
impl OrderbookAggregator for OrderbookAggregatorService<'static> {
    type BookSummaryStream = impl Stream<Item = Result<Summary, tonic::Status>>;

    async fn book_summary(
        &self,
        _: Request<Empty>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        Ok(Response::new(
            BroadcastStream::new(self.summary_sender.subscribe()).filter_map(
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
