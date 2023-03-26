use crate::proto;
use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PriceLevel {
    pub price: Decimal,
    pub quantity: Decimal,
}
impl<'de> Deserialize<'de> for PriceLevel {
    fn deserialize<D>(deserializer: D) -> Result<PriceLevel, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let (price, quantity): (String, String) = Deserialize::deserialize(deserializer)?;

        Ok(PriceLevel {
            price: Decimal::from_str_exact(&price).map_err(serde::de::Error::custom)?,
            quantity: Decimal::from_str_exact(&quantity).map_err(serde::de::Error::custom)?,
        })
    }
}
impl From<PriceLevel> for proto::PriceLevel {
    fn from(value: PriceLevel) -> Self {
        Self {
            exchange: "binance".to_owned(),
            amount: Some(value.quantity.into()),
            price: Some(value.price.into()),
        }
    }
}

#[cfg(test)]
mod price_level_tests;

#[derive(Debug, Deserialize)]
pub struct OrderBook {
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
}
impl From<OrderBook> for proto::Summary {
    fn from(value: OrderBook) -> Self {
        Self {
            spread: Some(value.spread().unwrap_or(Decimal::ZERO).into()),
            bids: value.bids.into_iter().map(From::from).collect(),
            asks: value.asks.into_iter().map(From::from).collect(),
        }
    }
}
impl OrderBook {
    fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids
            .iter()
            .max_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
    }
    fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks
            .iter()
            .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
    }
    fn spread(&self) -> Option<rust_decimal::Decimal> {
        self.best_bid()
            .zip(self.best_ask())
            .map(|(highest_bid, lowest_ask)| lowest_ask.price - highest_bid.price)
    }
}

#[cfg(test)]
mod order_book_test;
