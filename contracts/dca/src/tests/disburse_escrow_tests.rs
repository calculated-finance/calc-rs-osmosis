use super::{
    helpers::{
        instantiate_contract_with_community_pool_fee_collector,
        setup_active_dca_plus_vault_with_funds,
    },
    mocks::ADMIN,
};
use crate::{
    constants::{ONE_DECIMAL, TEN},
    contract::AFTER_BANK_SWAP_REPLY_ID,
    handlers::disburse_escrow::disburse_escrow_handler,
    state::vaults::update_vault,
    tests::{helpers::set_fin_price, mocks::FEE_COLLECTOR},
    types::dca_plus_config::DcaPlusConfig,
};
use base::vaults::vault::VaultStatus;
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
    set_fin_price(&mut deps, &ONE_DECIMAL);

    let mut vault = setup_active_dca_plus_vault_with_funds(deps.as_mut(), env.clone());

    vault.status = VaultStatus::Inactive;
    vault.dca_plus_config = Some(DcaPlusConfig {
        escrow_level: Decimal::percent(5),
        model_id: 50,
        total_deposit: TEN,
        standard_dca_swapped_amount: vault.swap_amount,
        standard_dca_received_amount: vault.swap_amount,
        escrowed_balance: vault.swap_amount * Decimal::percent(5),
    });

    update_vault(deps.as_mut().storage, &vault).unwrap();

    let response = disburse_escrow_handler(deps.as_mut(), env, info, vault.id).unwrap();

    assert!(response.messages.contains(&SubMsg::reply_on_success(
        CosmosMsg::Bank(BankMsg::Send {
            to_address: vault.destinations[0].address.to_string(),
            amount: vec![Coin::new(
                vault
                    .dca_plus_config
                    .clone()
                    .unwrap()
                    .escrowed_balance
                    .into(),
                vault.get_receive_denom()
            )]
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
    set_fin_price(&mut deps, &ONE_DECIMAL);

    let mut vault = setup_active_dca_plus_vault_with_funds(deps.as_mut(), env.clone());

    vault.status = VaultStatus::Inactive;
    vault.dca_plus_config = Some(DcaPlusConfig {
        escrow_level: Decimal::percent(5),
        model_id: 50,
        total_deposit: TEN,
        standard_dca_swapped_amount: vault.swap_amount,
        standard_dca_received_amount: vault.swap_amount / Uint128::new(10),
        escrowed_balance: vault.swap_amount / Uint128::new(10) * Decimal::percent(5),
    });

    update_vault(deps.as_mut().storage, &vault).unwrap();

    let response = disburse_escrow_handler(deps.as_mut(), env, info, vault.id).unwrap();

    assert!(response
        .messages
        .contains(&SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: FEE_COLLECTOR.to_string(),
            amount: vec![Coin::new(
                vault
                    .dca_plus_config
                    .clone()
                    .unwrap()
                    .escrowed_balance
                    .into(),
                vault.get_receive_denom()
            )]
        }))));
}
