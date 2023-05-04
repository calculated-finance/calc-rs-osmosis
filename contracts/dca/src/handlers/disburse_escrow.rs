use crate::{
    error::ContractError,
    helpers::{
        coin::{empty_of, subtract},
        disbursement::get_disbursement_messages,
        fees::{get_fee_messages, get_performance_fee},
        price::query_belief_price,
        validation::assert_sender_is_executor,
    },
    state::{
        disburse_escrow_tasks::delete_disburse_escrow_task,
        events::create_event,
        pairs::find_pair,
        vaults::{get_vault, update_vault},
    },
    types::{
        event::{EventBuilder, EventData},
        vault::Vault,
    },
};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};

pub fn disburse_escrow_handler(
    deps: DepsMut,
    env: &Env,
    info: MessageInfo,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    assert_sender_is_executor(deps.storage, env, &info.sender)?;

    let vault = get_vault(deps.storage, vault_id)?;

    if vault.escrowed_amount.amount.is_zero() {
        return Ok(Response::new());
    }

    let pair = find_pair(deps.storage, &vault.denoms())?;
    let current_price = query_belief_price(&deps.querier, &pair, vault.get_swap_denom())?;
    let performance_fee = get_performance_fee(&vault, current_price)?;
    let amount_to_disburse = subtract(&vault.escrowed_amount, &performance_fee)?;

    let vault = Vault {
        escrowed_amount: empty_of(vault.escrowed_amount),
        ..vault
    };

    update_vault(deps.storage, &vault)?;

    create_event(
        deps.storage,
        EventBuilder::new(
            vault.id,
            env.block.clone(),
            EventData::DcaVaultEscrowDisbursed {
                amount_disbursed: amount_to_disburse.clone(),
                performance_fee: performance_fee.clone(),
            },
        ),
    )?;

    delete_disburse_escrow_task(deps.storage, vault.id)?;

    Ok(Response::new()
        .add_submessages(get_disbursement_messages(
            &vault,
            amount_to_disburse.amount,
        )?)
        .add_submessages(get_fee_messages(
            deps.as_ref(),
            vec![performance_fee.amount],
            vault.target_denom,
        )?)
        .add_attribute("disburse_escrow", "true")
        .add_attribute("vault_id", vault.id)
        .add_attribute("performance_fee", format!("{:?}", performance_fee))
        .add_attribute("escrow_disbursed", format!("{:?}", amount_to_disburse)))
}

#[cfg(test)]
mod disburse_escrow_tests {
    use super::*;
    use crate::{
        constants::{ONE, TEN},
        handlers::get_events_by_resource_id::get_events_by_resource_id_handler,
        state::{
            config::get_config,
            disburse_escrow_tasks::{get_disburse_escrow_tasks, save_disburse_escrow_task},
            vaults::get_vault,
        },
        tests::{
            helpers::{instantiate_contract, setup_vault},
            mocks::{calc_mock_dependencies, ADMIN, DENOM_STAKE, DENOM_UOSMO},
        },
        types::{
            destination::Destination,
            event::{Event, EventData},
            performance_assessment_strategy::PerformanceAssessmentStrategy,
            swap_adjustment_strategy::SwapAdjustmentStrategy,
            vault::{Vault, VaultStatus},
        },
    };
    use cosmwasm_std::{
        testing::{mock_env, mock_info},
        to_binary, BankMsg, Coin, Decimal, StdError, SubMsg, Uint128,
    };
    use osmosis_std::types::osmosis::gamm::v2::QuerySpotPriceResponse;

