use base::triggers::time_trigger::TimeInterval;
use base::vaults::dca_vault::PositionType;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, from_binary, Coin, Uint128, Uint64};

use crate::contract::{execute, instantiate, query};
use crate::msg::{
    ExecuteMsg, ExecutionsResponse, InstantiateMsg, PairsResponse, QueryMsg, VaultResponse,
    VaultsResponse,
};

pub const INVALID_ADDRESS: &str = "";
pub const VALID_ADDRESS_ONE: &str = "kujira16q6jpx7ns0ugwghqay73uxd5aq30du3uqgxf0d";
pub const VALID_ADDRESS_TWO: &str = "kujira1cvlzqz80rp70xtmux9x69j4sr0rndh3yws2lfv";
pub const VALID_ADDRESS_THREE: &str = "kujira1cvlzqz80rp70xtmux9x69j4sr0rndh3yws2lf1";

// puill out common setup (instantiate and create pair)

#[test]
fn instantiation_with_valid_admin_address_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("", &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
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
        admin: String::from(INVALID_ADDRESS),
    };

    let result = instantiate(deps.as_mut(), env, info, instantiate_message).unwrap_err();

    assert_eq!(
        result.to_string(),
        "Generic error: Invalid input: human address too short"
    )
}

#[test]
fn create_pair_with_valid_address_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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
        admin: String::from(VALID_ADDRESS_ONE),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let _create_first_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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
        address: String::from(VALID_ADDRESS_TWO),
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
        admin: String::from(VALID_ADDRESS_ONE),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let execute_message = ExecuteMsg::CreatePair {
        address: String::from(INVALID_ADDRESS),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };

    let result = execute(deps.as_mut(), env, info, execute_message).unwrap_err();

    assert_eq!(
        result.to_string(),
        "Generic error: Invalid input: human address too short"
    )
}

#[test]
fn create_pair_with_unauthorised_sender_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
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
        address: String::from(INVALID_ADDRESS),
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
        admin: String::from(VALID_ADDRESS_ONE),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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
        address: String::from(VALID_ADDRESS_TWO),
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
        admin: String::from(VALID_ADDRESS_ONE),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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

    let get_all_pairs_query_message = QueryMsg::GetAllPairs {};
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
        admin: String::from(VALID_ADDRESS_ONE),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let get_all_pairs_query_message = QueryMsg::GetAllPairs {};
    let binary = query(deps.as_ref(), env, get_all_pairs_query_message).unwrap();
    let response: PairsResponse = from_binary(&binary).unwrap();
    assert_eq!(response.pairs.len(), 0);
}

#[test]
fn create_vault_with_time_trigger_and_valid_inputs_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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

    let create_vault_with_time_trigger_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
        pair_address: String::from(VALID_ADDRESS_TWO),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(30),
        total_triggers: 4,
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1762770365)),
    };

    let coin = Coin {
        denom: String::from("quote"),
        amount: Uint128::new(100),
    };

    let info_with_funds = mock_info(VALID_ADDRESS_THREE, &vec![coin]);

    let result = execute(
        deps.as_mut(),
        env,
        info_with_funds,
        create_vault_with_time_trigger_execute_message,
    )
    .unwrap();

    assert_eq!(
        result.attributes,
        vec![
            attr("method", "create_vault"),
            attr("id", "1"),
            attr("owner", "kujira1cvlzqz80rp70xtmux9x69j4sr0rndh3yws2lf1"),
            attr("vault_id", "1")
        ]
    )
}

#[test]
fn create_vault_with_time_trigger_and_no_target_start_time_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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

    let create_vault_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
        pair_address: String::from(VALID_ADDRESS_TWO),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(30),
        total_triggers: 4,
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: None,
    };

    let coin = Coin {
        denom: String::from("quote"),
        amount: Uint128::new(100),
    };

    let info_with_funds = mock_info(VALID_ADDRESS_THREE, &vec![coin]);

    let result = execute(
        deps.as_mut(),
        env.clone(),
        info_with_funds,
        create_vault_execute_message,
    )
    .unwrap();

    assert_eq!(
        result.attributes,
        vec![
            attr("method", "create_vault"),
            attr("id", "1"),
            attr("owner", "kujira1cvlzqz80rp70xtmux9x69j4sr0rndh3yws2lf1"),
            attr("vault_id", "1")
        ]
    )
}

