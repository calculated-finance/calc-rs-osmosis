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
    - save a new time trigger using the vault `time_interval`
  - else:
    - publish a `DCAVaultExecutionSkipped` event with the relevant skipped reason
- if the trigger is a fin limit order trigger:
  - withdraw the limit order from fin
  - if the fin limit withdrawal is successful:
    - delete the fin limit order trigger
    - save a new time trigger using the vault `time_interval`
  - else:
    - return an error to be logged by the off-chain scheduler
- reduce the vault balance by the swap amount
- send the CALC fee to the `fee_collector` address
- distribute remaining swapped funds to all vault `destinations` based on destination allocations
- use `authz` permissions to delegate funds from destination addresses to validators for destinations with action type `PostExecutionAction:Delegate`
- save a `DCAVaultExecutionCompleted` event

#### Cancellation

1. Cancellation (pt.1)
   - load vault using owner address and vault id
   - match on the vaults trigger variant (in this case we match for the FINLimitOrder variant)
   - delete the future time trigger configuration (the trigger that would have been assigned to the vault after the price trigger executed)
   - load the fin limit order trigger using the trigger id given from the vault
   - query the existing limit order using the fin limit order trigger's order idx (allows us to determine the status of the order and what needs to be refunded - this needs to be done before retract order as we can't query this info once the order has been retracted)
   - save fin limit order details to the fin limit order cache (so this info can be referenced in replies)
   - create a retract order sub message using the fin limit order triggers order ix (after the order is retracted we can see how much we get back, and if any partially filled order needs to be withdrawn)
   - same vault owner and id to the cache (so we can reference the vault in the replies)
   - send fin retract order sub message
2. Cancellation (pt.2)
   - load the cache
   - load the vault using the info we stored in the cache
   - load the fin limit order cache
   - load the fin limit order trigger
   - parse the fin retract order result to find the amount of token that was retracted
   - compare the amount of token that was retracted to the origial offer amount (fin limit order cache) - if the they are not equal we need to withdraw the partially filled order
3. Cancellation (pt.2.1 withdrawing partially filled orders)
   - send retracted amount of coin back to vault owner (some fraction of the vaults total value _[0% - 100%)_ )
   - create a fin withdraw limit order message using the fin limit order triggers order idx
   - send fin withdraw limit order message
4. Cancellation (pt.2.2 no partially filled order to retract)
   - get the remaining balance of the vault
   - create a new bank message with the vaults remaining balanced calculated previously
   - save the vault with the empty balance to cancelled vaults using the vaults owner and vault id
   - remove the vault from active vaults using the vaults owner and vault id
   - remove the fin limit order trigger using the trigger id
   - remove the fin limit order trigger id using the order idx
   - remove fin limit order cache
   - remove cache
5. Cancellation (pt.3)
   - load the cache
   - load the vault using the info from the cache
   - load the fin limit order cache
   - load the fin limit order trigger using the vaults trigger id
   - get the received denom from the limit order and filled amount to be sent to the user (the filled amount was stored in the fin limit order cache at pt.1 of this flow)
   - set the vaults balance to zero as we sent the owner a combination of initial assets, and assets received from the swap
   - remove the fin limit order trigger by trigger id
   - remove the fin limit order trigger ids using the fin limit order triggers order idx
   - remove the active vault using the vault owner and vault id
   - save the vault to cancelled vaults using the vault owner and vault id
   - remove the limit order cache
   - remove the cache
   - send bank message to user
