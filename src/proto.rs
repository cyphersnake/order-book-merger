tonic::include_proto!("orderbook");

const DEFAULT_DECIMAL_SCALE: u32 = 100;

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