#[test]
fn create_vault_with_time_trigger_and_no_funds_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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

    let create_vault_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
        pair_address: String::from(VALID_ADDRESS_TWO),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(30),
        total_triggers: 4,
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1762770365)),
    };

    let funds: Vec<Coin> = Vec::new();

    let info_with_no_funds = mock_info(VALID_ADDRESS_THREE, &funds);

    let result = execute(
        deps.as_mut(),
        env,
        info_with_no_funds,
        create_vault_execute_message,
    )
    .unwrap_err();

    assert_eq!(result.to_string(), "Error: no funds were sent")
}

#[test]
fn create_vault_with_time_trigger_with_too_many_executions_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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

    let create_vault_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
        pair_address: String::from(VALID_ADDRESS_TWO),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(40),
        total_triggers: 4,
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1762770365)),
    };

    let coin = Coin {
        denom: String::from("quote"),
        amount: Uint128::new(100),
    };

    let info_with_funds = mock_info(VALID_ADDRESS_THREE, &vec![coin]);

    let result = execute(
        deps.as_mut(),
        env,
        info_with_funds,
        create_vault_execute_message,
    )
    .unwrap_err();

    assert_eq!(
        result.to_string(),
        "Error: invalid number of executions: 4, swap amount: 40, starting balance: 100"
    )
}

#[test]
fn create_vault_with_time_trigger_and_too_few_triggers_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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

    let create_vault_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
        pair_address: String::from(VALID_ADDRESS_TWO),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(10),
        total_triggers: 1,
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1762770365)),
    };

    let coin = Coin {
        denom: String::from("quote"),
        amount: Uint128::new(100),
    };

    let info_with_funds = mock_info(VALID_ADDRESS_THREE, &vec![coin]);

    let result = execute(
        deps.as_mut(),
        env,
        info_with_funds,
        create_vault_execute_message,
    )
    .unwrap_err();

    assert_eq!(
        result.to_string(),
        "Error: invalid number of executions: 1, swap amount: 10, starting balance: 100"
    )
}

#[test]
fn create_vault_with_time_trigger_and_unwhitelisted_pair_address_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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

    let create_vault_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
        pair_address: String::from(VALID_ADDRESS_THREE),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(100),
        total_triggers: 1,
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1762770365)),
    };

    let coin = Coin {
        denom: String::from("quote"),
        amount: Uint128::new(100),
    };

    let info_with_funds = mock_info(VALID_ADDRESS_THREE, &vec![coin]);

    let result = execute(
        deps.as_mut(),
        env,
        info_with_funds,
        create_vault_execute_message,
    )
    .unwrap_err();

    assert_eq!(result.to_string(), "calc_base::pair::Pair not found")
}

#[test]
fn create_vault_with_time_trigger_and_trigger_time_in_past_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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

    let create_vault_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
        pair_address: String::from(VALID_ADDRESS_TWO),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(30),
        total_triggers: 4,
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1562770365)),
    };

    let coin = Coin {
        denom: String::from("quote"),
        amount: Uint128::new(100),
    };

    let info_with_funds = mock_info(VALID_ADDRESS_THREE, &vec![coin]);

    let result = execute(
        deps.as_mut(),
        env,
        info_with_funds,
        create_vault_execute_message,
    )
    .unwrap_err();

    assert_eq!(
        result.to_string(),
        "Error: target_start_time_utc_seconds must be some time in the future"
    )
}

#[test]
fn cancel_vault_by_address_and_id_with_valid_inputs_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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

    let create_vault_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
        pair_address: String::from(VALID_ADDRESS_TWO),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(30),
        total_triggers: 4,
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1762770365)),
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

    let cancel_vault_by_address_and_id_execute_message = ExecuteMsg::CancelVaultByAddressAndId {
        address: String::from(VALID_ADDRESS_THREE),
        vault_id: Uint128::new(1),
    };

    let result = execute(
        deps.as_mut(),
        env,
        info,
        cancel_vault_by_address_and_id_execute_message,
    )
    .unwrap();

    assert_eq!(
        result.attributes,
        vec![
            attr("method", "cancel_vault_by_address_and_id"),
            attr("owner", "kujira1cvlzqz80rp70xtmux9x69j4sr0rndh3yws2lf1"),
            attr("vault_id", "1")
        ]
    );
}

