use crate::error::ContractError;
use crate::helpers::validation::{
    assert_address_is_valid, assert_contract_is_not_paused,
    assert_destination_allocations_add_up_to_one, assert_destination_callback_addresses_are_valid,
    assert_destinations_limit_is_not_breached, assert_exactly_one_asset,
    assert_no_destination_allocations_are_zero, assert_send_denom_is_in_pair_denoms,
    assert_swap_amount_is_greater_than_50000, assert_target_start_time_is_in_future,
    assert_time_interval_is_valid,
};
use crate::helpers::vault::get_dca_plus_model_id;
use crate::msg::ExecuteMsg;
use crate::state::cache::{VaultCache, VAULT_CACHE};
use crate::state::config::get_config;
use crate::state::events::create_event;
use crate::state::pairs::PAIRS;
use crate::state::triggers::save_trigger;
use crate::state::vaults::save_vault;
use crate::types::dca_plus_config::DcaPlusConfig;
use crate::types::destination::Destination;
use crate::types::event::{EventBuilder, EventData};
use crate::types::position_type::PositionType;
use crate::types::time_interval::TimeInterval;
use crate::types::trigger::{Trigger, TriggerConfiguration};
use crate::types::vault::VaultStatus;
use crate::types::vault_builder::VaultBuilder;
use cosmwasm_std::{coin, to_binary, Addr, CosmosMsg, Decimal, WasmMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Timestamp, Uint128, Uint64};

