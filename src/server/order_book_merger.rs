use std::{cmp, collections::HashMap};

use crate::merge_iter::MergeSortedIter;
use crate::order_book::OrderBook;
use crate::proto::Summary;
use tracing::*;

pub type ExchangeName = String;

#[derive(Debug)]
pub struct OrderBookMerger {
    exchanges_summaries: HashMap<ExchangeName, OrderBook>,
    summary_size: usize,
}
impl OrderBookMerger {
    pub fn new(summary_size: usize) -> Self {
        Self {
            summary_size,
            ..Self::default()
        }
    }
}
impl Default for OrderBookMerger {
    fn default() -> Self {
        Self {
            exchanges_summaries: Default::default(),
            summary_size: 10,
        }
    }
}
impl OrderBookMerger {
    pub fn insert(&mut self, exchange: &ExchangeName, order_book: OrderBook) {
        // TODO Optimise insertion so there is no string copying every time
        self.exchanges_summaries
            .insert(exchange.clone(), order_book);
    }

    pub fn get_summary(&self) -> Summary {
        info!(
            "Exchanges for merge: {exchanges:?}",
            exchanges = self.exchanges_summaries.keys(),
        );

        let asks = MergeSortedIter::new(
            self.exchanges_summaries
                .iter()
                .map(|(exchange, sum)| sum.asks.iter().map(|l| l.to_proto(exchange))),
        )
        .take(self.summary_size)
        .collect();

        let bids = MergeSortedIter::new(self.exchanges_summaries.iter().map(
            |(exchange, order_book)| {
                order_book
                    .bids
                    .iter()
                    .map(|l| cmp::Reverse(l.to_proto(exchange)))
            },
        ))
        .take(self.summary_size)
        .map(|reversed| reversed.0)
        .collect();

        Summary::new(asks, bids)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::order_book::PriceLevel;
    use rust_decimal::Decimal;

    macro_rules! decimal {
        ($s:literal) => {
            rust_decimal::Decimal::from_str($s).unwrap()
        };
        ($val:ident) => {
            rust_decimal::Decimal::from(&$val)
        };
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

    #[test]
    fn test_merged_summary_get_summary() {
        let exchange1 = "exchange1".to_string();
        let exchange2 = "exchange2".to_string();

        let mut merged_summary = OrderBookMerger::default();

        merged_summary.insert(
            &exchange1,
            create_order_book(
                vec![
                    (decimal!("100.0"), decimal!("1.0")),
                    (decimal!("90.0"), decimal!("2.0")),
                ],
                vec![
                    (decimal!("110.0"), decimal!("3.0")),
                    (decimal!("120.0"), decimal!("4.0")),
                ],
            ),
        );

        merged_summary.insert(
            &exchange2,
            create_order_book(
                vec![
                    (decimal!("95.0"), decimal!("1.5")),
                    (decimal!("85.0"), decimal!("2.5")),
                ],
                vec![
                    (decimal!("115.0"), decimal!("3.5")),
                    (decimal!("125.0"), decimal!("4.5")),
                ],
            ),
        );

        let summary = merged_summary.get_summary();
        assert_eq!(summary.asks.len(), 4);
        assert_eq!(summary.bids.len(), 4);

        assert_eq!(
            summary.asks,
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
                PriceLevel {
                    price: decimal!("120.0"),
                    quantity: decimal!("4.0"),
                }
                .to_proto(&exchange1),
                PriceLevel {
                    price: decimal!("125.0"),
                    quantity: decimal!("4.5"),
                }
                .to_proto(&exchange2),
            ]
        );

        assert_eq!(
            summary.bids,
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
                PriceLevel {
                    price: decimal!("90.0"),
                    quantity: decimal!("2.0"),
                }
                .to_proto(&exchange1),
                PriceLevel {
                    price: decimal!("85.0"),
                    quantity: decimal!("2.5"),
                }
                .to_proto(&exchange2),
            ]
        );
    }
}
