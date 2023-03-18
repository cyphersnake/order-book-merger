use std::assert_matches::assert_matches;

use super::PriceLevel;

#[test]
fn test_deserialize_price_level() {
    let json_str = r#"["0.01248900","0.34500000"]"#;
    let expected = PriceLevel {
        price: 0.012489,
        quantity: 0.345,
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

