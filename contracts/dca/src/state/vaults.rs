use base::{
    pair::Pair,
    triggers::trigger::{TimeInterval, TriggerConfiguration},
    vaults::vault::{Destination, PositionType, VaultStatus},
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal256, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, UniqueIndex};

use crate::vault::{Vault, VaultBuilder};

use super::{pairs::PAIRS, state_helpers::fetch_and_increment_counter, triggers::get_trigger};

const VAULT_COUNTER: Item<u64> = Item::new("vault_counter_v7");

#[cw_serde]
struct VaultDTO {
    pub id: Uint128,
    pub created_at: Timestamp,
    pub owner: Addr,
    pub label: Option<String>,
    pub destinations: Vec<Destination>,
    pub status: VaultStatus,
    pub balance: Coin,
    pub pair_address: Addr,
    pub swap_amount: Uint128,
    pub position_type: Option<PositionType>,
    pub slippage_tolerance: Option<Decimal256>,
    pub price_threshold: Option<Decimal256>,
    pub time_interval: TimeInterval,
    pub started_at: Option<Timestamp>,
    pub swapped_amount: Coin,
    pub received_amount: Coin,
}

impl From<Vault> for VaultDTO {
    fn from(vault: Vault) -> Self {
        Self {
            id: vault.id,
            created_at: vault.created_at,
            owner: vault.owner,
            label: vault.label,
            destinations: vault.destinations,
            status: vault.status,
            balance: vault.balance,
            pair_address: vault.pair.address,
            swap_amount: vault.swap_amount,
            position_type: vault.position_type,
            slippage_tolerance: vault.slippage_tolerance,
            price_threshold: vault.price_threshold,
            time_interval: vault.time_interval,
            started_at: vault.started_at,
            swapped_amount: vault.swapped_amount,
            received_amount: vault.received_amount,
        }
    }
}

fn vault_from(data: &VaultDTO, pair: Pair, trigger: Option<TriggerConfiguration>) -> Vault {
    Vault {
        id: data.id,
        created_at: data.created_at,
        owner: data.owner.clone(),
        label: data.label.clone(),
        destinations: data.destinations.clone(),
        status: data.status.clone(),
        balance: data.balance.clone(),
        pair,
        swap_amount: data.swap_amount,
        position_type: data.position_type.clone(),
        slippage_tolerance: data.slippage_tolerance,
        price_threshold: data.price_threshold,
        time_interval: data.time_interval.clone(),
        started_at: data.started_at,
        swapped_amount: data.swapped_amount.clone(),
        received_amount: data.received_amount.clone(),
        trigger,
    }
}

struct VaultIndexes<'a> {
    pub owner: UniqueIndex<'a, (Addr, u128), VaultDTO, u128>,
    pub owner_status: UniqueIndex<'a, (Addr, u8, u128), VaultDTO, u128>,
}

impl<'a> IndexList<VaultDTO> for VaultIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<VaultDTO>> + '_> {
        let v: Vec<&dyn Index<VaultDTO>> = vec![&self.owner, &self.owner_status];
        Box::new(v.into_iter())
    }
}

fn vault_store<'a>() -> IndexedMap<'a, u128, VaultDTO, VaultIndexes<'a>> {
    let indexes = VaultIndexes {
        owner: UniqueIndex::new(|v| (v.owner.clone(), v.id.into()), "vaults_v7__owner"),
        owner_status: UniqueIndex::new(
            |v| (v.owner.clone(), v.status.clone() as u8, v.id.into()),
            "vaults_v7__owner_status",
        ),
    };
    IndexedMap::new("vaults_v7", indexes)
}

pub fn save_vault(store: &mut dyn Storage, vault_builder: VaultBuilder) -> StdResult<Vault> {
    let vault = vault_builder.build(fetch_and_increment_counter(store, VAULT_COUNTER)?.into());
    vault_store().save(store, vault.id.into(), &vault.clone().into())?;
    Ok(vault)
}

pub fn get_vault(store: &dyn Storage, vault_id: Uint128) -> StdResult<Vault> {
    let data = vault_store().load(store, vault_id.into())?;
    Ok(vault_from(
        &data,
        PAIRS.load(store, data.pair_address.clone())?,
        get_trigger(store, vault_id)?.map(|t| t.configuration),
    ))
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
        .map(|result| {
            let (_, data) =
                result.expect(format!("a vault with id after {:?}", start_after).as_str());
            vault_from(
                &data,
                PAIRS
                    .load(store, data.pair_address.clone())
                    .expect(format!("a pair for pair address {:?}", data.pair_address).as_str()),
                get_trigger(store, data.id.into())
                    .expect(format!("a trigger for vault id {:?}", data.id).as_str())
                    .map(|t| t.configuration),
            )
        })
        .collect::<Vec<Vault>>())
}

pub fn update_vault<T>(store: &mut dyn Storage, vault_id: Uint128, update_fn: T) -> StdResult<Vault>
where
    T: FnOnce(Option<Vault>) -> StdResult<Vault>,
{
    let old_data = vault_store().load(store, vault_id.into())?;
    let old_vault = vault_from(
        &old_data,
        PAIRS.load(store, old_data.pair_address.clone())?,
        None,
    );
    let new_vault = update_fn(Some(old_vault.clone()))?;
    vault_store().replace(
        store,
        vault_id.into(),
        Some(&new_vault.clone().into()),
        Some(&old_data),
    )?;
    Ok(new_vault)
}

pub fn clear_vaults(store: &mut dyn Storage) {
    vault_store().clear(store);
    VAULT_COUNTER.remove(store)
}
