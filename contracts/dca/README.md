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
- `time_interval`: the time interval at which the executions should take place once the vault executions have started
- `model_id`: the DCA+ model id to use for swap adjustments (auto selected based on expected execution duration)

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
- destinations of type `PostExecutionAction::Send` must have valid bech32 addresses
- destinations of type `PostExecutionAction::ZDelegate` must have valid validator addresses
- the sum of all destination allocations must == 1.0
- all destination allocations must be > 0.0
- the submitted `pair_address` must be a valid bech32 address
- the submitted `pair_address` must match an existing pair stored in the contract
- the submitted `pair_address.quote_denom` must match the denom of the funds included in the message
- at least one of `target_start_time_utc_seconds` and `target_receive_amount` must be `None`
- if `target_start_time_utc_seconds` is `Some`, it must be set to some timestamp in the future
- if `target_receive_amount` is `Some`, it must be greater than or equal to `minimum_receive_amount`

#### Domain Logic

- save a vault using the submitted vault details
- save a vault created event
- save a vault funds deposited event
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

Execute trigger accepts a trigger_id. For DCA vaults, the `trigger_id` is equal to the vault `id`. An off chain scheduler obtains `trigger_id`s for triggers that are ready to be executed via a combination of Fin order queries and the `GetTriggerIdByFinLimitOrderIdx` query for price triggers, and via the `GetTimeTriggerIds` query for time triggers.

#### Validation

- the vault must not be cancelled
- the vault must have a trigger
- if the trigger is a time trigger:
  - the `target_time` must be in the past
  - if the vault `position_type` is `PositionType::Enter`:
    - the current price of the swap asset must be lower than the price threshold (if there is one)
  - if the vault `position_type` is `PositionType::Exit`:
    - the current price of the swap asset must be higher than the price threshold (if there is one)
- if the trigger is a fin limit order trigger:
  - the fin limit `order_idx` must be stored against the trigger
  - the fin limit order must be completely filled

#### Domain Logic

- delete the current trigger
- if the trigger was a fin limit order trigger:
  - withdraw the limit order from fin
- if the vault was scheduled
  - make the vault active
  - set the vault started time to the current block time
- if the vault does not have sufficient funds (> 50000)
  - make the vault inactive
- if the vault is a DCA+ vault
  - update the standard DCA execution stats
- if the vault is active OR the vault is a DCA+ vault and it standard DCA would still be running
  - create a new time trigger
- if the vault is not active
  - finish execution
- create a execution triggered event
- if the vault has a price threshold & it is exceeded
  - create an execution skipped event
  - finish execution
- if the vault is a DCA+ vault AND it is inactive AND standard DCA would have finished
  - disburse the escrowed funds
  - finish execution
- execute a fin swap
- if the swap is successful:
  - create an execution completed event
  - if the vault is a DCA+ vault
    - store the escrowed amount
  - reduce the vault balance by the swap amount
  - distribute the swap and automation fees to the fee collectors
  - distribute remaining swapped funds to all vault `destinations` based on destination allocations
  - use `authz` permissions to delegate funds from destination addresses to validators for destinations with action type `PostExecutionAction:Delegate`
- else
  - create an execution skipped event with reason:
    - `SlippageToleranceExceeded` when the vault has enough funds to make the swap
    - `UnknownFailure` when the vault may not have had enough funds to make the swap

#### Assertions

- no vault should have a balance < 0
- no execution should redistribute more funds than the vault swap amount
- no execution should redistribute more funds than the vault balance
- every execution should reduce the vault balance by the amount of funds redistributed + calc fee

### Cancel Vault

#### Validation

- the sender address must be the vault owner or admin
- the vault must not already be cancelled

#### Domain Logic

- update the vault to have `status == VaultStatus::Cancelled`
- update the vault balance to 0
- if the vault has a price trigger:
  - retract & withdraw and the associated fin limit order trigger
- delete the vault trigger
- return the remaining vault balance to the vault owner

#### Assertions

- all cancelled vaults must have a balance of 0
- all cancelled vaults must have a status of cancelled
- all cancelled vaults must not have a trigger
- all funds are to be redistributed to the vault owner address

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
