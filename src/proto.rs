use std::cmp;

tonic::include_proto!("orderbook");

const DEFAULT_DECIMAL_SCALE: u32 = 100;

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
