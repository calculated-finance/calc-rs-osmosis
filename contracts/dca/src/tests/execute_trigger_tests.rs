use super::constants::ONE_HUNDRED;
use super::mocks::fin_contract_fail_slippage_tolerance;
use crate::msg::{ExecuteMsg, QueryMsg, TriggersResponse, VaultResponse};
use crate::tests::constants::{ONE, ONE_THOUSAND, TEN};
use crate::tests::helpers::{
    assert_address_balances, assert_events_published, assert_vault_balance,
};
use crate::tests::mocks::{
    fin_contract_filled_limit_order, fin_contract_partially_filled_order,
    fin_contract_unfilled_limit_order, MockApp, ADMIN, DENOM_UKUJI, DENOM_UTEST, USER,
};
use base::events::event::{EventBuilder, EventData};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_multi_test::Executor;

#[test]
fn fin_limit_order_trigger_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let swap_amount_after_fee = swap_amount * (Decimal::percent(100) - Decimal::percent(2));
    let mut mock = MockApp::new(fin_contract_filled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            Coin::new(vault_deposit.u128(), DENOM_UKUJI),
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
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UTEST,
                ONE_THOUSAND + swap_amount,
            ),
        ],
    );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &&QueryMsg::GetVaultById { vault_id },
        )
        .unwrap();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::ExecuteTrigger {
                trigger_id: vault_response.vault.trigger_id.unwrap(),
            },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(0)),
            (&user_address, DENOM_UTEST, swap_amount_after_fee),
            (
                &mock.dca_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + vault_deposit - swap_amount,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
    );

    assert_events_published(
        &mock,
        vault_id,
        &[
            EventBuilder::new(
                vault_id,
                mock.app.block_info(),
                EventData::DCAVaultExecutionTriggered {
                    trigger_id: vault_response.vault.trigger_id.unwrap(),
                },
            )
            .build(2),
            EventBuilder::new(
                vault_id,
                mock.app.block_info(),
                EventData::DCAVaultExecutionCompleted {
                    sent: Coin::new(swap_amount.u128(), DENOM_UKUJI),
                    received: Coin::new(swap_amount.u128(), DENOM_UTEST),
                    fee: Coin::new((swap_amount * mock.fee_rate).u128(), DENOM_UTEST),
                },
            )
            .build(3),
        ],
    );

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        &user_address,
        Uint128::new(1),
        vault_deposit - swap_amount,
    );

    let get_all_time_triggers_response: TriggersResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetTimeTriggers {},
        )
        .unwrap();

    assert_eq!(get_all_time_triggers_response.triggers.len(), 1);
}

#[test]
fn when_order_partially_filled_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_partially_filled_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_partially_filled_fin_limit_price_trigger(
            &user_address,
            Coin::new(vault_deposit.u128(), DENOM_UKUJI.to_string()),
            swap_amount,
            "fin",
        );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(0)),
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

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &&QueryMsg::GetVaultById {
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
            },
        )
        .unwrap();

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::ExecuteTrigger {
                trigger_id: vault_response.vault.trigger_id.unwrap(),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: fin limit order has not been completely filled"
    );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(0)),
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

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        &user_address,
        Uint128::new(1),
        vault_deposit,
    );
}

#[test]
fn when_executions_result_in_empty_vault_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let user_funds = ONE_HUNDRED;
    let vault_deposit = ONE * Uint128::new(3) / Uint128::new(2);
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_filled_limit_order())
        .with_funds_for(&user_address, user_funds, DENOM_UKUJI)
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            Coin::new(vault_deposit.u128(), DENOM_UKUJI.to_string()),
            swap_amount,
            "fin",
        );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_funds - vault_deposit),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (
                &mock.dca_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + vault_deposit - swap_amount,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UTEST,
                ONE_THOUSAND + swap_amount,
            ),
        ],
    );

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &&QueryMsg::GetVaultById {
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
            },
        )
        .unwrap();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::ExecuteTrigger {
                trigger_id: vault_response.vault.trigger_id.unwrap(),
            },
            &[],
        )
        .unwrap();

    let vault_with_time_trigger_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &&QueryMsg::GetVaultById {
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
            },
        )
        .unwrap();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::ExecuteTrigger {
                trigger_id: vault_with_time_trigger_response.vault.trigger_id.unwrap(),
            },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, ONE_HUNDRED - vault_deposit),
            (
                &user_address,
                DENOM_UTEST,
                vault_deposit * (Decimal::percent(100) - mock.fee_rate),
            ),
            (&mock.dca_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + swap_amount / Uint128::new(2),
            ),
            (
                &mock.fin_contract_address,
                DENOM_UTEST,
                ONE_THOUSAND - swap_amount / Uint128::new(2),
            ),
        ],
    );

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        &user_address,
        vault_with_time_trigger_response.vault.id,
        Uint128::new(0),
    );
}

#[test]
fn after_target_time_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let swap_amount_after_fee = swap_amount * (Decimal::percent(100) - Decimal::percent(2));
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_time_trigger(
            &user_address,
            Coin::new(vault_deposit.u128(), DENOM_UKUJI),
            swap_amount,
            "time",
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

    let vault_id = mock.vault_ids.get("time").unwrap().to_owned();

    mock.elapse_time(10);

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::ExecuteTrigger {
                trigger_id: Uint128::new(1),
            },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(0)),
            (&user_address, DENOM_UTEST, swap_amount_after_fee),
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
            (
                &mock.fin_contract_address,
                DENOM_UTEST,
                ONE_THOUSAND - swap_amount,
            ),
        ],
    );

    assert_events_published(
        &mock,
        vault_id,
        &[
            EventBuilder::new(
                vault_id,
                mock.app.block_info(),
                EventData::DCAVaultExecutionTriggered {
                    trigger_id: Uint128::new(1),
                },
            )
            .build(2),
            EventBuilder::new(
                vault_id,
                mock.app.block_info(),
                EventData::DCAVaultExecutionCompleted {
                    sent: Coin::new(swap_amount.u128(), DENOM_UKUJI),
                    received: Coin::new(swap_amount.u128(), DENOM_UTEST),
                    fee: Coin::new((swap_amount * mock.fee_rate).u128(), DENOM_UTEST),
                },
            )
            .build(3),
        ],
    );

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        &user_address,
        Uint128::new(1),
        TEN - ONE,
    );
}

#[test]
fn before_target_time_limit_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, TEN, DENOM_UKUJI)
        .with_vault_with_time_trigger(
            &user_address,
            Coin::new(vault_deposit.u128(), DENOM_UKUJI),
            swap_amount,
            "time",
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

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::ExecuteTrigger {
                trigger_id: Uint128::new(1),
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: trigger execution time has not yet elapsed"
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

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        &user_address,
        Uint128::new(1),
        TEN,
    );
}

#[test]
fn when_slippage_exceeds_limit_should_skip_execution() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_fail_slippage_tolerance())
        .with_funds_for(&user_address, TEN, DENOM_UKUJI)
        .with_vault_with_time_trigger(
            &user_address,
            Coin::new(vault_deposit.u128(), DENOM_UKUJI),
            swap_amount,
            "time",
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

    mock.elapse_time(10);

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::ExecuteTrigger {
                trigger_id: Uint128::new(1),
            },
            &[],
        )
        .unwrap();

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

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        &user_address,
        Uint128::new(1),
        vault_deposit,
    );
}
