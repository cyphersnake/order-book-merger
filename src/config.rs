use std::net::SocketAddr;

pub use envconfig::Envconfig;
use url::Url;

#[derive(Debug, Envconfig, PartialEq)]
pub struct Config {
    #[envconfig(from = "ORDERBOOK_ADDR", default = "127.0.0.1:7777")]
    pub addr: SocketAddr,
    #[envconfig(
        from = "BINANCE_WEBSOCKET_ADDR",
        default = "wss://stream.binance.com:443/ws"
    )]
    pub binance_websocket_addr: Url,
    #[envconfig(from = "BITSTAMP_WEBSOCKET_ADDR", default = "wss://ws.bitstamp.net/")]
    pub bitstamp_websocket_addr: Url,
    #[envconfig(from = "BASE_CURRENCY", default = "btc")]
    pub base_currency: String,
    #[envconfig(from = "QUOTE_CURRENCY", default = "usdt")]
    pub quote_currency: String,
    #[envconfig(from = "SUMMARY_SIZE", default = "10")]
    pub summary_size: usize,
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use maplit::hashmap;

    use super::*;

    #[test]
    fn success_parse() {
        assert_eq!(
            SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 8080),
            Config::init_from_hashmap(&hashmap! {
                "ORDERBOOK_ADDR".to_owned() => "127.0.0.1:8080".to_owned()
            })
            .unwrap()
            .addr
        );
    }

    #[test]
    fn failed_parse() {
        assert_eq!(
            Config::init_from_hashmap(&hashmap! {
                "ORDERBOOK_ADDR".to_owned() => "".to_owned()
            }),
            Err(envconfig::Error::ParseError {
                name: "ORDERBOOK_ADDR"
            }),
        );
    }
}
