use cosmos_sdk_proto::{cosmos::authz::v1beta1::MsgExec, traits::Message, Any};
use cosmwasm_std::{Addr, Binary, CosmosMsg};

pub fn create_exec_message(grantee: Addr, protobuf_msg: Any) -> CosmosMsg {
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

pub fn create_protobuf_msg<T: Message>(type_url: String, msg: T) -> Any {
    let mut buffer = vec![];
    msg.encode(&mut buffer).unwrap();

    Any {
        type_url,
        value: buffer,
    }
}
