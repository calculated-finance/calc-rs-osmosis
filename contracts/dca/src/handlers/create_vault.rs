use crate::contract::FIN_LIMIT_ORDER_SUBMITTED_ID;
use crate::error::ContractError;
use crate::state::{create_event, save_trigger, vault_store, Cache, Config, CACHE, CONFIG, PAIRS};
use crate::validation_helpers::{
    assert_denom_matches_pair_denom, assert_destination_allocations_add_up_to_one,
    assert_destinations_limit_is_not_breached, assert_exactly_one_asset,
    assert_swap_amount_is_less_than_or_equal_to_balance, assert_target_start_time_is_in_future,
};
use crate::vault::Vault;
use base::events::event::{EventBuilder, EventData};
use base::triggers::trigger::{TimeInterval, Trigger, TriggerConfiguration};
use base::vaults::vault::{Destination, PositionType, VaultStatus};
use cosmwasm_std::{Decimal, Decimal256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdResult, Timestamp, Uint128, Uint64};
use fin_helpers::limit_orders::create_limit_order_sub_msg;

pub fn create_vault(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut destinations: Vec<Destination>,
    pair_address: String,
    position_type: PositionType,
    slippage_tolerance: Option<Decimal256>,
    swap_amount: Uint128,
    time_interval: TimeInterval,
    target_start_time_utc_seconds: Option<Uint64>,
    target_price: Option<Decimal256>,
) -> Result<Response, ContractError> {
    assert_exactly_one_asset(info.funds.clone())?;
    assert_destinations_limit_is_not_breached(&destinations)?;

    if destinations.is_empty() {
        destinations.push(Destination {
            address: info.sender.clone(),
            allocation: Decimal::percent(100),
        });
    }

    assert_destination_allocations_add_up_to_one(&destinations)?;

    let pair = PAIRS.load(deps.storage, deps.api.addr_validate(&pair_address)?)?;

    assert_denom_matches_pair_denom(pair.clone(), info.funds.clone(), position_type.clone())?;
    assert_swap_amount_is_less_than_or_equal_to_balance(swap_amount, info.funds[0].clone())?;

    let config = CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
        config.vault_count = config.vault_count.checked_add(Uint128::new(1))?;
        Ok(config)
    })?;

    let vault = Vault {
        id: config.vault_count,
        owner: info.sender.clone(),
        destinations,
        created_at: env.block.time,
        status: VaultStatus::Active,
        pair,
        swap_amount,
        position_type,
        slippage_tolerance,
        balance: info.funds[0].clone(),
        time_interval: time_interval.clone(),
        started_at: None,
    };

    vault_store().save(deps.storage, vault.id.into(), &vault)?;

    create_event(
        deps.storage,
        EventBuilder::new(vault.id, env.block.clone(), EventData::DCAVaultCreated),
    )?;

    match (target_start_time_utc_seconds, target_price) {
        (None, None) | (Some(_), None) => {
            create_time_trigger(deps, env, vault, target_start_time_utc_seconds)
        }
        (None, Some(_)) => create_fin_limit_order_trigger(deps, vault, target_price.unwrap()),
        _ => Err(ContractError::CustomError {
            val: String::from(
                "Cannot provide both a target_start_time_utc_seconds and a target_price",
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

    assert_target_start_time_is_in_future(env.block.time, target_time)?;

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
        FIN_LIMIT_ORDER_SUBMITTED_ID,
    );

    Ok(Response::new()
        .add_attribute("method", "create_vault")
        .add_attribute("owner", vault.owner)
        .add_attribute("vault_id", vault.id)
        .add_submessage(fin_limit_order_sub_msg))
}
