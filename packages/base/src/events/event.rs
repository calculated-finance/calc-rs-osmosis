use cosmwasm_schema::cw_serde;
use cosmwasm_std::{BlockInfo, Coin, Timestamp, Uint128};

#[cw_serde]
pub enum ExecutionSkippedReason {
    SlippageToleranceExceeded,
    InsufficientFunds,
    UnknownFailure,
}

#[cw_serde]
pub enum EventData {
    DCAVaultCreated,
    DCAVaultFundsDeposited {
        amount: Coin,
    },
    DCAVaultExecutionTriggered,
    DCAVaultExecutionCompleted {
        sent: Coin,
        received: Coin,
        fee: Coin,
    },
    DCAVaultExecutionSkipped {
        reason: ExecutionSkippedReason,
    },
    DCAVaultCancelled,
    DCAVaultDelegationSucceeded {
        validator_address: String,
        delegation: Coin
    },
    DCAVaultDelegationFailed,
}

#[cw_serde]
pub struct Event {
    pub id: u64,
    pub resource_id: Uint128,
    pub timestamp: Timestamp,
    pub block_height: u64,
    pub data: EventData,
}

pub struct EventBuilder {
    resource_id: Uint128,
    timestamp: Timestamp,
    block_height: u64,
    data: EventData,
}

impl EventBuilder {
    pub fn new(resource_id: Uint128, block: BlockInfo, data: EventData) -> EventBuilder {
        EventBuilder {
            resource_id,
            timestamp: block.time,
            block_height: block.height,
            data,
        }
    }

    pub fn build(self, id: u64) -> Event {
        Event {
            id,
            resource_id: self.resource_id,
            timestamp: self.timestamp,
            block_height: self.block_height,
            data: self.data,
        }
    }
}
