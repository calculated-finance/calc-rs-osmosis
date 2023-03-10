use cosmwasm_std::{Order, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Map, PrefixBound};
use std::marker::PhantomData;

pub const DISBURSE_ESCROW_TASKS: Map<(u64, u128), u128> =
    Map::new("disburse_escrow_task_by_timestamp_v20");

pub fn save_disburse_escrow_task(
    store: &mut dyn Storage,
    vault_id: Uint128,
    due_date: Timestamp,
) -> StdResult<()> {
    DISBURSE_ESCROW_TASKS.save(
        store,
        (due_date.seconds(), vault_id.into()),
        &vault_id.into(),
    )
}

pub fn get_disburse_escrow_tasks(
    store: &dyn Storage,
    due_before: Timestamp,
) -> StdResult<Vec<Uint128>> {
    Ok(DISBURSE_ESCROW_TASKS
        .prefix_range(
            store,
            None,
            Some(PrefixBound::Inclusive((due_before.seconds(), PhantomData))),
            Order::Ascending,
        )
        .flat_map(|result| result.map(|(_, vault_id)| vault_id.into()))
        .collect::<Vec<Uint128>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::Uint128;

    #[test]
    fn fetches_vault_ids_for_tasks_that_are_due() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let vault_id = Uint128::one();

        save_disburse_escrow_task(&mut deps.storage, vault_id, env.block.time).unwrap();

        let vault_ids =
            get_disburse_escrow_tasks(&deps.storage, env.block.time.plus_seconds(10)).unwrap();

        assert_eq!(vault_ids, vec![vault_id]);
    }

    #[test]
    fn does_not_fetch_vault_ids_for_tasks_that_are_not_due() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        save_disburse_escrow_task(
            &mut deps.storage,
            Uint128::one(),
            env.block.time.plus_seconds(10),
        )
        .unwrap();

        let vault_ids = get_disburse_escrow_tasks(&deps.storage, env.block.time).unwrap();

        assert!(vault_ids.is_empty());
    }

    #[test]
    fn stores_and_fetches_separate_tasks_at_the_same_timestamp() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let vault_id_1 = Uint128::one();
        let vault_id_2 = Uint128::new(2);

        save_disburse_escrow_task(&mut deps.storage, vault_id_1, env.block.time).unwrap();
        save_disburse_escrow_task(&mut deps.storage, vault_id_2, env.block.time).unwrap();

        let vault_ids =
            get_disburse_escrow_tasks(&deps.storage, env.block.time.plus_seconds(10)).unwrap();

        assert_eq!(vault_ids, vec![vault_id_1, vault_id_2]);
    }
}
