use std::str::FromStr;

use base::triggers::trigger::TimeInterval;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, from_binary, Addr, Coin, Decimal, Uint128, Uint64};

use crate::contract::{execute, instantiate, query};
use crate::msg::{
    EventsResponse, ExecuteMsg, InstantiateMsg, PairsResponse, QueryMsg, VaultResponse,
    VaultsResponse,
};
use crate::state::config::FeeCollector;
pub const INVALID_ADDRESS: &str = "";
pub const VALID_ADDRESS_ONE: &str = "kujira16q6jpx7ns0ugwghqay73uxd5aq30du3uqgxf0d";
pub const VALID_ADDRESS_TWO: &str = "kujira1cvlzqz80rp70xtmux9x69j4sr0rndh3yws2lfv";
pub const VALID_ADDRESS_THREE: &str = "kujira1cvlzqz80rp70xtmux9x69j4sr0rndh3yws2lf1";

// pull out common setup (instantiate and create pair)

#[test]
fn instantiation_with_valid_admin_address_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("", &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let result = instantiate(deps.as_mut(), env, info, instantiate_message).unwrap();

    assert_eq!(
        result.attributes,
        vec![
            attr("method", "instantiate"),
            attr("admin", "kujira16q6jpx7ns0ugwghqay73uxd5aq30du3uqgxf0d")
        ]
    )
}

#[test]
fn instantiation_with_invalid_admin_address_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("", &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(INVALID_ADDRESS),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let result = instantiate(deps.as_mut(), env, info, instantiate_message).unwrap_err();

    assert_eq!(
        result.to_string(),
        "Generic error: Invalid input: human address too short for this mock implementation (must be >= 3)."
    )
}

#[test]
fn instantiation_with_invalid_fee_collector_address_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("", &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: INVALID_ADDRESS.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let result = instantiate(deps.as_mut(), env, info, instantiate_message).unwrap_err();

    assert_eq!(
        result.to_string(),
        "Error: fee collector address  is invalid"
    )
}

#[test]
fn instantiation_with_fee_collector_amounts_not_equal_to_100_percent_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("", &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let result = instantiate(deps.as_mut(), env, info, instantiate_message).unwrap_err();

    assert_eq!(
        result.to_string(),
        "Error: fee collector allocations must add up to 1"
    )
}

#[test]
fn create_pair_with_valid_address_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: Addr::unchecked(VALID_ADDRESS_TWO),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };

    let result = execute(deps.as_mut(), env, info, create_pair_execute_message).unwrap();

    assert_eq!(
        result.attributes,
        vec![
            attr("method", "create_pair"),
            attr("address", "kujira1cvlzqz80rp70xtmux9x69j4sr0rndh3yws2lfv"),
            attr("base_denom", "base"),
            attr("quote_denom", "quote")
        ]
    )
}

#[test]
fn create_pair_that_already_exists_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let _create_first_pair_execute_message = ExecuteMsg::CreatePair {
        address: Addr::unchecked(VALID_ADDRESS_TWO),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };

    let _result_one = execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        _create_first_pair_execute_message,
    )
    .unwrap();

    let _create_second_pair_execute_message = ExecuteMsg::CreatePair {
        address: Addr::unchecked(VALID_ADDRESS_TWO),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };

    let result = execute(
        deps.as_mut(),
        env,
        info,
        _create_second_pair_execute_message,
    )
    .unwrap_err();

    assert_eq!(
        result.to_string(),
        "Error: pair already exists at given address"
    )
}

#[test]
fn create_pair_with_invalid_address_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let execute_message = ExecuteMsg::CreatePair {
        address: Addr::unchecked(INVALID_ADDRESS),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };

    let result = execute(deps.as_mut(), env, info, execute_message).unwrap_err();

    assert_eq!(
        result.to_string(),
        "Generic error: Invalid input: human address too short for this mock implementation (must be >= 3)."
    )
}

#[test]
fn create_pair_with_unauthorised_sender_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let info_with_unauthorised_sender = mock_info(VALID_ADDRESS_THREE, &vec![]);
    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: Addr::unchecked(INVALID_ADDRESS),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };

    let result = execute(
        deps.as_mut(),
        env,
        info_with_unauthorised_sender,
        create_pair_execute_message,
    )
    .unwrap_err();

    assert_eq!(result.to_string(), "Unauthorized")
}

