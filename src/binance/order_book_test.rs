use std::str::FromStr;

use super::{OrderBook, PriceLevel};
use rust_decimal::Decimal;

#[test]
fn test_order_book_deserialization() {
    const INPUT: &str = r#"{
                "lastUpdateId": 3007552324,
                "bids": [
                    ["0.01253600", "10.35400000"],
                    ["0.01253500", "3.17100000"],
                    ["0.01253400", "2.24200000"],
                    ["0.01253300", "0.31800000"],
                    ["0.01253200", "0.31800000"],
                    ["0.01253100", "0.31800000"],
                    ["0.01253000", "0.31800000"],
                    ["0.01252900", "0.31800000"],
                    ["0.01252800", "0.31800000"],
                    ["0.01252700", "0.31800000"]
                ],
                "asks": [
                    ["0.01253700", "2.30000000"],
                    ["0.01253800", "1.35300000"],
                    ["0.01253900", "5.32200000"],
                    ["0.01254000", "5.38700000"],
                    ["0.01254100", "2.24100000"],
                    ["0.01254200", "6.00200000"],
                    ["0.01254300", "2.90600000"],
                    ["0.01254400", "0.42400000"],
                    ["0.01254500", "0.86500000"],
                    ["0.01254600", "2.65300000"]
                ]
            }"#;

    let order_book: OrderBook = serde_json::from_str(INPUT).unwrap();

    assert_eq!(
        order_book.bids[0],
        PriceLevel {
            price: Decimal::from_str("0.01253600").unwrap(),
            quantity: Decimal::from_str("10.35400000").unwrap(),
        }
    );
    assert_eq!(
        order_book.bids[9],
        PriceLevel {
            price: Decimal::from_str("0.01252700").unwrap(),
            quantity: Decimal::from_str("0.31800000").unwrap(),
        }
    );

    assert_eq!(
        order_book.asks[0],
        PriceLevel {
            price: Decimal::from_str("0.01253700").unwrap(),
            quantity: Decimal::from_str("2.30000000").unwrap(),
        }
    );
    assert_eq!(
        order_book.asks[9],
        PriceLevel {
            price: Decimal::from_str("0.01254600").unwrap(),
            quantity: Decimal::from_str("2.65300000").unwrap(),
        }
    );
}

#[test]
fn test_spread() {
    // Example data taken from a Binance order book API response
    let orderbook_json = serde_json::json!({
        "lastUpdateId": 12345,
        "bids": [
            ["0.03562200", "7.90700000"],
            ["0.03561700", "12.20300000"]
        ],
        "asks": [
            ["0.03563400", "6.10000000"],
            ["0.03564400", "1.00000000"]
        ]
    });

    let orderbook: OrderBook = serde_json::from_value(orderbook_json).unwrap();

    let best_bid = orderbook.best_bid().map(|l| l.price);
    assert_eq!(best_bid, Some(Decimal::from_str("0.03562200").unwrap()));

    let best_ask = orderbook.best_ask().map(|l| l.price);
    assert_eq!(best_ask, Some(Decimal::from_str("0.03563400").unwrap()));

    let spread = match (best_bid, best_ask) {
        (Some(bid), Some(ask)) => ask - bid,
        _ => Decimal::ZERO,
    };

    assert_eq!(spread, Decimal::from_str("0.000012").unwrap());
    assert_eq!(spread, orderbook.spread().unwrap());
}
