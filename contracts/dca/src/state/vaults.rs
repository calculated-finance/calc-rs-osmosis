use super::{pairs::PAIRS, state_helpers::fetch_and_increment_counter, triggers::get_trigger};
use crate::types::{price_delta_limit::PriceDeltaLimit, vault::Vault, vault_builder::VaultBuilder};
use base::{
    pair::Pair,
    triggers::trigger::{TimeInterval, TriggerConfiguration},
    vaults::vault::{
        Destination, DestinationDeprecated, PostExecutionAction, PostExecutionActionDeprecated,
        VaultStatus,
    },
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Coin, Decimal256, StdResult, Storage, Timestamp, Uint128,
};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, UniqueIndex};

const VAULT_COUNTER: Item<u64> = Item::new("vault_counter_v20");

#[cw_serde]
struct VaultDTO {
    pub id: Uint128,
    pub created_at: Timestamp,
    pub owner: Addr,
    pub label: Option<String>,
    pub destinations: Vec<DestinationDeprecated>,
    pub status: VaultStatus,
    pub balance: Coin,
    pub pair_address: Addr,
    pub swap_amount: Uint128,
    pub slippage_tolerance: Option<Decimal256>,
    pub minimum_receive_amount: Option<Uint128>,
    pub time_interval: TimeInterval,
    pub started_at: Option<Timestamp>,
    pub swapped_amount: Coin,
    pub received_amount: Coin,
    pub price_delta_limits: Vec<PriceDeltaLimit>,
}

impl From<Vault> for VaultDTO {
    fn from(vault: Vault) -> Self {
        Self {
            id: vault.id,
            created_at: vault.created_at,
            owner: vault.owner,
            label: vault.label,
            destinations: vec![],
            status: vault.status,
            balance: vault.balance,
            pair_address: vault.pair.address,
            swap_amount: vault.swap_amount,
            slippage_tolerance: vault.slippage_tolerance,
            minimum_receive_amount: vault.minimum_receive_amount,
            time_interval: vault.time_interval,
            started_at: vault.started_at,
            swapped_amount: vault.swapped_amount,
            received_amount: vault.received_amount,
            price_delta_limits: vec![],
        }
    }
}

fn vault_from(
    data: &VaultDTO,
    pair: Pair,
    trigger: Option<TriggerConfiguration>,
    destinations: &mut Vec<Destination>,
) -> Vault {
    destinations.append(
        &mut data
            .destinations
            .clone()
            .into_iter()
            .map(|destination| Destination {
                address: destination.address,
                allocation: destination.allocation,
                action: match destination.action {
                    PostExecutionActionDeprecated::Send => PostExecutionAction::Send,
                    PostExecutionActionDeprecated::ZDelegate => PostExecutionAction::ZDelegate,
                },
            })
            .collect(),
    );
    Vault {
        id: data.id,
        created_at: data.created_at,
        owner: data.owner.clone(),
        label: data.label.clone(),
        destinations: destinations.clone(),
        status: data.status.clone(),
        balance: data.balance.clone(),
        pair,
        swap_amount: data.swap_amount,
        slippage_tolerance: data.slippage_tolerance,
        minimum_receive_amount: data.minimum_receive_amount,
        time_interval: data.time_interval.clone(),
        started_at: data.started_at,
        swapped_amount: data.swapped_amount.clone(),
        received_amount: data.received_amount.clone(),
        trigger,
    }
}

const DESTINATIONS: Map<u128, Binary> = Map::new("destinations_v20");

fn get_destinations(store: &dyn Storage, vault_id: Uint128) -> StdResult<Vec<Destination>> {
    let destinations = DESTINATIONS.may_load(store, vault_id.into())?;
    match destinations {
        Some(destinations) => Ok(from_binary(&destinations)?),
        None => Ok(vec![]),
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
        owner: UniqueIndex::new(|v| (v.owner.clone(), v.id.into()), "vaults_v20__owner"),
        owner_status: UniqueIndex::new(
            |v| (v.owner.clone(), v.status.clone() as u8, v.id.into()),
            "vaults_v20__owner_status",
        ),
    };
    IndexedMap::new("vaults_v20", indexes)
}

pub fn save_vault(store: &mut dyn Storage, vault_builder: VaultBuilder) -> StdResult<Vault> {
    let vault = vault_builder.build(fetch_and_increment_counter(store, VAULT_COUNTER)?.into());
    DESTINATIONS.save(
        store,
        vault.id.into(),
        &to_binary(&vault.destinations).expect("serialised destinations"),
    )?;
    vault_store().save(store, vault.id.into(), &vault.clone().into())?;
    Ok(vault)
}

pub fn get_vault(store: &dyn Storage, vault_id: Uint128) -> StdResult<Vault> {
    let data = vault_store().load(store, vault_id.into())?;
    Ok(vault_from(
        &data,
        PAIRS.load(store, data.pair_address.clone())?,
        get_trigger(store, vault_id)?.map(|t| t.configuration),
        &mut get_destinations(store, vault_id)?,
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
            let (_, vault_data) =
                result.expect(format!("a vault with id after {:?}", start_after).as_str());
            vault_from(
                &vault_data,
                PAIRS.load(store, vault_data.pair_address.clone()).expect(
                    format!("a pair for pair address {:?}", vault_data.pair_address).as_str(),
                ),
                get_trigger(store, vault_data.id.into())
                    .expect(format!("a trigger for vault id {}", vault_data.id).as_str())
                    .map(|trigger| trigger.configuration),
                &mut get_destinations(store, vault_data.id).expect("vault destinations"),
            )
        })
        .collect::<Vec<Vault>>())
}

pub fn get_vaults(
    store: &dyn Storage,
    start_after: Option<u128>,
    limit: Option<u16>,
) -> StdResult<Vec<Vault>> {
    Ok(vault_store()
        .range(
            store,
            start_after.map(|vault_id| Bound::exclusive(vault_id)),
            None,
            cosmwasm_std::Order::Ascending,
        )
        .take(limit.unwrap_or(30) as usize)
        .map(|result| {
            let (_, vault_data) =
                result.expect(format!("a vault with id after {:?}", start_after).as_str());
            vault_from(
                &vault_data,
                PAIRS.load(store, vault_data.pair_address.clone()).expect(
                    format!("a pair for pair address {:?}", vault_data.pair_address).as_str(),
                ),
                get_trigger(store, vault_data.id.into())
                    .expect(format!("a trigger for vault id {}", vault_data.id).as_str())
                    .map(|trigger| trigger.configuration),
                &mut get_destinations(store, vault_data.id).expect("vault destinations"),
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
        &mut get_destinations(store, old_data.id)?,
    );
    let new_vault = update_fn(Some(old_vault.clone()))?;
    DESTINATIONS.save(
        store,
        new_vault.id.into(),
        &to_binary(&new_vault.destinations).expect("serialised destinations"),
    )?;
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
