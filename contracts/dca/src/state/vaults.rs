use super::{pairs::PAIRS, state_helpers::fetch_and_increment_counter, triggers::get_trigger};
use crate::types::{
    dca_plus_config::DCAPlusConfig, price_delta_limit::PriceDeltaLimit, vault::Vault,
    vault_builder::VaultBuilder,
};
use base::{
    pair::Pair,
    triggers::trigger::{TimeInterval, TriggerConfiguration},
    vaults::vault::{Destination, DestinationDeprecated, VaultStatus},
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
    dca_plus_config: Option<DCAPlusConfig>,
) -> Vault {
    destinations.append(
        &mut data
            .destinations
            .clone()
            .into_iter()
            .map(|destination| destination.into())
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
        dca_plus_config,
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

const DCA_PLUS_CONFIGS: Map<u128, DCAPlusConfig> = Map::new("dca_plus_configs_v20");

fn get_dca_plus_config(store: &dyn Storage, vault_id: Uint128) -> Option<DCAPlusConfig> {
    DCA_PLUS_CONFIGS
        .may_load(store, vault_id.into())
        .unwrap_or(None)
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
    if let Some(dca_plus_config) = vault.dca_plus_config.clone() {
        DCA_PLUS_CONFIGS.save(store, vault.id.into(), &dca_plus_config)?;
    }
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
        get_dca_plus_config(store, vault_id),
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
                get_dca_plus_config(store, vault_data.id),
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
                get_dca_plus_config(store, vault_data.id),
            )
        })
        .collect::<Vec<Vault>>())
}

pub fn update_vault(store: &mut dyn Storage, vault: &Vault) -> StdResult<()> {
    DESTINATIONS.save(
        store,
        vault.id.into(),
        &to_binary(&vault.destinations).expect("serialised destinations"),
    )?;
    if let Some(dca_plus_config) = &vault.dca_plus_config {
        DCA_PLUS_CONFIGS.save(store, vault.id.into(), dca_plus_config)?;
    }
    vault_store().save(store, vault.id.into(), &vault.clone().into())
}

pub fn clear_vaults(store: &mut dyn Storage) {
    vault_store().clear(store);
    VAULT_COUNTER.remove(store)
}

#[cfg(test)]
mod destination_store_tests {
    use super::*;
    use crate::types::vault_builder::VaultBuilder;
    use base::{
        pair::Pair,
        triggers::trigger::TimeInterval,
        vaults::vault::{Destination, PostExecutionAction, VaultStatus},
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        Addr, Coin, Decimal, Env, Uint128,
    };

