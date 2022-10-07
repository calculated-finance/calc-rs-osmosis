use cosmwasm_std::{Decimal, Uint128, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use base::executions::dca_execution::DCAExecutionInformation;
use base::executions::execution::Execution;
use base::pair::Pair;
use base::triggers::time_configuration::TimeInterval;
use base::triggers::trigger::Trigger;
use base::vaults::dca_vault::{DCAConfiguration, DCAStatus, PositionType};
use base::vaults::vault::Vault;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {
    pub admin: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreatePair {
        address: String,
        base_denom: String,
        quote_denom: String,
    },
    DeletePair {
        address: String,
    },
    CreateVaultWithTimeTrigger {
        pair_address: String,
        position_type: PositionType,
        slippage_tolerance: Option<Decimal>,
        swap_amount: Uint128,
        time_interval: TimeInterval,
        target_start_time_utc_seconds: Option<Uint64>,
    },
    CreateVaultWithFINLimitOrderTrigger {
        pair_address: String,
        position_type: PositionType,
        slippage_tolerance: Option<Decimal>,
        swap_amount: Uint128,
        time_interval: TimeInterval,
        target_price: Decimal,
    },
    CancelVaultByAddressAndId {
        address: String,
        vault_id: Uint128,
    },
    ExecuteTimeTriggerById {
        trigger_id: Uint128,
    },
    ExecuteFINLimitOrderTriggerByOrderIdx {
        order_idx: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetAllPairs {},
    GetAllTimeTriggers {},
    GetAllVaults {},
    GetVaultByAddressAndId { address: String, vault_id: Uint128 },
    GetAllVaultsByAddress { address: String },
    GetAllExecutionsByVaultId { vault_id: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairsResponse {
    pub pairs: Vec<Pair>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TriggersResponse<T> {
    pub triggers: Vec<Trigger<T>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TriggerIdsResponse {
    pub trigger_ids: Vec<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct VaultResponse {
    pub vault: Vault<DCAConfiguration, DCAStatus>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct VaultsResponse {
    pub vaults: Vec<Vault<DCAConfiguration, DCAStatus>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ExecutionsResponse {
    pub executions: Vec<Execution<DCAExecutionInformation>>,
}
