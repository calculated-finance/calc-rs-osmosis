use cosmwasm_schema::cw_serde;
use cosmwasm_std::{BlockInfo, Coin, Decimal, Timestamp, Uint128};

#[cw_serde]
pub enum ExecutionSkippedReason {
    SlippageToleranceExceeded,
    PriceThresholdExceeded {
        price: Decimal,
    },
    PriceDeltaLimitExceeded {
        duration_in_seconds: u64,
        max_price_delta: Decimal,
        actual_price_delta: Decimal,
    },
    UnknownFailure,
}

#[cw_serde]
pub enum EventData {
    DcaVaultCreated {},
    DcaVaultFundsDeposited {
        amount: Coin,
    },
    DcaVaultExecutionTriggered {
        base_denom: String,
        quote_denom: String,
        asset_price: Decimal,
    },
    DcaVaultExecutionCompleted {
        sent: Coin,
        received: Coin,
        fee: Coin,
    },
    DcaPlusVaultExecutionCompleted {
        sent: Coin,
        received: Coin,
        fee: Coin,
    },
    DcaVaultExecutionSkipped {
        reason: ExecutionSkippedReason,
    },
    DcaVaultCancelled {},
    DcaVaultZDelegationSucceeded {
        validator_address: String,
        delegation: Coin,
    },
    DcaVaultDelegationFailed {},
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
