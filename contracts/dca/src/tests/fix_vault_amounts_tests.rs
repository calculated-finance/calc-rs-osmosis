use base::{
    events::event::{EventBuilder, EventData},
    helpers::math_helpers::checked_mul,
    vaults::vault::PostExecutionAction,
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    BankMsg, Coin, SubMsg, Uint128,
};

use crate::{
    constants::{ONE, TEN},
    contract::AFTER_BANK_SWAP_REPLY_ID,
    handlers::fix_vault_amounts::fix_vault_amounts,
    state::{
        cache::{SwapCache, SWAP_CACHE},
        config::get_config,
        events::create_events,
        vaults::{get_vault, update_vault},
    },
    tests::{
        helpers::{instantiate_contract, setup_active_vault_with_funds},
        mocks::ADMIN,
    },
};

#[test]
fn should_adjust_swapped_amount_stat_upwards() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let receive_amount = Uint128::new(234312312);
    let updated_swapped_amount = Uint128::new(11000);
    let updated_receive_amount = Uint128::new(11000);

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(receive_amount.into(), vault.get_receive_denom())],
    );

    fix_vault_amounts(
        deps.as_mut(),
        env.clone(),
        mock_info(ADMIN, &vec![]),
        vault.id,
        Coin::new(updated_swapped_amount.into(), vault.get_swap_denom()),
        Coin::new(updated_receive_amount.into(), vault.get_receive_denom()),
    )
    .unwrap();

    let updated_vault = get_vault(&deps.storage, vault.id).unwrap();
    let config = get_config(&deps.storage).unwrap();

    let mut fee = config.swap_fee_percent * updated_receive_amount;

    vault
        .destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::ZDelegate)
        .for_each(|destination| {
            let allocation_amount =
                checked_mul(updated_receive_amount - fee, destination.allocation).unwrap();
            let allocation_automation_fee =
                checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
            fee = fee.checked_add(allocation_automation_fee).unwrap();
        });

    assert_eq!(updated_vault.swapped_amount.amount, updated_swapped_amount);
}

#[test]
fn should_adjust_swapped_amount_stat_downwards() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    update_vault(
        deps.as_mut().storage,
        vault.id,
        |stored_vault| match stored_vault {
            Some(mut stored_vault) => {
                stored_vault.swapped_amount = Coin::new(TEN.into(), vault.get_swap_denom());
                Ok(stored_vault)
            }
            None => panic!("Vault not found"),
        },
    )
    .unwrap();

    let receive_amount = Uint128::new(234312312);
    let updated_swapped_amount = TEN - ONE;
    let updated_receive_amount = Uint128::new(11000);

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(receive_amount.into(), vault.get_receive_denom())],
    );

    fix_vault_amounts(
        deps.as_mut(),
        env.clone(),
        mock_info(ADMIN, &vec![]),
        vault.id,
        Coin::new(updated_swapped_amount.into(), vault.get_swap_denom()),
        Coin::new(updated_receive_amount.into(), vault.get_receive_denom()),
    )
    .unwrap();

    let updated_vault = get_vault(&deps.storage, vault.id).unwrap();
    assert_eq!(updated_vault.swapped_amount.amount, updated_swapped_amount);
}

#[test]
fn should_adjust_received_amount_stat_upwards() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let existing_swapped_amount = Uint128::new(1000000);
    let existing_receive_amount = Uint128::new(1000000);
    let existing_fee_amount = Uint128::new(1000);
    let updated_swapped_amount = Uint128::new(11000000);
    let updated_receive_amount = Uint128::new(11000000);

    update_vault(
        deps.as_mut().storage,
        vault.id,
        |stored_vault| match stored_vault.clone() {
            Some(mut vault) => {
                vault.swapped_amount.amount = existing_swapped_amount;
                vault.received_amount.amount = existing_receive_amount;
                Ok(vault)
            }
            None => panic!("Vault not found"),
        },
    )
    .unwrap();

    create_events(
        deps.as_mut().storage,
        vec![EventBuilder::new(
            vault.id,
            env.block.clone(),
            EventData::DcaVaultExecutionCompleted {
                sent: Coin::new(existing_swapped_amount.into(), vault.get_swap_denom()),
                received: Coin::new(existing_receive_amount.into(), vault.get_receive_denom()),
                fee: Coin::new(existing_fee_amount.into(), vault.get_receive_denom()),
            },
        )],
    )
    .unwrap();

    fix_vault_amounts(
        deps.as_mut(),
        env.clone(),
        mock_info(ADMIN, &vec![]),
        vault.id,
        Coin::new(updated_swapped_amount.into(), vault.get_swap_denom()),
        Coin::new(updated_receive_amount.into(), vault.get_receive_denom()),
    )
    .unwrap();

    let updated_vault = get_vault(&deps.storage, vault.id).unwrap();
    let config = get_config(&deps.storage).unwrap();

    let amount_to_disburse_before_fees =
        updated_receive_amount - (existing_receive_amount + existing_fee_amount);
    let mut fee = config.swap_fee_percent * amount_to_disburse_before_fees;

    vault
        .destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::ZDelegate)
        .for_each(|destination| {
            let allocation_amount =
                checked_mul(amount_to_disburse_before_fees - fee, destination.allocation).unwrap();
            let allocation_automation_fee =
                checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
            fee = fee.checked_add(allocation_automation_fee).unwrap();
        });

    fee += existing_fee_amount;

    assert_eq!(
        updated_vault.received_amount.amount,
        updated_receive_amount - fee
    );
}

