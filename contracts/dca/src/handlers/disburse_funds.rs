use crate::error::ContractError;
use crate::helpers::coin::add_to;
use crate::helpers::disbursement::get_disbursement_messages;
use crate::helpers::fees::{get_delegation_fee_rate, get_fee_messages, get_swap_fee_rate};
use crate::helpers::math::checked_mul;
use crate::helpers::vault::get_swap_amount;
use crate::msg::ExecuteMsg;
use crate::state::cache::{SWAP_CACHE, VAULT_CACHE};
use crate::state::events::create_event;
use crate::state::triggers::delete_trigger;
use crate::state::vaults::{get_vault, update_vault};
use crate::types::event::{EventBuilder, EventData, ExecutionSkippedReason};
use crate::types::swap_adjustment_strategy::SwapAdjustmentStrategy;
use crate::types::vault::VaultStatus;
use cosmwasm_std::{to_binary, Decimal, SubMsg, SubMsgResult, WasmMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{Attribute, Coin, DepsMut, Env, Reply, Response};

pub fn disburse_funds_handler(
    deps: DepsMut,
    env: &Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = VAULT_CACHE.load(deps.storage)?;
    let mut vault = get_vault(deps.storage, cache.vault_id)?;

    let mut attributes: Vec<Attribute> = Vec::new();
    let mut sub_msgs: Vec<SubMsg> = Vec::new();

    match reply.result {
        SubMsgResult::Ok(_) => {
            let swap_cache = SWAP_CACHE.load(deps.storage)?;

            let swap_denom_balance = &deps
                .querier
                .query_balance(&env.contract.address, vault.get_swap_denom())?;

            let receive_denom_balance = &deps
                .querier
                .query_balance(&env.contract.address, vault.target_denom.clone())?;

            let coin_sent = Coin::new(
                (swap_cache.swap_denom_balance.amount - swap_denom_balance.amount).into(),
                swap_denom_balance.denom.clone(),
            );

            let coin_received = Coin::new(
                (receive_denom_balance.amount - swap_cache.receive_denom_balance.amount).into(),
                receive_denom_balance.denom.clone(),
            );

            let swap_fee_rate = match vault.swap_adjustment_strategy {
                Some(_) => Decimal::zero(),
                None => get_swap_fee_rate(deps.storage, &vault)?,
            };

            let automation_fee_rate = match vault.swap_adjustment_strategy {
                Some(_) => Decimal::zero(),
                None => get_delegation_fee_rate(deps.storage, &vault)?,
            };

            let swap_fee = checked_mul(coin_received.amount, swap_fee_rate)?;
            let total_after_swap_fee = coin_received.amount - swap_fee;
            let automation_fee = checked_mul(total_after_swap_fee, automation_fee_rate)?;
            let total_fee = swap_fee + automation_fee;
            let mut total_after_total_fee = coin_received.amount - total_fee;

            sub_msgs.append(&mut get_fee_messages(
                deps.as_ref(),
                vec![swap_fee, automation_fee],
                coin_received.denom.clone(),
            )?);

            vault.balance.amount -= get_swap_amount(&deps.as_ref(), env, &vault)?.amount;
            vault.swapped_amount = add_to(vault.swapped_amount, coin_sent.amount);
            vault.received_amount = add_to(vault.received_amount, total_after_total_fee);

            if let Some(swap_adjustment_strategy) = vault.swap_adjustment_strategy.clone() {
                let amount_to_escrow =
                    total_after_total_fee * swap_adjustment_strategy.dca_plus_escrow_level();
                total_after_total_fee -= amount_to_escrow;

                vault.swap_adjustment_strategy = vault.swap_adjustment_strategy.map(
                    |SwapAdjustmentStrategy::DcaPlus {
                         escrow_level,
                         model_id,
                         total_deposit,
                         standard_dca_swapped_amount,
                         standard_dca_received_amount,
                         ..
                     }| SwapAdjustmentStrategy::DcaPlus {
                        escrowed_balance: add_to(
                            swap_adjustment_strategy.dca_plus_escrowed_balance(),
                            amount_to_escrow,
                        ),
                        escrow_level,
                        model_id,
                        total_deposit,
                        standard_dca_swapped_amount,
                        standard_dca_received_amount,
                    },
                );
            }

            if vault.balance.amount.is_zero() {
                vault.status = VaultStatus::Inactive;
            }

            sub_msgs.append(&mut get_disbursement_messages(
                &vault,
                total_after_total_fee,
            )?);

            create_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block.clone(),
                    EventData::DcaVaultExecutionCompleted {
                        sent: coin_sent,
                        received: coin_received.clone(),
                        fee: Coin::new(total_fee.into(), coin_received.denom),
                    },
                ),
            )?;

            attributes.push(Attribute::new("execution_result", "success"));
        }
        SubMsgResult::Err(_) => {
            create_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block.to_owned(),
                    EventData::DcaVaultExecutionSkipped {
                        reason: ExecutionSkippedReason::SlippageToleranceExceeded,
                    },
                ),
            )?;

            attributes.push(Attribute::new("status", "skipped"));
        }
    }

    update_vault(deps.storage, &vault)?;

    if vault.is_finished_dca_plus_vault() {
        sub_msgs.push(SubMsg::new(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::DisburseEscrow { vault_id: vault.id })?,
            funds: vec![],
        }));

        delete_trigger(deps.storage, vault.id)?;
    }

    Ok(Response::new()
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_attributes(attributes)
        .add_submessages(sub_msgs))
}

