use base::vaults::vault::VaultStatus;
use cosmwasm_std::{Addr, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, UniqueIndex};

use crate::vault::{Vault, VaultBuilder};

use super::state_helpers::fetch_and_increment_counter;

const VAULT_COUNTER: Item<u64> = Item::new("vault_counter_v2");

struct VaultIndexes<'a> {
    pub owner: UniqueIndex<'a, (Addr, u128), Vault, u128>,
    pub owner_status: UniqueIndex<'a, (Addr, u8, u128), Vault, u128>,
}

impl<'a> IndexList<Vault> for VaultIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Vault>> + '_> {
        let v: Vec<&dyn Index<Vault>> = vec![&self.owner, &self.owner_status];
        Box::new(v.into_iter())
    }
}

fn vault_store<'a>() -> IndexedMap<'a, u128, Vault, VaultIndexes<'a>> {
    let indexes = VaultIndexes {
        owner: UniqueIndex::new(|v| (v.owner.clone(), v.id.into()), "vaults_v5__owner"),
        owner_status: UniqueIndex::new(
            |v| (v.owner.clone(), v.status.clone() as u8, v.id.into()),
            "vaults_v5__owner_status",
        ),
    };
    IndexedMap::new("vaults_v5", indexes)
}

pub fn save_vault(store: &mut dyn Storage, vault_builder: VaultBuilder) -> StdResult<Vault> {
    let vault = vault_builder.build(fetch_and_increment_counter(store, VAULT_COUNTER)?.into());
    vault_store().save(store, vault.id.into(), &vault)?;
    Ok(vault)
}

pub fn get_vault(store: &dyn Storage, vault_id: Uint128) -> StdResult<Vault> {
    vault_store().load(store, vault_id.into())
}

pub fn get_vaults_by_address(
    store: &dyn Storage,
    address: Addr,
    status: Option<VaultStatus>,
    start_after: Option<u128>,
    limit: Option<u16>,
) -> StdResult<Vec<Vault>> {
    let partition = match status {
        Some(status) => vault_store()
            .idx
            .owner_status
            .prefix((address, status as u8)),
        None => vault_store().idx.owner.prefix(address),
    };

    Ok(partition
        .range(
            store,
            start_after.map(|vault_id| Bound::exclusive(vault_id)),
            None,
            cosmwasm_std::Order::Ascending,
        )
        .take(limit.unwrap_or(30) as usize)
        .map(|result| result.expect("a vault stored by id").1)
        .collect::<Vec<Vault>>())
}

pub fn update_vault<T>(store: &mut dyn Storage, vault_id: Uint128, update_fn: T) -> StdResult<Vault>
where
    T: FnOnce(Option<Vault>) -> StdResult<Vault>,
{
    vault_store().update(store, vault_id.into(), update_fn)
}

pub fn clear_vaults(store: &mut dyn Storage) {
    vault_store().clear(store);
    VAULT_COUNTER.remove(store)
}
