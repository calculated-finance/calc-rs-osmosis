use super::{
    helpers::{
        instantiate_contract_with_community_pool_fee_collector,
        setup_active_dca_plus_vault_with_funds,
    },
    mocks::ADMIN,
};
use crate::{
    constants::{ONE, ONE_DECIMAL, TEN, TEN_MICRONS},
    contract::AFTER_BANK_SWAP_REPLY_ID,
    handlers::{
        disburse_escrow::disburse_escrow_handler,
        get_events_by_resource_id::get_events_by_resource_id,
    },
    state::vaults::{get_vault, update_vault},
    tests::{
        helpers::{set_fin_price, setup_active_vault_with_funds},
        mocks::FEE_COLLECTOR,
    },
    types::dca_plus_config::DcaPlusConfig,
};
use base::{
    events::event::{Event, EventData},
    helpers::coin_helpers::subtract,
    vaults::vault::VaultStatus,
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    BankMsg, Coin, CosmosMsg, Decimal, SubMsg, Uint128,
};

#[test]
fn when_no_fee_is_owed_returns_entire_escrow_to_owner() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract_with_community_pool_fee_collector(
        deps.as_mut(),
        env.clone(),
        info.clone(),
    );
    set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

    let mut vault = setup_active_dca_plus_vault_with_funds(deps.as_mut(), env.clone());

    vault.status = VaultStatus::Inactive;
    vault.dca_plus_config = Some(DcaPlusConfig {
        escrow_level: Decimal::percent(5),
        model_id: 50,
        total_deposit: Coin::new(TEN.into(), vault.get_swap_denom()),
        standard_dca_swapped_amount: Coin::new(vault.swap_amount.into(), vault.get_swap_denom()),
        standard_dca_received_amount: Coin::new(
            vault.swap_amount.into(),
            vault.get_receive_denom(),
        ),
        escrowed_balance: Coin::new(
            (vault.swap_amount * Decimal::percent(5)).into(),
            vault.get_receive_denom(),
        ),
    });

    update_vault(deps.as_mut().storage, &vault).unwrap();

    let response = disburse_escrow_handler(deps.as_mut(), env, info, vault.id).unwrap();

    assert!(response.messages.contains(&SubMsg::reply_on_success(
        CosmosMsg::Bank(BankMsg::Send {
            to_address: vault.destinations[0].address.to_string(),
            amount: vec![vault.dca_plus_config.clone().unwrap().escrowed_balance]
        }),
        AFTER_BANK_SWAP_REPLY_ID
    )));
}

#[test]
fn when_large_fee_is_owed_returns_entire_escrow_to_fee_collector() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract_with_community_pool_fee_collector(
        deps.as_mut(),
        env.clone(),
        info.clone(),
    );

    set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

    let mut vault = setup_active_dca_plus_vault_with_funds(deps.as_mut(), env.clone());

    vault.status = VaultStatus::Inactive;
    vault.dca_plus_config = Some(DcaPlusConfig {
        escrow_level: Decimal::percent(5),
        model_id: 50,
        total_deposit: Coin::new(TEN.into(), vault.get_swap_denom()),
        standard_dca_swapped_amount: Coin::new(vault.swap_amount.into(), vault.get_swap_denom()),
        standard_dca_received_amount: Coin::new(
            (vault.swap_amount / Uint128::new(10)).into(),
            vault.get_receive_denom(),
        ),
        escrowed_balance: Coin::new(
            (vault.swap_amount / Uint128::new(10) * Decimal::percent(5)).into(),
            vault.get_receive_denom(),
        ),
    });

    update_vault(deps.as_mut().storage, &vault).unwrap();

    let response = disburse_escrow_handler(deps.as_mut(), env, info, vault.id).unwrap();

    assert!(response
        .messages
        .contains(&SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: FEE_COLLECTOR.to_string(),
            amount: vec![vault.dca_plus_config.clone().unwrap().escrowed_balance]
        }))));
}

#[test]
fn publishes_escrow_disbursed_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract_with_community_pool_fee_collector(
        deps.as_mut(),
        env.clone(),
        info.clone(),
    );

    set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

    let mut vault = setup_active_dca_plus_vault_with_funds(deps.as_mut(), env.clone());

    let escrowed_balance = Coin::new(
        ((TEN + ONE) * Decimal::percent(5)).into(),
        vault.get_receive_denom(),
    );

    vault.status = VaultStatus::Inactive;
    vault.swapped_amount = Coin::new(TEN.into(), vault.get_swap_denom());
    vault.received_amount = Coin::new((TEN + ONE).into(), vault.get_receive_denom());
    vault.dca_plus_config = Some(DcaPlusConfig {
        escrow_level: Decimal::percent(5),
        model_id: 50,
        total_deposit: Coin::new(TEN.into(), vault.get_swap_denom()),
        standard_dca_swapped_amount: Coin::new(TEN.into(), vault.get_swap_denom()),
        standard_dca_received_amount: Coin::new(TEN.into(), vault.get_receive_denom()),
        escrowed_balance: escrowed_balance.clone(),
    });

    update_vault(deps.as_mut().storage, &vault).unwrap();

    disburse_escrow_handler(deps.as_mut(), env.clone(), info, vault.id).unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
        .unwrap()
        .events;

    let performance_fee = Coin::new(
        (ONE * Decimal::percent(20)).into(), // rounding error
        vault.get_receive_denom(),
    );

    assert!(events.contains(&Event {
        id: 1,
        resource_id: vault.id,
        timestamp: env.block.time,
        block_height: env.block.height,
        data: EventData::DcaVaultEscrowDisbursed {
            amount_disbursed: Coin::new(
                (subtract(&escrowed_balance, &performance_fee).unwrap())
                    .amount
                    .into(),
                vault.get_receive_denom()
            ),
            performance_fee,
        }
    }))
}

#[test]
fn sets_escrow_balance_to_zero() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract_with_community_pool_fee_collector(
        deps.as_mut(),
        env.clone(),
        info.clone(),
    );
    set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

    let mut vault = setup_active_dca_plus_vault_with_funds(deps.as_mut(), env.clone());

    vault.status = VaultStatus::Inactive;
    vault.dca_plus_config = Some(DcaPlusConfig {
        escrow_level: Decimal::percent(5),
        model_id: 50,
        total_deposit: Coin::new(TEN.into(), vault.get_swap_denom()),
        standard_dca_swapped_amount: Coin::new(vault.swap_amount.into(), vault.get_swap_denom()),
        standard_dca_received_amount: Coin::new(
            vault.swap_amount.into(),
            vault.get_receive_denom(),
        ),
        escrowed_balance: Coin::new(
            (vault.swap_amount * Decimal::percent(5)).into(),
            vault.get_receive_denom(),
        ),
    });

    update_vault(deps.as_mut().storage, &vault).unwrap();

    disburse_escrow_handler(deps.as_mut(), env, info, vault.id).unwrap();

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
fn when_not_a_dca_vault_returns_an_error() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract_with_community_pool_fee_collector(
        deps.as_mut(),
        env.clone(),
        info.clone(),
    );

    set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let response = disburse_escrow_handler(deps.as_mut(), env, info, vault.id).unwrap_err();

    assert_eq!(response.to_string(), "Error: Vault is not a DCA+ vault");
}
