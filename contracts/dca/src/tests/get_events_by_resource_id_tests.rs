use super::{helpers::instantiate_contract, mocks::ADMIN};
use crate::{
    handlers::get_events_by_resource_id::get_events_by_resource_id,
    state::events::{create_event, create_events},
};
use base::events::event::{Event, EventBuilder, EventData};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Coin, Deps, Uint128,
};

fn assert_events_returned(
    deps: Deps,
    resource_id: Uint128,
    expected_events: Vec<Event>,
    start_after: Option<u64>,
    limit: Option<u16>,
) {
    let events_response = get_events_by_resource_id(deps, resource_id, start_after, limit).unwrap();
    assert_eq!(expected_events, events_response.events);
}

#[test]
fn with_no_events_should_return_empty_list() {
    let mut deps = mock_dependencies();
    instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &vec![]));

    assert_events_returned(deps.as_ref(), Uint128::one(), vec![], None, None);
}

#[test]
fn with_one_event_should_return_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault_id = Uint128::one();

    create_event(
        &mut deps.storage,
        EventBuilder::new(vault_id, env.block.clone(), EventData::DcaVaultCreated {}),
    )
    .unwrap();

    assert_events_returned(
        deps.as_ref(),
        vault_id,
        vec![Event {
            id: 1,
            resource_id: vault_id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::DcaVaultCreated {},
        }],
        None,
        None,
    );
}

#[test]
fn with_events_for_different_resources_should_return_one_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault_id_1 = Uint128::one();
    let vault_id_2 = Uint128::new(2);

    create_events(
        &mut deps.storage,
        vec![
            EventBuilder::new(vault_id_1, env.block.clone(), EventData::DcaVaultCreated {}),
            EventBuilder::new(vault_id_2, env.block.clone(), EventData::DcaVaultCreated {}),
        ],
    )
    .unwrap();

    assert_events_returned(
        deps.as_ref(),
        vault_id_1,
        vec![Event {
            id: 1,
            resource_id: vault_id_1,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::DcaVaultCreated {},
        }],
        None,
        None,
    );
}

#[test]
fn with_two_events_should_return_events() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault_id = Uint128::one();

    create_events(
        &mut deps.storage,
        vec![
            EventBuilder::new(vault_id, env.block.clone(), EventData::DcaVaultCreated {}),
            EventBuilder::new(
                vault_id,
                env.block.clone(),
                EventData::DcaVaultFundsDeposited {
                    amount: Coin::new(100, "ukuji".to_string()),
                },
            ),
        ],
    )
    .unwrap();

    assert_events_returned(
        deps.as_ref(),
        vault_id,
        vec![
            Event {
                id: 1,
                resource_id: vault_id,
                timestamp: env.block.time,
                block_height: env.block.height,
                data: EventData::DcaVaultCreated {},
            },
            Event {
                id: 2,
                resource_id: vault_id,
                timestamp: env.block.time,
                block_height: env.block.height,
                data: EventData::DcaVaultFundsDeposited {
                    amount: Coin::new(100, "ukuji".to_string()),
                },
            },
        ],
        None,
        None,
    );
}

#[test]
fn with_start_after_should_return_later_events() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault_id = Uint128::one();

    create_events(
        &mut deps.storage,
        vec![
            EventBuilder::new(vault_id, env.block.clone(), EventData::DcaVaultCreated {}),
            EventBuilder::new(
                vault_id,
                env.block.clone(),
                EventData::DcaVaultFundsDeposited {
                    amount: Coin::new(100, "ukuji".to_string()),
                },
            ),
        ],
    )
    .unwrap();

    assert_events_returned(
        deps.as_ref(),
        vault_id,
        vec![Event {
            id: 2,
            resource_id: vault_id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::DcaVaultFundsDeposited {
                amount: Coin::new(100, "ukuji".to_string()),
            },
        }],
        Some(1),
        None,
    );
}

#[test]
fn with_limit_should_return_limited_events() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault_id = Uint128::one();

    create_events(
        &mut deps.storage,
        vec![
            EventBuilder::new(vault_id, env.block.clone(), EventData::DcaVaultCreated {}),
            EventBuilder::new(
                vault_id,
                env.block.clone(),
                EventData::DcaVaultFundsDeposited {
                    amount: Coin::new(100, "ukuji".to_string()),
                },
            ),
        ],
    )
    .unwrap();

    assert_events_returned(
        deps.as_ref(),
        vault_id,
        vec![Event {
            id: 1,
            resource_id: vault_id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::DcaVaultCreated {},
        }],
        None,
        Some(1),
    );
}

#[test]
fn with_start_after_and_limit_should_return_limited_later_events() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault_id = Uint128::one();

    create_events(
        &mut deps.storage,
        vec![
            EventBuilder::new(vault_id, env.block.clone(), EventData::DcaVaultCreated {}),
            EventBuilder::new(
                vault_id,
                env.block.clone(),
                EventData::DcaVaultFundsDeposited {
                    amount: Coin::new(100, "ukuji".to_string()),
                },
            ),
            EventBuilder::new(vault_id, env.block.clone(), EventData::DcaVaultCancelled),
        ],
    )
    .unwrap();

    assert_events_returned(
        deps.as_ref(),
        vault_id,
        vec![Event {
            id: 2,
            resource_id: vault_id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::DcaVaultFundsDeposited {
                amount: Coin::new(100, "ukuji".to_string()),
            },
        }],
        Some(1),
        Some(1),
    );
}