#[test]
fn delete_pair_with_valid_address_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: Addr::unchecked(VALID_ADDRESS_TWO),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };

    let _create_pair_execute_message_result = execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        create_pair_execute_message,
    )
    .unwrap();

    let delete_pair_execute_message = ExecuteMsg::DeletePair {
        address: Addr::unchecked(VALID_ADDRESS_TWO),
    };

    let result = execute(deps.as_mut(), env, info, delete_pair_execute_message).unwrap();

    assert_eq!(
        result.attributes,
        vec![
            attr("method", "delete_pair"),
            attr("address", "kujira1cvlzqz80rp70xtmux9x69j4sr0rndh3yws2lfv")
        ]
    )
}

#[test]
fn get_all_pairs_with_one_whitelisted_pair_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: Addr::unchecked(VALID_ADDRESS_TWO),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };
    let _create_pair_execute_message_result = execute(
        deps.as_mut(),
        env.clone(),
        info,
        create_pair_execute_message,
    )
    .unwrap();

    let get_all_pairs_query_message = QueryMsg::GetPairs {};
    let binary = query(deps.as_ref(), env, get_all_pairs_query_message).unwrap();
    let response: PairsResponse = from_binary(&binary).unwrap();
    assert_eq!(response.pairs.len(), 1);
    assert_eq!(
        response.pairs[0].address.to_string(),
        String::from("kujira1cvlzqz80rp70xtmux9x69j4sr0rndh3yws2lfv")
    );
    assert_eq!(response.pairs[0].base_denom, String::from("base"));
    assert_eq!(response.pairs[0].quote_denom, String::from("quote"));
}

#[test]
fn get_all_pairs_with_no_whitelisted_pairs_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let get_all_pairs_query_message = QueryMsg::GetPairs {};
    let binary = query(deps.as_ref(), env, get_all_pairs_query_message).unwrap();
    let response: PairsResponse = from_binary(&binary).unwrap();
    assert_eq!(response.pairs.len(), 0);
}

#[test]
fn cancel_vault_with_valid_inputs_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: Addr::unchecked(VALID_ADDRESS_TWO),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };
    let _create_pair_execute_message_result = execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        create_pair_execute_message,
    )
    .unwrap();

    let create_vault_execute_message = ExecuteMsg::CreateVault {
        owner: None,
        label: Some("label".to_string()),
        destinations: None,
        pair_address: Addr::unchecked(VALID_ADDRESS_TWO),
        position_type: None,
        slippage_tolerance: None,
        swap_amount: Uint128::new(50001u128),
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1762770365)),
        target_receive_amount: None,
        minimum_receive_amount: None,
    };

    let coin = Coin {
        denom: String::from("quote"),
        amount: Uint128::new(100),
    };

    let info_with_funds = mock_info(VALID_ADDRESS_THREE, &vec![coin]);

    let _create_vault_execute_message_result = execute(
        deps.as_mut(),
        env.clone(),
        info_with_funds.clone(),
        create_vault_execute_message,
    )
    .unwrap();

    let cancel_vault_execute_message = ExecuteMsg::CancelVault {
        vault_id: Uint128::new(1),
    };

    let result = execute(deps.as_mut(), env, info, cancel_vault_execute_message).unwrap();

    assert_eq!(
        result.attributes,
        vec![
            attr("method", "cancel_vault"),
            attr("owner", "kujira1cvlzqz80rp70xtmux9x69j4sr0rndh3yws2lf1"),
            attr("vault_id", "1")
        ]
    );
}

#[test]
fn get_active_vault_by_address_and_id_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: Addr::unchecked(VALID_ADDRESS_TWO),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };
    let _create_pair_execute_message_result = execute(
        deps.as_mut(),
        env.clone(),
        info,
        create_pair_execute_message,
    )
    .unwrap();

    let create_vault_execute_message = ExecuteMsg::CreateVault {
        owner: None,
        label: Some("label".to_string()),
        destinations: None,
        pair_address: Addr::unchecked(VALID_ADDRESS_TWO),
        position_type: None,
        slippage_tolerance: None,
        swap_amount: Uint128::new(50001u128),
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1662770365)),
        target_receive_amount: None,
        minimum_receive_amount: None,
    };

    let coin = Coin {
        denom: String::from("quote"),
        amount: Uint128::new(100),
    };

    let info_with_funds = mock_info(VALID_ADDRESS_THREE, &vec![coin]);

    let _create_vault_execute_message = execute(
        deps.as_mut(),
        env.clone(),
        info_with_funds,
        create_vault_execute_message,
    )
    .unwrap();

    let get_vault_query_message = QueryMsg::GetVault {
        vault_id: Uint128::new(1),
    };

    let binary = query(deps.as_ref(), env, get_vault_query_message).unwrap();

    let result: VaultResponse = from_binary(&binary).unwrap();

    assert_eq!(result.vault.owner.to_string(), VALID_ADDRESS_THREE);
}

