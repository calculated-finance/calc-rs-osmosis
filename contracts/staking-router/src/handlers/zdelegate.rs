use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin;
use cosmos_sdk_proto::{
    cosmos::{authz::v1beta1::MsgExec, staking::v1beta1::MsgDelegate},
    traits::Message,
    Any,
};
use cosmwasm_std::{Addr, Binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128};

use crate::validation_helpers::assert_sender_is_allowed_z_caller;
use crate::ContractError;

pub fn zdelegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    delegator_address: Addr,
    validator_address: Addr,
    denom: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    assert_sender_is_allowed_z_caller(deps.storage, info.sender)?;

    let protobuf_delegate_msg =
        create_protobuf_delegate_msg(delegator_address, validator_address, denom, amount);
    let msg = create_exec_message(env.contract.address, protobuf_delegate_msg);
    Ok(Response::new()
        .add_attribute("method", "zdelegate")
        .add_message(msg))
}

fn create_exec_message(grantee: Addr, protobuf_msg: Any) -> CosmosMsg {
    let mut buffer = vec![];
    MsgExec {
        grantee: grantee.to_string(),
        msgs: vec![protobuf_msg],
    }
    .encode(&mut buffer)
    .unwrap();

    CosmosMsg::Stargate {
        type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
        value: Binary::from(buffer),
    }
}

fn create_protobuf_delegate_msg(
    delegator_address: Addr,
    validator_address: Addr,
    denom: String,
    amount: Uint128,
) -> Any {
    let mut buffer = vec![];
    MsgDelegate {
        delegator_address: delegator_address.to_string(),
        validator_address: validator_address.to_string(),
        amount: Some(ProtoCoin {
            denom,
            amount: amount.to_string(),
        }),
    }
    .encode(&mut buffer)
    .unwrap();

    Any {
        type_url: "/cosmos.staking.v1beta1.MsgDelegate".to_string(),
        value: buffer,
    }
}
