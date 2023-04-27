use crate::constants::AFTER_DELEGATION_REPLY_ID;
use crate::helpers::authz::create_authz_exec_message;
use crate::helpers::validation::{
    assert_address_is_valid, assert_denom_is_bond_denom, assert_validator_is_valid,
};
use crate::{error::ContractError, helpers::validation::assert_exactly_one_asset};
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
use cosmwasm_std::{Addr, BankMsg, Deps, Env, MessageInfo, Reply, Response, SubMsg, SubMsgResult};

pub fn z_delegate_handler(
    deps: Deps,
    env: Env,
    info: MessageInfo,
    delegator_address: Addr,
    validator_address: Addr,
) -> Result<Response, ContractError> {
    assert_exactly_one_asset(info.funds.clone())?;
    assert_address_is_valid(deps, delegator_address.clone(), "delegator address")?;
    assert_validator_is_valid(deps, validator_address.to_string())?;

    let amount_to_delegate = info.funds[0].clone();

    assert_denom_is_bond_denom(amount_to_delegate.denom.clone())?;

    Ok(Response::new()
        .add_attributes(vec![
            ("delegation", amount_to_delegate.to_string()),
            ("delegator", delegator_address.to_string()),
            ("validator", validator_address.to_string()),
        ])
        .add_submessages(vec![
            SubMsg::new(BankMsg::Send {
                to_address: delegator_address.to_string(),
                amount: vec![amount_to_delegate.clone()],
            }),
            SubMsg::reply_always(
                create_authz_exec_message(
                    env.contract.address,
                    String::from("/cosmos.staking.v1beta1.MsgDelegate"),
                    MsgDelegate {
                        delegator_address: delegator_address.to_string(),
                        validator_address: validator_address.to_string(),
                        amount: Some(ProtoCoin {
                            denom: amount_to_delegate.denom,
                            amount: amount_to_delegate.amount.to_string(),
                        }),
                    },
                ),
                AFTER_DELEGATION_REPLY_ID,
            ),
        ]))
}

pub fn log_delegation_result(reply: Reply) -> Result<Response, ContractError> {
    let result = match reply.result {
        SubMsgResult::Ok(_) => "success".to_string(),
        SubMsgResult::Err(_) => "failure".to_string(),
    };

    Ok(Response::new().add_attribute("delegate_result", result))
}

#[cfg(test)]
mod z_delegate_tests {
    use super::*;
    use crate::{
        helpers::authz::create_authz_exec_message,
        tests::mocks::{DENOM_STAKE, DENOM_UOSMO, USER, VALIDATOR},
    };
    use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin;
    use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, Attribute, BankMsg, Coin, SubMsg, SubMsgResponse,
    };

    #[test]
    fn with_no_asset_fails() {
        let info = mock_info(USER, &[]);

        let response = z_delegate_handler(
            mock_dependencies().as_ref(),
            mock_env(),
            info.clone(),
            Addr::unchecked(USER),
            Addr::unchecked(VALIDATOR),
        )
        .unwrap_err();

        assert_eq!(
            response.to_string(),
            "Error: received 0 denoms but required exactly 1",
        );
    }

    #[test]
    fn with_more_than_one_asset_fails() {
        let info = mock_info(
            USER,
            &[Coin::new(100, DENOM_STAKE), Coin::new(100, DENOM_UOSMO)],
        );

        let response = z_delegate_handler(
            mock_dependencies().as_ref(),
            mock_env(),
            info.clone(),
            Addr::unchecked(USER),
            Addr::unchecked(VALIDATOR),
        )
        .unwrap_err();

        assert_eq!(
            response.to_string(),
            "Error: received 2 denoms but required exactly 1",
        );
    }

    #[test]
    fn sends_bank_message() {
        let amount_to_delegate = Coin::new(100, DENOM_UOSMO);
        let info = mock_info(USER, &[amount_to_delegate.clone()]);

        let delegator_address = Addr::unchecked(info.sender.clone());

        let response = z_delegate_handler(
            mock_dependencies().as_ref(),
            mock_env(),
            info.clone(),
            delegator_address.clone(),
            Addr::unchecked(VALIDATOR),
        )
        .unwrap();

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: delegator_address.to_string(),
            amount: vec![amount_to_delegate.clone()],
        })));
    }

    #[test]
    fn sends_delegate_message() {
        let amount_to_delegate = Coin::new(100, DENOM_UOSMO);
        let info = mock_info(USER, &[amount_to_delegate.clone()]);

        let delegator_address = Addr::unchecked(info.sender.clone());
        let validator_address = Addr::unchecked(VALIDATOR);

        let response = z_delegate_handler(
            mock_dependencies().as_ref(),
            mock_env(),
            info.clone(),
            delegator_address.clone(),
            validator_address.clone(),
        )
        .unwrap();

        assert!(response.messages.contains(&SubMsg::reply_always(
            create_authz_exec_message(
                mock_env().contract.address,
                String::from("/cosmos.staking.v1beta1.MsgDelegate"),
                MsgDelegate {
                    delegator_address: delegator_address.to_string(),
                    validator_address: validator_address.to_string(),
                    amount: Some(ProtoCoin {
                        denom: amount_to_delegate.denom,
                        amount: amount_to_delegate.amount.to_string(),
                    }),
                },
            ),
            AFTER_DELEGATION_REPLY_ID
        )));
    }

    #[test]
    fn logs_the_delegate_result_on_success() {
        let response = log_delegation_result(Reply {
            id: AFTER_DELEGATION_REPLY_ID,
            result: cosmwasm_std::SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        })
        .unwrap();

        assert!(response
            .attributes
            .contains(&Attribute::new("delegate_result", "success")));
    }

    #[test]
    fn logs_the_delegate_result_on_failure() {
        let response = log_delegation_result(Reply {
            id: AFTER_DELEGATION_REPLY_ID,
            result: cosmwasm_std::SubMsgResult::Err("error code 4".to_string()),
        })
        .unwrap();

        assert!(response
            .attributes
            .contains(&Attribute::new("delegate_result", "failure")));
    }
}
