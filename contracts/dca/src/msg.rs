use base::events::event::Event;
use base::pair::Pair;
use base::triggers::trigger::{TimeInterval, Trigger};
use base::vaults::vault::{Destination, PositionType};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal256, Uint128, Uint64};

use crate::vault::Vault;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Addr,
    pub fee_collector: Addr,
    pub fee_percent: Uint128,
    pub staking_router_address: Addr,
}

#[cw_serde]
pub struct MigrateMsg {
    pub admin: Addr,
    pub fee_collector: Addr,
    pub fee_percent: Uint128,
    pub staking_router_address: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreatePair {
        address: Addr,
        base_denom: String,
        quote_denom: String,
    },
    DeletePair {
        address: Addr,
    },
    CreateVault {
        owner: Option<Addr>,
        label: String,
        destinations: Option<Vec<Destination>>,
        pair_address: Addr,
        position_type: PositionType,
        slippage_tolerance: Option<Decimal256>,
        price_threshold: Option<Decimal256>,
        swap_amount: Uint128,
        time_interval: TimeInterval,
        target_start_time_utc_seconds: Option<Uint64>,
        target_price: Option<Decimal256>,
    },
    Deposit {
        address: Addr,
        vault_id: Uint128,
    },
    CancelVault {
        address: Addr,
        vault_id: Uint128,
    },
    ExecuteTrigger {
        trigger_id: Uint128,
    },
    UpdateConfig {
        fee_collector: Option<Addr>,
        fee_percent: Option<Uint128>,
        staking_router_address: Option<Addr>,
    },
    UpdateVault {
        address: Addr,
        vault_id: Uint128,
        label: Option<String>,
    },
}

#[cw_serde]
pub enum QueryMsg {
    GetPairs {},
    GetTimeTriggerIds {},
    GetTriggerIdByFinLimitOrderIdx {
        order_idx: Uint128,
    },
    GetVault {
        address: Addr,
        vault_id: Uint128,
    },
    GetVaultsByAddress {
        address: Addr,
        start_after: Option<u128>,
        limit: Option<u8>,
    },
    GetEventsByResourceId {
        resource_id: Uint128,
    },
    GetEvents {
        start_after: Option<u64>,
        limit: Option<u8>,
    },
}

#[cw_serde]
pub struct PairsResponse {
    pub pairs: Vec<Pair>,
}

#[cw_serde]
pub struct TriggerIdResponse {
    pub trigger_id: Uint128,
}

#[cw_serde]
pub struct TriggerIdsResponse {
    pub trigger_ids: Vec<Uint128>,
}

#[cw_serde]
pub struct VaultResponse {
    pub vault: Vault,
    pub trigger: Trigger,
}

#[cw_serde]
pub struct VaultsResponse {
    pub vaults: Vec<Vault>,
}

#[cw_serde]
pub struct EventsResponse {
    pub events: Vec<Event>,
}
