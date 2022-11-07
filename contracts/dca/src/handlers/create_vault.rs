use crate::contract::AFTER_FIN_LIMIT_ORDER_SUBMITTED_REPLY_ID;
use crate::error::ContractError;
use crate::state::cache::{Cache, CACHE};
use crate::state::events::create_event;
use crate::state::pairs::PAIRS;
use crate::state::triggers::save_trigger;
use crate::state::vaults::save_vault;
use crate::types::vault::Vault;
use crate::types::vault_builder::VaultBuilder;
use crate::validation_helpers::{
    assert_address_is_valid, assert_delegation_denom_is_stakeable,
    assert_destination_allocations_add_up_to_one, assert_destination_send_addresses_are_valid,
    assert_destination_validator_addresses_are_valid, assert_destinations_limit_is_not_breached,
    assert_exactly_one_asset, assert_no_destination_allocations_are_zero,
    assert_send_denom_is_in_pair_denoms, assert_swap_amount_is_less_than_or_equal_to_balance,
    assert_swap_amount_is_not_zero, assert_target_start_time_is_in_future,
};
use base::events::event::{EventBuilder, EventData};
use base::triggers::trigger::{TimeInterval, Trigger, TriggerConfiguration};
use base::vaults::vault::{Destination, PositionType, PostExecutionAction, VaultStatus};
use cosmwasm_std::{Addr, Decimal, Decimal256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Timestamp, Uint128, Uint64};
use fin_helpers::limit_orders::create_limit_order_sub_msg;

use super::execute_trigger::execute_trigger;

pub fn create_vault(
    mut deps: DepsMut,
    env: Env,
    info: &MessageInfo,
    owner: Addr,
    label: Option<String>,
    mut destinations: Vec<Destination>,
    pair_address: Addr,
    position_type: Option<PositionType>,
    slippage_tolerance: Option<Decimal256>,
    minimum_receive_amount: Option<Uint128>,
    swap_amount: Uint128,
    time_interval: TimeInterval,
    target_start_time_utc_seconds: Option<Uint64>,
    target_price: Option<Decimal256>,
) -> Result<Response, ContractError> {
    assert_address_is_valid(deps.as_ref(), owner.clone(), "owner".to_string())?;
    assert_exactly_one_asset(info.funds.clone())?;
    assert_swap_amount_is_not_zero(swap_amount)?;
    assert_swap_amount_is_less_than_or_equal_to_balance(swap_amount, info.funds[0].clone())?;
    assert_destinations_limit_is_not_breached(&destinations)?;

    if let Some(target_time) = target_start_time_utc_seconds {
        assert_target_start_time_is_in_future(
            env.block.time,
            Timestamp::from_seconds(target_time.u64()),
        )?;
    }

    if destinations.is_empty() {
        destinations.push(Destination {
            address: owner.clone(),
            allocation: Decimal::percent(100),
            action: PostExecutionAction::Send,
        });
    }

    assert_destination_send_addresses_are_valid(deps.as_ref(), &destinations)?;
    assert_destination_validator_addresses_are_valid(deps.as_ref(), &destinations)?;
    assert_no_destination_allocations_are_zero(&destinations)?;
    assert_destination_allocations_add_up_to_one(&destinations)?;

    deps.api.addr_validate(&pair_address.to_string())?;
    let pair = PAIRS.load(deps.storage, pair_address)?;

    let send_denom = info.funds[0].denom.clone();

    assert_send_denom_is_in_pair_denoms(pair.clone(), send_denom.clone())?;

    let receive_denom = if send_denom == pair.quote_denom {
        pair.base_denom.clone()
    } else {
        pair.quote_denom.clone()
    };

    assert_delegation_denom_is_stakeable(&destinations, receive_denom)?;

    let vault_builder = VaultBuilder {
        owner,
        label,
        destinations,
        created_at: env.block.time,
        status: VaultStatus::Scheduled,
        pair,
        swap_amount,
        position_type,
        slippage_tolerance,
        minimum_receive_amount,
        balance: info.funds[0].clone(),
        time_interval: time_interval.clone(),
        started_at: None,
    };

    let vault = save_vault(deps.storage, vault_builder)?;

    CACHE.save(
        deps.storage,
        &Cache {
            vault_id: vault.id.clone(),
            owner: vault.owner.clone(),
        },
    )?;

    create_event(
        deps.storage,
        EventBuilder::new(vault.id, env.block.clone(), EventData::DCAVaultCreated),
    )?;

    create_event(
        deps.storage,
        EventBuilder::new(
            vault.id,
            env.block.clone(),
            EventData::DCAVaultFundsDeposited {
                amount: info.funds[0].clone(),
            },
        ),
    )?;

    let response = Response::new()
        .add_attribute("method", "create_vault")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id);

    match (target_start_time_utc_seconds, target_price) {
        (None, None) | (Some(_), None) => {
            let response = create_time_trigger(
                &mut deps,
                &env,
                &vault,
                target_start_time_utc_seconds,
                &response,
            )
            .expect("time trigger created");

            if target_start_time_utc_seconds.is_none() {
                return Ok(
                    execute_trigger(deps, env, vault.id, response).expect("time trigger executed")
                );
            }

            Ok(response)
        }
        (None, Some(_)) => {
            create_fin_limit_order_trigger(deps, vault, target_price.unwrap(), response)
        }
        (Some(_), Some(_)) => Err(ContractError::CustomError {
            val: String::from(
                "cannot provide both a target_start_time_utc_seconds and a target_price",
            ),
        }),
    }
}

fn create_time_trigger(
    deps: &mut DepsMut,
    env: &Env,
    vault: &Vault,
    target_start_time_utc_seconds: Option<Uint64>,
    response: &Response,
) -> Result<Response, ContractError> {
    let target_time: Timestamp = match target_start_time_utc_seconds {
        Some(time) => Timestamp::from_seconds(time.u64()),
        None => env.block.time,
    };

    save_trigger(
        deps.storage,
        Trigger {
            vault_id: vault.id,
            configuration: TriggerConfiguration::Time { target_time },
        },
    )?;

    Ok(response.to_owned())
}

fn create_fin_limit_order_trigger(
    deps: DepsMut,
    vault: Vault,
    target_price: Decimal256,
    response: Response,
) -> Result<Response, ContractError> {
    save_trigger(
        deps.storage,
        Trigger {
            vault_id: vault.id,
            configuration: TriggerConfiguration::FINLimitOrder {
                order_idx: None,
                target_price: target_price.clone(),
            },
        },
    )?;

    let fin_limit_order_sub_msg = create_limit_order_sub_msg(
        vault.pair.address.clone(),
        target_price,
        vault.get_swap_amount(),
        AFTER_FIN_LIMIT_ORDER_SUBMITTED_REPLY_ID,
    );

    Ok(response.add_submessage(fin_limit_order_sub_msg))
}
