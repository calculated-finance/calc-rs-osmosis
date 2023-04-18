use cosmos_sdk_proto::{cosmos::authz::v1beta1::MsgExec, Any};
use cosmwasm_std::{Addr, Binary, CosmosMsg};
use prost::Message;

pub fn create_authz_exec_message<T: Message>(grantee: Addr, type_url: String, msg: T) -> CosmosMsg {
    CosmosMsg::Stargate {
        type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
        value: Binary::from(
            MsgExec {
                grantee: grantee.to_string(),
                msgs: vec![Any {
                    type_url,
                    value: msg.encode_to_vec(),
                }],
            }
            .encode_to_vec(),
        ),
    }
}