#[test]
fn get_all_active_vaults_with_one_vault_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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

    let create_vault_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
        pair_address: String::from(VALID_ADDRESS_TWO),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(30),
        total_triggers: 4,
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1762770365)),
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

    let get_all_active_vaults_query_message = QueryMsg::GetAllActiveVaults {};
    let binary = query(deps.as_ref(), env, get_all_active_vaults_query_message).unwrap();
    let result: VaultsResponse = from_binary(&binary).unwrap();

    assert_eq!(result.vaults.len(), 1);
}

#[test]
fn get_all_active_vaults_with_no_vaults_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let get_all_active_vaults_query_message = QueryMsg::GetAllActiveVaults {};
    let binary = query(deps.as_ref(), env, get_all_active_vaults_query_message).unwrap();
    let result: VaultsResponse = from_binary(&binary).unwrap();

    assert_eq!(result.vaults.len(), 0);
}

#[test]
fn get_active_vault_by_address_and_id_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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

    let create_vault_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
        pair_address: String::from(VALID_ADDRESS_TWO),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(30),
        total_triggers: 4,
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1662770365)),
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

    let get_active_vault_by_address_and_id_query_message = QueryMsg::GetActiveVaultByAddressAndId {
        address: String::from(VALID_ADDRESS_THREE),
        vault_id: Uint128::new(1),
    };
    let binary = query(
        deps.as_ref(),
        env,
        get_active_vault_by_address_and_id_query_message,
    )
    .unwrap();
    let result: VaultResponse = from_binary(&binary).unwrap();

    assert_eq!(result.vault.owner.to_string(), VALID_ADDRESS_THREE);
}

#[test]
fn get_all_active_vaults_by_address_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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

    let create_vault_execute_message_one = ExecuteMsg::CreateVaultWithTimeTrigger {
        pair_address: String::from(VALID_ADDRESS_TWO),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(30),
        total_triggers: 4,
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1662770365)),
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

    let create_vault_execute_message_two = ExecuteMsg::CreateVaultWithTimeTrigger {
        pair_address: String::from(VALID_ADDRESS_TWO),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(30),
        total_triggers: 4,
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1662770365)),
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

    let get_all_active_vaults_by_address_query_message = QueryMsg::GetAllActiveVaultsByAddress {
        address: String::from(VALID_ADDRESS_THREE),
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
fn get_all_executions_by_vault_id_for_new_vault_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: String::from(VALID_ADDRESS_TWO),
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

    let create_vault_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
        pair_address: String::from(VALID_ADDRESS_TWO),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(30),
        total_triggers: 4,
        time_interval: TimeInterval::Daily,
        target_start_time_utc_seconds: Some(Uint64::new(1762770365)),
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

    let get_all_executions_by_vault_id_query_message = QueryMsg::GetAllExecutionsByVaultId {
        vault_id: Uint128::new(1),
    };
    let binary = query(
        deps.as_ref(),
        env,
        get_all_executions_by_vault_id_query_message,
    )
    .unwrap();
    let result: ExecutionsResponse = from_binary(&binary).unwrap();

    assert_eq!(result.executions.len(), 0);
}

#[test]
fn get_all_executions_by_vault_id_for_non_existant_vault_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: String::from(VALID_ADDRESS_ONE),
    };
    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let get_all_executions_by_vault_id_query_message = QueryMsg::GetAllExecutionsByVaultId {
        vault_id: Uint128::new(1),
    };
    let binary = query(
        deps.as_ref(),
        env,
        get_all_executions_by_vault_id_query_message,
    )
    .unwrap_err();

    assert_eq!(
        binary.to_string(),
        "alloc::vec::Vec<calc_base::executions::execution::Execution<calc_base::executions::dca_execution::DCAExecutionInformation>> not found"
    );
}
