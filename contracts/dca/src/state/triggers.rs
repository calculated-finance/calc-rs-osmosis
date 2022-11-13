use base::triggers::trigger::{Trigger, TriggerConfiguration};
use cosmwasm_std::{StdResult, Storage, Uint128};
use cw_storage_plus::Map;

pub const TRIGGERS: Map<u128, Trigger> = Map::new("triggers_v10");

pub const TRIGGER_ID_BY_FIN_LIMIT_ORDER_IDX: Map<u128, u128> =
    Map::new("trigger_id_by_fin_limit_order_idx_v10");

pub const TRIGGER_IDS_BY_TARGET_TIME: Map<u64, Vec<u128>> =
    Map::new("trigger_ids_by_target_time_v10");

pub fn save_trigger(store: &mut dyn Storage, trigger: Trigger) -> StdResult<Uint128> {
    TRIGGERS.save(store, trigger.vault_id.into(), &trigger)?;
    match trigger.configuration {
        TriggerConfiguration::Time { target_time } => {
            let existing_triggers_at_time =
                TRIGGER_IDS_BY_TARGET_TIME.may_load(store, target_time.seconds())?;

            match existing_triggers_at_time {
                Some(_) => {
                    let mut triggers = existing_triggers_at_time.unwrap();
                    triggers.push(trigger.vault_id.into());
                    TRIGGER_IDS_BY_TARGET_TIME.save(store, target_time.seconds(), &triggers)?;
                }
                None => {
                    let mut triggers = Vec::new();
                    triggers.push(trigger.vault_id.into());
                    TRIGGER_IDS_BY_TARGET_TIME.save(store, target_time.seconds(), &triggers)?;
                }
            }
        }
        TriggerConfiguration::FinLimitOrder { order_idx, .. } => {
            if order_idx.is_some() {
                TRIGGER_ID_BY_FIN_LIMIT_ORDER_IDX.save(
                    store,
                    order_idx.unwrap().u128(),
                    &trigger.vault_id.into(),
                )?;
            }
        }
    }
    Ok(trigger.vault_id)
}

pub fn get_trigger(store: &dyn Storage, vault_id: Uint128) -> StdResult<Option<Trigger>> {
    TRIGGERS.may_load(store, vault_id.into())
}

pub fn delete_trigger(store: &mut dyn Storage, vault_id: Uint128) -> StdResult<Uint128> {
    let trigger = TRIGGERS.load(store, vault_id.into())?;
    TRIGGERS.remove(store, trigger.vault_id.into());
    match trigger.configuration {
        TriggerConfiguration::Time { target_time } => {
            let existing_triggers_at_time =
                TRIGGER_IDS_BY_TARGET_TIME.may_load(store, target_time.seconds())?;

            if existing_triggers_at_time.is_some() {
                let mut triggers = existing_triggers_at_time.unwrap();
                triggers.retain(|&t| t != vault_id.into());
                if triggers.is_empty() {
                    TRIGGER_IDS_BY_TARGET_TIME.remove(store, target_time.seconds());
                } else {
                    TRIGGER_IDS_BY_TARGET_TIME.save(store, target_time.seconds(), &triggers)?;
                }
            }
        }
        TriggerConfiguration::FinLimitOrder { order_idx, .. } => {
            if order_idx.is_some() {
                TRIGGER_ID_BY_FIN_LIMIT_ORDER_IDX.remove(store, order_idx.unwrap().u128());
            }
        }
    }
    Ok(trigger.vault_id)
}

pub fn clear_triggers(store: &mut dyn Storage) {
    TRIGGERS.clear(store);
    TRIGGER_IDS_BY_TARGET_TIME.clear(store);
    TRIGGER_ID_BY_FIN_LIMIT_ORDER_IDX.clear(store);
}

#[cfg(test)]
mod remove_trigger_tests {
    use cosmwasm_std::{testing::MockStorage, Timestamp};

    use super::*;

    #[test]
    fn removes_timestamp_from_index_if_no_triggers_at_time() {
        let mut store = MockStorage::new();
        let target_time = Timestamp::from_seconds(100);

        let trigger = Trigger {
            vault_id: Uint128::from(1u128),
            configuration: TriggerConfiguration::Time { target_time },
        };

        save_trigger(&mut store, trigger.clone()).unwrap();

        let triggers_at_time = TRIGGER_IDS_BY_TARGET_TIME
            .may_load(&store, target_time.seconds())
            .unwrap();

        assert_eq!(triggers_at_time.unwrap().len(), 1);

        delete_trigger(&mut store, trigger.vault_id).unwrap();

        let triggers_at_time = TRIGGER_IDS_BY_TARGET_TIME
            .may_load(&store, target_time.seconds())
            .unwrap();

        assert!(triggers_at_time.is_none());
    }
}
