use crate::helpers::authz_helpers::create_authz_exec_message;
use crate::{error::ContractError, helpers::validation_helpers::assert_exactly_one_asset};
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
use cosmwasm_std::{Addr, MessageInfo, Response, SubMsg};

pub fn z_delegate(
    info: MessageInfo,
    delegator_address: Addr,
    validator_address: Addr,
) -> Result<Response, ContractError> {
    assert_exactly_one_asset(info.funds.clone())?;

    let amount_to_delegate = info.funds[0].clone();

    Ok(
        Response::new().add_submessage(SubMsg::new(create_authz_exec_message(
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
        ))),
    )
}