#[test]
fn should_adjust_received_amount_stat_downwards() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let existing_swapped_amount = Uint128::new(1000000);
    let existing_receive_amount = TEN + ONE;
    let existing_fee_amount = Uint128::new(1000);
    let updated_swapped_amount = Uint128::new(11000000);
    let updated_receive_amount = TEN;

    update_vault(
        deps.as_mut().storage,
        vault.id,
        |stored_vault| match stored_vault.clone() {
            Some(mut vault) => {
                vault.swapped_amount.amount = existing_swapped_amount;
                vault.received_amount.amount = existing_receive_amount;
                Ok(vault)
            }
            None => panic!("Vault not found"),
        },
    )
    .unwrap();

    create_events(
        deps.as_mut().storage,
        vec![EventBuilder::new(
            vault.id,
            env.block.clone(),
            EventData::DcaVaultExecutionCompleted {
                sent: Coin::new(existing_swapped_amount.into(), vault.get_swap_denom()),
                received: Coin::new(existing_receive_amount.into(), vault.get_receive_denom()),
                fee: Coin::new(existing_fee_amount.into(), vault.get_receive_denom()),
            },
        )],
    )
    .unwrap();

    fix_vault_amounts(
        deps.as_mut(),
        env.clone(),
        mock_info(ADMIN, &vec![]),
        vault.id,
        Coin::new(updated_swapped_amount.into(), vault.get_swap_denom()),
        Coin::new(updated_receive_amount.into(), vault.get_receive_denom()),
    )
    .unwrap();

    let updated_vault = get_vault(&deps.storage, vault.id).unwrap();
    assert_eq!(
        updated_vault.received_amount.amount,
        updated_receive_amount - existing_fee_amount
    );
}

#[test]
fn should_disburse_funds() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let existing_swapped_amount = Uint128::new(1000000);
    let existing_receive_amount = Uint128::new(1000000);
    let existing_fee_amount = Uint128::new(1000);
    let updated_swapped_amount = Uint128::new(11000000);
    let updated_receive_amount = Uint128::new(11000000);

    update_vault(
        deps.as_mut().storage,
        vault.id,
        |stored_vault| match stored_vault.clone() {
            Some(mut vault) => {
                vault.swapped_amount.amount = existing_swapped_amount;
                vault.received_amount.amount = existing_receive_amount;
                Ok(vault)
            }
            None => panic!("Vault not found"),
        },
    )
    .unwrap();

    create_events(
        deps.as_mut().storage,
        vec![EventBuilder::new(
            vault.id,
            env.block.clone(),
            EventData::DcaVaultExecutionCompleted {
                sent: Coin::new(existing_swapped_amount.into(), vault.get_swap_denom()),
                received: Coin::new(existing_receive_amount.into(), vault.get_receive_denom()),
                fee: Coin::new(existing_fee_amount.into(), vault.get_receive_denom()),
            },
        )],
    )
    .unwrap();

    let response = fix_vault_amounts(
        deps.as_mut(),
        env.clone(),
        mock_info(ADMIN, &vec![]),
        vault.id,
        Coin::new(updated_swapped_amount.into(), vault.get_swap_denom()),
        Coin::new(updated_receive_amount.into(), vault.get_receive_denom()),
    )
    .unwrap();

    let config = get_config(&deps.storage).unwrap();

    let amount_to_disburse_before_fees =
        updated_receive_amount - (existing_receive_amount + existing_fee_amount);
    let mut fee = config.swap_fee_percent * amount_to_disburse_before_fees;

    vault
        .destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::ZDelegate)
        .for_each(|destination| {
            let allocation_amount =
                checked_mul(amount_to_disburse_before_fees - fee, destination.allocation).unwrap();
            let allocation_automation_fee =
                checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
            fee = fee.checked_add(allocation_automation_fee).unwrap();
        });

    let amount_to_disburse = amount_to_disburse_before_fees - fee;

    assert!(response.messages.contains(&SubMsg::reply_on_success(
        BankMsg::Send {
            to_address: vault.destinations.first().unwrap().address.to_string(),
            amount: vec![Coin::new(
                amount_to_disburse.into(),
                vault.get_receive_denom(),
            )],
        },
        AFTER_BANK_SWAP_REPLY_ID,
    )));
}

