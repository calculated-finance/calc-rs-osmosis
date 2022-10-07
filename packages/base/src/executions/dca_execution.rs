use cosmwasm_std::{Coin, Uint128, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::execution::{Execution, ExecutionBuilder};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DCAExecutionInformation {
    pub result: DCAExecutionResult,
    pub sent: Option<Coin>,
    pub received: Option<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DCAExecutionResult {
    SuccessTimeTrigger,
    SuccessFINLimitOrderTrigger,
    SlippageToleranceExceeded,
    InsufficientFunds,
    Error,
}

impl ExecutionBuilder<DCAExecutionInformation> {
    pub fn new() -> ExecutionBuilder<DCAExecutionInformation> {
        ExecutionBuilder {
            vault_id: Uint128::zero(),
            sequence_number: 0,
            block_height: Uint64::zero(),
            execution_information: Some(DCAExecutionInformation {
                result: DCAExecutionResult::SuccessTimeTrigger,
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

    pub fn vault_id(mut self, vault_id: Uint128) -> ExecutionBuilder<DCAExecutionInformation> {
        self.vault_id = vault_id;
        self
    }

    pub fn sequence_id(
        mut self,
        sequence_number: u16,
    ) -> ExecutionBuilder<DCAExecutionInformation> {
        self.sequence_number = sequence_number;
        self
    }

    pub fn block_height(mut self, block_height: u64) -> ExecutionBuilder<DCAExecutionInformation> {
        self.block_height = Uint64::new(block_height);
        self
    }

    pub fn success_time_trigger(
        mut self,
        sent: Coin,
        received: Coin,
    ) -> ExecutionBuilder<DCAExecutionInformation> {
        self.execution_information = Some(DCAExecutionInformation {
            result: DCAExecutionResult::SuccessTimeTrigger,
            sent: Some(sent),
            received: Some(received),
        });
        self
    }

    pub fn success_fin_limit_order_trigger(
        mut self,
        sent: Coin,
        received: Coin,
    ) -> ExecutionBuilder<DCAExecutionInformation> {
        self.execution_information = Some(DCAExecutionInformation {
            result: DCAExecutionResult::SuccessFINLimitOrderTrigger,
            sent: Some(sent),
            received: Some(received),
        });
        self
    }

    pub fn fail_slippage(mut self) -> ExecutionBuilder<DCAExecutionInformation> {
        self.execution_information = Some(DCAExecutionInformation {
            result: DCAExecutionResult::SlippageToleranceExceeded,
            sent: None,
            received: None,
        });
        self
    }

    pub fn fail_insufficient_funds(mut self) -> ExecutionBuilder<DCAExecutionInformation> {
        self.execution_information = Some(DCAExecutionInformation {
            result: DCAExecutionResult::InsufficientFunds,
            sent: None,
            received: None,
        });
        self
    }

    pub fn error(mut self) -> ExecutionBuilder<DCAExecutionInformation> {
        self.execution_information = Some(DCAExecutionInformation {
            result: DCAExecutionResult::Error,
            sent: None,
            received: None,
        });
        self
    }

    pub fn build(self) -> Execution<DCAExecutionInformation> {
        Execution {
            vault_id: self.vault_id,
            sequence_number: self.sequence_number,
            block_height: self.block_height,
            execution_information: self.execution_information,
        }
    }
}
