use crate::{
    constants::TWO_MICRONS,
    contract::{
        AFTER_FIN_LIMIT_ORDER_RETRACTED_FOR_MIGRATE_REPLY_ID,
        AFTER_FIN_LIMIT_ORDER_SUBMITTED_FOR_MIGRATE_REPLY_ID,
    },
    error::ContractError,
    state::{
        cache::{Cache, CACHE},
        triggers::{get_trigger, save_trigger},
        vaults::{get_vault, update_vault},
    },
    types::vault::Vault,
};
use base::{
    helpers::message_helpers::get_attribute_in_event,
    triggers::trigger::{Trigger, TriggerConfiguration},
};
use cosmwasm_std::{Coin, DepsMut, Reply, Response, StdError, Uint128};
use fin_helpers::{
    limit_orders::{create_retract_order_sub_msg, create_submit_order_sub_msg},
    queries::query_order_details,
};

pub fn migrate_price_trigger(deps: DepsMut, vault_id: Uint128) -> Result<Response, ContractError> {
    let vault = get_vault(deps.storage, vault_id)?;

    CACHE.save(
        deps.storage,
        &Cache {
            vault_id: vault_id.to_owned(),
            owner: vault.owner.clone(),
        },
    )?;

    update_vault(
        deps.storage,
        vault_id,
        |existing_vault: Option<Vault>| -> Result<Vault, StdError> {
            match existing_vault {
                Some(mut existing_vault) => {
                    existing_vault.balance.amount -= TWO_MICRONS;
                    Ok(existing_vault)
                }
                None => Err(StdError::GenericErr {
                    msg: format!("Vault {} not found", vault_id).to_string(),
                }),
            }
        },
    )?;

    match vault.trigger.as_ref().expect("fin limit order trigger") {
        TriggerConfiguration::FinLimitOrder { order_idx, .. } => match order_idx {
            Some(order_idx) => {
                let limit_order_details = query_order_details(
                    deps.querier,
                    vault.pair.address.clone(),
                    order_idx.to_owned(),
                )?;

                if limit_order_details.filled_amount > Uint128::zero() {
                    return Err(ContractError::CustomError {
                        val: "fin limit order is already partially or completely filled"
                            .to_string(),
                    });
                }

                Ok(Response::new()
                    .add_submessage(create_retract_order_sub_msg(
                        vault.pair.address.clone(),
                        order_idx.to_owned(),
                        AFTER_FIN_LIMIT_ORDER_RETRACTED_FOR_MIGRATE_REPLY_ID,
                    ))
                    .add_submessage(create_submit_order_sub_msg(
                        vault.pair.address.clone(),
                        limit_order_details.quote_price,
                        Coin::new(TWO_MICRONS.into(), vault.get_swap_denom()),
                        AFTER_FIN_LIMIT_ORDER_SUBMITTED_FOR_MIGRATE_REPLY_ID,
                    )))
            }
            _ => Err(ContractError::CustomError {
                val: "fin limit order does not have an order idx".to_string(),
            }),
        },
        _ => Err(ContractError::CustomError {
            val: "vault does not have a fin limit order".to_string(),
        }),
    }
}

pub fn after_fin_limit_order_submitted_for_migrate_trigger(
    deps: DepsMut,
    reply: Reply,
) -> Result<Response, ContractError> {
    let fin_submit_order_response = reply.result.into_result().unwrap();

    let order_idx = get_attribute_in_event(&fin_submit_order_response.events, "wasm", "order_idx")?
        .parse::<Uint128>()
        .expect("returned order_idx should be a valid Uint128");

    let cache = CACHE.load(deps.storage)?;

    let trigger = get_trigger(deps.storage, cache.vault_id)?
        .expect(format!("fin limit order trigger for vault {}", cache.vault_id).as_str());

    match trigger.configuration {
        TriggerConfiguration::FinLimitOrder { target_price, .. } => {
            save_trigger(
                deps.storage,
                Trigger {
                    vault_id: cache.vault_id,
                    configuration: TriggerConfiguration::FinLimitOrder {
                        order_idx: Some(order_idx),
                        target_price,
                    },
                },
            )?;
            Ok(Response::new())
        }
        _ => Err(ContractError::CustomError {
            val: "vault does not have a fin limit order trigger".to_string(),
        }),
    }
}

#[cfg(test)]
mod migrate_price_trigger_tests {
    use super::*;
    use crate::{
        constants::{ONE, ONE_THOUSAND, TEN},
        msg::{ExecuteMsg, QueryMsg, VaultResponse},
        tests::{
            helpers::assert_address_balances,
            mocks::{
                fin_contract_unfilled_limit_order, MockApp, ADMIN, DENOM_UKUJI, DENOM_UTEST, USER,
            },
        },
    };
    use cosmwasm_std::Addr;
    use cw_multi_test::Executor;

    #[test]
    fn fails_when_called_by_non_admin_address() {
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

        let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

        let err = mock
            .app
            .execute_contract(
                Addr::unchecked("bad-actor"),
                mock.dca_contract_address.clone(),
                &ExecuteMsg::MigratePriceTrigger { vault_id },
                &[],
            )
            .unwrap_err();

        assert_eq!(err.root_cause().to_string(), "Unauthorized")
    }

    #[test]
    fn updates_the_account_balances() {
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

        let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

        assert_address_balances(
            &mock,
            &[
                (&user_address, DENOM_UKUJI, user_balance - vault_deposit),
                (&user_address, DENOM_UTEST, Uint128::zero()),
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
                &ExecuteMsg::MigratePriceTrigger { vault_id },
                &[],
            )
            .unwrap();

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
    }

    #[test]
    fn updates_the_vault_balance() {
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

        let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

        let vault_before_response: VaultResponse = mock
            .app
            .wrap()
            .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
            .unwrap();

        mock.app
            .execute_contract(
                Addr::unchecked(ADMIN),
                mock.dca_contract_address.clone(),
                &ExecuteMsg::MigratePriceTrigger { vault_id },
                &[],
            )
            .unwrap();

        let vault_after_response: VaultResponse = mock
            .app
            .wrap()
            .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
            .unwrap();

        assert_eq!(
            vault_before_response.vault.balance.amount - TWO_MICRONS,
            vault_after_response.vault.balance.amount
        );
    }

    #[test]
    fn updates_the_price_trigger_order_idx() {
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

        let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

        let vault_before_response: VaultResponse = mock
            .app
            .wrap()
            .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
            .unwrap();

        mock.app
            .execute_contract(
                Addr::unchecked(ADMIN),
                mock.dca_contract_address.clone(),
                &ExecuteMsg::MigratePriceTrigger { vault_id },
                &[],
            )
            .unwrap();

        let vault_after_response: VaultResponse = mock
            .app
            .wrap()
            .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
            .unwrap();

        assert_ne!(
            vault_before_response.vault.trigger,
            vault_after_response.vault.trigger
        );
    }
}
