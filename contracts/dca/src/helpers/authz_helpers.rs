use cosmos_sdk_proto::{cosmos::authz::v1beta1::MsgExec, Any};
use cosmwasm_std::{Addr, Binary, CosmosMsg};
use prost::Message;

pub fn create_authz_exec_message<T: Message>(grantee: Addr, type_url: String, msg: T) -> CosmosMsg {
    let mut buffer = vec![];

    MsgExec {
        grantee: grantee.to_string(),
        msgs: vec![create_protobuf_msg(type_url, msg)],
    }
    .encode(&mut buffer)
    .unwrap();

    CosmosMsg::Stargate {
        type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
        value: Binary::from(buffer),
    }
}

fn create_protobuf_msg<T: ::prost::Message>(type_url: String, msg: T) -> Any {
    let mut buffer = vec![];

    msg.encode(&mut buffer).unwrap();

    Any {
        type_url,
        value: buffer,
    }
}