    fn create_vault_builder(env: Env) -> VaultBuilder {
        VaultBuilder::new(
            env.block.time,
            Addr::unchecked("owner"),
            None,
            vec![Destination {
                address: Addr::unchecked("owner"),
                allocation: Decimal::one(),
                action: PostExecutionAction::Send,
            }],
            VaultStatus::Active,
            Coin::new(1000u128, "ukuji".to_string()),
            Pair {
                address: Addr::unchecked("pair"),
                base_denom: "demo".to_string(),
                quote_denom: "ukuji".to_string(),
            },
            Uint128::new(100),
            None,
            None,
            None,
            TimeInterval::Daily,
            None,
            Coin {
                denom: "demo".to_string(),
                amount: Uint128::zero(),
            },
            Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::zero(),
            },
            None,
        )
    }

    #[test]
    fn saving_new_vault_stores_destinations() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let store = deps.as_mut().storage;

        let vault_builder = create_vault_builder(env);
        let vault = save_vault(store, vault_builder).unwrap();

        let destinations: Vec<Destination> =
            from_binary(&DESTINATIONS.load(store, vault.id.into()).unwrap()).unwrap();
        assert_eq!(destinations, vault.destinations);
        assert!(!destinations.is_empty());
    }

    #[test]
    fn fetching_new_vault_returns_destinations() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let store = deps.as_mut().storage;

        let pair = Pair {
            address: Addr::unchecked("pair"),
            base_denom: "demo".to_string(),
            quote_denom: "ukuji".to_string(),
        };

        PAIRS
            .save(store, pair.address.clone(), &pair.clone())
            .unwrap();

        let vault_builder = create_vault_builder(env);
        let vault = save_vault(store, vault_builder).unwrap();
        let fetched_vault = get_vault(store, vault.id).unwrap();

        assert_eq!(fetched_vault.destinations, vault.destinations);
        assert!(!fetched_vault.destinations.is_empty());
    }

    #[test]
    fn fetching_new_vault_after_update_returns_destinations() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let store = deps.as_mut().storage;

        let pair = Pair {
            address: Addr::unchecked("pair"),
            base_denom: "demo".to_string(),
            quote_denom: "ukuji".to_string(),
        };

        PAIRS
            .save(store, pair.address.clone(), &pair.clone())
            .unwrap();

        let vault_builder = create_vault_builder(env);
        let mut vault = save_vault(store, vault_builder).unwrap();

        vault.status = VaultStatus::Inactive;
        update_vault(store, &vault).unwrap();

        let fetched_vault = get_vault(store, vault.id).unwrap();
        assert_eq!(fetched_vault.destinations, vault.destinations);
        assert!(!fetched_vault.destinations.is_empty());
    }

    #[test]
    fn fetching_old_vault_returns_destinations() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let store = deps.as_mut().storage;

        let pair = Pair {
            address: Addr::unchecked("pair"),
            base_denom: "demo".to_string(),
            quote_denom: "ukuji".to_string(),
        };

        PAIRS
            .save(store, pair.address.clone(), &pair.clone())
            .unwrap();

        let vault = create_vault_builder(env).build(Uint128::one());

        let mut vault_dto: VaultDTO = vault.clone().into();
        vault_dto.destinations = vault
            .clone()
            .destinations
            .clone()
            .into_iter()
            .map(|d| d.into())
            .collect();

        vault_store()
            .save(store, vault.id.into(), &vault_dto)
            .unwrap();

        let fetched_vault = get_vault(store, vault.id).unwrap();
        assert_eq!(fetched_vault.destinations, vault.destinations);
        assert!(!fetched_vault.destinations.is_empty());
    }

    #[test]
    fn updating_old_vault_stores_destinations() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let store = deps.as_mut().storage;

        let pair = Pair {
            address: Addr::unchecked("pair"),
            base_denom: "demo".to_string(),
            quote_denom: "ukuji".to_string(),
        };

        PAIRS
            .save(store, pair.address.clone(), &pair.clone())
            .unwrap();

        let mut vault = create_vault_builder(env).build(Uint128::one());

        let mut vault_dto: VaultDTO = vault.clone().into();
        vault_dto.destinations = vault
            .clone()
            .destinations
            .clone()
            .into_iter()
            .map(|d| d.into())
            .collect();

        vault_store()
            .save(store, vault.id.into(), &vault_dto)
            .unwrap();

        vault.status = VaultStatus::Inactive;
        update_vault(store, &vault).unwrap();

        let destinations: Vec<Destination> =
            from_binary(&DESTINATIONS.load(store, vault.id.into()).unwrap()).unwrap();
        assert_eq!(destinations, vault.destinations);
        assert!(!destinations.is_empty());
    }

    #[test]
    fn fetching_old_vault_after_update_returns_destinations() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let store = deps.as_mut().storage;

        let pair = Pair {
            address: Addr::unchecked("pair"),
            base_denom: "demo".to_string(),
            quote_denom: "ukuji".to_string(),
        };

        PAIRS
            .save(store, pair.address.clone(), &pair.clone())
            .unwrap();

        let mut vault = create_vault_builder(env).build(Uint128::one());

        let mut vault_dto: VaultDTO = vault.clone().into();
        vault_dto.destinations = vault
            .clone()
            .destinations
            .clone()
            .into_iter()
            .map(|d| d.into())
            .collect();

        vault_store()
            .save(store, vault.id.into(), &vault_dto)
            .unwrap();

        vault.status = VaultStatus::Inactive;
        update_vault(store, &vault).unwrap();

        let fetched_vault = get_vault(store, vault.id).unwrap();
        assert_eq!(fetched_vault.destinations, vault.destinations);
        assert!(!fetched_vault.destinations.is_empty());
    }
}
