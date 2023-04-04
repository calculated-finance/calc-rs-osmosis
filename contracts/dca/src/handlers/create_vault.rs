use crate::error::ContractError;
use crate::helpers::validation_helpers::{
    assert_address_is_valid, assert_contract_is_not_paused, assert_delegation_denom_is_stakeable,
    assert_destination_allocations_add_up_to_one, assert_destination_send_addresses_are_valid,
    assert_destination_validator_addresses_are_valid, assert_destinations_limit_is_not_breached,
    assert_exactly_one_asset, assert_no_destination_allocations_are_zero,
    assert_send_denom_is_in_pool_denoms, assert_swap_amount_is_greater_than_50000,
    assert_target_start_time_is_in_future,
};
use crate::helpers::vault_helpers::get_dca_plus_model_id;
use crate::msg::ExecuteMsg;
use crate::state::cache::{Cache, CACHE};
use crate::state::config::get_config;
use crate::state::events::create_event;
use crate::state::pools::POOLS;
use crate::state::triggers::save_trigger;
use crate::state::vaults::save_vault;
use crate::types::dca_plus_config::DcaPlusConfig;
use crate::types::vault::Vault;
use crate::types::vault_builder::VaultBuilder;
use base::events::event::{EventBuilder, EventData};
use base::triggers::trigger::{TimeInterval, Trigger, TriggerConfiguration};
use base::vaults::vault::{Destination, PostExecutionAction, VaultStatus};
use cosmwasm_std::{coin, to_binary, Addr, CosmosMsg, Decimal, WasmMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Timestamp, Uint128, Uint64};
use osmosis_helpers::position_type::PositionType;

pub fn create_vault(
    mut deps: DepsMut,
    env: Env,
    info: &MessageInfo,
    owner: Addr,
    label: Option<String>,
    mut destinations: Vec<Destination>,
    pool_id: u64,
    position_type: Option<PositionType>,
    slippage_tolerance: Option<Decimal>,
    minimum_receive_amount: Option<Uint128>,
    swap_amount: Uint128,
    time_interval: TimeInterval,
    target_start_time_utc_seconds: Option<Uint64>,
    target_receive_amount: Option<Uint128>,
    use_dca_plus: Option<bool>,
) -> Result<Response, ContractError> {
    assert_contract_is_not_paused(deps.storage)?;
    assert_address_is_valid(deps.as_ref(), owner.clone(), "owner".to_string())?;
    assert_exactly_one_asset(info.funds.clone())?;
    assert_swap_amount_is_greater_than_50000(swap_amount)?;
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

    let pool = POOLS.load(deps.storage, pool_id)?;

    let send_denom = info.funds[0].denom.clone();

    assert_send_denom_is_in_pool_denoms(pool.clone(), send_denom.clone())?;

    let receive_denom = if send_denom == pool.quote_denom {
        pool.base_denom.clone()
    } else {
        pool.quote_denom.clone()
    };

    assert_delegation_denom_is_stakeable(&destinations, receive_denom.clone())?;

    let config = get_config(deps.storage)?;

    let dca_plus_config = use_dca_plus.map_or(None, |use_dca_plus| {
        if !use_dca_plus {
            return None;
        }

        Some(DcaPlusConfig::new(
            config.dca_plus_escrow_level,
            get_dca_plus_model_id(
                &env.block.time,
                &info.funds[0],
                &swap_amount,
                &time_interval,
            ),
            info.funds[0].clone(),
            receive_denom,
        ))
    });

    let vault_builder = VaultBuilder {
        owner,
        label,
        destinations,
        created_at: env.block.time,
        status: if info.funds[0].amount.clone() <= Uint128::from(50000u128) {
            VaultStatus::Inactive
        } else {
            VaultStatus::Scheduled
        },
        pool: pool.clone(),
        swap_amount,
        position_type,
        slippage_tolerance,
        minimum_receive_amount,
        balance: info.funds[0].clone(),
        time_interval: time_interval.clone(),
        started_at: None,
        swapped_amount: coin(0, info.funds[0].clone().denom.clone()),
        received_amount: coin(
            0,
            match info.funds[0].clone().denom == pool.quote_denom {
                true => pool.base_denom,
                false => pool.quote_denom,
            },
        ),
        dca_plus_config,
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
        EventBuilder::new(vault.id, env.block.clone(), EventData::DcaVaultCreated {}),
    )?;

    create_event(
        deps.storage,
        EventBuilder::new(
            vault.id,
            env.block.clone(),
            EventData::DcaVaultFundsDeposited {
                amount: info.funds[0].clone(),
            },
        ),
    )?;

    let response = Response::new()
        .add_attribute("method", "create_vault")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id);

    if vault.is_inactive() {
        return Ok(response);
    }

    match (target_start_time_utc_seconds, target_receive_amount) {
        (None, None) | (Some(_), None) => {
            let mut response = create_time_trigger(
                &mut deps,
                &env,
                &vault,
                target_start_time_utc_seconds,
                &response,
            )
            .expect("time trigger created");

            if target_start_time_utc_seconds.is_none() {
                response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: env.contract.address.to_string(),
                    msg: to_binary(&ExecuteMsg::ExecuteTrigger {
                        trigger_id: vault.id,
                    })
                    .unwrap(),
                    funds: vec![],
                }));
            }

            Ok(response)
        }
        (None, Some(_)) => {
            unimplemented!()
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
