use super::mocks::MockApp;
use crate::msg::{QueryMsg, VaultResponse};
use cosmwasm_std::{Addr, Event, Uint128};

pub fn assert_address_balances(app: &MockApp, address_balances: &[(&Addr, &str, Uint128)]) {
    address_balances
        .iter()
        .for_each(|(address, denom, expected_balance)| {
            assert_eq!(
                app.get_balance(address, denom),
                expected_balance,
                "Balance mismatch for {} at {}",
                address,
                denom
            );
        })
}

pub fn assert_response_events(actual_events: &[Event], expected_events: &[Event]) {
    expected_events.iter().for_each(|expected_event| {
        assert!(
            actual_events.contains(expected_event),
            "Expected actual_events: \n\n{:?}\n\nto contain event:\n\n{:?}\n\n but it wasn't found",
            actual_events,
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
        .query_wasm_smart(
            contract_address,
            &QueryMsg::GetVault {
                address: owner.to_string(),
                vault_id,
            },
        )
        .unwrap();

    let vault = &vault_response.vault;

    assert_eq!(
        vault.balances[0].amount, balance,
        "Vault balance mismatch for vault_id: {}, owner: {}",
        vault_id, owner
    );
}
