use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};
use tokio_stream::{
    wrappers::{errors::BroadcastStreamRecvError, BroadcastStream},
    Stream, StreamExt,
};
use tonic::{Request, Response, Status};
use tracing::*;

use crate::proto::{orderbook_aggregator_server::OrderbookAggregator, Empty, Summary};

mod order_book_merger;

#[derive(Debug, thiserror::Error, Clone)]
pub enum Error {
    #[error("")]
    SummaryStreamError(Arc<dyn std::error::Error + Send + Sync>),
}

impl From<Error> for tonic::Status {
    fn from(value: Error) -> Self {
        // WARN A more detailed status can and
        // should be made depending on the error,
        // but let's keep it simple
        tonic::Status::internal(value.to_string())
    }
}

use order_book_merger::{ExchangeName, OrderBookMerger};

pub type SummarySender = broadcast::Sender<Result<Summary, Error>>;

pub struct OrderbookAggregatorService {
    summary_sender: SummarySender,
    base_currency: String,
    quote_currency: String,
    merged_summary: Arc<RwLock<OrderBookMerger>>,
    pub producer: tokio::task::JoinSet<()>,
}
impl OrderbookAggregatorService {
    pub fn new(base_currency: &str, quote_currency: &str, summary_size: usize) -> Self {
        Self {
            summary_sender: broadcast::channel(10).0,
            base_currency: base_currency.to_string(),
            quote_currency: quote_currency.to_string(),
            merged_summary: Arc::new(RwLock::new(OrderBookMerger::new(summary_size))),
            producer: tokio::task::JoinSet::default(),
        }
    }

    pub async fn add_orderbook_source<G: crate::order_book::GetOrderBooksStream>(
        &mut self,
        exchange_name: ExchangeName,
        summary_stream_getter: G,
    ) -> Result<(), Error>
    where
        G::Error: std::error::Error + Send + Sync + 'static,
        G::OrderBooksStream: Unpin + Send + Sync + 'static,
    {
        let mut stream = summary_stream_getter
            .get_order_books_stream(self.base_currency.as_str(), self.quote_currency.as_str())
            .await
            .map_err(|err| Error::SummaryStreamError(Arc::new(err)))?;

        let merged_summary = self.merged_summary.clone();
        let summary_sender = self.summary_sender.clone();
        let trace_span = span!(
            Level::TRACE,
            "stream handler",
            exchange_name = exchange_name
        );
        let info_span = span!(Level::INFO, "stream handler", exchange_name = exchange_name);

        self.producer.spawn(
            async move {
                info!("Start stream handler task");

                while let Some(order_book) = stream.next().await {
                    info!("Receive orderbook");
                    trace!("Receive orderbook: {order_book:?}");

                    match order_book {
                        Ok(order_book) => {
                            let mut locked_merged_summary = merged_summary.write().await;
                            locked_merged_summary.insert(&exchange_name, order_book);

                            _ = summary_sender
                                .send(Ok(locked_merged_summary.get_summary()))
                                .inspect(|receiver_count| {
                                    info!("Send summary to {receiver_count} receiver")
                                });
                        }
                        Err(err) => {
                            error!("Error while receive order book: {err:?}")
                        }
                    }
                }
            }
            .instrument(trace_span)
            .instrument(info_span),
        );

        Ok(())
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
