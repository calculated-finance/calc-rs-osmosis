use super::{dca_plus_config::DcaPlusConfig, vault::Vault};
use base::{
    pair::Pair,
    triggers::trigger::TimeInterval,
    vaults::vault::{Destination, VaultStatus},
};
use cosmwasm_std::{Addr, Coin, Decimal256, Timestamp, Uint128};
use fin_helpers::position_type::PositionType;

pub struct VaultBuilder {
    pub created_at: Timestamp,
    pub owner: Addr,
    pub label: Option<String>,
    pub destinations: Vec<Destination>,
    pub status: VaultStatus,
    pub balance: Coin,
    pub pair: Pair,
    pub swap_amount: Uint128,
    pub position_type: Option<PositionType>,
    pub slippage_tolerance: Option<Decimal256>,
    pub minimum_receive_amount: Option<Uint128>,
    pub time_interval: TimeInterval,
    pub started_at: Option<Timestamp>,
    pub swapped_amount: Coin,
    pub received_amount: Coin,
    pub dca_plus_config: Option<DcaPlusConfig>,
}

impl VaultBuilder {
    pub fn new(
        created_at: Timestamp,
        owner: Addr,
        label: Option<String>,
        destinations: Vec<Destination>,
        status: VaultStatus,
        balance: Coin,
        pair: Pair,
        swap_amount: Uint128,
        position_type: Option<PositionType>,
        slippage_tolerance: Option<Decimal256>,
        minimum_receive_amount: Option<Uint128>,
        time_interval: TimeInterval,
        started_at: Option<Timestamp>,
        swapped_amount: Coin,
        received_amount: Coin,
        dca_plus_config: Option<DcaPlusConfig>,
    ) -> VaultBuilder {
        VaultBuilder {
            created_at,
            owner,
            label,
            destinations,
            status,
            balance,
            pair,
            swap_amount,
            position_type,
            slippage_tolerance,
            minimum_receive_amount,
            time_interval,
            started_at,
            swapped_amount,
            received_amount,
            dca_plus_config,
        }
    }

    pub fn build(self, id: Uint128) -> Vault {
        Vault {
            id,
            created_at: self.created_at,
            owner: self.owner,
            label: self.label,
            destinations: self.destinations,
            status: self.status,
            balance: self.balance.clone(),
            pair: self.pair.clone(),
            swap_amount: self.swap_amount,
            slippage_tolerance: self.slippage_tolerance,
            minimum_receive_amount: self.minimum_receive_amount,
            time_interval: self.time_interval,
            started_at: self.started_at,
            swapped_amount: self.swapped_amount,
            received_amount: self.received_amount,
            trigger: None,
            dca_plus_config: self.dca_plus_config,
        }
    }
}
