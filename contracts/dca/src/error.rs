use cosmwasm_std::{CheckedMultiplyRatioError, OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Error: {val}")]
    CustomError { val: String },
}

impl From<OverflowError> for ContractError {
    fn from(from: OverflowError) -> Self {
        ContractError::Std(StdError::overflow(from))
    }
}

impl From<CheckedMultiplyRatioError> for ContractError {
    fn from(from: CheckedMultiplyRatioError) -> Self {
        ContractError::CustomError {
            val: format!("Error: {:#?}", from),
        }
    }
}
