use crate::proto;
use futures_util::Stream;
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

impl PriceLevel {
    pub fn to_proto(&self, exchange: &str) -> proto::PriceLevel {
        proto::PriceLevel {
            exchange: exchange.to_string(),
            amount: Some(self.quantity.into()),
            price: Some(self.price.into()),
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

#[tonic::async_trait]
pub trait GetOrderBooksStream {
    type Error;
    type OrderBooksStream: Stream<Item = Result<OrderBook, Self::Error>>;

    async fn get_order_books_stream(
        &self,
        base_currency: &str,
        quote_currency: &str,
    ) -> Result<Self::OrderBooksStream, Self::Error>;
}

#[cfg(test)]
mod order_book_test;
