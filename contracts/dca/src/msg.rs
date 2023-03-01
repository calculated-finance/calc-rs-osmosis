use base::events::event::Event;
use base::pair::Pair;
use base::triggers::trigger::TimeInterval;
use base::vaults::vault::{Destination, VaultStatus};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Decimal256, Uint128, Uint64};
use fin_helpers::position_type::PositionType;

use crate::state::config::{Config, FeeCollector};
use crate::state::data_fixes::DataFix;

use crate::types::vault::Vault;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Addr,
    pub fee_collectors: Vec<FeeCollector>,
    pub swap_fee_percent: Decimal,
    pub delegation_fee_percent: Decimal,
    pub staking_router_address: Addr,
    pub page_limit: u16,
    pub paused: bool,
    pub dca_plus_escrow_level: Decimal,
}

#[cw_serde]
pub struct MigrateMsg {
    pub admin: Addr,
    pub fee_collectors: Vec<FeeCollector>,
    pub swap_fee_percent: Decimal,
    pub delegation_fee_percent: Decimal,
    pub staking_router_address: Addr,
    pub page_limit: u16,
    pub paused: bool,
    pub dca_plus_escrow_level: Decimal,
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
        label: Option<String>,
        destinations: Option<Vec<Destination>>,
        pair_address: Addr,
        position_type: Option<PositionType>,
        slippage_tolerance: Option<Decimal256>,
        minimum_receive_amount: Option<Uint128>,
        swap_amount: Uint128,
        time_interval: TimeInterval,
        target_start_time_utc_seconds: Option<Uint64>,
        target_receive_amount: Option<Uint128>,
        use_dca_plus: Option<bool>,
    },
    Deposit {
        address: Addr,
        vault_id: Uint128,
    },
    CancelVault {
        vault_id: Uint128,
    },
    ExecuteTrigger {
        trigger_id: Uint128,
    },
    UpdateConfig {
        fee_collectors: Option<Vec<FeeCollector>>,
        swap_fee_percent: Option<Decimal>,
        delegation_fee_percent: Option<Decimal>,
        staking_router_address: Option<Addr>,
        page_limit: Option<u16>,
        paused: Option<bool>,
        dca_plus_escrow_level: Option<Decimal>,
    },
    UpdateVault {
        address: Addr,
        vault_id: Uint128,
        label: Option<String>,
    },
    CreateCustomSwapFee {
        denom: String,
        swap_fee_percent: Decimal,
    },
    RemoveCustomSwapFee {
        denom: String,
    },
    UpdateSwapAdjustments {
        position_type: PositionType,
        adjustments: Vec<(u8, Decimal)>,
    },
    ClaimEscrowedFunds {
        vault_id: Uint128,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(PairsResponse)]
    GetPairs {},
    #[returns(TriggerIdsResponse)]
    GetTimeTriggerIds { limit: Option<u16> },
    #[returns(TriggerIdResponse)]
    GetTriggerIdByFinLimitOrderIdx { order_idx: Uint128 },
    #[returns(VaultResponse)]
    GetVault { vault_id: Uint128 },
    #[returns(VaultsResponse)]
    GetVaultsByAddress {
        address: Addr,
        status: Option<VaultStatus>,
        start_after: Option<u128>,
        limit: Option<u16>,
    },
    #[returns(VaultsResponse)]
    GetVaults {
        start_after: Option<u128>,
        limit: Option<u16>,
    },
    #[returns(EventsResponse)]
    GetEventsByResourceId {
        resource_id: Uint128,
        start_after: Option<u64>,
        limit: Option<u16>,
    },
    #[returns(EventsResponse)]
    GetEvents {
        start_after: Option<u64>,
        limit: Option<u16>,
    },
    #[returns(CustomFeesResponse)]
    GetCustomSwapFees {},
    #[returns(DataFixesResponse)]
    GetDataFixesByResourceId {
        resource_id: Uint128,
        start_after: Option<u64>,
        limit: Option<u16>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
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
}

#[cw_serde]
pub struct VaultsResponse {
    pub vaults: Vec<Vault>,
}

#[cw_serde]
pub struct EventsResponse {
    pub events: Vec<Event>,
}

#[cw_serde]
pub struct CustomFeesResponse {
    pub custom_fees: Vec<(String, Decimal)>,
}

#[cw_serde]
pub struct DataFixesResponse {
    pub fixes: Vec<DataFix>,
}
