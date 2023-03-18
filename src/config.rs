use std::net::SocketAddr;

pub use envconfig::Envconfig;

#[derive(Debug, Envconfig, PartialEq)]
pub struct Config {
    #[envconfig(from = "ORDERBOOK_ADDR", default = "127.0.0.1:8080")]
    pub addr: SocketAddr,
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;
    use maplit::hashmap;

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

