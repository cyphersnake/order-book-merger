use std::ops::Not;

use futures_util::sink::SinkExt;
use some_to_err::ErrOr;
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
    #[error("This pair not supported by service")]
    PairNotSupported {
        base_currency: String,
        quote_currency: String,
    },
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
    url: Url,
    base_currency: &str,
    quote_currency: &str,
) -> Result<impl Stream<Item = Result<OrderBook, Error>>, Error> {
    let (mut ws, _) = ws_connect(url).await?;
    ws.send(Message::Text(format!(
        r#"{{
            "event": "bts:subscribe",
            "data": {{
                "channel": "order_book_{base_currency}{quote_currency}"
            }}
        }}"#,
    )))
    .await?;

    Ok(ws.filter_map(|event| match event {
        Ok(Message::Text(text)) => {
            trace!("Receive: {text:?}");

            #[derive(Debug, serde::Deserialize)]
            struct Response {
                data: OrderBook,
            }

            match serde_json::from_str::<'_, Response>(&text) {
                Ok(response) => Some(Ok(response.data)),
                Err(error) => {
                    error!("{error:?}");
                    Some(Err(error.into()))
                }
            }
        }
        Err(err) => {
            error!("Error while handle bitstamp ws: {err:?}");
            Some(Err(Error::from(err)))
        }
        _something => {
            error!("Something: {_something:?}");
            None
        }
    }))
}

pub struct Bitstamp {
    ws_url: Url,
    supported_pairs: im::HashSet<&'static str>,
}
#[rustfmt::skip]
impl Default for Bitstamp {
    fn default() -> Self {
        Self {
            ws_url: "wss://ws.bitstamp.net/".parse().unwrap(),
            supported_pairs: im::HashSet::from_iter([
                "btcusd", "btceur", "btcgbp", "btcpax", "gbpusd", "gbpeur", "eurusd", "xrpusd",
                "xrpeur", "xrpbtc", "xrpgbp", "ltcbtc", "ltcusd", "ltceur", "ltcgbp", "ethbtc",
                "ethusd", "etheur", "ethgbp", "ethpax", "bchusd", "bcheur", "bchbtc", "paxusd",
                "xlmbtc", "xlmusd", "xlmeur", "xlmgbp", "linkusd", "linkeur", "linkgbp", "linkbtc",
                "omgusd", "omgeur", "omggbp", "omgbtc", "usdcusd", "usdceur", "btcusdc", "ethusdc",
                "eth2eth", "aaveusd", "aaveeur", "aavebtc", "batusd", "bateur", "umausd", "umaeur",
                "daiusd", "kncusd", "knceur", "mkrusd", "mkreur", "zrxusd", "zrxeur", "gusdusd",
                "algousd", "algoeur", "algobtc", "audiousd", "audioeur", "audiobtc", "crvusd",
                "crveur", "snxusd", "snxeur", "uniusd", "unieur", "unibtc", "yfiusd", "yfieur",
                "compusd", "compeur", "grtusd", "grteur", "lrcusd", "lrceur", "usdtusd", "usdteur",
                "usdcusdt", "btcusdt", "ethusdt", "xrpusdt", "eurteur", "eurtusd", "flrusd",
                "flreur", "manausd", "manaeur", "maticusd", "maticeur", "sushiusd", "sushieur",
                "chzusd", "chzeur", "enjusd", "enjeur", "hbarusd", "hbareur", "alphausd",
                "alphaeur", "axsusd", "axseur", "sandusd", "sandeur", "storjusd", "storjeur", "adausd",
                "adaeur", "adabtc", "fetusd", "feteur", "sklusd", "skleur", "slpusd", "slpeur", "sxpusd", "sxpeur",
                "sgbusd", "sgbeur", "avaxusd", "avaxeur", "dydxusd", "dydxeur", "ftmusd", "ftmeur", "shibusd",
                "shibeur", "ampusd", "ampeur", "ensusd", "enseur", "galausd", "galaeur", "perpusd", "perpeur",
                "wbtcbtc", "ctsiusd", "ctsieur", "cvxusd", "cvxeur", "imxusd", "imxeur", "nexousd", "nexoeur",
                "antusd", "anteur", "godsusd", "godseur", "radusd", "radeur", "bandusd", "bandeur", "injusd", "injeur",
                "rlyusd", "rlyeur", "rndrusd", "rndreur", "vegausd", "vegaeur", "1inchusd", "1incheur", "solusd",
                "soleur", "apeusd", "apeeur", "mplusd", "mpleur", "eurocusdc", "euroceur", "dotusd", "doteur",
                "nearusd", "neareur", "dogeusd", "dogeeur",
            ]),
        }
    }
}

impl Bitstamp {
    pub fn new(ws_url: Url) -> Self {
        Self {
            ws_url,
            ..Self::default()
        }
    }
}

#[tonic::async_trait]
impl GetOrderBooksStream for Bitstamp {
    type Error = Error;
    type OrderBooksStream = impl Stream<Item = Result<OrderBook, Self::Error>>;

    async fn get_order_books_stream(
        &self,
        base_currency: &str,
        quote_currency: &str,
    ) -> Result<Self::OrderBooksStream, Self::Error> {
        let pair = format!(
            "{base_currency}{quote_currency}",
            base_currency = base_currency.to_lowercase(),
            quote_currency = quote_currency.to_lowercase()
        );

        self.supported_pairs
            .contains(pair.as_str())
            .not()
            .then_some(Error::PairNotSupported {
                base_currency: base_currency.to_owned(),
                quote_currency: quote_currency.to_owned(),
            })
            .err_or(())?;

        get_summary_stream(self.ws_url.clone(), base_currency, quote_currency).await
    }
}
