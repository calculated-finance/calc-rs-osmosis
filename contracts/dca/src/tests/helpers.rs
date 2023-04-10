use super::mocks::{ADMIN, DENOM_STAKE, DENOM_UOSMO, USER};
use crate::{
    constants::{ONE, TEN},
    contract::instantiate,
    handlers::get_vault::get_vault,
    msg::InstantiateMsg,
    state::{
        cache::{Cache, CACHE},
        config::{Config, FeeCollector},
        pairs::PAIRS,
        triggers::save_trigger,
        vaults::update_vault,
    },
    types::{
        dca_plus_config::DcaPlusConfig,
        destination::Destination,
        pair::Pair,
        post_execution_action::PostExecutionAction,
        time_interval::TimeInterval,
        trigger::{Trigger, TriggerConfiguration},
        vault::{Vault, VaultStatus},
    },
};
use cosmwasm_std::{Addr, Coin, Decimal, DepsMut, Env, MessageInfo, Timestamp, Uint128};
use std::{cmp::max, str::FromStr};

pub fn instantiate_contract(deps: DepsMut, env: Env, info: MessageInfo) {
    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        fee_collectors: vec![FeeCollector {
            address: ADMIN.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.0165").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked("staking-router"),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.0075").unwrap(),
    };

    instantiate(deps, env.clone(), info.clone(), instantiate_message).unwrap();
}

pub fn instantiate_contract_with_multiple_fee_collectors(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    fee_collectors: Vec<FeeCollector>,
) {
    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        fee_collectors,
        swap_fee_percent: Decimal::from_str("0.0165").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(ADMIN),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.0075").unwrap(),
    };

    instantiate(deps, env.clone(), info.clone(), instantiate_message).unwrap();
}

impl Default for Config {
    fn default() -> Self {
        Self {
            admin: Addr::unchecked(ADMIN),
            fee_collectors: vec![FeeCollector {
                address: ADMIN.to_string(),
                allocation: Decimal::from_str("1").unwrap(),
            }],
            swap_fee_percent: Decimal::from_str("0.0165").unwrap(),
            delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
            staking_router_address: Addr::unchecked(ADMIN),
            page_limit: 1000,
            paused: false,
            dca_plus_escrow_level: Decimal::from_str("0.0075").unwrap(),
        }
    }
}

impl Default for Pair {
    fn default() -> Self {
        Self {
            address: Addr::unchecked("pair"),
            base_denom: DENOM_UOSMO.to_string(),
            quote_denom: DENOM_STAKE.to_string(),
            route: vec![3],
        }
    }
}

impl Default for Vault {
    fn default() -> Self {
        Self {
            id: Uint128::zero(),
            created_at: Timestamp::default(),
            owner: Addr::unchecked(USER),
            label: Some("vault".to_string()),
            destinations: vec![Destination {
                address: Addr::unchecked(USER),
                allocation: Decimal::percent(100),
                action: PostExecutionAction::ZDelegate,
            }],
            status: VaultStatus::Active,
            balance: Coin::new(TEN.into(), DENOM_UOSMO),
            pair: Pair::default(),
            swap_amount: ONE,
            slippage_tolerance: None,
            minimum_receive_amount: None,
            time_interval: TimeInterval::Daily,
            started_at: None,
            swapped_amount: Coin::new(0, DENOM_UOSMO),
            received_amount: Coin::new(0, DENOM_STAKE),
            trigger: Some(TriggerConfiguration::Time {
                target_time: Timestamp::from_seconds(0),
            }),
            dca_plus_config: None,
        }
    }
}

impl Default for DcaPlusConfig {
    fn default() -> Self {
        Self {
            escrow_level: Decimal::percent(10),
            model_id: 30,
            total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
            standard_dca_swapped_amount: Coin::new(0, DENOM_UOSMO),
            standard_dca_received_amount: Coin::new(0, DENOM_STAKE),
            escrowed_balance: Coin::new(0, DENOM_STAKE),
        }
    }
}

pub fn setup_new_vault(deps: DepsMut, env: Env, mut vault: Vault) -> Vault {
    PAIRS
        .save(deps.storage, vault.pair.address.clone(), &vault.pair)
        .unwrap();

    let mut existing_vault = get_vault(deps.as_ref(), vault.id);

    while existing_vault.is_ok() {
        vault.id = existing_vault.unwrap().vault.id + Uint128::one();
        existing_vault = get_vault(deps.as_ref(), vault.id);
    }

    update_vault(deps.storage, &vault).unwrap();

    if let Some(TriggerConfiguration::Time { target_time }) = vault.trigger {
        let trigger_time =
            Timestamp::from_seconds(max(target_time.seconds(), env.block.time.seconds()));

        save_trigger(
            deps.storage,
            Trigger {
                vault_id: vault.id,
                configuration: TriggerConfiguration::Time {
                    target_time: trigger_time,
                },
            },
        )
        .unwrap();
    }

    CACHE
        .save(
            deps.storage,
            &Cache {
                vault_id: vault.id,
                owner: vault.owner,
            },
        )
        .unwrap();

    get_vault(deps.as_ref(), vault.id).unwrap().vault
}