#[cfg(test)]
mod disburse_funds_tests {
    use super::*;
    use crate::{
        constants::{AFTER_SWAP_REPLY_ID, ONE, TEN, TWO_MICRONS},
        handlers::get_events_by_resource_id::get_events_by_resource_id_handler,
        helpers::vault::get_swap_amount,
        state::{
            cache::{SwapCache, SWAP_CACHE},
            config::{create_custom_fee, get_config, FeeCollector},
            swap_adjustments::update_swap_adjustments,
            vaults::get_vault,
        },
        tests::{
            helpers::{
                instantiate_contract, instantiate_contract_with_multiple_fee_collectors,
                setup_vault,
            },
            mocks::{ADMIN, DENOM_STAKE, DENOM_UOSMO},
        },
        types::{
            destination::Destination,
            event::{Event, EventBuilder, EventData, ExecutionSkippedReason},
            position_type::PositionType,
            swap_adjustment_strategy::SwapAdjustmentStrategy,
            vault::{Vault, VaultStatus},
        },
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        BankMsg, Coin, Decimal, Reply, SubMsg, SubMsgResponse, SubMsgResult, Uint128,
    };
    use std::{cmp::min, str::FromStr};

    #[test]
    fn with_succcesful_swap_returns_funds_to_destination() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                destinations: vec![Destination::default()],
                ..Vault::default()
            },
        );

        let receive_amount = Uint128::new(10000);

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(receive_amount.into(), vault.target_denom.clone())],
        );

        let response = disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let fee = get_config(&deps.storage).unwrap().swap_fee_percent * receive_amount;

        let automation_fee = get_config(&deps.storage).unwrap().delegation_fee_percent;

        let automation_fees = vault.destinations.iter().filter(|d| d.msg.is_some()).fold(
            Coin::new(0, vault.target_denom.clone()),
            |mut accum, destination| {
                let allocation_amount =
                    checked_mul(receive_amount - fee, destination.allocation).unwrap();
                let allocation_automation_fee =
                    checked_mul(allocation_amount, automation_fee).unwrap();
                accum.amount = accum.amount.checked_add(allocation_automation_fee).unwrap();
                accum
            },
        );

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: vault.destinations.first().unwrap().address.to_string(),
            amount: vec![Coin::new(
                (receive_amount - fee - automation_fees.amount).into(),
                vault.target_denom,
            )],
        },)));
    }

    #[test]
    fn with_succcesful_swap_returns_fee_to_fee_collector() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());
        let receive_amount = Uint128::new(234312312);

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(receive_amount.into(), vault.target_denom.clone())],
        );

        let response = disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let config = get_config(&deps.storage).unwrap();
        let swap_fee = config.swap_fee_percent * receive_amount;
        let total_after_swap_fee = receive_amount - swap_fee;

        let automation_fee = vault.destinations.iter().filter(|d| d.msg.is_some()).fold(
            Uint128::zero(),
            |acc, destination| {
                let allocation_amount =
                    checked_mul(total_after_swap_fee, destination.allocation).unwrap();
                let allocation_automation_fee =
                    checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
                acc.checked_add(allocation_automation_fee).unwrap()
            },
        );

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: config.fee_collectors[0].address.to_string(),
            amount: vec![Coin::new(swap_fee.into(), vault.target_denom.clone())]
        })));

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: config.fee_collectors[0].address.to_string(),
            amount: vec![Coin::new(automation_fee.into(), vault.target_denom.clone())]
        })));
    }

    #[test]
    fn with_succcesful_swap_returns_fee_to_multiple_fee_collectors() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract_with_multiple_fee_collectors(
            deps.as_mut(),
            env.clone(),
            mock_info(ADMIN, &vec![]),
            vec![
                FeeCollector {
                    address: "fee_collector_1".to_string(),
                    allocation: Decimal::percent(20),
                },
                FeeCollector {
                    address: "fee_collector_2".to_string(),
                    allocation: Decimal::percent(80),
                },
            ],
        );

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());
        let receive_amount = Uint128::new(234312312);

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(receive_amount.into(), vault.target_denom.clone())],
        );

        let response = disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let config = get_config(&deps.storage).unwrap();
        let swap_fee = config.swap_fee_percent * receive_amount;
        let total_after_swap_fee = receive_amount - swap_fee;

        let automation_fee = vault.destinations.iter().filter(|d| d.msg.is_some()).fold(
            Uint128::zero(),
            |acc, destination| {
                let allocation_amount =
                    checked_mul(total_after_swap_fee, destination.allocation).unwrap();
                let allocation_automation_fee =
                    checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
                acc.checked_add(allocation_automation_fee).unwrap()
            },
        );

        for fee_collector in config.fee_collectors.iter() {
            assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
                to_address: fee_collector.address.to_string(),
                amount: vec![Coin::new(
                    checked_mul(swap_fee, fee_collector.allocation)
                        .unwrap()
                        .into(),
                    vault.target_denom.clone()
                )]
            })));

            assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
                to_address: fee_collector.address.to_string(),
                amount: vec![Coin::new(
                    checked_mul(automation_fee, fee_collector.allocation)
                        .unwrap()
                        .into(),
                    vault.target_denom.clone()
                )]
            })));
        }
    }

    #[test]
    fn with_succcesful_swap_adjusts_vault_balance() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());
        let receive_amount = Uint128::new(234312312);

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(receive_amount.into(), vault.target_denom.clone())],
        );

        disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let updated_vault = get_vault(&deps.storage, vault.id).unwrap();

        assert_eq!(
            updated_vault.balance.amount,
            vault.balance.amount
                - get_swap_amount(&deps.as_ref(), &env, &vault)
                    .unwrap()
                    .amount
        );
    }

    #[test]
    fn with_succcesful_swap_adjusts_swapped_amount_stat() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());
        let receive_amount = Uint128::new(234312312);

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![
                Coin::new(
                    (vault.balance.amount
                        - get_swap_amount(&deps.as_ref(), &env, &vault)
                            .unwrap()
                            .amount)
                        .into(),
                    vault.get_swap_denom(),
                ),
                Coin::new(receive_amount.into(), vault.target_denom.clone()),
            ],
        );

        disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let updated_vault = get_vault(&deps.storage, vault.id).unwrap();

        assert_eq!(
            updated_vault.swapped_amount.amount,
            get_swap_amount(&deps.as_ref(), &env, &vault)
                .unwrap()
                .amount
        );
    }

    #[test]
    fn with_succcesful_swap_adjusts_received_amount_stat() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());
        let receive_amount = Uint128::new(234312312);

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(receive_amount.into(), vault.target_denom.clone())],
        );

        disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let updated_vault = get_vault(&deps.storage, vault.id).unwrap();
        let config = get_config(&deps.storage).unwrap();

        let mut fee = config.swap_fee_percent * receive_amount;

        vault
            .destinations
            .iter()
            .filter(|d| d.msg.is_some())
            .for_each(|destination| {
                let allocation_amount =
                    checked_mul(receive_amount - fee, destination.allocation).unwrap();
                let allocation_automation_fee =
                    checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
                fee = fee.checked_add(allocation_automation_fee).unwrap();
            });

        assert_eq!(updated_vault.received_amount.amount, receive_amount - fee);
    }

    #[test]
    fn with_succcesful_swap_with_dca_plus_escrows_funds() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                destinations: vec![Destination::default()],
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        let receive_amount = Uint128::new(10000);

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(receive_amount.into(), vault.target_denom.clone())],
        );

        update_swap_adjustments(
            deps.as_mut().storage,
            PositionType::Exit,
            vec![
                (30, Decimal::from_str("1.0").unwrap()),
                (35, Decimal::from_str("1.0").unwrap()),
                (40, Decimal::from_str("1.0").unwrap()),
                (45, Decimal::from_str("1.0").unwrap()),
                (50, Decimal::from_str("1.0").unwrap()),
                (55, Decimal::from_str("1.0").unwrap()),
                (60, Decimal::from_str("1.0").unwrap()),
                (70, Decimal::from_str("1.0").unwrap()),
                (80, Decimal::from_str("1.0").unwrap()),
                (90, Decimal::from_str("1.0").unwrap()),
            ],
            env.block.time,
        )
        .unwrap();

        let response = disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let updated_vault = get_vault(&deps.storage, vault.id).unwrap();

        let escrow_level = updated_vault
            .swap_adjustment_strategy
            .clone()
            .unwrap()
            .dca_plus_escrow_level();
        let escrow_amount = escrow_level * receive_amount;

        assert_eq!(
            escrow_amount,
            updated_vault
                .swap_adjustment_strategy
                .clone()
                .unwrap()
                .dca_plus_escrowed_balance()
                .amount
        );
        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: updated_vault
                .destinations
                .first()
                .unwrap()
                .address
                .to_string(),
            amount: vec![Coin::new(
                (receive_amount - escrow_amount).into(),
                updated_vault.target_denom,
            )],
        },)));
        assert_ne!(escrow_level, Decimal::zero());
        assert_ne!(escrow_amount, Uint128::zero());
    }

    #[test]
    fn with_succcesful_swap_publishes_dca_execution_completed_event() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let receive_amount = Uint128::new(10000);

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(receive_amount.into(), vault.target_denom.clone())],
        );

        update_swap_adjustments(
            deps.as_mut().storage,
            PositionType::Exit,
            vec![
                (30, Decimal::from_str("1.0").unwrap()),
                (35, Decimal::from_str("1.0").unwrap()),
                (40, Decimal::from_str("1.0").unwrap()),
                (45, Decimal::from_str("1.0").unwrap()),
                (50, Decimal::from_str("1.0").unwrap()),
                (55, Decimal::from_str("1.0").unwrap()),
                (60, Decimal::from_str("1.0").unwrap()),
                (70, Decimal::from_str("1.0").unwrap()),
                (80, Decimal::from_str("1.0").unwrap()),
                (90, Decimal::from_str("1.0").unwrap()),
            ],
            env.block.time,
        )
        .unwrap();

        disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let updated_vault = get_vault(&deps.storage, vault.id).unwrap();

        let events = get_events_by_resource_id_handler(deps.as_ref(), vault.id, None, None, None)
            .unwrap()
            .events;

        let config = get_config(deps.as_ref().storage).unwrap();

        let inverted_fee_rate =
            Decimal::one() - (config.swap_fee_percent + config.delegation_fee_percent);
        let received_amount =
            updated_vault.received_amount.amount * (Decimal::one() / inverted_fee_rate);
        let fee = received_amount - updated_vault.received_amount.amount - Uint128::new(2); // rounding

        assert!(events.contains(&Event {
            id: 1,
            resource_id: vault.id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::DcaVaultExecutionCompleted {
                sent: updated_vault.swapped_amount,
                received: add_to(updated_vault.received_amount, fee),
                fee: Coin::new(fee.into(), vault.target_denom.clone())
            }
        }))
    }

    #[test]
    fn with_succcesful_swap_with_dca_plus_publishes_execution_completed_event() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        let receive_amount = Uint128::new(10000);

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(receive_amount.into(), vault.target_denom.clone())],
        );

        update_swap_adjustments(
            deps.as_mut().storage,
            PositionType::Exit,
            vec![
                (30, Decimal::from_str("1.0").unwrap()),
                (35, Decimal::from_str("1.0").unwrap()),
                (40, Decimal::from_str("1.0").unwrap()),
                (45, Decimal::from_str("1.0").unwrap()),
                (50, Decimal::from_str("1.0").unwrap()),
                (55, Decimal::from_str("1.0").unwrap()),
                (60, Decimal::from_str("1.0").unwrap()),
                (70, Decimal::from_str("1.0").unwrap()),
                (80, Decimal::from_str("1.0").unwrap()),
                (90, Decimal::from_str("1.0").unwrap()),
            ],
            env.block.time,
        )
        .unwrap();

        disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let updated_vault = get_vault(&deps.storage, vault.id).unwrap();

        let events = get_events_by_resource_id_handler(deps.as_ref(), vault.id, None, None, None)
            .unwrap()
            .events;

        assert!(events.contains(&Event {
            id: 1,
            resource_id: vault.id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::DcaVaultExecutionCompleted {
                sent: updated_vault.swapped_amount,
                received: updated_vault.received_amount,
                fee: Coin::new(0, vault.target_denom.clone())
            }
        }))
    }

    #[test]
    fn with_failed_swap_and_insufficient_funds_does_not_reduce_vault_balance() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let balance = Coin::new(TWO_MICRONS.into(), DENOM_UOSMO);

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                balance: balance.clone(),
                ..Vault::default()
            },
        );

        let reply = Reply {
            id: AFTER_SWAP_REPLY_ID,
            result: SubMsgResult::Err("Generic failure".to_string()),
        };

        disburse_funds_handler(deps.as_mut(), &env, reply).unwrap();

        let updated_vault = get_vault(&mut deps.storage, vault.id).unwrap();

        assert_eq!(vault.balance, balance);
        assert_eq!(updated_vault.balance, balance);
    }

    #[test]
    fn with_failed_swap_publishes_skipped_event_with_slippage_failure() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let reply = Reply {
            id: AFTER_SWAP_REPLY_ID,
            result: SubMsgResult::Err("failed for slippage".to_string()),
        };

        disburse_funds_handler(deps.as_mut(), &env, reply).unwrap();

        let events = get_events_by_resource_id_handler(deps.as_ref(), vault.id, None, None, None)
            .unwrap()
            .events;

        assert!(events.contains(
            &EventBuilder::new(
                vault.id,
                env.block.clone(),
                EventData::DcaVaultExecutionSkipped {
                    reason: ExecutionSkippedReason::SlippageToleranceExceeded
                }
            )
            .build(1)
        ));
    }

    #[test]
    fn with_failed_swap_leaves_vault_active() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let reply = Reply {
            id: AFTER_SWAP_REPLY_ID,
            result: SubMsgResult::Err("failed for slippage".to_string()),
        };

        disburse_funds_handler(deps.as_mut(), &env, reply).unwrap();

        let vault = get_vault(&mut deps.storage, vault.id).unwrap();

        assert_eq!(vault.status, VaultStatus::Active);
    }

    #[test]
    fn with_failed_swap_does_not_reduce_vault_balance() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let reply = Reply {
            id: AFTER_SWAP_REPLY_ID,
            result: SubMsgResult::Err("failed for slippage".to_string()),
        };

        disburse_funds_handler(deps.as_mut(), &env, reply).unwrap();

        let vault = get_vault(&mut deps.storage, vault.id).unwrap();

        assert_eq!(vault.balance, Coin::new(TEN.into(), vault.get_swap_denom()));
    }

    #[test]
    fn with_custom_fee_for_base_denom_takes_custom_fee() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let custom_fee_percent = Decimal::percent(20);

        create_custom_fee(
            &mut deps.storage,
            vault.get_swap_denom(),
            custom_fee_percent,
        )
        .unwrap();

        let receive_amount = Uint128::new(234312312);

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(receive_amount.into(), vault.target_denom.clone())],
        );

        let response = disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let config = get_config(&deps.storage).unwrap();
        let swap_fee = custom_fee_percent * receive_amount;
        let total_after_swap_fee = receive_amount - swap_fee;

        let automation_fee = vault.destinations.iter().filter(|d| d.msg.is_some()).fold(
            Uint128::zero(),
            |acc, destination| {
                let allocation_amount =
                    checked_mul(total_after_swap_fee, destination.allocation).unwrap();
                let allocation_automation_fee =
                    checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
                acc.checked_add(allocation_automation_fee).unwrap()
            },
        );

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: config.fee_collectors[0].address.to_string(),
            amount: vec![Coin::new(swap_fee.into(), vault.target_denom.clone())]
        })));

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: config.fee_collectors[0].address.to_string(),
            amount: vec![Coin::new(automation_fee.into(), vault.target_denom.clone())]
        })));
    }

    #[test]
    fn with_custom_fee_for_quote_denom_takes_custom_fee() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let custom_fee_percent = Decimal::percent(20);

        create_custom_fee(
            &mut deps.storage,
            vault.target_denom.clone(),
            custom_fee_percent,
        )
        .unwrap();

        let receive_amount = Uint128::new(234312312);

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(receive_amount.into(), vault.target_denom.clone())],
        );

        let response = disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let config = get_config(&deps.storage).unwrap();
        let swap_fee = custom_fee_percent * receive_amount;
        let total_after_swap_fee = receive_amount - swap_fee;

        let automation_fee = vault.destinations.iter().filter(|d| d.msg.is_some()).fold(
            Uint128::zero(),
            |acc, destination| {
                let allocation_amount =
                    checked_mul(total_after_swap_fee, destination.allocation).unwrap();
                let allocation_automation_fee =
                    checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
                acc.checked_add(allocation_automation_fee).unwrap()
            },
        );

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: config.fee_collectors[0].address.to_string(),
            amount: vec![Coin::new(swap_fee.into(), vault.target_denom.clone())]
        })));

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: config.fee_collectors[0].address.to_string(),
            amount: vec![Coin::new(automation_fee.into(), vault.target_denom.clone())]
        })));
    }

    #[test]
    fn with_custom_fee_for_both_denoms_takes_lower_fee() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let swap_denom_fee_percent = Decimal::percent(20);
        let receive_denom_fee_percent = Decimal::percent(40);

        create_custom_fee(
            &mut deps.storage,
            vault.get_swap_denom(),
            swap_denom_fee_percent,
        )
        .unwrap();

        create_custom_fee(
            &mut deps.storage,
            vault.target_denom.clone(),
            receive_denom_fee_percent,
        )
        .unwrap();

        let receive_amount = Uint128::new(234312312);

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(receive_amount.into(), vault.target_denom.clone())],
        );

        let response = disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let config = get_config(&deps.storage).unwrap();
        let swap_fee = min(swap_denom_fee_percent, receive_denom_fee_percent) * receive_amount;
        let total_after_swap_fee = receive_amount - swap_fee;

        let automation_fee = vault.destinations.iter().filter(|d| d.msg.is_some()).fold(
            Uint128::zero(),
            |acc, destination| {
                let allocation_amount =
                    checked_mul(total_after_swap_fee, destination.allocation).unwrap();
                let allocation_automation_fee =
                    checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
                acc.checked_add(allocation_automation_fee).unwrap()
            },
        );

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: config.fee_collectors[0].address.to_string(),
            amount: vec![Coin::new(swap_fee.into(), vault.target_denom.clone())]
        })));

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: config.fee_collectors[0].address.to_string(),
            amount: vec![Coin::new(automation_fee.into(), vault.target_denom.clone())]
        })));
    }

    #[test]
    fn with_insufficient_remaining_funds_sets_vault_to_inactive() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                balance: Coin::new(ONE.into(), DENOM_UOSMO),
                swap_amount: ONE,
                ..Vault::default()
            },
        );

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(1000000, vault.target_denom.clone())],
        );

        disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let vault = get_vault(&deps.storage, vault.id).unwrap();
        assert_eq!(vault.status, VaultStatus::Inactive);
    }

    #[test]
    fn for_dca_plus_vault_with_failed_swap_publishes_slippage_tolerance_exceeded_event() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                balance: Coin::new(ONE.into(), DENOM_UOSMO),
                swap_amount: ONE,
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(1000000, vault.target_denom.clone())],
        );

        disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Err("slippage exceeded".to_string()),
            },
        )
        .unwrap();

        let events = get_events_by_resource_id_handler(deps.as_ref(), vault.id, None, None, None)
            .unwrap()
            .events;

        assert!(events.contains(&Event {
            id: 1,
            resource_id: vault.id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::DcaVaultExecutionSkipped {
                reason: ExecutionSkippedReason::SlippageToleranceExceeded
            }
        }));
    }

    #[test]
    fn for_dca_plus_vault_with_insufficient_remaining_funds_sets_vault_to_inactive() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                balance: Coin::new(49999, DENOM_UOSMO),
                swap_amount: ONE,
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(1000000, vault.target_denom.clone())],
        );

        disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let vault = get_vault(&deps.storage, vault.id).unwrap();
        assert_eq!(vault.status, VaultStatus::Inactive);
    }

    #[test]
    fn for_finished_standard_and_plus_disburses_escrow() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                balance: Coin::new(0, DENOM_UOSMO),
                status: VaultStatus::Inactive,
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::DcaPlus {
                    total_deposit: Coin::new(ONE.into(), DENOM_UOSMO),
                    standard_dca_swapped_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                    model_id: 30,
                    escrow_level: Decimal::percent(5),
                    standard_dca_received_amount: Coin::new(ONE.into(), DENOM_STAKE),
                    escrowed_balance: Coin::new((ONE * Decimal::percent(5)).into(), DENOM_STAKE),
                }),
                ..Vault::default()
            },
        );

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(1000000, vault.target_denom.clone())],
        );

        let response = disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        assert!(response.messages.contains(&SubMsg::new(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::DisburseEscrow { vault_id: vault.id }).unwrap(),
            funds: vec![],
        })));
    }

    #[test]
    fn for_finished_standard_and_plus_deletes_trigger() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                balance: Coin::new(0, DENOM_UOSMO),
                status: VaultStatus::Inactive,
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::DcaPlus {
                    total_deposit: Coin::new(ONE.into(), DENOM_UOSMO),
                    standard_dca_swapped_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                    model_id: 30,
                    escrow_level: Decimal::percent(5),
                    standard_dca_received_amount: Coin::new(ONE.into(), DENOM_STAKE),
                    escrowed_balance: Coin::new((ONE * Decimal::percent(5)).into(), DENOM_STAKE),
                }),
                ..Vault::default()
            },
        );

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(1000000, vault.target_denom.clone())],
        );

        disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let vault = get_vault(&deps.storage, vault.id).unwrap();
        assert!(vault.trigger.is_none());
    }

    #[test]
    fn for_unfinished_standard_and_finished_plus_keeps_trigger() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                balance: Coin::new(0, DENOM_UOSMO),
                status: VaultStatus::Inactive,
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::DcaPlus {
                    total_deposit: Coin::new(ONE.into(), DENOM_UOSMO),
                    standard_dca_swapped_amount: Coin::new((ONE / TWO_MICRONS).into(), DENOM_UOSMO),
                    model_id: 30,
                    escrow_level: Decimal::percent(5),
                    standard_dca_received_amount: Coin::new(
                        (ONE / TWO_MICRONS).into(),
                        DENOM_STAKE,
                    ),
                    escrowed_balance: Coin::new(
                        (ONE / TWO_MICRONS * Decimal::percent(5)).into(),
                        DENOM_STAKE,
                    ),
                }),
                ..Vault::default()
            },
        );

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    swap_denom_balance: vault.balance.clone(),
                    receive_denom_balance: Coin::new(0, vault.target_denom.clone()),
                },
            )
            .unwrap();

        deps.querier.update_balance(
            "cosmos2contract",
            vec![Coin::new(1000000, vault.target_denom.clone())],
        );

        disburse_funds_handler(
            deps.as_mut(),
            &env,
            Reply {
                id: AFTER_SWAP_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    events: vec![],
                    data: None,
                }),
            },
        )
        .unwrap();

        let vault = get_vault(&deps.storage, vault.id).unwrap();
        assert!(vault.trigger.is_some());
    }
}
