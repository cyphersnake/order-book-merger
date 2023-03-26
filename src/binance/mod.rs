use rust_decimal::Decimal;
use serde::Deserialize;
use tokio_stream::{Stream, StreamExt};
use tracing::*;
use url::Url;

use async_tungstenite::{tokio::connect_async as ws_connect, tungstenite::Message};

use crate::{proto, GetSummaryStream};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Client(#[from] async_tungstenite::tungstenite::Error),
    #[error(transparent)]
    Format(#[from] serde_json::Error),
    #[error("The input URL cannot be a base URL. Please provide a full URL.")]
    UrlCannotBeBase,
}

#[derive(Debug, PartialEq)]
struct PriceLevel {
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
struct OrderBook {
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Depth {
    _5,
    _10,
    _20,
}
impl From<Depth> for u8 {
    fn from(value: Depth) -> Self {
        match value {
            Depth::_5 => 5,
            Depth::_10 => 10,
            Depth::_20 => 20,
        }
    }
}

pub async fn get_summary_stream(
    mut url: Url,
    base_currency: &str,
    quote_currency: &str,
    depth: Depth,
) -> Result<impl Stream<Item = Result<proto::Summary, Error>>, Error> {
    url.path_segments_mut()
        .map_err(|()| Error::UrlCannotBeBase)?
        .push(
            format!(
                "{base_currency}{quote_currency}@depth{depth}",
                depth = u8::from(depth)
            )
            .as_str(),
        );

    Ok(ws_connect(url).await?.0.filter_map(|event| match event {
        Ok(Message::Text(text)) => {
            trace!("Receive: {text:?}");
            match serde_json::from_str::<'_, OrderBook>(&text) {
                Ok(order_book) => Some(Ok(proto::Summary::from(order_book))),
                Err(error) => Some(Err(Error::Format(error))),
            }
        }
        Err(err) => {
            error!("Error while handle binance ws: {err:?}");
            Some(Err(Error::from(err)))
        }
        _ => None,
    }))
}

pub struct Binance {
    pub ws_url: Url,
    pub depth: Depth,
}

#[tonic::async_trait]
impl GetSummaryStream for Binance {
    type Error = Error;
    type SummaryStream = impl Stream<Item = Result<proto::Summary, Self::Error>>;

    async fn get_summary_stream(
        &self,
        base_currency: &str,
        quote_currency: &str,
    ) -> Result<Self::SummaryStream, Self::Error> {
        get_summary_stream(
            self.ws_url.clone(),
            base_currency,
            quote_currency,
            self.depth,
        )
        .await
    }
}
