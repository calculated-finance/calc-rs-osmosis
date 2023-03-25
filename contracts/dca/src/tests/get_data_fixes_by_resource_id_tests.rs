use super::{
    helpers::instantiate_contract,
    mocks::{ADMIN, DENOM_UKUJI, DENOM_UTEST},
};
use crate::{
    handlers::get_data_fixes_by_resource_id::get_data_fixes_by_resource_id,
    state::data_fixes::{save_data_fix, save_data_fixes, DataFix, DataFixBuilder, DataFixData},
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Coin, Deps, Uint128,
};

fn assert_data_fixes_returned(
    deps: Deps,
    resource_id: Uint128,
    expected_data_fixes: Vec<DataFix>,
    start_after: Option<u64>,
    limit: Option<u16>,
) {
    let data_fixes_response =
        get_data_fixes_by_resource_id(deps, resource_id, start_after, limit).unwrap();
    assert_eq!(expected_data_fixes, data_fixes_response.fixes);
}

#[test]
fn with_no_data_fixes_should_return_empty_list() {
    let mut deps = mock_dependencies();
    instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &vec![]));

    assert_data_fixes_returned(deps.as_ref(), Uint128::one(), vec![], None, None);
}

#[test]
fn with_one_event_should_return_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault_id = Uint128::one();

    save_data_fix(
        &mut deps.storage,
        DataFixBuilder::new(
            vault_id,
            env.block.clone(),
            DataFixData::VaultAmounts {
                old_swapped: Coin::new(0, DENOM_UKUJI),
                old_received: Coin::new(0, DENOM_UTEST),
                new_swapped: Coin::new(0, DENOM_UKUJI),
                new_received: Coin::new(0, DENOM_UTEST),
            },
        ),
    )
    .unwrap();

    assert_data_fixes_returned(
        deps.as_ref(),
        vault_id,
        vec![DataFix {
            id: 1,
            resource_id: vault_id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: DataFixData::VaultAmounts {
                old_swapped: Coin::new(0, DENOM_UKUJI),
                old_received: Coin::new(0, DENOM_UTEST),
                new_swapped: Coin::new(0, DENOM_UKUJI),
                new_received: Coin::new(0, DENOM_UTEST),
            },
        }],
        None,
        None,
    );
}

#[test]
fn with_data_fixes_for_different_resources_should_return_one_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault_id_1 = Uint128::one();
    let vault_id_2 = Uint128::new(2);

    save_data_fixes(
        &mut deps.storage,
        vec![
            DataFixBuilder::new(
                vault_id_1,
                env.block.clone(),
                DataFixData::VaultAmounts {
                    old_swapped: Coin::new(0, DENOM_UKUJI),
                    old_received: Coin::new(0, DENOM_UTEST),
                    new_swapped: Coin::new(0, DENOM_UKUJI),
                    new_received: Coin::new(0, DENOM_UTEST),
                },
            ),
            DataFixBuilder::new(
                vault_id_2,
                env.block.clone(),
                DataFixData::VaultAmounts {
                    old_swapped: Coin::new(0, DENOM_UKUJI),
                    old_received: Coin::new(0, DENOM_UTEST),
                    new_swapped: Coin::new(0, DENOM_UKUJI),
                    new_received: Coin::new(0, DENOM_UTEST),
                },
            ),
        ],
    )
    .unwrap();

    assert_data_fixes_returned(
        deps.as_ref(),
        vault_id_1,
        vec![DataFix {
            id: 1,
            resource_id: vault_id_1,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: DataFixData::VaultAmounts {
                old_swapped: Coin::new(0, DENOM_UKUJI),
                old_received: Coin::new(0, DENOM_UTEST),
                new_swapped: Coin::new(0, DENOM_UKUJI),
                new_received: Coin::new(0, DENOM_UTEST),
            },
        }],
        None,
        None,
    );
}

#[test]
fn with_two_data_fixes_should_return_data_fixes() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault_id = Uint128::one();

    save_data_fixes(
        &mut deps.storage,
        vec![
            DataFixBuilder::new(
                vault_id,
                env.block.clone(),
                DataFixData::VaultAmounts {
                    old_swapped: Coin::new(0, DENOM_UKUJI),
                    old_received: Coin::new(0, DENOM_UTEST),
                    new_swapped: Coin::new(0, DENOM_UKUJI),
                    new_received: Coin::new(0, DENOM_UTEST),
                },
            ),
            DataFixBuilder::new(
                vault_id,
                env.block.clone(),
                DataFixData::VaultAmounts {
                    old_swapped: Coin::new(0, DENOM_UKUJI),
                    old_received: Coin::new(0, DENOM_UTEST),
                    new_swapped: Coin::new(0, DENOM_UKUJI),
                    new_received: Coin::new(0, DENOM_UTEST),
                },
            ),
        ],
    )
    .unwrap();

    assert_data_fixes_returned(
        deps.as_ref(),
        vault_id,
        vec![
            DataFix {
                id: 1,
                resource_id: vault_id,
                timestamp: env.block.time,
                block_height: env.block.height,
                data: DataFixData::VaultAmounts {
                    old_swapped: Coin::new(0, DENOM_UKUJI),
                    old_received: Coin::new(0, DENOM_UTEST),
                    new_swapped: Coin::new(0, DENOM_UKUJI),
                    new_received: Coin::new(0, DENOM_UTEST),
                },
            },
            DataFix {
                id: 2,
                resource_id: vault_id,
                timestamp: env.block.time,
                block_height: env.block.height,
                data: DataFixData::VaultAmounts {
                    old_swapped: Coin::new(0, DENOM_UKUJI),
                    old_received: Coin::new(0, DENOM_UTEST),
                    new_swapped: Coin::new(0, DENOM_UKUJI),
                    new_received: Coin::new(0, DENOM_UTEST),
                },
            },
        ],
        None,
        None,
    );
}

