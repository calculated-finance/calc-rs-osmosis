use crate::types::config::Config;
use crate::types::destination::Destination;
use crate::types::event::Event;
use crate::types::fee_collector::FeeCollector;
use crate::types::pair::Pair;
use crate::types::performance_assessment_strategy::PerformanceAssessmentStrategyParams;
use crate::types::position_type::PositionType;
use crate::types::post_execution_action::LockableDuration;
use crate::types::swap_adjustment_strategy::{
    SwapAdjustmentStrategy, SwapAdjustmentStrategyParams,
};
use crate::types::time_interval::TimeInterval;
use crate::types::vault::{Vault, VaultStatus};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128, Uint64};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Addr,
    pub executors: Vec<Addr>,
    pub fee_collectors: Vec<FeeCollector>,
    pub swap_fee_percent: Decimal,
    pub delegation_fee_percent: Decimal,
    pub page_limit: u16,
    pub paused: bool,
    pub risk_weighted_average_escrow_level: Decimal,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    CreatePair {
        base_denom: String,
        quote_denom: String,
        route: Vec<u64>,
    },
    CreateVault {
        owner: Option<Addr>,
        label: Option<String>,
        destinations: Option<Vec<Destination>>,
        target_denom: String,
        position_type: Option<PositionType>,
        slippage_tolerance: Option<Decimal>,
        minimum_receive_amount: Option<Uint128>,
        swap_amount: Uint128,
        time_interval: TimeInterval,
        target_start_time_utc_seconds: Option<Uint64>,
        performance_assessment_strategy: Option<PerformanceAssessmentStrategyParams>,
        swap_adjustment_strategy: Option<SwapAdjustmentStrategyParams>,
    },
    Deposit {
        address: Addr,
        vault_id: Uint128,
    },
    UpdateVault {
        vault_id: Uint128,
        label: Option<String>,
    },
    CancelVault {
        vault_id: Uint128,
    },
    ExecuteTrigger {
        trigger_id: Uint128,
    },
    UpdateConfig {
        executors: Option<Vec<Addr>>,
        fee_collectors: Option<Vec<FeeCollector>>,
        swap_fee_percent: Option<Decimal>,
        delegation_fee_percent: Option<Decimal>,
        page_limit: Option<u16>,
        paused: Option<bool>,
        risk_weighted_average_escrow_level: Option<Decimal>,
    },
    CreateCustomSwapFee {
        denom: String,
        swap_fee_percent: Decimal,
    },
    RemoveCustomSwapFee {
        denom: String,
    },
    UpdateSwapAdjustment {
        strategy: SwapAdjustmentStrategy,
        value: Decimal,
    },
    DisburseEscrow {
        vault_id: Uint128,
    },
    ZDelegate {
        delegator_address: Addr,
        validator_address: Addr,
    },
    ZProvideLiquidity {
        provider_address: Addr,
        pool_id: u64,
        duration: LockableDuration,
        slippage_tolerance: Option<Decimal>,
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
        reverse: Option<bool>,
    },
    #[returns(EventsResponse)]
    GetEvents {
        start_after: Option<u64>,
        limit: Option<u16>,
        reverse: Option<bool>,
    },
    #[returns(CustomFeesResponse)]
    GetCustomSwapFees {},
    #[returns(VaultPerformanceResponse)]
    GetVaultPerformance { vault_id: Uint128 },
    #[returns(DisburseEscrowTasksResponse)]
    GetDisburseEscrowTasks { limit: Option<u16> },
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
pub struct VaultPerformanceResponse {
    pub fee: Coin,
    pub factor: Decimal,
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
pub struct DisburseEscrowTasksResponse {
    pub vault_ids: Vec<Uint128>,
}
