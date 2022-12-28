use crate::{
    error::ContractError,
    state::{
        data_fixes::{save_data_fix, DataFixBuilder, DataFixData},
        events::event_store,
    },
    validation_helpers::assert_sender_is_admin,
};
use base::events::event::EventData;
use cosmwasm_std::{Coin, DepsMut, Env, MessageInfo, Response};

pub fn fix_event_amounts(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    event_id: u64,
    expected_sent: Coin,
    expected_received: Coin,
    expected_fee: Coin,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;

    let event = event_store().load(deps.storage, event_id)?;

    match event.data {
        EventData::DcaVaultExecutionCompleted {
            sent,
            received,
            fee,
        } => {
            if expected_sent.denom != sent.denom {
                return Err(ContractError::CustomError {
                    val: "Expected sent denom does not match event sent denom".to_string(),
                });
            }
            if expected_received.denom != received.denom {
                return Err(ContractError::CustomError {
                    val: "Expected received denom does not match event received denom".to_string(),
                });
            }
            if expected_fee.denom != fee.denom {
                return Err(ContractError::CustomError {
                    val: "Expected fee denom does not match event fee denom".to_string(),
                });
            }
            save_data_fix(
                deps.storage,
                DataFixBuilder::new(
                    event.resource_id,
                    env.block,
                    DataFixData::ExecutionCompletedEventAmounts {
                        old_sent: sent,
                        old_received: received,
                        old_fee: fee,
                        new_sent: expected_sent.clone(),
                        new_received: expected_received.clone(),
                        new_fee: expected_fee.clone(),
                    },
                ),
            )?;
        }
        _ => {
            return Err(ContractError::CustomError {
                val: "Event is not a DcaVaultExecutionCompleted".to_string(),
            })
        }
    };

    event_store().update(deps.storage, event.id, |stored_event| {
        match stored_event.clone() {
            Some(mut stored_event) => {
                stored_event.data = EventData::DcaVaultExecutionCompleted {
                    sent: expected_sent.clone(),
                    received: expected_received.clone(),
                    fee: expected_fee.clone(),
                };

                Ok(stored_event)
            }
            None => Err(ContractError::CustomError {
                val: "Event disappeared".to_string(),
            }),
        }
    })?;

    Ok(Response::new()
        .add_attribute("method", "fix_event_amounts")
        .add_attribute("event_id", event.id.to_string()))
}

#[cfg(test)]
mod tests {
    use crate::tests::helpers::instantiate_contract;

    use super::*;
    use base::events::event::EventBuilder;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Uint128,
    };

    #[test]
    fn updates_event() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let event_id = 1;
        let sent = Coin::new(100, "ukuji");
        let received = Coin::new(100, "uusk");
        let fee = Coin::new(1, "uusk");

        let event = EventBuilder::new(
            Uint128::one(),
            env.block.clone(),
            EventData::DcaVaultExecutionCompleted {
                sent,
                received,
                fee,
            },
        )
        .build(1);

        event_store()
            .save(deps.as_mut().storage, event_id, &event)
            .unwrap();

        let expected_sent = Coin::new(1000, "ukuji");
        let expected_received = Coin::new(1000, "uusk");
        let expected_fee = Coin::new(10, "uusk");

        fix_event_amounts(
            deps.as_mut(),
            env,
            info,
            event.id,
            expected_sent.clone(),
            expected_received.clone(),
            expected_fee.clone(),
        )
        .unwrap();

        let updated_event = event_store().load(deps.as_ref().storage, event_id).unwrap();

        assert_eq!(
            updated_event.data,
            EventData::DcaVaultExecutionCompleted {
                sent: expected_sent,
                received: expected_received,
                fee: expected_fee,
            }
        )
    }

    #[test]
    fn incorrect_event_type_fails() {
        let mut deps = mock_dependencies();
        let info = mock_info("admin", &[]);
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let event_id = 1;

        let event = EventBuilder::new(
            Uint128::one(),
            env.block.clone(),
            EventData::DcaVaultCreated {},
        )
        .build(1);

        event_store()
            .save(deps.as_mut().storage, event_id, &event)
            .unwrap();

        let expected_sent = Coin::new(1000, "ukuji");
        let expected_received = Coin::new(1000, "uusk");
        let expected_fee = Coin::new(10, "uusk");

        let response = fix_event_amounts(
            deps.as_mut(),
            env,
            info,
            event.id,
            expected_sent.clone(),
            expected_received.clone(),
            expected_fee.clone(),
        )
        .unwrap_err();

        assert_eq!(
            response.to_string(),
            "Error: Event is not a DcaVaultExecutionCompleted"
        )
    }
}
