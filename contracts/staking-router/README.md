# Staking Router

## Overview

The staking router uses the cosmos sdk authz module to create delegation messages on behalf of a user. Operations relating to the creation of these messages are prepending with the letter "Z" to denote the use of the authZ module.

This contract can only be called by addresses known as `allowed_z_callers` which are saved to this contracts storage and referenced at run time.

By only allowing `allowed_z_delegators` to call this contract, we can control when these delegation messages are created. It is the intent that this contract will be called by other CALC contracts upon the completion of some action.

## ZCallers & ZDelegate

### Z Delegate

A delegation message will be sent on a users behalf by supplying the users `Addr`, a validators `Addr`, the name of the denomination to delegate and the amount of that denomination to delegate

Z delegate assumes that the user for which the delegate message is being created for has granted authz permission to this contract to send delegate messages on their behalf. If this is not true, the message simply fails.

#### Validation

- no validation is needed for the delegator address because invalid addresses will fail
- no validation is needed for the validator address because invalid addresses will fail 

#### Domain Logic

- create a delegate message where `delegator_address`, `validator_address`, `denomn` and `amount` are all passed in from the ZDelegate call
- wrap the delegate message in an `exec` message so that it will be executed using the authz module & supply this contracts address given as the `grantee`

#### Assertions

- function can only be called by an allowed z caller stored in `config`
- delegator addresses which have not given permission to this contract to create delegate messages on their behalf will result in a failed z delegation call

### Add Allowed Z Caller

Allowed z callers are manually added by supplying an `Addr`

#### Validation

- no validation is needed because invalid address can not make calls to this contract

#### Domain Logic

- if the `allowed_z_caller` already exists in `config` do nothing
- if the `allowed_z_caller` does not exist in the `config` add it to the list

#### Assertions

- function can only be called by the admin storage in `config`

### Remove Allowed Z Caller

Allowed z callers are manually removed by supplying an `Addr`

#### Validation

- no validation is needed because invalid address can not make calls to this contract

#### Domain Logic

- if the `allowed_z_caller` exists in `config` remove it
- if the `allowed_z_caller` does not exist in the `config` do nothing

#### Assertions

- function can only be called by the admin storage in `config`