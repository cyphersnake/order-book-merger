use std::{assert_matches::assert_matches, str::FromStr};

use rust_decimal::Decimal;

use super::PriceLevel;

#[test]
fn test_deserialize_price_level() {
    let json_str = r#"["0.01248900","0.34500000"]"#;
    let expected = PriceLevel {
        price: Decimal::from_str("0.012489").unwrap(),
        quantity: Decimal::from_str("0.345").unwrap(),
    };
    let price_level: PriceLevel = serde_json::from_str(json_str).unwrap();
    assert_eq!(price_level, expected);
}

#[test]
fn test_deserialize_price_level_invalid_price() {
    let json_str = r#"["abc","0.34500000"]"#;
    let price_level_result: Result<PriceLevel, _> = serde_json::from_str(json_str);
    assert_matches!(price_level_result, Err(_));
}

#[test]
fn test_deserialize_price_level_invalid_quantity() {
    let json_str = r#"["0.01248900","abc"]"#;
    let price_level_result: Result<PriceLevel, _> = serde_json::from_str(json_str);
    assert_matches!(price_level_result, Err(_));
}