#[test]
fn with_start_after_should_return_later_data_fixes() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault_id = Uint128::one();

    save_data_fixes(
        &mut deps.storage,
        vec![
            DataFixBuilder::new(
                vault_id,
                env.block.clone(),
                DataFixData::VaultAmounts {
                    old_swapped: Coin::new(0, DENOM_UKUJI),
                    old_received: Coin::new(0, DENOM_UTEST),
                    new_swapped: Coin::new(0, DENOM_UKUJI),
                    new_received: Coin::new(0, DENOM_UTEST),
                },
            ),
            DataFixBuilder::new(
                vault_id,
                env.block.clone(),
                DataFixData::VaultAmounts {
                    old_swapped: Coin::new(0, DENOM_UKUJI),
                    old_received: Coin::new(0, DENOM_UTEST),
                    new_swapped: Coin::new(0, DENOM_UKUJI),
                    new_received: Coin::new(0, DENOM_UTEST),
                },
            ),
        ],
    )
    .unwrap();

    assert_data_fixes_returned(
        deps.as_ref(),
        vault_id,
        vec![DataFix {
            id: 2,
            resource_id: vault_id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: DataFixData::VaultAmounts {
                old_swapped: Coin::new(0, DENOM_UKUJI),
                old_received: Coin::new(0, DENOM_UTEST),
                new_swapped: Coin::new(0, DENOM_UKUJI),
                new_received: Coin::new(0, DENOM_UTEST),
            },
        }],
        Some(1),
        None,
    );
}

#[test]
fn with_limit_should_return_limited_data_fixes() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault_id = Uint128::one();

    save_data_fixes(
        &mut deps.storage,
        vec![
            DataFixBuilder::new(
                vault_id,
                env.block.clone(),
                DataFixData::VaultAmounts {
                    old_swapped: Coin::new(0, DENOM_UKUJI),
                    old_received: Coin::new(0, DENOM_UTEST),
                    new_swapped: Coin::new(0, DENOM_UKUJI),
                    new_received: Coin::new(0, DENOM_UTEST),
                },
            ),
            DataFixBuilder::new(
                vault_id,
                env.block.clone(),
                DataFixData::VaultAmounts {
                    old_swapped: Coin::new(0, DENOM_UKUJI),
                    old_received: Coin::new(0, DENOM_UTEST),
                    new_swapped: Coin::new(0, DENOM_UKUJI),
                    new_received: Coin::new(0, DENOM_UTEST),
                },
            ),
        ],
    )
    .unwrap();

    assert_data_fixes_returned(
        deps.as_ref(),
        vault_id,
        vec![DataFix {
            id: 1,
            resource_id: vault_id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: DataFixData::VaultAmounts {
                old_swapped: Coin::new(0, DENOM_UKUJI),
                old_received: Coin::new(0, DENOM_UTEST),
                new_swapped: Coin::new(0, DENOM_UKUJI),
                new_received: Coin::new(0, DENOM_UTEST),
            },
        }],
        None,
        Some(1),
    );
}

#[test]
fn with_start_after_and_limit_should_return_limited_later_data_fixes() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault_id = Uint128::one();

    save_data_fixes(
        &mut deps.storage,
        vec![
            DataFixBuilder::new(
                vault_id,
                env.block.clone(),
                DataFixData::VaultAmounts {
                    old_swapped: Coin::new(0, DENOM_UKUJI),
                    old_received: Coin::new(0, DENOM_UTEST),
                    new_swapped: Coin::new(0, DENOM_UKUJI),
                    new_received: Coin::new(0, DENOM_UTEST),
                },
            ),
            DataFixBuilder::new(
                vault_id,
                env.block.clone(),
                DataFixData::VaultAmounts {
                    old_swapped: Coin::new(0, DENOM_UKUJI),
                    old_received: Coin::new(0, DENOM_UTEST),
                    new_swapped: Coin::new(0, DENOM_UKUJI),
                    new_received: Coin::new(0, DENOM_UTEST),
                },
            ),
            DataFixBuilder::new(
                vault_id,
                env.block.clone(),
                DataFixData::VaultAmounts {
                    old_swapped: Coin::new(0, DENOM_UKUJI),
                    old_received: Coin::new(0, DENOM_UTEST),
                    new_swapped: Coin::new(0, DENOM_UKUJI),
                    new_received: Coin::new(0, DENOM_UTEST),
                },
            ),
        ],
    )
    .unwrap();

    assert_data_fixes_returned(
        deps.as_ref(),
        vault_id,
        vec![DataFix {
            id: 2,
            resource_id: vault_id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: DataFixData::VaultAmounts {
                old_swapped: Coin::new(0, DENOM_UKUJI),
                old_received: Coin::new(0, DENOM_UTEST),
                new_swapped: Coin::new(0, DENOM_UKUJI),
                new_received: Coin::new(0, DENOM_UTEST),
            },
        }],
        Some(1),
        Some(1),
    );
}
