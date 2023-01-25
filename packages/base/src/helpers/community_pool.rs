use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as Protocoin;
use cosmos_sdk_proto::{cosmos::distribution::v1beta1::MsgFundCommunityPool, traits::Message};
use cosmwasm_std::{Binary, CosmosMsg, Coin};

pub fn create_fund_community_pool_msg(
    from_address: String,
    funds: Vec<Coin>,
) -> CosmosMsg {
    let amount: Vec<Protocoin> = funds
        .iter()
        .map(|coin| Protocoin {
            denom: coin.denom.clone(),
            amount: coin.amount.to_string(),
        })
        .collect();

    let mut buffer = vec![];

    MsgFundCommunityPool {
        amount,
        depositor: from_address,
    }
    .encode(&mut buffer)
    .unwrap();

    CosmosMsg::Stargate {
        type_url: "/cosmos.distribution.v1beta1.MsgFundCommunityPool".to_string(),
        value: Binary::from(buffer),
    }
}
