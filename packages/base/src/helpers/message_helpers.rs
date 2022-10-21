use cosmwasm_std::{Attribute, Coin, Event, Uint128};
use std::collections::HashMap;

use crate::ContractError;

pub fn get_coin_from_display_formatted_coin(formatted_coin: String) -> Coin {
    let denom_start_index = formatted_coin
        .chars()
        .position(|c| !c.is_numeric())
        .unwrap();

    let amount: Uint128 = formatted_coin[0..denom_start_index]
        .to_string()
        .parse::<Uint128>()
        .unwrap();

    let denom = formatted_coin[denom_start_index..formatted_coin.len()].to_string();

    Coin { denom, amount }
}

pub fn get_flat_map_for_event_type(
    events: &[Event],
    event_type: &str,
) -> Result<HashMap<String, String>, ContractError> {
    let events_with_type = events.iter().filter(|event| event.ty == event_type);

    events_with_type
        .into_iter()
        .flat_map(|event| event.attributes.iter())
        .try_fold(HashMap::new(), |mut map, attribute| {
            map.insert(attribute.key.clone(), attribute.value.clone());
            Ok::<_, ContractError>(map)
        })
}

pub fn find_first_event_by_type(
    events: &[Event],
    target_type: &str,
) -> Result<Event, ContractError> {
    return events
        .iter()
        .find(|event| event.ty == target_type)
        .map(|event| event.to_owned())
        .ok_or_else(|| ContractError::CustomError {
            val: format!("could not find event with type: {}", &target_type),
        });
}

pub fn find_first_attribute_by_key(
    attributes: &[Attribute],
    target_key: &str,
) -> Result<Attribute, ContractError> {
    return attributes
        .iter()
        .find(|attribute| attribute.key == target_key)
        .map(|attribute| attribute.to_owned())
        .ok_or_else(|| ContractError::CustomError {
            val: format!("could not find attribute with key: {}", target_key),
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_first_event_by_type_finds_event_successfully() {
        let mock_event = Event::new("mock-event");
        let mock_wasm_trade_event = Event::new("wasm-trade");
        let events = vec![mock_event, mock_wasm_trade_event];
        let result = find_first_event_by_type(&events, "wasm-trade").unwrap();

        assert_eq!(result.ty, "wasm-trade");
    }

    #[test]
    fn find_first_event_by_type_given_two_matching_events_finds_first_event_successfully() {
        let mock_wasm_trade_event_one = Event::new("wasm-trade").add_attribute("index", "1");
        let mock_wasm_trade_event_two = Event::new("wasm-trade").add_attribute("index", "2");

        let events = vec![mock_wasm_trade_event_one, mock_wasm_trade_event_two];
        let result = find_first_event_by_type(&events, "wasm-trade").unwrap();

        assert_eq!(result.ty, "wasm-trade");

        assert_eq!(result.attributes[0].value, "1")
    }

    #[test]
    fn find_first_event_by_type_given_no_matching_events_should_fail() {
        let mock_wasm_trade_event = vec![Event::new("not-wasm-trade")];
        let result = find_first_event_by_type(&mock_wasm_trade_event, "wasm-trade").unwrap_err();

        assert_eq!(
            result.to_string(),
            "Custom Error val: \"could not find event with type: wasm-trade\""
        );
    }

    #[test]
    fn find_first_event_by_type_given_no_events_should_fail() {
        let empty: Vec<Event> = Vec::new();
        let result = find_first_event_by_type(&empty, "wasm-trade").unwrap_err();

        assert_eq!(
            result.to_string(),
            "Custom Error val: \"could not find event with type: wasm-trade\""
        );
    }

    #[test]
    fn find_first_attribute_by_key_finds_attribute_successfully() {
        let mock_attribute_one = Attribute::new("test-one", "value");
        let mock_attribute_two = Attribute::new("test-two", "value");
        let attributes = vec![mock_attribute_one, mock_attribute_two];
        let result = find_first_attribute_by_key(&attributes, "test-one").unwrap();

        assert_eq!(result.key, "test-one");
    }

    #[test]
    fn find_first_attribute_by_key_given_two_matching_attributes_finds_first_attribute_successfully(
    ) {
        let mock_attribute_one = Attribute::new("test", "1");
        let mock_attribute_two = Attribute::new("test", "2");
        let attributes = vec![mock_attribute_one, mock_attribute_two];
        let result = find_first_attribute_by_key(&attributes, "test").unwrap();

        assert_eq!(result.key, "test");

        assert_eq!(result.value, "1");
    }

    #[test]
    fn find_first_attribute_by_key_given_no_matching_attributes_should_fail() {
        let mock_attribute_one = Attribute::new("mock", "value");
        let attributes = vec![mock_attribute_one];
        let result = find_first_attribute_by_key(&attributes, "test-one").unwrap_err();

        assert_eq!(
            result.to_string(),
            "Custom Error val: \"could not find attribute with key: test-one\""
        );
    }

    #[test]
    fn find_first_attribute_by_key_given_no_attributes_should_fail() {
        let attributes: Vec<Attribute> = Vec::new();
        let result = find_first_attribute_by_key(&attributes, "test-one").unwrap_err();

        assert_eq!(
            result.to_string(),
            "Custom Error val: \"could not find attribute with key: test-one\""
        );
    }

    #[test]
    fn get_coin_from_amount_string_should_succeed_with_ukuji() {
        let mock_attribute_amount = String::from("492500ukuji");
        let result = get_coin_from_display_formatted_coin(mock_attribute_amount);

        assert_eq!(result.denom, "ukuji".to_string());
        assert_eq!(result.amount, Uint128::new(492500));
    }

    #[test]
    fn get_coin_from_amount_string_should_succeed_with_ibc() {
        let mock_attribute_amount = String::from(
            "123456789ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518",
        );
        let result = get_coin_from_display_formatted_coin(mock_attribute_amount);

        assert_eq!(
            result.denom,
            "ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518".to_string()
        );
        assert_eq!(result.amount, Uint128::new(123456789));
    }

    #[test]
    fn get_coin_from_amount_string_should_succeed_with_factory() {
        let mock_attribute_amount = String::from("123456789factory/kujira1r85reqy6h0lu02vyz0hnzhv5whsns55gdt4w0d7ft87utzk7u0wqr4ssll/uusk");
        let result = get_coin_from_display_formatted_coin(mock_attribute_amount);

        assert_eq!(
            result.denom,
            "factory/kujira1r85reqy6h0lu02vyz0hnzhv5whsns55gdt4w0d7ft87utzk7u0wqr4ssll/uusk"
                .to_string()
        );
        assert_eq!(result.amount, Uint128::new(123456789));
    }
}
