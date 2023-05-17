use cosmwasm_std::{Order, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, MultiIndex};
use std::marker::PhantomData;

use super::config::get_config;

struct DisburseEscrowTaskIndexes<'a> {
    pub due_date: MultiIndex<'a, u64, (u64, u128), u128>,
}

impl<'a> IndexList<(u64, u128)> for DisburseEscrowTaskIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<(u64, u128)>> + '_> {
        let v: Vec<&dyn Index<(u64, u128)>> = vec![&self.due_date];
        Box::new(v.into_iter())
    }
}

fn disburse_escrow_task_store<'a>(
) -> IndexedMap<'a, u128, (u64, u128), DisburseEscrowTaskIndexes<'a>> {
    let indexes = DisburseEscrowTaskIndexes {
        due_date: MultiIndex::new(
            |_, (due_date, _)| *due_date,
            "disburse_escrow_task_v8",
            "disburse_escrow_task_v8__due_date",
        ),
    };
    IndexedMap::new("disburse_escrow_task_v8", indexes)
}

pub fn save_disburse_escrow_task(
    store: &mut dyn Storage,
    vault_id: Uint128,
    due_date: Timestamp,
) -> StdResult<()> {
    disburse_escrow_task_store().save(
        store,
        vault_id.into(),
        &(due_date.seconds(), vault_id.into()),
    )
}

pub fn get_disburse_escrow_task_due_date(
    store: &dyn Storage,
    vault_id: Uint128,
) -> StdResult<Option<Timestamp>> {
    disburse_escrow_task_store()
        .may_load(store, vault_id.into())
        .map(|result| result.map(|(seconds, _)| Timestamp::from_seconds(seconds)))
}

pub fn get_disburse_escrow_tasks(
    store: &dyn Storage,
    due_before: Timestamp,
    limit: Option<u16>,
) -> StdResult<Vec<Uint128>> {
    Ok(disburse_escrow_task_store()
        .idx
        .due_date
        .range(
            store,
            None,
            Some(Bound::Inclusive((
                (due_before.seconds(), Uint128::MAX.into()),
                PhantomData,
            ))),
            Order::Ascending,
        )
        .take(limit.unwrap_or_else(|| get_config(store).unwrap().default_page_limit) as usize)
        .flat_map(|result| result.map(|(_, (_, vault_id))| vault_id.into()))
        .collect::<Vec<Uint128>>())
}

pub fn delete_disburse_escrow_task(store: &mut dyn Storage, vault_id: Uint128) -> StdResult<()> {
    disburse_escrow_task_store().remove(store, vault_id.into())
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
            get_disburse_escrow_tasks(&deps.storage, env.block.time.plus_seconds(10), Some(100))
                .unwrap();

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

        let vault_ids =
            get_disburse_escrow_tasks(&deps.storage, env.block.time, Some(100)).unwrap();

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
            get_disburse_escrow_tasks(&deps.storage, env.block.time.plus_seconds(10), Some(100))
                .unwrap();

        assert_eq!(vault_ids, vec![vault_id_1, vault_id_2]);
    }

    #[test]
    fn deletes_task_by_vault_id() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let vault_id = Uint128::one();

        save_disburse_escrow_task(&mut deps.storage, vault_id, env.block.time).unwrap();

        let vault_ids_before_delete =
            get_disburse_escrow_tasks(&deps.storage, env.block.time.plus_seconds(10), Some(100))
                .unwrap();

        delete_disburse_escrow_task(&mut deps.storage, vault_id).unwrap();

        let vault_ids_after_delete =
            get_disburse_escrow_tasks(&deps.storage, env.block.time.plus_seconds(10), Some(100))
                .unwrap();

        assert_eq!(vault_ids_before_delete, vec![vault_id]);
        assert!(vault_ids_after_delete.is_empty());
    }

    #[test]
    fn keeps_other_tasks_when_deleting_task_by_vault_id() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let vault_id_1 = Uint128::one();
        let vault_id_2 = Uint128::new(2);

        save_disburse_escrow_task(&mut deps.storage, vault_id_1, env.block.time).unwrap();
        save_disburse_escrow_task(&mut deps.storage, vault_id_2, env.block.time).unwrap();

        let vault_ids_before_delete =
            get_disburse_escrow_tasks(&deps.storage, env.block.time.plus_seconds(10), Some(100))
                .unwrap();

        delete_disburse_escrow_task(&mut deps.storage, vault_id_1).unwrap();

        let vault_ids_after_delete =
            get_disburse_escrow_tasks(&deps.storage, env.block.time.plus_seconds(10), Some(100))
                .unwrap();

        assert_eq!(vault_ids_before_delete, vec![vault_id_1, vault_id_2]);
        assert_eq!(vault_ids_after_delete, vec![vault_id_2]);
    }
}
