use cosmwasm_std::{Coin, Event, StdError, StdResult, Uint128};
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

pub fn get_attribute_in_event(
    events: &[Event],
    event_type: &str,
    attribute_key: &str,
) -> StdResult<String> {
    let events_with_type = events.iter().filter(|event| event.ty == event_type);

    let attribute = events_with_type
        .into_iter()
        .flat_map(|event| event.attributes.iter())
        .find(|attribute| attribute.key == attribute_key)
        .ok_or(StdError::generic_err(format!(
            "unable to find {} attribute in {} event",
            attribute_key, event_type
        )))?;

    Ok(attribute.value.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_flat_map_for_event_type_finds_event_successfully() {
        let result = get_flat_map_for_event_type(
            &vec![Event::new("mock-event"), Event::new("wasm-trade")],
            "wasm-trade",
        )
        .unwrap();

        assert_eq!(result, HashMap::new());
    }

    #[test]
    fn get_flat_map_for_event_type_uses_later_values_in_result_conflicts() {
        let result = get_flat_map_for_event_type(
            &vec![
                Event::new("wasm-trade").add_attribute("index", "1"),
                Event::new("wasm-trade").add_attribute("index", "2"),
            ],
            "wasm-trade",
        )
        .unwrap();

        assert_eq!(
            result,
            HashMap::from([("index".to_string(), "2".to_string())])
        );
    }

    #[test]
    fn get_flat_map_for_event_type_with_no_matching_event_type_should_return_empty_map() {
        let result =
            get_flat_map_for_event_type(&vec![Event::new("not-wasm-trade")], "wasm-trade").unwrap();

        assert_eq!(result, HashMap::new());
    }

    #[test]
    fn get_flat_map_for_event_type_with_no_events_should_return_empty_map() {
        let result = get_flat_map_for_event_type(&vec![], "wasm-trade").unwrap();

        assert_eq!(result, HashMap::new());
    }

    #[test]
    fn get_flat_map_for_event_type_should_combine_events() {
        let result = get_flat_map_for_event_type(
            &vec![
                Event::new("wasm-trade").add_attribute("amount", "1"),
                Event::new("wasm-trade").add_attribute("index", "2"),
            ],
            "wasm-trade",
        )
        .unwrap();

        assert_eq!(
            result,
            HashMap::from([
                ("amount".to_string(), "1".to_string()),
                ("index".to_string(), "2".to_string())
            ])
        );
    }

    #[test]
    fn get_attribute_in_event_finds_value_succesfully() {
        let result = get_attribute_in_event(
            &vec![Event::new("wasm-trade").add_attribute("index", "1")],
            "wasm-trade",
            "index",
        )
        .unwrap();

        assert_eq!(result, "1");
    }

    #[test]
    fn get_attribute_in_event_finds_last_value_succesfully() {
        let result = get_attribute_in_event(
            &vec![
                Event::new("wasm-trade").add_attribute("index", "1"),
                Event::new("wasm-trade").add_attribute("index", "2"),
            ],
            "wasm-trade",
            "index",
        )
        .unwrap();

        assert_eq!(result, "1");
    }

    #[test]
    fn get_attribute_in_event_with_no_matching_event_type_should_return_error() {
        let result = get_attribute_in_event(
            &vec![Event::new("not-wasm-trade").add_attribute("index", "1")],
            "wasm-trade",
            "index",
        );

        assert_eq!(
            result.unwrap_err().to_string(),
            "Generic error: unable to find index attribute in wasm-trade event"
        );
    }

    #[test]
    fn get_attribute_in_event_with_no_matching_attribute_should_return_error() {
        let result = get_attribute_in_event(
            &vec![Event::new("wasm-trade").add_attribute("not-index", "1")],
            "wasm-trade",
            "index",
        );

        assert_eq!(
            result.unwrap_err().to_string(),
            "Generic error: unable to find index attribute in wasm-trade event"
        );
    }

    #[test]
    fn get_attribute_in_event_with_no_events_should_return_error() {
        let result = get_attribute_in_event(&vec![], "wasm-trade", "index");

        assert_eq!(
            result.unwrap_err().to_string(),
            "Generic error: unable to find index attribute in wasm-trade event"
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
