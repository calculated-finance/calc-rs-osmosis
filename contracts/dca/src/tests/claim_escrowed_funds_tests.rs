use super::{
    helpers::assert_address_balances,
    mocks::{fin_contract_unfilled_limit_order, MockApp, ADMIN, DENOM_UKUJI, DENOM_UTEST, USER},
};
use crate::{
    constants::{ONE, TEN},
    msg::{ExecuteMsg, QueryMsg, VaultResponse},
};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_multi_test::Executor;
use fin_helpers::position_type::PositionType;
use std::str::FromStr;

#[test]
fn should_disburse_escrowed_amount_to_the_vault_destinations() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;

    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_time_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "time",
            None,
            Some(true),
        );

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::UpdateSwapAdjustments {
                position_type: PositionType::Enter,
                adjustments: vec![
                    (30, Decimal::from_str("1.3").unwrap()),
                    (35, Decimal::from_str("1.3").unwrap()),
                    (40, Decimal::from_str("1.3").unwrap()),
                    (45, Decimal::from_str("1.3").unwrap()),
                    (50, Decimal::from_str("1.3").unwrap()),
                    (55, Decimal::from_str("1.3").unwrap()),
                    (60, Decimal::from_str("1.3").unwrap()),
                    (70, Decimal::from_str("1.3").unwrap()),
                    (80, Decimal::from_str("1.3").unwrap()),
                    (90, Decimal::from_str("1.3").unwrap()),
                ],
            },
            &[],
        )
        .unwrap();

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

    let vault = mock
        .app
        .wrap()
        .query_wasm_smart::<VaultResponse>(
            mock.dca_contract_address.clone(),
            &QueryMsg::GetVault {
                vault_id: Uint128::one(),
            },
        )
        .unwrap()
        .vault;

    let dca_plus_config = vault.dca_plus_config.unwrap();

    assert_address_balances(
        &mock,
        &[(
            &user_address,
            DENOM_UTEST,
            vault.received_amount.amount - dca_plus_config.escrowed_balance,
        )],
    );

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::ClaimEscrowedFunds {
                vault_id: Uint128::new(1),
            },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[(&user_address, DENOM_UTEST, vault.received_amount.amount)],
    );
}
