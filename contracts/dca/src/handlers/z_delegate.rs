use crate::helpers::authz_helpers::create_authz_exec_message;
use crate::{error::ContractError, helpers::validation_helpers::assert_exactly_one_asset};
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
use cosmwasm_std::{Addr, MessageInfo, Reply, Response, SubMsg, SubMsgResult};

pub fn z_delegate_handler(
    info: MessageInfo,
    delegator_address: Addr,
    validator_address: Addr,
) -> Result<Response, ContractError> {
    assert_exactly_one_asset(info.funds.clone())?;

    let amount_to_delegate = info.funds[0].clone();

    Ok(Response::new()
        .add_attributes(vec![
            ("delegate_amount", amount_to_delegate.to_string()),
            ("delegate_owner", delegator_address.to_string()),
            ("delegate_validator", validator_address.to_string()),
        ])
        .add_submessage(SubMsg::new(create_authz_exec_message(
            delegator_address.clone(),
            String::from("/cosmos.staking.v1beta1.MsgDelegate"),
            MsgDelegate {
                delegator_address: delegator_address.to_string(),
                validator_address: validator_address.to_string(),
                amount: Some(ProtoCoin {
                    denom: amount_to_delegate.denom,
                    amount: amount_to_delegate.amount.to_string(),
                }),
            },
        ))))
}

pub fn log_delegation_result(reply: Reply) -> Result<Response, ContractError> {
    let result = match reply.result {
        SubMsgResult::Ok(_) => "success".to_string(),
        SubMsgResult::Err(_) => "failure".to_string(),
    };

    Ok(Response::new().add_attribute("delegate_result", result))
}
