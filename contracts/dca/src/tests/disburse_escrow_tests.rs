use super::mocks::ADMIN;
use crate::{
    constants::{ONE, TEN},
    handlers::{
        disburse_escrow::disburse_escrow, get_events_by_resource_id::get_events_by_resource_id,
    },
    state::{
        config::get_config,
        disburse_escrow_tasks::{get_disburse_escrow_tasks, save_disburse_escrow_task},
        vaults::get_vault,
    },
    tests::{
        helpers::{instantiate_contract, setup_new_vault},
        mocks::{calc_mock_dependencies, DENOM_STAKE, DENOM_UOSMO},
    },
    types::{
        dca_plus_config::DcaPlusConfig,
        event::{Event, EventData},
        vault::{Vault, VaultStatus},
    },
};
use base::helpers::coin_helpers::subtract;
use cosmwasm_std::{
    testing::{mock_env, mock_info},
    to_binary, BankMsg, Coin, CosmosMsg, Decimal, StdError, SubMsg, Uint128,
};
use osmosis_std::types::osmosis::gamm::v2::QuerySpotPriceResponse;

#[test]
fn when_no_fee_is_owed_returns_entire_escrow_to_owner() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Inactive,
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                standard_dca_received_amount: Coin::new(ONE.into(), DENOM_STAKE),
                escrowed_balance: Coin::new((ONE * Decimal::percent(5)).into(), DENOM_STAKE),
                ..DcaPlusConfig::default()
            }),
            ..Vault::default()
        },
    );

    let response = disburse_escrow(deps.as_mut(), &env, info, vault.id).unwrap();

    assert!(response
        .messages
        .contains(&SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: vault.destinations[0].address.to_string(),
            amount: vec![vault.dca_plus_config.clone().unwrap().escrowed_balance]
        }))));
}

#[test]
fn when_large_fee_is_owed_returns_entire_escrow_to_fee_collector() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Inactive,
            swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
            received_amount: Coin::new(TEN.into(), DENOM_STAKE),
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                standard_dca_received_amount: Coin::new(ONE.into(), DENOM_STAKE),
                escrowed_balance: Coin::new((ONE * Decimal::percent(5)).into(), DENOM_STAKE),
                ..DcaPlusConfig::default()
            }),
            ..Vault::default()
        },
    );

    deps.querier.update_stargate(|path, _| match path {
        "/osmosis.gamm.v2.Query/SpotPrice" => to_binary(&QuerySpotPriceResponse {
            spot_price: "10.0".to_string(),
        }),
        _ => Err(StdError::generic_err("message not customised")),
    });

    let config = get_config(&deps.storage).unwrap();

    let response = disburse_escrow(deps.as_mut(), &env, info, vault.id).unwrap();

    assert!(response
        .messages
        .contains(&SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.fee_collectors[0].address.to_string(),
            amount: vec![vault.dca_plus_config.clone().unwrap().escrowed_balance]
        }))));
}

#[test]
fn publishes_escrow_disbursed_event() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Inactive,
            swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
            received_amount: Coin::new((TEN + ONE).into(), DENOM_STAKE),
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                escrowed_balance: Coin::new(
                    ((TEN + ONE) * Decimal::percent(5)).into(),
                    DENOM_STAKE,
                ),
                ..DcaPlusConfig::default()
            }),
            ..Vault::default()
        },
    );

    disburse_escrow(deps.as_mut(), &env, info, vault.id).unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
        .unwrap()
        .events;

    let performance_fee = Coin::new(
        (ONE * Decimal::percent(20) - Uint128::one()).into(),
        vault.get_receive_denom(),
    );

    println!("{:#?}", events);

    assert!(events.contains(&Event {
        id: 1,
        resource_id: vault.id,
        timestamp: env.block.time,
        block_height: env.block.height,
        data: EventData::DcaVaultEscrowDisbursed {
            amount_disbursed: Coin::new(
                (subtract(
                    &vault.dca_plus_config.unwrap().escrowed_balance,
                    &performance_fee
                )
                .unwrap())
                .amount
                .into(),
                DENOM_STAKE
            ),
            performance_fee,
        }
    }))
}

#[test]
fn sets_escrow_balance_to_zero() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Inactive,
            swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
            received_amount: Coin::new((TEN + ONE).into(), DENOM_STAKE),
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                escrowed_balance: Coin::new(
                    ((TEN + ONE) * Decimal::percent(5)).into(),
                    DENOM_STAKE,
                ),
                ..DcaPlusConfig::default()
            }),
            ..Vault::default()
        },
    );

    disburse_escrow(deps.as_mut(), &env, info, vault.id).unwrap();

    let dca_plus_config = get_vault(deps.as_ref().storage, vault.id)
        .unwrap()
        .dca_plus_config
        .unwrap();

    assert_eq!(
        dca_plus_config.escrowed_balance,
        Coin::new(0, vault.get_receive_denom())
    );
}

#[test]
fn deletes_disburse_escrow_task() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Inactive,
            swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
            received_amount: Coin::new((TEN + ONE).into(), DENOM_STAKE),
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                escrowed_balance: Coin::new(
                    ((TEN + ONE) * Decimal::percent(5)).into(),
                    DENOM_STAKE,
                ),
                ..DcaPlusConfig::default()
            }),
            ..Vault::default()
        },
    );

    save_disburse_escrow_task(
        deps.as_mut().storage,
        vault.id,
        env.block.time.minus_seconds(10),
    )
    .unwrap();

    let disburse_escrow_tasks_before =
        get_disburse_escrow_tasks(deps.as_ref().storage, env.block.time, None).unwrap();

    disburse_escrow(deps.as_mut(), &env, info, vault.id).unwrap();

    let disburse_escrow_tasks_after =
        get_disburse_escrow_tasks(deps.as_ref().storage, env.block.time, None).unwrap();

    assert_eq!(disburse_escrow_tasks_before.len(), 1);
    assert_eq!(disburse_escrow_tasks_after.len(), 0);
}

#[test]
fn when_not_a_dca_vault_returns_an_error() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Inactive,
            ..Vault::default()
        },
    );

    let response = disburse_escrow(deps.as_mut(), &env, info, vault.id).unwrap_err();

    assert_eq!(response.to_string(), "Error: Vault is not a DCA+ vault");
}
