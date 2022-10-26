use crate::constants::{ONE, ONE_THOUSAND, TEN};
use crate::msg::{ExecuteMsg, QueryMsg, VaultResponse};
use crate::tests::helpers::{assert_address_balances, assert_events_published};
use crate::tests::mocks::{
    fin_contract_partially_filled_order, fin_contract_unfilled_limit_order, MockApp, ADMIN,
    DENOM_UKUJI, DENOM_UTEST, USER,
};
use base::events::event::{EventBuilder, EventData};
use base::vaults::vault::VaultStatus;
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::Executor;

#[test]
fn when_vault_has_unfilled_fin_limit_order_trigger_should_update_address_balances() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_unfilled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(user_balance.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance - vault_deposit),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (
                &mock.dca_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + vault_deposit - swap_amount,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + swap_amount,
            ),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
    );

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
            },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
    );
}

#[test]
fn when_vault_has_unfilled_fin_limit_order_trigger_should_publish_events() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_unfilled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(user_balance.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
            },
            &[],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(
            vault_id,
            mock.app.block_info(),
            EventData::DCAVaultCancelled,
        )
        .build(3)],
    );
}

#[test]
fn when_vault_has_unfilled_fin_limit_order_trigger_should_cancel_vault() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_unfilled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(user_balance.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
            },
            &[],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVault {
                address: user_address.clone(),
                vault_id,
            },
        )
        .unwrap();

    assert_eq!(vault_response.vault.status, VaultStatus::Cancelled);
}

#[test]
fn when_vault_has_unfilled_fin_limit_order_trigger_should_empty_vault_balance() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_unfilled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(user_balance.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
            },
            &[],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVault {
                address: user_address.clone(),
                vault_id,
            },
        )
        .unwrap();

    assert_eq!(vault_response.vault.balance.amount, Uint128::zero());
}

#[test]
fn when_vault_has_partially_filled_price_trigger_should_update_address_balances() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_partially_filled_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_partially_filled_fin_limit_price_trigger(
            &user_address,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance - vault_deposit),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (
                &mock.dca_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + vault_deposit - swap_amount,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + swap_amount / Uint128::new(2),
            ),
            (
                &mock.fin_contract_address,
                DENOM_UTEST,
                ONE_THOUSAND + swap_amount / Uint128::new(2),
            ),
        ],
    );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id,
            },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (
                &user_address,
                DENOM_UKUJI,
                vault_deposit - swap_amount + swap_amount / Uint128::new(2),
            ),
            (&user_address, DENOM_UTEST, swap_amount / Uint128::new(2)),
            (&mock.dca_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
    );
}

#[test]
fn when_vault_has_partially_filled_price_trigger_should_publish_events() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_partially_filled_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_partially_filled_fin_limit_price_trigger(
            &user_address,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id,
            },
            &[],
        )
        .unwrap();

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(
            vault_id,
            mock.app.block_info(),
            EventData::DCAVaultCancelled,
        )
        .build(3)],
    );
}

#[test]
fn when_vault_has_partially_filled_price_trigger_should_empty_vault_balance() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_partially_filled_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_partially_filled_fin_limit_price_trigger(
            &user_address,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id,
            },
            &[],
        )
        .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVault {
                address: user_address.clone(),
                vault_id,
            },
        )
        .unwrap();

    assert_eq!(vault_response.vault.balance.amount, Uint128::zero());
}

#[test]
fn when_vault_has_partially_filled_price_trigger_should_cancel_vault() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_partially_filled_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_partially_filled_fin_limit_price_trigger(
            &user_address,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id,
            },
            &[],
        )
        .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVault {
                address: user_address.clone(),
                vault_id,
            },
        )
        .unwrap();

    assert_eq!(vault_response.vault.status, VaultStatus::Cancelled);
}

#[test]
fn when_vault_has_time_trigger_should_update_address_balances() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, TEN, DENOM_UKUJI)
        .with_vault_with_time_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
            None,
        );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance - vault_deposit),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (
                &mock.dca_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + vault_deposit,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
    );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id,
            },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
    );
}

#[test]
fn when_vault_has_time_trigger_should_publish_events() {
    let user_address = Addr::unchecked(USER);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, TEN, DENOM_UKUJI)
        .with_vault_with_time_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
            None,
        );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id,
            },
            &[],
        )
        .unwrap();

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(
            vault_id,
            mock.app.block_info(),
            EventData::DCAVaultCancelled,
        )
        .build(3)],
    );
}

#[test]
fn when_vault_has_time_trigger_should_cancel_vault() {
    let user_address = Addr::unchecked(USER);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, TEN, DENOM_UKUJI)
        .with_vault_with_time_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
            None,
        );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id,
            },
            &[],
        )
        .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVault {
                address: user_address.clone(),
                vault_id,
            },
        )
        .unwrap();

    assert_eq!(vault_response.vault.status, VaultStatus::Cancelled);
}

#[test]
fn when_vault_has_time_trigger_should_empty_vault_balance() {
    let user_address = Addr::unchecked(USER);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, TEN, DENOM_UKUJI)
        .with_vault_with_time_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
            None,
        );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id,
            },
            &[],
        )
        .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVault {
                address: user_address.clone(),
                vault_id,
            },
        )
        .unwrap();

    assert_eq!(vault_response.vault.balance.amount, Uint128::zero());
}

#[test]
fn on_already_cancelled_vault_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_unfilled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(user_balance.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
            },
            &[],
        )
        .unwrap();

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                address: user_address.clone(),
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
            },
            &[],
        )
        .unwrap_err();

    assert!(response
        .root_cause()
        .to_string()
        .contains("Error: vault is already cancelled"));
}
