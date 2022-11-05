use std::str::FromStr;

use super::mocks::{MockApp, ADMIN};
use crate::{
    contract::instantiate,
    msg::{EventsResponse, InstantiateMsg, QueryMsg, VaultResponse},
};
use base::events::event::Event;
use cosmwasm_std::{Addr, Decimal, DepsMut, Env, MessageInfo, Uint128};

pub fn instantiate_contract(deps: DepsMut, env: Env, info: MessageInfo) {
    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        fee_collector: Addr::unchecked(ADMIN),
        fee_percent: Decimal::from_str("0.015").unwrap(),
        staking_router_address: Addr::unchecked(ADMIN),
        page_limit: 1000,
    };

    let _instantiate_result =
        instantiate(deps, env.clone(), info.clone(), instantiate_message).unwrap();
}

pub fn assert_address_balances(mock: &MockApp, address_balances: &[(&Addr, &str, Uint128)]) {
    address_balances
        .iter()
        .for_each(|(address, denom, expected_balance)| {
            assert_eq!(
                mock.get_balance(address, denom),
                expected_balance,
                "Balance mismatch for {} at {}",
                address,
                denom
            );
        })
}

pub fn assert_events_published(mock: &MockApp, resource_id: Uint128, expected_events: &[Event]) {
    let events_response: EventsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetEventsByResourceId {
                resource_id,
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    expected_events.iter().for_each(|expected_event| {
        assert!(
            events_response.events.contains(expected_event),
            "Expected actual_events: \n\n{:?}\n\nto contain event:\n\n{:?}\n\n but it wasn't found",
            events_response.events,
            expected_event
        );
    });
}

pub fn assert_vault_balance(
    mock: &MockApp,
    contract_address: &Addr,
    address: Addr,
    vault_id: Uint128,
    balance: Uint128,
) {
    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    let vault = &vault_response.vault;

    assert_eq!(
        vault.balance.amount, balance,
        "Vault balance mismatch for vault_id: {}, owner: {}",
        vault_id, address
    );
}
