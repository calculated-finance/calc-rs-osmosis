use cosmwasm_std::{Coin, Uint128, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::event::{Event, EventBuilder};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DCAEventInfo {
    pub result: DCAEventResult,
    pub sent: Option<Coin>,
    pub received: Option<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DCAEventResult {
    SuccessDeposit,
    SuccessTimeTrigger,
    SuccessFINLimitOrderTrigger,
    SlippageToleranceExceeded,
    InsufficientFunds,
    Error,
}

impl EventBuilder<DCAEventInfo> {
    pub fn new() -> EventBuilder<DCAEventInfo> {
        EventBuilder {
            vault_id: Uint128::zero(),
            sequence_number: 0,
            block_height: Uint64::zero(),
            event_info: Some(DCAEventInfo {
                result: DCAEventResult::SuccessTimeTrigger,
                sent: Some(Coin {
                    denom: "".to_string(),
                    amount: Uint128::zero(),
                }),
                received: Some(Coin {
                    denom: "".to_string(),
                    amount: Uint128::zero(),
                }),
            }),
        }
    }

    pub fn vault_id(mut self, vault_id: Uint128) -> EventBuilder<DCAEventInfo> {
        self.vault_id = vault_id;
        self
    }

    pub fn sequence_id(
        mut self,
        sequence_number: u16,
    ) -> EventBuilder<DCAEventInfo> {
        self.sequence_number = sequence_number;
        self
    }

    pub fn block_height(mut self, block_height: u64) -> EventBuilder<DCAEventInfo> {
        self.block_height = Uint64::new(block_height);
        self
    }

    pub fn success_deposit(mut self, sent: Coin) -> EventBuilder<DCAEventInfo> {
        self.event_info = Some(DCAEventInfo {
            result: DCAEventResult::SuccessDeposit,
            sent: Some(sent),
            received: None,
        });
        self
    }

    pub fn success_time_trigger(
        mut self,
        sent: Coin,
        received: Coin,
    ) -> EventBuilder<DCAEventInfo> {
        self.event_info = Some(DCAEventInfo {
            result: DCAEventResult::SuccessTimeTrigger,
            sent: Some(sent),
            received: Some(received),
        });
        self
    }

    pub fn success_fin_limit_order_trigger(
        mut self,
        sent: Coin,
        received: Coin,
    ) -> EventBuilder<DCAEventInfo> {
        self.event_info = Some(DCAEventInfo {
            result: DCAEventResult::SuccessFINLimitOrderTrigger,
            sent: Some(sent),
            received: Some(received),
        });
        self
    }

    pub fn fail_slippage(mut self) -> EventBuilder<DCAEventInfo> {
        self.event_info = Some(DCAEventInfo {
            result: DCAEventResult::SlippageToleranceExceeded,
            sent: None,
            received: None,
        });
        self
    }

    pub fn fail_insufficient_funds(mut self) -> EventBuilder<DCAEventInfo> {
        self.event_info = Some(DCAEventInfo {
            result: DCAEventResult::InsufficientFunds,
            sent: None,
            received: None,
        });
        self
    }

    pub fn error(mut self) -> EventBuilder<DCAEventInfo> {
        self.event_info = Some(DCAEventInfo {
            result: DCAEventResult::Error,
            sent: None,
            received: None,
        });
        self
    }

    pub fn build(self) -> Event<DCAEventInfo> {
        Event {
            vault_id: self.vault_id,
            sequence_number: self.sequence_number,
            block_height: self.block_height,
            event_info: self.event_info,
        }
    }
}
