# DCA

## Cache life-cycle

Because cosmos chains implement the actor pattern, we can be certain that anything read from the cache will be relevant to the current transaction. Cache is never read from at the start of a brand new transaction, only ever written to.

## Vaults & Triggers

Vaults store information relating to the overall DCA strategy the user has requested including (but not only):

- `owner`: only the owner can cancel the vault
- `destinations`: the addresses to distribute funds to after vault executions
- `status`: `Active`, `Inactive` or `Cancelled`
- `balance`: the current balance of the vault
- `pair`: the FIN pair address and denomination ordering for execution swaps and limit orders
- `swap_amount`: the amount to be swapped
- `position_type`: whether the vault is DCA in (investing) or DCA out (profit-taking)
- `time_interval`: the time interval at which the executions should take place once the vault executions have started

Triggers store the information required decide whether to execute a vault or not. Currently, there are 2 trigger types:.

1. Price triggers - set using fin limit orders, and executed once the full limit order has been filled.
2. Time triggers - set using the vault time interval and scheduled start date, executed once the trigger `target_time` has passed.

### Create Vault

Vaults are created by users via the CALC frontend application.

#### Validation

- if an owner is provided, it must be a valid address
- only a single asset can be provided in the message funds
- the vault `swap_amount` must be less than or equal to the vault balance
- the number of destinations provided must not exceed the limit set in config
- destinations of type `PostExecutionAction::Send` must have vaild bech32 addresses
- the sum of all destination allocations must == 1.0
- the submitted `pair_address` must be a valid bech32 address
- the submitted `pair_address` must match an existing pair stored in the contract
- the submitted `pair_address.quote_denom` must match the denom of the funds included in the message
- at least one of `target_start_time_utc_seconds` and `target_price` must be `None`
- if `target_start_time_utc_seconds` is `Some`, it must be set to some timestamp in the future

#### Domain Logic

- save a vault using the submitted vault details
- save a vault created event
- if the submitted `target_price` was `None`:
  - save a time trigger with the submitted `target_start_time_utc_seconds` or the block time if `target_start_time_utc_seconds` was `None`
- else:
  - create a fin limit order for the submitted `swap_amount` and `target_price`
  - save a fin limit order trigger with the generated `order_idx` from fin

#### Assertions

- all vaults should be created with a price trigger or a time trigger
- all vaults should be created in the scheduled status
- all vaults should be created with a balance > 0

### Execute Trigger

Execute trigger accepts a trigger_id. For DCA vaults, the `trigger_id` is equal to the vault `id`. An off chain scheduler obtains `trigger_id`s for triggers that are ready to be executed via FIN and the `GetTriggerIdByFinLimitOrderIdx` query for price triggers, and via the `GetTimeTriggerIds` query for time triggers.

#### Validation

- if the trigger is a time trigger:
  - the `target_time` must be in the past
  - if the vault `position_type` is `PositionType::Enter`:
    - the current price of the swap asset must be lower than the price threshold (if there is one)
  - if the vault `position_type` is `PositionType::Exit`:
    - the current price of the swap asset must be higher than the price threshold (if there is one)
- if the trigger is a fin limit order trigger:
  - the fin limit order must be completely filled

#### Domain Logic

- if the trigger is a time trigger:
  - execute a fin swap for the vault pair
  - if the fin swap is successful:
    - delete the current time trigger
    - if the vault balance > 0:
      - save a new time trigger using the vault `time_interval`
  - else:
    - publish a `DCAVaultExecutionSkipped` event with the relevant skipped reason
- if the trigger is a fin limit order trigger:
  - withdraw the limit order from fin
  - if the fin limit withdrawal is successful:
    - delete the fin limit order trigger
    - if the vault balance > 0:
      - save a new time trigger using the vault `time_interval`
  - else:
    - return an error to be logged by the off-chain scheduler
- reduce the vault balance by the swap amount
- send the CALC fee to the `fee_collector` address
- distribute remaining swapped funds to all vault `destinations` based on destination allocations
- use `authz` permissions to delegate funds from destination addresses to validators for destinations with action type `PostExecutionAction:Delegate`
- save a `DCAVaultExecutionCompleted` event

#### Assertions

- no vault should have a balance < 0
- no execution should redistribute more funds than the vault swap amount
- no execution should redistribute more funds than the vault balance
- every execution should reduce the vault balance by the amount of funds redistributed + calc fee
- no inactive or cancelled vault should have a trigger

### Cancel Vault

#### Validation

- the sender address must be the vault owner or admin
- the vault to be cancelled must not be already cancelled

#### Domain Logic

- update the vault to have `status == VaultStatus::Cancelled`
- if the vault has a time trigger:
  - delete the trigger and return the vault balance to the vault owner address
- if the vault has a price trigger:
  - withdraw and the associated fin limit order trigger
  - if the fin limit order has a non-zero filled amount:
    - retract the filled portion of the fin limit order
  - return the withdrawn and filled funds from the fin limit order to the vault owner address
  - return the remaining balance to the vault owner address

#### Assertions

- all cancelled vaults must have a balance of 0
- all cancelled vaults must have a status of cancelled
- all funds are to be redistributed to the vault owner address, including any partially filled fin limit orders

### Deposit

#### Vaildation

- the provided address must match the vault `owner`
- the vault must not be cancelled
- only a single asset must be provided
- the deposited funds denom must match the vault swap denom

#### Domain Logic

- update the vault balance to include the deposited funds
- if the vault status is inactive:
  - update the vault status to active
- save a vault funds deposited event

#### Assertions

- no vault should ever have balance < 0
- every vault that gets topped up should be active afterwards