    #[test]
    fn when_escrowed_balance_is_empty_sends_no_messages() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                escrowed_amount: Coin::new(0, DENOM_STAKE),
                ..Vault::default()
            },
        );

        let response = disburse_escrow_handler(deps.as_mut(), &env, info, vault.id).unwrap();

        assert!(response.messages.is_empty());
    }

    #[test]
    fn when_no_fee_is_owed_returns_entire_escrow_to_owner() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Inactive,
                destinations: vec![Destination::default()],
                deposited_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                escrowed_amount: Coin::new((ONE * Decimal::percent(5)).into(), DENOM_STAKE),
                performance_assessment_strategy: Some(
                    PerformanceAssessmentStrategy::CompareToStandardDca {
                        swapped_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                        received_amount: Coin::new(ONE.into(), DENOM_STAKE),
                    },
                ),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        let response = disburse_escrow_handler(deps.as_mut(), &env, info, vault.id).unwrap();

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: vault.destinations[0].address.to_string(),
            amount: vec![vault.escrowed_amount]
        })));
    }

    #[test]
    fn when_large_fee_is_owed_returns_entire_escrow_to_fee_collector() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Inactive,
                swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                deposited_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                escrowed_amount: Coin::new((ONE * Decimal::percent(5)).into(), DENOM_STAKE),
                performance_assessment_strategy: Some(
                    PerformanceAssessmentStrategy::CompareToStandardDca {
                        swapped_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                        received_amount: Coin::new(ONE.into(), DENOM_STAKE),
                    },
                ),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        deps.querier.update_stargate(|path, _| match path {
            "/osmosis.gamm.v2.Query/SpotPrice" => to_binary(&QuerySpotPriceResponse {
                spot_price: "10.0".to_string(),
            }),
            _ => Err(StdError::generic_err("message not customised")),
        });

        let config = get_config(&deps.storage).unwrap();

        let response = disburse_escrow_handler(deps.as_mut(), &env, info, vault.id).unwrap();

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: config.fee_collectors[0].address.to_string(),
            amount: vec![vault.escrowed_amount]
        })));
    }

    #[test]
    fn publishes_escrow_disbursed_event() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Inactive,
                swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                received_amount: Coin::new((TEN + ONE).into(), DENOM_STAKE),
                deposited_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                escrowed_amount: Coin::new(((TEN + ONE) * Decimal::percent(5)).into(), DENOM_STAKE),
                performance_assessment_strategy: Some(
                    PerformanceAssessmentStrategy::CompareToStandardDca {
                        swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                        received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                    },
                ),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        disburse_escrow_handler(deps.as_mut(), &env, info, vault.id).unwrap();

        let events = get_events_by_resource_id_handler(deps.as_ref(), vault.id, None, None, None)
            .unwrap()
            .events;

        let performance_fee = Coin::new(
            (ONE * Decimal::percent(20) - Uint128::one()).into(),
            vault.target_denom,
        );

        assert!(events.contains(&Event {
            id: 1,
            resource_id: vault.id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::DcaVaultEscrowDisbursed {
                amount_disbursed: Coin::new(
                    (subtract(&vault.escrowed_amount, &performance_fee).unwrap())
                        .amount
                        .into(),
                    DENOM_STAKE
                ),
                performance_fee,
            }
        }))
    }

    #[test]
    fn sets_escrow_balance_to_zero() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Inactive,
                swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                received_amount: Coin::new((TEN + ONE).into(), DENOM_STAKE),
                deposited_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                escrowed_amount: Coin::new(((TEN + ONE) * Decimal::percent(5)).into(), DENOM_STAKE),
                performance_assessment_strategy: Some(
                    PerformanceAssessmentStrategy::CompareToStandardDca {
                        swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                        received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                    },
                ),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        disburse_escrow_handler(deps.as_mut(), &env, info, vault.id).unwrap();

        let vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_eq!(vault.escrowed_amount, Coin::new(0, vault.target_denom));
    }

    #[test]
    fn deletes_disburse_escrow_task() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Inactive,
                swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                received_amount: Coin::new((TEN + ONE).into(), DENOM_STAKE),
                deposited_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                escrowed_amount: Coin::new(((TEN + ONE) * Decimal::percent(5)).into(), DENOM_STAKE),
                performance_assessment_strategy: Some(
                    PerformanceAssessmentStrategy::CompareToStandardDca {
                        swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                        received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                    },
                ),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        save_disburse_escrow_task(
            deps.as_mut().storage,
            vault.id,
            env.block.time.minus_seconds(10),
        )
        .unwrap();

        let disburse_escrow_tasks_before =
            get_disburse_escrow_tasks(deps.as_ref().storage, env.block.time, None).unwrap();

        disburse_escrow_handler(deps.as_mut(), &env, info, vault.id).unwrap();

        let disburse_escrow_tasks_after =
            get_disburse_escrow_tasks(deps.as_ref().storage, env.block.time, None).unwrap();

        assert_eq!(disburse_escrow_tasks_before.len(), 1);
        assert_eq!(disburse_escrow_tasks_after.len(), 0);
    }
}
