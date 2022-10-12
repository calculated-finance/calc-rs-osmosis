use crate::contract::FIN_LIMIT_ORDER_SUBMITTED_ID;
use crate::error::ContractError;
use crate::state::{
    save_event, trigger_store, vault_store, Cache, Config, CACHE, CONFIG,
    FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID, PAIRS, TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID,
};
use crate::validation_helpers::{
    assert_denom_matches_pair_denom, assert_exactly_one_asset,
    assert_swap_amount_is_less_than_or_equal_to_balance, assert_target_start_time_is_in_future,
};
use base::events::event::{EventBuilder, EventData};
use base::triggers::trigger::{TimeInterval, Trigger, TriggerConfiguration};
use base::vaults::vault::{PositionType, Vault, VaultConfiguration, VaultStatus};
use cosmwasm_std::Decimal256;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdResult, Timestamp, Uint128, Uint64};
use fin_helpers::limit_orders::create_limit_order_sub_msg;

pub fn create_vault(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_address: String,
    position_type: PositionType,
    slippage_tolerance: Option<Decimal256>,
    swap_amount: Uint128,
    time_interval: TimeInterval,
    target_start_time_utc_seconds: Option<Uint64>,
    target_price: Option<Decimal256>,
) -> Result<Response, ContractError> {
    match (target_start_time_utc_seconds, target_price) {
        (None, None) | (Some(_), None) => create_vault_with_time_trigger(
            deps,
            env,
            info,
            pair_address,
            position_type,
            slippage_tolerance,
            swap_amount,
            time_interval,
            target_start_time_utc_seconds,
        ),
        (None, Some(_)) => create_vault_with_fin_limit_order_trigger(
            deps,
            env,
            info,
            pair_address,
            position_type,
            slippage_tolerance,
            swap_amount,
            time_interval,
            target_price.unwrap(),
        ),
        _ => Err(ContractError::CustomError {
            val: String::from(
                "Cannot provide both a target_start_time_utc_seconds and a target_price",
            ),
        }),
    }
}

fn create_vault_with_time_trigger(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_address: String,
    position_type: PositionType,
    slippage_tolerance: Option<Decimal256>,
    swap_amount: Uint128,
    time_interval: TimeInterval,
    target_start_time_utc_seconds: Option<Uint64>,
) -> Result<Response, ContractError> {
    assert_exactly_one_asset(info.funds.clone())?;

    // if no target start time is given execute immediately
    let target_time: Timestamp = match target_start_time_utc_seconds {
        Some(time) => Timestamp::from_seconds(time.u64()),
        None => env.block.time,
    };

    assert_target_start_time_is_in_future(env.block.time, target_time)?;

    let validated_pair_address = deps.api.addr_validate(&pair_address)?;
    let existing_pair = PAIRS.load(deps.storage, validated_pair_address)?;

    assert_denom_matches_pair_denom(
        existing_pair.clone(),
        info.funds.clone(),
        position_type.clone(),
    )?;

    assert_swap_amount_is_less_than_or_equal_to_balance(swap_amount, info.funds[0].clone())?;

    let config = CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
        config.vault_count = config.vault_count.checked_add(Uint128::new(1))?;
        config.trigger_count = config.trigger_count.checked_add(Uint128::new(1))?;
        Ok(config)
    })?;

    let trigger = Trigger {
        id: config.trigger_count,
        owner: info.sender.clone(),
        vault_id: config.vault_count,
        configuration: TriggerConfiguration::Time {
            time_interval,
            target_time,
        },
    };

    let vault: Vault = Vault {
        id: config.vault_count,
        owner: info.sender.clone(),
        created_at: env.block.time,
        balances: vec![info.funds[0].clone()],
        status: VaultStatus::Active,
        configuration: VaultConfiguration::DCA {
            pair: existing_pair,
            swap_amount,
            position_type,
            slippage_tolerance,
        },
        trigger_id: Some(trigger.id),
    };

    trigger_store().save(deps.storage, trigger.id.u128(), &trigger)?;

    vault_store().save(deps.storage, vault.id.u128(), &vault)?;

    save_event(
        deps.storage,
        EventBuilder::new(vault.id, env.block, EventData::VaultCreated),
    )?;

    Ok(Response::new()
        .add_attribute("method", "create_vault_with_time_trigger")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id))
}

fn create_vault_with_fin_limit_order_trigger(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_address: String,
    position_type: PositionType,
    slippage_tolerance: Option<Decimal256>,
    swap_amount: Uint128,
    time_interval: TimeInterval,
    target_price: Decimal256,
) -> Result<Response, ContractError> {
    assert_exactly_one_asset(info.funds.clone())?;

    let validated_pair_address = deps.api.addr_validate(&pair_address)?;
    let existing_pair = PAIRS.load(deps.storage, validated_pair_address)?;

    assert_denom_matches_pair_denom(
        existing_pair.clone(),
        info.funds.clone(),
        position_type.clone(),
    )?;

    assert_swap_amount_is_less_than_or_equal_to_balance(swap_amount, info.funds[0].clone())?;

    let config = CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
        config.vault_count = config.vault_count.checked_add(Uint128::new(1))?;
        Ok(config)
    })?;

    // trigger information is updated upon successful limit order creation
    let vault: Vault = Vault {
        id: config.vault_count,
        owner: info.sender.clone(),
        created_at: env.block.time,
        balances: vec![info.funds[0].clone()],
        status: VaultStatus::Active,
        configuration: VaultConfiguration::DCA {
            pair: existing_pair.clone(),
            swap_amount,
            position_type,
            slippage_tolerance,
        },
        trigger_id: None,
    };

    let coin_to_send = vault.get_swap_amount();

    let fin_limit_order_sub_msg = create_limit_order_sub_msg(
        existing_pair.address,
        target_price,
        coin_to_send.clone(),
        FIN_LIMIT_ORDER_SUBMITTED_ID,
    );

    // removed when trigger change over occurs
    TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID.save(
        deps.storage,
        vault.id.u128(),
        &TriggerConfiguration::Time {
            target_time: env.block.time,
            time_interval,
        },
    )?;

    // removed with successful limit order creation
    FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID.save(
        deps.storage,
        vault.id.u128(),
        &TriggerConfiguration::FINLimitOrder {
            target_price,
            order_idx: None,
        },
    )?;

    vault_store().save(deps.storage, vault.id.u128(), &vault)?;

    save_event(
        deps.storage,
        EventBuilder::new(vault.id, env.block, EventData::VaultCreated),
    )?;

    CACHE.save(
        deps.storage,
        &Cache {
            vault_id: vault.id.clone(),
            owner: vault.owner.clone(),
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "create_vault_with_fin_limit_order_trigger")
        .add_attribute("owner", vault.owner)
        .add_attribute("vault_id", vault.id.to_string())
        .add_submessage(fin_limit_order_sub_msg))
}
