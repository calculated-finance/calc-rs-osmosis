use crate::contract::AFTER_FIN_LIMIT_ORDER_SUBMITTED_REPLY_ID;
use crate::error::ContractError;
use crate::state::{create_event, save_trigger, save_vault, Cache, CACHE, PAIRS};
use crate::validation_helpers::{
    assert_address_is_valid, assert_denom_is_bond_denom, assert_denom_matches_pair_denom,
    assert_destination_allocations_add_up_to_one, assert_destination_send_addresses_are_valid,
    assert_destination_validator_addresses_are_valid, assert_destinations_limit_is_not_breached,
    assert_exactly_one_asset, assert_swap_amount_is_less_than_or_equal_to_balance,
    assert_target_start_time_is_in_future,
};
use crate::vault::{Vault, VaultBuilder};
use base::events::event::{EventBuilder, EventData};
use base::triggers::trigger::{TimeInterval, Trigger, TriggerConfiguration};
use base::vaults::vault::{Destination, PositionType, PostExecutionAction, VaultStatus};
use cosmwasm_std::{Addr, Decimal, Decimal256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Timestamp, Uint128, Uint64};
use fin_helpers::limit_orders::create_limit_order_sub_msg;

pub fn create_vault(
    deps: DepsMut,
    env: Env,
    info: &MessageInfo,
    owner: Addr,
    label: Option<String>,
    mut destinations: Vec<Destination>,
    pair_address: Addr,
    position_type: PositionType,
    slippage_tolerance: Option<Decimal256>,
    price_threshold: Option<Decimal256>,
    swap_amount: Uint128,
    time_interval: TimeInterval,
    target_start_time_utc_seconds: Option<Uint64>,
    target_price: Option<Decimal256>,
) -> Result<Response, ContractError> {
    assert_address_is_valid(deps.as_ref(), owner.clone(), "owner".to_string())?;
    assert_exactly_one_asset(info.funds.clone())?;
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
            address: info.sender.clone(),
            allocation: Decimal::percent(100),
            action: PostExecutionAction::Send,
        });
    }

    assert_destination_send_addresses_are_valid(deps.as_ref(), &destinations)?;
    assert_destination_validator_addresses_are_valid(deps.as_ref(), &destinations)?;
    assert_destination_allocations_add_up_to_one(&destinations)?;

    deps.api.addr_validate(&pair_address.to_string())?;
    let pair = PAIRS.load(deps.storage, pair_address)?;

    assert_denom_matches_pair_denom(pair.clone(), info.funds.clone(), position_type.clone())?;

    // if there is atleast one zdelegate action assert denom can be bonded
    if destinations
        .iter()
        .find(|destination| destination.action == PostExecutionAction::ZDelegate)
        .is_some()
    {
        match position_type {
            PositionType::Enter => {
                assert_denom_is_bond_denom(pair.base_denom.clone())?;
            }
            PositionType::Exit => {
                assert_denom_is_bond_denom(pair.quote_denom.clone())?;
            }
        }
    }

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
        price_threshold,
        balance: info.funds[0].clone(),
        time_interval: time_interval.clone(),
        started_at: None,
    };

    let vault = save_vault(deps.storage, vault_builder)?;

    create_event(
        deps.storage,
        EventBuilder::new(vault.id, env.block.clone(), EventData::DCAVaultCreated),
    )?;

    match (target_start_time_utc_seconds, target_price) {
        (None, None) | (Some(_), None) => {
            create_time_trigger(deps, env, vault, target_start_time_utc_seconds)
        }
        (None, Some(_)) => create_fin_limit_order_trigger(deps, vault, target_price.unwrap()),
        (Some(_), Some(_)) => Err(ContractError::CustomError {
            val: String::from(
                "cannot provide both a target_start_time_utc_seconds and a target_price",
            ),
        }),
    }
}

fn create_time_trigger(
    deps: DepsMut,
    env: Env,
    vault: Vault,
    target_start_time_utc_seconds: Option<Uint64>,
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

    Ok(Response::new()
        .add_attribute("method", "create_vault")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id))
}

fn create_fin_limit_order_trigger(
    deps: DepsMut,
    vault: Vault,
    target_price: Decimal256,
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

    CACHE.save(
        deps.storage,
        &Cache {
            vault_id: vault.id.clone(),
            owner: vault.owner.clone(),
        },
    )?;

    let fin_limit_order_sub_msg = create_limit_order_sub_msg(
        vault.pair.address.clone(),
        target_price,
        vault.get_swap_amount(),
        AFTER_FIN_LIMIT_ORDER_SUBMITTED_REPLY_ID,
    );

    Ok(Response::new()
        .add_attribute("method", "create_vault")
        .add_attribute("owner", vault.owner)
        .add_attribute("vault_id", vault.id)
        .add_submessage(fin_limit_order_sub_msg))
}