#[test]
fn get_all_active_vaults_by_address_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: Addr::unchecked(VALID_ADDRESS_TWO),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };
    let _create_pair_execute_message_result = execute(
        deps.as_mut(),
        env.clone(),
        info,
        create_pair_execute_message,
    )
    .unwrap();

    let create_vault_execute_message_one = ExecuteMsg::CreateVault {
        owner: None,
        label: Some("label".to_string()),
        destinations: None,
        pair_address: Addr::unchecked(VALID_ADDRESS_TWO),
        position_type: None,
        slippage_tolerance: None,
        swap_amount: Uint128::new(50001u128),
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1662770365)),
        target_receive_amount: None,
        minimum_receive_amount: None,
    };

    let coin_one = Coin {
        denom: String::from("quote"),
        amount: Uint128::new(100),
    };

    let info_with_funds_one = mock_info(VALID_ADDRESS_THREE, &vec![coin_one]);
    let _create_vault_execute_message_one = execute(
        deps.as_mut(),
        env.clone(),
        info_with_funds_one,
        create_vault_execute_message_one,
    )
    .unwrap();

    let create_vault_execute_message_two = ExecuteMsg::CreateVault {
        owner: None,
        label: Some("label".to_string()),
        destinations: None,
        pair_address: Addr::unchecked(VALID_ADDRESS_TWO),
        position_type: None,
        slippage_tolerance: None,
        swap_amount: Uint128::new(50001u128),
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1662770365)),
        target_receive_amount: None,
        minimum_receive_amount: None,
    };

    let coin_two = Coin {
        denom: String::from("quote"),
        amount: Uint128::new(100),
    };

    let info_with_funds_two = mock_info(VALID_ADDRESS_ONE, &vec![coin_two]);
    let _create_vault_execute_message_two = execute(
        deps.as_mut(),
        env.clone(),
        info_with_funds_two,
        create_vault_execute_message_two,
    )
    .unwrap();

    let get_all_active_vaults_by_address_query_message = QueryMsg::GetVaultsByAddress {
        address: Addr::unchecked(VALID_ADDRESS_THREE),
        status: None,
        start_after: None,
        limit: None,
    };
    let binary = query(
        deps.as_ref(),
        env,
        get_all_active_vaults_by_address_query_message,
    )
    .unwrap();
    let result: VaultsResponse = from_binary(&binary).unwrap();

    assert_eq!(result.vaults.len(), 1);
}

#[test]
fn get_all_events_by_vault_id_for_new_vault_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: Addr::unchecked(VALID_ADDRESS_TWO),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };
    let _create_pair_execute_message_result = execute(
        deps.as_mut(),
        env.clone(),
        info,
        create_pair_execute_message,
    )
    .unwrap();

    let create_vault_execute_message = ExecuteMsg::CreateVault {
        owner: None,
        label: Some("label".to_string()),
        destinations: None,
        pair_address: Addr::unchecked(VALID_ADDRESS_TWO),
        position_type: None,
        slippage_tolerance: None,
        swap_amount: Uint128::new(50001u128),
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1762770365)),
        target_receive_amount: None,
        minimum_receive_amount: None,
    };

    let coin = Coin {
        denom: String::from("quote"),
        amount: Uint128::new(100),
    };

    let funds_with_info = mock_info(VALID_ADDRESS_THREE, &vec![coin]);

    let _ = execute(
        deps.as_mut(),
        env.clone(),
        funds_with_info,
        create_vault_execute_message,
    )
    .unwrap();

    let get_all_events_by_resource_id_query_message = QueryMsg::GetEventsByResourceId {
        resource_id: Uint128::new(1),
        start_after: None,
        limit: None,
    };
    let binary = query(
        deps.as_ref(),
        env,
        get_all_events_by_resource_id_query_message,
    )
    .unwrap();
    let result: EventsResponse = from_binary(&binary).unwrap();

    assert_eq!(result.events.len(), 2);
}

#[test]
fn get_all_events_by_vault_id_for_non_existent_vault_should_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let get_all_events_by_resource_id_query_message = QueryMsg::GetEventsByResourceId {
        resource_id: Uint128::new(1),
        start_after: None,
        limit: None,
    };

    let response: EventsResponse = from_binary(
        &query(
            deps.as_ref(),
            env,
            get_all_events_by_resource_id_query_message,
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(response.events, vec![]);
}
