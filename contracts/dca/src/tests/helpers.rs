use super::mocks::MockApp;
use crate::{
    msg::{EventsResponse, QueryMsg, VaultResponse},
    vault::Vault,
};
use base::events::event::Event;
use cosmwasm_std::{Addr, Uint128};

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

pub fn assert_vault_eq(mock: &MockApp, vault_id: Uint128, expected_vault: Vault) {
    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    assert_eq!(vault_response.vault, expected_vault);
}

pub fn assert_events_published(mock: &MockApp, resource_id: Uint128, expected_events: &[Event]) {
    let events_response: EventsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetEventsByResourceId { resource_id },
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
    owner: &Addr,
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
        vault_id, owner
    );
}
