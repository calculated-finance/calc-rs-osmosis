use super::helpers::{instantiate_contract, setup_active_dca_plus_vault_with_funds};
use crate::constants::{ONE, ONE_THOUSAND, TEN, TWO_MICRONS};
use crate::handlers::cancel_vault::cancel_vault;
use crate::msg::{ExecuteMsg, QueryMsg, VaultResponse};
use crate::state::disburse_escrow_tasks::get_disburse_escrow_tasks;
use crate::tests::helpers::{assert_address_balances, assert_events_published};
use crate::tests::mocks::{
    fin_contract_partially_filled_order, fin_contract_unfilled_limit_order, MockApp, ADMIN,
    DENOM_UKUJI, DENOM_UTEST, USER,
};
use base::events::event::{EventBuilder, EventData};
use base::vaults::vault::VaultStatus;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
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
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance - vault_deposit),
            (&user_address, DENOM_UTEST, Uint128::zero()),
            (
                &mock.dca_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + vault_deposit - TWO_MICRONS,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + TWO_MICRONS,
            ),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
    );

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
            },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance - TWO_MICRONS),
            (&user_address, DENOM_UTEST, Uint128::zero()),
            (
                &mock.dca_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + TWO_MICRONS,
            ),
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
            EventData::DcaVaultCancelled {},
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
            &QueryMsg::GetVault { vault_id },
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
            &QueryMsg::GetVault { vault_id },
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
                ONE_THOUSAND + vault_deposit - TWO_MICRONS,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + Uint128::one(),
            ),
            (
                &mock.fin_contract_address,
                DENOM_UTEST,
                ONE_THOUSAND + Uint128::one(),
            ),
        ],
    );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault { vault_id },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, vault_deposit - TWO_MICRONS),
            (&user_address, DENOM_UTEST, Uint128::zero()),
            (
                &mock.dca_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + Uint128::one(),
            ),
            (
                &mock.dca_contract_address,
                DENOM_UTEST,
                ONE_THOUSAND + Uint128::one(),
            ),
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
            &ExecuteMsg::CancelVault { vault_id },
            &[],
        )
        .unwrap();

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(
            vault_id,
            mock.app.block_info(),
            EventData::DcaVaultCancelled {},
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
            &ExecuteMsg::CancelVault { vault_id },
            &[],
        )
        .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVault { vault_id },
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
            &ExecuteMsg::CancelVault { vault_id },
            &[],
        )
        .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVault { vault_id },
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
            &ExecuteMsg::CancelVault { vault_id },
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
            None,
        );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault { vault_id },
            &[],
        )
        .unwrap();

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(
            vault_id,
            mock.app.block_info(),
            EventData::DcaVaultCancelled {},
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
            None,
        );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault { vault_id },
            &[],
        )
        .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVault { vault_id },
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
            None,
        );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault { vault_id },
            &[],
        )
        .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVault { vault_id },
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

#[test]
fn for_vault_with_different_onwer_should_fail() {
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

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked("not-the-owner"),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(response.root_cause().to_string(), "Unauthorized");
}

#[test]
fn when_vault_has_no_trigger_should_cancel_vault() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, TEN, DENOM_UKUJI)
        .with_inactive_vault(&user_address, None, "fin");

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault { vault_id },
            &[],
        )
        .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVault { vault_id },
        )
        .unwrap();

    assert_eq!(vault_response.vault.status, VaultStatus::Cancelled);
}

#[test]
fn when_vault_has_no_trigger_should_empty_vault_balance() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, TEN, DENOM_UKUJI)
        .with_inactive_vault(&user_address, None, "fin");

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault { vault_id },
            &[],
        )
        .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVault { vault_id },
        )
        .unwrap();

    assert_eq!(vault_response.vault.balance.amount, Uint128::zero());
}

#[test]
fn when_vault_has_no_trigger_should_publish_events() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, TEN, DENOM_UKUJI)
        .with_inactive_vault(&user_address, None, "fin");

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault { vault_id },
            &[],
        )
        .unwrap();

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(
            vault_id,
            mock.app.block_info(),
            EventData::DcaVaultCancelled {},
        )
        .build(3)],
    );
}

#[test]
fn with_dca_plus_should_save_disburse_escrow_task() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_active_dca_plus_vault_with_funds(deps.as_mut(), env.clone());

    cancel_vault(deps.as_mut(), env.clone(), info, vault.id).unwrap();

    let disburse_escrow_tasks_before = get_disburse_escrow_tasks(
        deps.as_ref().storage,
        vault
            .get_expected_execution_completed_date(env.block.time)
            .minus_seconds(10),
        Some(100),
    )
    .unwrap();

    assert!(disburse_escrow_tasks_before.is_empty());

    let disburse_escrow_tasks_after = get_disburse_escrow_tasks(
        deps.as_ref().storage,
        vault
            .get_expected_execution_completed_date(env.block.time)
            .plus_seconds(10),
        Some(100),
    )
    .unwrap();

    assert!(disburse_escrow_tasks_after.contains(&vault.id));
}
