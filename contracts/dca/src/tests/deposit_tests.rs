use crate::constants::{ONE, TEN, ONE_THOUSAND, ONE_HUNDRED};
use crate::msg::ExecuteMsg;
use crate::tests::mocks::{
    fin_contract_unfilled_limit_order, MockApp, ADMIN,
    DENOM_UKUJI, USER,
};
use base::events::event::EventBuilder;
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::Executor;

use super::helpers::{assert_address_balances, assert_events_published, assert_vault_balance};
use super::mocks::DENOM_UTEST;

#[test]
fn when_vault_is_active_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let user_balance = ONE_HUNDRED;
    let swap_amount = ONE;
    let vault_deposit = TEN;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_unfilled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    mock
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::Deposit {
                address: user_address.clone(),
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
            },
            &[Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap();

        assert_address_balances(
            &mock,
            &[
                (&user_address, DENOM_UKUJI, user_balance - vault_deposit),
                (&user_address, DENOM_UTEST, Uint128::new(0)),
                (&mock.dca_contract_address, DENOM_UKUJI, ONE_THOUSAND - swap_amount + vault_deposit + vault_deposit),
                (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
                (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND + ONE),
                (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
            ],
        );

}

#[test]
fn when_vault_is_active_should_update_vault_balance() {
    let user_address = Addr::unchecked(USER);
    let user_balance = ONE_HUNDRED;
    let swap_amount = ONE;
    let vault_deposit = TEN;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_unfilled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::Deposit {
                address: user_address.clone(),
                vault_id,
            },
            &[Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap();

        assert_vault_balance(
            &mock,
            &mock.dca_contract_address,
            user_address,
            Uint128::new(1),
            vault_deposit + vault_deposit,
        );

}

#[test]
fn when_vault_is_active_should_create_event() {
    let user_address = Addr::unchecked(USER);
    let user_balance = ONE_HUNDRED;
    let swap_amount = ONE;
    let vault_deposit = TEN;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_unfilled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::Deposit {
                address: user_address.clone(),
                vault_id,
            },
            &[Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap();

        assert_events_published(
            &mock,
            vault_id,
            &[
                EventBuilder::new(
                    vault_id,
                    mock.app.block_info(),
                    base::events::event::EventData::DCAVaultFundsDeposited { amount: Coin::new(TEN.into(), DENOM_UKUJI) }
                )
                .build(2),
            ],
        );

}

#[test]
fn when_vault_is_cancelled_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let swap_amount = ONE;
    let vault_deposit = TEN;
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
            &ExecuteMsg::Deposit {
                address: user_address.clone(),
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
            },
            &[Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap_err();

    assert!(response
        .root_cause()
        .to_string()
        .contains("Error: vault is already cancelled"));
}
