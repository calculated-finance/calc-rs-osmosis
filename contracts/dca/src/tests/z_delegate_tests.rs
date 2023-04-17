use crate::{
    contract::AFTER_DELEGATION_REPLY_ID,
    handlers::z_delegate::z_delegate_handler,
    helpers::authz_helpers::create_authz_exec_message,
    tests::mocks::{DENOM_STAKE, DENOM_UOSMO, USER, VALIDATOR},
};
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin;
use cosmos_sdk_proto::cosmos::staking::v1beta1::MsgDelegate;
use cosmwasm_std::{
    testing::{mock_dependencies, mock_info},
    Addr, BankMsg, Coin, SubMsg,
};

#[test]
fn with_no_asset_fails() {
    let info = mock_info(USER, &[]);

    let response = z_delegate_handler(
        mock_dependencies().as_ref(),
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
        info.clone(),
        delegator_address.clone(),
        validator_address.clone(),
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::reply_always(
        create_authz_exec_message(
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
        ),
        AFTER_DELEGATION_REPLY_ID
    )));
}
