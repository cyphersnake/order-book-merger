use tokio_stream::{Stream, StreamExt};
use tracing::*;
use url::Url;

use async_tungstenite::{tokio::connect_async as ws_connect, tungstenite::Message};

use crate::order_book::{GetOrderBooksStream, OrderBook};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Client(#[from] async_tungstenite::tungstenite::Error),
    #[error(transparent)]
    Format(#[from] serde_json::Error),
    #[error("The input URL cannot be a base URL. Please provide a full URL.")]
    UrlCannotBeBase,
}

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
) -> Result<impl Stream<Item = Result<OrderBook, Error>>, Error> {
    url.path_segments_mut()
        .map_err(|()| Error::UrlCannotBeBase)?
        .push(
            format!(
                "{base_currency}{quote_currency}@depth{depth}",
                depth = u8::from(depth)
            )
            .as_str(),
        );

    info!("Connect to binance by {url}");

    Ok(ws_connect(url).await?.0.filter_map(|event| match event {
        Ok(Message::Text(text)) => {
            trace!("Receive: {text:?}");
            match serde_json::from_str::<'_, OrderBook>(&text) {
                Ok(order_book) => Some(Ok(order_book)),
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
impl GetOrderBooksStream for Binance {
    type Error = Error;
    type OrderBooksStream = impl Stream<Item = Result<OrderBook, Self::Error>>;

    async fn get_order_books_stream(
        &self,
        base_currency: &str,
        quote_currency: &str,
    ) -> Result<Self::OrderBooksStream, Self::Error> {
        get_summary_stream(
            self.ws_url.clone(),
            base_currency,
            quote_currency,
            self.depth,
        )
        .await
    }
}
