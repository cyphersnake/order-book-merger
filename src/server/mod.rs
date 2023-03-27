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
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::order_book::{GetOrderBooksStream, OrderBook, PriceLevel};
    use rust_decimal::Decimal;
    use tokio_stream::wrappers::BroadcastStream;

    use super::*;

    macro_rules! decimal {
        ($s:literal) => {
            rust_decimal::Decimal::from_str($s).unwrap()
        };
        ($val:ident) => {
            rust_decimal::Decimal::from(&$val)
        };
    }

    #[derive(Clone)]
    struct MockOrderBookStream {
        order_books: Vec<OrderBook>,
    }

    impl MockOrderBookStream {
        fn new(order_books: Vec<OrderBook>) -> Self {
            Self { order_books }
        }
    }

    #[derive(Debug, thiserror::Error)]
    enum Error {}

    #[tonic::async_trait]
    impl GetOrderBooksStream for MockOrderBookStream {
        type Error = Error;
        type OrderBooksStream = impl Stream<Item = Result<OrderBook, Self::Error>>;

        async fn get_order_books_stream(
            &self,
            _base_currency: &str,
            _quote_currency: &str,
        ) -> Result<Self::OrderBooksStream, Self::Error> {
            Ok(Box::pin(tokio_stream::iter(
                self.order_books.clone().into_iter().map(Ok),
            )))
        }
    }

    fn create_order_book(
        bids: Vec<(Decimal, Decimal)>,
        asks: Vec<(Decimal, Decimal)>,
    ) -> OrderBook {
        let bids = bids
            .into_iter()
            .map(|(price, quantity)| PriceLevel { price, quantity })
            .collect();

        let asks = asks
            .into_iter()
            .map(|(price, quantity)| PriceLevel { price, quantity })
            .collect();

        OrderBook { bids, asks }
    }

    #[tokio::test]
    async fn test_orderbook_aggregator_service() {
        let base_currency = "BTC";
        let quote_currency = "USD";
        let summary_size = 2;

        let order_book1 = create_order_book(
            vec![
                (decimal!("100.0"), decimal!("1.0")),
                (decimal!("90.0"), decimal!("2.0")),
            ],
            vec![
                (decimal!("110.0"), decimal!("3.0")),
                (decimal!("120.0"), decimal!("4.0")),
            ],
        );

        let order_book2 = create_order_book(
            vec![
                (decimal!("95.0"), decimal!("1.5")),
                (decimal!("85.0"), decimal!("2.5")),
            ],
            vec![
                (decimal!("115.0"), decimal!("3.5")),
                (decimal!("125.0"), decimal!("4.5")),
            ],
        );

        let exchange1 = "exchange1".to_string();
        let exchange2 = "exchange2".to_string();

        let mock_stream1 = MockOrderBookStream::new(vec![order_book1]);
        let mock_stream2 = MockOrderBookStream::new(vec![order_book2]);

        let mut aggregator =
            OrderbookAggregatorService::new(base_currency, quote_currency, summary_size);
        aggregator
            .add_orderbook_source(exchange1.clone(), mock_stream1)
            .await
            .unwrap();

        aggregator
            .add_orderbook_source(exchange2.clone(), mock_stream2)
            .await
            .unwrap();

        let mut receiver = BroadcastStream::new(aggregator.summary_sender.subscribe());

        // Skip first summary, equal for first exchange
        _ = receiver.next().await;
        if let Some(result_with_summary) = receiver.next().await {
            let (asks, bids, spread) = match result_with_summary.unwrap() {
                Ok(crate::proto::Summary {
                    asks,
                    bids,
                    spread: Some(spread),
                }) => (asks, bids, spread),
                other => panic!("Unexpected summary: {other:?}"),
            };

            assert_eq!(spread, decimal!("10").into());
            assert_eq!(asks.len(), 2);
            assert_eq!(bids.len(), 2);

            assert_eq!(
                asks,
                vec![
                    PriceLevel {
                        price: decimal!("110.0"),
                        quantity: decimal!("3.0"),
                    }
                    .to_proto(&exchange1),
                    PriceLevel {
                        price: decimal!("115.0"),
                        quantity: decimal!("3.5"),
                    }
                    .to_proto(&exchange2),
                ]
            );

            assert_eq!(
                bids,
                vec![
                    PriceLevel {
                        price: decimal!("100.0"),
                        quantity: decimal!("1.0"),
                    }
                    .to_proto(&exchange1),
                    PriceLevel {
                        price: decimal!("95.0"),
                        quantity: decimal!("1.5"),
                    }
                    .to_proto(&exchange2),
                ]
            );
        } else {
            panic!("Failed to receive first summary from the aggregator");
        }
    }
}
