use crate::types::trigger::{Trigger, TriggerConfiguration};
use cosmwasm_std::{Order, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, MultiIndex};
use std::marker::PhantomData;

struct TriggerIndexes<'a> {
    pub due_date: MultiIndex<'a, u64, Trigger, u128>,
}

impl<'a> IndexList<Trigger> for TriggerIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Trigger>> + '_> {
        let v: Vec<&dyn Index<Trigger>> = vec![&self.due_date];
        Box::new(v.into_iter())
    }
}

fn trigger_store<'a>() -> IndexedMap<'a, u128, Trigger, TriggerIndexes<'a>> {
    let indexes = TriggerIndexes {
        due_date: MultiIndex::new(
            |_, trigger| match trigger.configuration {
                TriggerConfiguration::Time { target_time } => target_time.seconds(),
            },
            "triggers_v8",
            "triggers_v8__due_date",
        ),
    };
    IndexedMap::new("triggers_v8", indexes)
}

pub fn save_trigger(store: &mut dyn Storage, trigger: Trigger) -> StdResult<()> {
    trigger_store().save(store, trigger.vault_id.into(), &trigger)
}

pub fn get_trigger(store: &dyn Storage, vault_id: Uint128) -> StdResult<Option<Trigger>> {
    trigger_store().may_load(store, vault_id.into())
}

pub fn delete_trigger(store: &mut dyn Storage, vault_id: Uint128) -> StdResult<()> {
    trigger_store().remove(store, vault_id.into())
}

pub fn get_time_triggers(
    store: &dyn Storage,
    due_before: Timestamp,
    limit: Option<u16>,
) -> StdResult<Vec<Uint128>> {
    Ok(trigger_store()
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
        .take(limit.unwrap_or(30) as usize)
        .flat_map(|result| result.map(|(_, trigger)| trigger.vault_id))
        .collect::<Vec<Uint128>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::trigger::TriggerConfiguration;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::Uint128;

    #[test]
    fn fetches_trigger_ids_for_triggers_that_are_due() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let trigger = Trigger {
            vault_id: Uint128::from(1u128),
            configuration: TriggerConfiguration::Time {
                target_time: env.block.time,
            },
        };

        save_trigger(&mut deps.storage, trigger.clone()).unwrap();

        let trigger_ids =
            get_time_triggers(&deps.storage, env.block.time.plus_seconds(10), Some(100)).unwrap();

        assert_eq!(trigger_ids, vec![trigger.vault_id]);
    }

    #[test]
    fn does_not_fetch_trigger_ids_for_triggers_that_are_not_due() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let trigger = Trigger {
            vault_id: Uint128::from(1u128),
            configuration: TriggerConfiguration::Time {
                target_time: env.block.time.plus_seconds(10),
            },
        };

        save_trigger(&mut deps.storage, trigger.clone()).unwrap();

        let trigger_ids = get_time_triggers(&deps.storage, env.block.time, Some(100)).unwrap();

        assert!(trigger_ids.is_empty());
    }

    #[test]
    fn stores_and_fetches_separate_tasks_at_the_same_timestamp() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let trigger_1 = Trigger {
            vault_id: Uint128::from(1u128),
            configuration: TriggerConfiguration::Time {
                target_time: env.block.time,
            },
        };
        let trigger_2 = Trigger {
            vault_id: Uint128::from(2u128),
            configuration: TriggerConfiguration::Time {
                target_time: env.block.time,
            },
        };

        save_trigger(&mut deps.storage, trigger_1.clone()).unwrap();
        save_trigger(&mut deps.storage, trigger_2.clone()).unwrap();

        let trigger_ids = get_time_triggers(&deps.storage, env.block.time, Some(100)).unwrap();

        assert_eq!(trigger_ids, vec![trigger_1.vault_id, trigger_2.vault_id]);
    }

    #[test]
    fn deletes_trigger_by_vault_id() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let trigger = Trigger {
            vault_id: Uint128::from(1u128),
            configuration: TriggerConfiguration::Time {
                target_time: env.block.time,
            },
        };

        save_trigger(&mut deps.storage, trigger.clone()).unwrap();

        let trigger_ids_before_delete =
            get_time_triggers(&deps.storage, env.block.time, Some(100)).unwrap();

        delete_trigger(&mut deps.storage, trigger.vault_id).unwrap();

        let trigger_ids_after_delete =
            get_time_triggers(&deps.storage, env.block.time, Some(100)).unwrap();

        assert_eq!(trigger_ids_before_delete, vec![trigger.vault_id]);
        assert!(trigger_ids_after_delete.is_empty());
    }

    #[test]
    fn keeps_other_tasks_when_deleting_task_by_vault_id() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let trigger_1 = Trigger {
            vault_id: Uint128::from(1u128),
            configuration: TriggerConfiguration::Time {
                target_time: env.block.time,
            },
        };
        let trigger_2 = Trigger {
            vault_id: Uint128::from(2u128),
            configuration: TriggerConfiguration::Time {
                target_time: env.block.time,
            },
        };

        save_trigger(&mut deps.storage, trigger_1.clone()).unwrap();
        save_trigger(&mut deps.storage, trigger_2.clone()).unwrap();

        let trigger_ids_before_delete =
            get_time_triggers(&deps.storage, env.block.time, Some(100)).unwrap();

        delete_trigger(&mut deps.storage, trigger_1.vault_id).unwrap();

        let trigger_ids_after_delete =
            get_time_triggers(&deps.storage, env.block.time, Some(100)).unwrap();

        assert_eq!(
            trigger_ids_before_delete,
            vec![trigger_1.vault_id, trigger_2.vault_id]
        );
        assert_eq!(trigger_ids_after_delete, vec![trigger_2.vault_id]);
    }
}
