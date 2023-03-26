use std::cmp;

tonic::include_proto!("orderbook");

const DEFAULT_DECIMAL_SCALE: u32 = 25;

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        // WARN This is a very resource-intensive comparison, the right
        // thing to do here would be to make a simplified version of
        // rust_decimal crate, but let's assume there is one here
        rust_decimal::Decimal::from(self).partial_cmp(&rust_decimal::Decimal::from(other))
    }
}

impl Eq for PriceLevel {}

impl PartialOrd for PriceLevel {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        match self.exchange.partial_cmp(&other.exchange) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.price.partial_cmp(&other.price) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.amount.partial_cmp(&other.amount)
    }
}
impl Ord for PriceLevel {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.partial_cmp(other).unwrap_or(cmp::Ordering::Equal)
    }
}

impl From<&Decimal> for rust_decimal::Decimal {
    fn from(value: &Decimal) -> Self {
        Self::from_parts(value.lo, value.mid, value.hi, false, DEFAULT_DECIMAL_SCALE)
    }
}
impl From<rust_decimal::Decimal> for Decimal {
    fn from(mut value: rust_decimal::Decimal) -> Self {
        value.rescale(DEFAULT_DECIMAL_SCALE);
        // We can bound flags by a bitwise mask to correspond to:
        //   Bits 0-15: unused
        //   Bits 16-23: Contains "e", a value between 0-28 that indicates the scale
        //   Bits 24-30: unused
        //   Bit 31: the sign of the Decimal value, 0 meaning positive and 1 meaning negative.
        let bytes = value.serialize();
        Self {
            lo: (bytes[4] as u32)
                | (bytes[5] as u32) << 8
                | (bytes[6] as u32) << 16
                | (bytes[7] as u32) << 24,
            mid: (bytes[8] as u32)
                | (bytes[9] as u32) << 8
                | (bytes[10] as u32) << 16
                | (bytes[11] as u32) << 24,
            hi: (bytes[12] as u32)
                | (bytes[13] as u32) << 8
                | (bytes[14] as u32) << 16
                | (bytes[15] as u32) << 24,
            view: value.to_string(),
        }
    }
}

impl Summary {
    pub fn new(asks: Vec<PriceLevel>, bids: Vec<PriceLevel>) -> Self {
        let mut self_ = Self {
            asks,
            bids,
            spread: None,
        };

        self_.spread = self_.calculate_spread();

        self_
    }

    fn calculate_spread(&self) -> Option<Decimal> {
        self.best_bid()
            .and_then(|bid| Some(rust_decimal::Decimal::from(bid.price.as_ref()?)))
            .zip(
                self.best_ask()
                    .and_then(|ask| Some(rust_decimal::Decimal::from(ask.price.as_ref()?))),
            )
            .map(|(highest_bid_price, lowest_ask_price)| {
                (lowest_ask_price - highest_bid_price).into()
            })
    }

    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.bids
            .iter()
            .max_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
    }
    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.asks
            .iter()
            .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_spread() {
        use std::str::FromStr;

        macro_rules! decimal {
            ($s:literal) => {
                rust_decimal::Decimal::from_str($s).unwrap().into()
            };
            ($val:ident) => {
                rust_decimal::Decimal::from(&$val)
            };
        }

        let orderbook = Summary {
            spread: Some(decimal!("0.0000120")),
            bids: vec![
                PriceLevel {
                    exchange: "unknown".to_owned(),
                    price: Some(decimal!("0.03562200")),
                    amount: Some(decimal!("7.90700000")),
                },
                PriceLevel {
                    exchange: "unknown".to_owned(),
                    price: Some(decimal!("0.03561700")),
                    amount: Some(decimal!("12.20300000")),
                },
            ],
            asks: vec![
                PriceLevel {
                    exchange: "unknown".to_owned(),
                    price: Some(decimal!("0.03563400")),
                    amount: Some(decimal!("6.10000000")),
                },
                PriceLevel {
                    exchange: "unkown".to_owned(),
                    price: Some(decimal!("0.03564400")),
                    amount: Some(decimal!("1.00000000")),
                },
            ],
        };

        let best_bid = orderbook.best_bid().and_then(|l| l.price.clone());
        assert_eq!(best_bid, Some(decimal!("0.03562200")));

        let best_ask = orderbook.best_ask().and_then(|l| l.price.clone());
        assert_eq!(best_ask, Some(decimal!("0.03563400")));

        let spread: Decimal = match (best_bid, best_ask) {
            (Some(bid), Some(ask)) => (decimal!(ask) - decimal!(bid)).into(),
            _ => decimal!("0"),
        };

        assert_eq!(spread, decimal!("0.0000120"));
        assert_eq!(spread, orderbook.calculate_spread().unwrap());
    }
}