pub fn create_vault_handler(
    deps: DepsMut,
    env: Env,
    info: &MessageInfo,
    owner: Addr,
    label: Option<String>,
    mut destinations: Vec<Destination>,
    pair_address: Addr,
    position_type: Option<PositionType>,
    slippage_tolerance: Option<Decimal>,
    minimum_receive_amount: Option<Uint128>,
    swap_amount: Uint128,
    time_interval: TimeInterval,
    target_start_time_utc_seconds: Option<Uint64>,
    use_dca_plus: Option<bool>,
) -> Result<Response, ContractError> {
    assert_contract_is_not_paused(deps.storage)?;
    assert_address_is_valid(deps.as_ref(), owner.clone(), "owner")?;
    assert_exactly_one_asset(info.funds.clone())?;
    assert_swap_amount_is_greater_than_50000(swap_amount)?;
    assert_destinations_limit_is_not_breached(&destinations)?;
    assert_time_interval_is_valid(&time_interval)?;

    if let Some(target_time) = target_start_time_utc_seconds {
        assert_target_start_time_is_in_future(
            env.block.time,
            Timestamp::from_seconds(target_time.u64()),
        )?;
    }

    if destinations.is_empty() {
        destinations.push(Destination {
            allocation: Decimal::percent(100),
            address: owner.clone(),
            msg: None,
        });
    }

    assert_destination_callback_addresses_are_valid(deps.as_ref(), &destinations)?;
    assert_no_destination_allocations_are_zero(&destinations)?;
    assert_destination_allocations_add_up_to_one(&destinations)?;

    let pair = PAIRS.load(deps.storage, pair_address)?;

    let send_denom = info.funds[0].denom.clone();

    assert_send_denom_is_in_pair_denoms(pair.clone(), send_denom.clone())?;

    let receive_denom = if send_denom == pair.quote_denom {
        pair.base_denom.clone()
    } else {
        pair.quote_denom.clone()
    };

    let config = get_config(deps.storage)?;

    let dca_plus_config = use_dca_plus.and_then(|use_dca_plus| {
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
        status: if info.funds[0].amount <= Uint128::from(50000u128) {
            VaultStatus::Inactive
        } else {
            VaultStatus::Scheduled
        },
        pair: pair.clone(),
        swap_amount,
        position_type,
        slippage_tolerance,
        minimum_receive_amount,
        balance: info.funds[0].clone(),
        time_interval,
        started_at: None,
        swapped_amount: coin(0, info.funds[0].clone().denom),
        received_amount: coin(
            0,
            match info.funds[0].clone().denom == pair.quote_denom {
                true => pair.base_denom,
                false => pair.quote_denom,
            },
        ),
        dca_plus_config,
    };

    let vault = save_vault(deps.storage, vault_builder)?;

    VAULT_CACHE.save(deps.storage, &VaultCache { vault_id: vault.id })?;

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

    let mut response = Response::new()
        .add_attribute("method", "create_vault")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id);

    if vault.is_inactive() {
        return Ok(response);
    }

    save_trigger(
        deps.storage,
        Trigger {
            vault_id: vault.id,
            configuration: TriggerConfiguration::Time {
                target_time: match target_start_time_utc_seconds {
                    Some(time) => Timestamp::from_seconds(time.u64()),
                    None => env.block.time,
                },
            },
        },
    )?;

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

#[cfg(test)]
mod create_vault_tests {
    use super::*;
    use crate::handlers::create_pair::create_pair_handler;
    use crate::handlers::get_events_by_resource_id::get_events_by_resource_id_handler;
    use crate::handlers::get_vault::get_vault_handler;
    use crate::msg::ExecuteMsg;
    use crate::state::config::{get_config, update_config, Config};
    use crate::tests::helpers::instantiate_contract;
    use crate::tests::mocks::{
        calc_mock_dependencies, ADMIN, DENOM_STAKE, DENOM_UOSMO, USER, VALIDATOR,
    };
    use crate::types::dca_plus_config::DcaPlusConfig;
    use crate::types::destination::Destination;
    use crate::types::event::{EventBuilder, EventData};
    use crate::types::pair::Pair;
    use crate::types::time_interval::TimeInterval;
    use crate::types::trigger::TriggerConfiguration;
    use crate::types::vault::{Vault, VaultStatus};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{
        to_binary, Addr, Coin, CosmosMsg, Decimal, SubMsg, Timestamp, Uint128, WasmMsg,
    };

    #[test]
    fn with_no_assets_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(10000),
            TimeInterval::Daily,
            None,
            Some(false),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: received 0 denoms but required exactly 1"
        );
    }

    #[test]
    fn with_multiple_assets_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(
            USER,
            &[Coin::new(10000, DENOM_UOSMO), Coin::new(10000, DENOM_STAKE)],
        );

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(10000),
            TimeInterval::Daily,
            None,
            Some(false),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: received 2 denoms but required exactly 1"
        );
    }

    #[test]
    fn with_non_existent_pool_id_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            None,
            Some(false),
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "dca::types::pair::Pair not found");
    }

    #[test]
    fn with_destination_allocations_less_than_100_percent_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![Destination {
                allocation: Decimal::percent(50),
                address: Addr::unchecked(USER),
                msg: None,
            }],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            None,
            Some(false),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: destination allocations must add up to 1"
        );
    }

    #[test]
    fn with_destination_allocation_equal_to_zero_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![
                Destination {
                    allocation: Decimal::percent(100),
                    address: Addr::unchecked(USER),
                    msg: None,
                },
                Destination {
                    allocation: Decimal::percent(0),
                    address: Addr::unchecked("other"),
                    msg: None,
                },
            ],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            None,
            Some(false),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: all destination allocations must be greater than 0"
        );
    }

    #[test]
    fn with_more_than_10_destination_allocations_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            (0..20)
                .into_iter()
                .map(|i| Destination {
                    allocation: Decimal::percent(5),
                    address: Addr::unchecked(format!("destination-{}", i)),
                    msg: None,
                })
                .collect(),
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            None,
            Some(false),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: no more than 10 destinations can be provided"
        );
    }

    #[test]
    fn with_swap_amount_less_than_50000_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(10000),
            TimeInterval::Daily,
            None,
            Some(false),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: swap amount must be greater than 50000"
        );
    }

    #[test]
    fn when_contract_is_paused_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let config = get_config(deps.as_ref().storage).unwrap();

        update_config(
            deps.as_mut().storage,
            Config {
                paused: true,
                ..config
            },
        )
        .unwrap();

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            None,
            Some(false),
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "Error: contract is paused")
    }

    #[test]
    fn with_time_trigger_with_target_time_in_the_past_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.minus_seconds(10).seconds().into()),
            Some(false),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: target_start_time_utc_seconds must be some time in the future"
        );
    }

    #[test]
    fn with_invalid_custom_time_interval_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Custom { seconds: 23 },
            None,
            Some(false),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: custom time interval must be at least 60 seconds"
        );
    }

    #[test]
    fn should_create_vault() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.address.clone(),
            pair.base_denom.clone(),
            pair.quote_denom.clone(),
            pair.route.clone(),
        )
        .unwrap();

        let swap_amount = Uint128::new(100000);
        info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            swap_amount,
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            Some(false),
        )
        .unwrap();

        let vault = get_vault_handler(deps.as_ref(), Uint128::one())
            .unwrap()
            .vault;

        assert_eq!(
            vault,
            Vault {
                minimum_receive_amount: None,
                label: None,
                id: Uint128::new(1),
                owner: info.sender,
                destinations: vec![Destination::default()],
                created_at: env.block.time,
                status: VaultStatus::Scheduled,
                time_interval: TimeInterval::Daily,
                balance: info.funds[0].clone(),
                slippage_tolerance: None,
                swap_amount,
                pair,
                started_at: None,
                swapped_amount: Coin::new(0, DENOM_STAKE.to_string()),
                received_amount: Coin::new(0, DENOM_UOSMO.to_string()),
                trigger: Some(TriggerConfiguration::Time {
                    target_time: Timestamp::from_seconds(env.block.time.plus_seconds(10).seconds()),
                }),
                dca_plus_config: None,
            }
        );
    }

    #[test]
    fn should_publish_deposit_event() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.address,
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            Some(false),
        )
        .unwrap();

        let events =
            get_events_by_resource_id_handler(deps.as_ref(), Uint128::one(), None, None, None)
                .unwrap()
                .events;

        assert!(events.contains(
            &EventBuilder::new(
                Uint128::one(),
                env.block,
                EventData::DcaVaultFundsDeposited {
                    amount: info.funds[0].clone()
                },
            )
            .build(1),
        ))
    }

    #[test]
    fn for_different_owner_should_succeed() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.address,
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        let owner = Addr::unchecked(USER);
        info = mock_info(ADMIN, &[Coin::new(100000, DENOM_STAKE)]);

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            owner,
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            Some(false),
        )
        .unwrap();

        let vault = get_vault_handler(deps.as_ref(), Uint128::one())
            .unwrap()
            .vault;

        assert_eq!(vault.owner, Addr::unchecked(USER));
    }

    #[test]
    fn with_multiple_destinations_should_succeed() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.address,
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

        let destinations = vec![
            Destination {
                allocation: Decimal::percent(50),
                address: env.contract.address.clone(),
                msg: Some(
                    to_binary(&ExecuteMsg::ZDelegate {
                        delegator_address: Addr::unchecked("dest-1"),
                        validator_address: Addr::unchecked(VALIDATOR),
                    })
                    .unwrap(),
                ),
            },
            Destination {
                allocation: Decimal::percent(50),
                address: env.contract.address.clone(),
                msg: Some(
                    to_binary(&ExecuteMsg::ZDelegate {
                        delegator_address: Addr::unchecked("dest-2"),
                        validator_address: Addr::unchecked(VALIDATOR),
                    })
                    .unwrap(),
                ),
            },
        ];

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            destinations.clone(),
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            Some(false),
        )
        .unwrap();

        let vault = get_vault_handler(deps.as_ref(), Uint128::one())
            .unwrap()
            .vault;

        assert_eq!(vault.destinations, destinations);
    }

    #[test]
    fn with_insufficient_funds_should_create_inactive_vault() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.address,
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        info = mock_info(USER, &[Coin::new(1, DENOM_STAKE)]);

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            Some(false),
        )
        .unwrap();

        let vault = get_vault_handler(deps.as_ref(), Uint128::one())
            .unwrap()
            .vault;

        assert_eq!(vault.status, VaultStatus::Inactive);
    }

    #[test]
    fn with_use_dca_plus_true_should_create_dca_plus_config() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.address,
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            Some(true),
        )
        .unwrap();

        let config = get_config(deps.as_ref().storage).unwrap();
        let vault = get_vault_handler(deps.as_ref(), Uint128::one())
            .unwrap()
            .vault;

        assert_eq!(
            vault.dca_plus_config,
            Some(DcaPlusConfig {
                escrow_level: config.dca_plus_escrow_level,
                model_id: 30,
                total_deposit: info.funds[0].clone(),
                standard_dca_swapped_amount: Coin::new(0, vault.balance.denom),
                standard_dca_received_amount: Coin::new(0, DENOM_UOSMO),
                escrowed_balance: Coin::new(0, DENOM_UOSMO)
            })
        );
    }

    #[test]
    fn with_large_deposit_should_select_longer_duration_model() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.address,
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        info = mock_info(USER, &[Coin::new(1000000000, DENOM_STAKE)]);

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            Some(true),
        )
        .unwrap();

        let vault = get_vault_handler(deps.as_ref(), Uint128::one())
            .unwrap()
            .vault;

        assert_eq!(vault.dca_plus_config.unwrap().model_id, 90);
    }

    #[test]
    fn with_no_target_time_should_execute_vault() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.address,
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

        let response = create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            Addr::unchecked("pair"),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            None,
            Some(true),
        )
        .unwrap();

        assert!(response
            .messages
            .contains(&SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteTrigger {
                    trigger_id: Uint128::one()
                })
                .unwrap()
            }))));
    }
}