#[test]
fn returns_fee_to_fee_collector() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));
    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let existing_swapped_amount = Uint128::new(1000000);
    let existing_receive_amount = Uint128::new(1000000);
    let existing_fee_amount = Uint128::new(1000);
    let updated_swapped_amount = Uint128::new(11000000);
    let updated_receive_amount = Uint128::new(11000000);

    update_vault(
        deps.as_mut().storage,
        vault.id,
        |stored_vault| match stored_vault.clone() {
            Some(mut vault) => {
                vault.swapped_amount.amount = existing_swapped_amount;
                vault.received_amount.amount = existing_receive_amount;
                Ok(vault)
            }
            None => panic!("Vault not found"),
        },
    )
    .unwrap();

    create_events(
        deps.as_mut().storage,
        vec![EventBuilder::new(
            vault.id,
            env.block.clone(),
            EventData::DcaVaultExecutionCompleted {
                sent: Coin::new(existing_swapped_amount.into(), vault.get_swap_denom()),
                received: Coin::new(existing_receive_amount.into(), vault.get_receive_denom()),
                fee: Coin::new(existing_fee_amount.into(), vault.get_receive_denom()),
            },
        )],
    )
    .unwrap();

    let response = fix_vault_amounts(
        deps.as_mut(),
        env.clone(),
        mock_info(ADMIN, &vec![]),
        vault.id,
        Coin::new(updated_swapped_amount.into(), vault.get_swap_denom()),
        Coin::new(updated_receive_amount.into(), vault.get_receive_denom()),
    )
    .unwrap();

    let config = get_config(&deps.storage).unwrap();

    let amount_to_disburse_before_fees =
        updated_receive_amount - (existing_receive_amount + existing_fee_amount);
    let swap_fee = config.swap_fee_percent * amount_to_disburse_before_fees;

    let automation_fee_rate = config
        .delegation_fee_percent
        .checked_mul(
            vault
                .destinations
                .iter()
                .filter(|destination| destination.action == PostExecutionAction::ZDelegate)
                .map(|destination| destination.allocation)
                .sum(),
        )
        .unwrap();

    let automation_fee = (amount_to_disburse_before_fees - swap_fee) * automation_fee_rate;

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collector.to_string(),
        amount: vec![Coin::new(swap_fee.into(), vault.get_receive_denom())]
    })));

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collector.to_string(),
        amount: vec![Coin::new(automation_fee.into(), vault.get_receive_denom())]
    })));
}

#[test]
fn with_correct_received_amount_should_do_nothing() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));
    let mut vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let existing_swapped_amount = Uint128::new(1000000);
    let existing_receive_amount = Uint128::new(1000000);
    let existing_fee_amount = Uint128::new(1000);
    let updated_swapped_amount = Uint128::new(1000000);
    let updated_receive_amount = Uint128::new(1001000);

    vault = update_vault(
        deps.as_mut().storage,
        vault.id,
        |stored_vault| match stored_vault.clone() {
            Some(mut vault) => {
                vault.swapped_amount.amount = existing_swapped_amount;
                vault.received_amount.amount = existing_receive_amount;
                Ok(vault)
            }
            None => panic!("Vault not found"),
        },
    )
    .unwrap();

    create_events(
        deps.as_mut().storage,
        vec![EventBuilder::new(
            vault.id,
            env.block.clone(),
            EventData::DcaVaultExecutionCompleted {
                sent: Coin::new(existing_swapped_amount.into(), vault.get_swap_denom()),
                received: Coin::new(existing_receive_amount.into(), vault.get_receive_denom()),
                fee: Coin::new(existing_fee_amount.into(), vault.get_receive_denom()),
            },
        )],
    )
    .unwrap();

    assert_eq!(
        vault.received_amount.amount,
        updated_receive_amount - existing_fee_amount
    );

    let response = fix_vault_amounts(
        deps.as_mut(),
        env.clone(),
        mock_info(ADMIN, &vec![]),
        vault.id,
        Coin::new(updated_swapped_amount.into(), vault.get_swap_denom()),
        Coin::new(updated_receive_amount.into(), vault.get_receive_denom()),
    )
    .unwrap();

    assert!(response.messages.is_empty());
}
