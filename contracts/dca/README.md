# DCA

## Cache life-cycle

Because cosmos chains implement the actor pattern, we can be certain that anything read from the cache will be relevant to the current transaction. Cache is never read from at the start of a brand new transaction, only ever written to.

## Vault life-cycles

### Create Vault

#### Validation

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
  - create a fin limit order trigger with the generated `order_idx` from fin

#### Trigger Execution

1. Trigger execution - off chain (p.1)
   - off-chain components query all orders under the contract address and find limit orders that have been fullfilled (fin limit order triggers)
   - the order idx of a fullfilled limit order can be used to execute a fin limit order trigger
2. Trigger execution - on chain (pt.2)
   - load fin limit order trigger by id using the given order idx
   - load fin limit order trigger by trigger id retrieved in previous step
   - load vault using the fin limit order trigger owner and vault id fields
   - use on chain querier to validate the fin limit order has completed by ensuring the 'offer_amount' field is 0 (there is no more tokens to swap the order is fullfilled)
   - save the amount of coins sent and received in the limit order to the limit order cache (used to update the vault balance after the fin limit order has been successfuly withdrawn)
   - create fin withdraw message using the given order idx and vault pair address
   - save vault id and owner in cache (to be used for finding the vault by id and owner in the fin withdraw order reply handler)
   - send fin withdraw order sub message
3. Trigger execution - on chain (pt.3)
   - reply handler to continue on after withdraw order sub message has replied
   - load cache
   - load limit order cache
   - load vault using information stored in cache
   - load fin limit order trigger using the vaults trigger id field
   - remove the fin limit order trigger id using the fin limit order trigger order idx field (this limit order has been withdrawm and no longer exists/the trigger is complete)
   - remove the fin limit order trigger using its own id (this trigger is now considered complete and we will switch over to a time trigger)
   - load config and increment trigger counter to generate a new trigger id
   - load the time trigger configuration using the vault id field
   - build the time trigger and update the id, vault id and owner
   - update the vault with new trigger information and reduce the current balance by the amount sent with the limit order (using the limit order cache)
   - if the time trigger has 0 trigger remaining (ie someone created a dca vault with a price trigger and 1 total execution) move vault to inactive and don't save the time trigger
   - save the time trigger using the trigger id (assuming vault is still active)
   - create coins that were sent and received from the limit order (to be used in execution information)
   - create bank send message to send the vault owner the assets received from the limit order
   - load executions to get a new sequence number for the next execution (sequence number indexes the order executions happened)
   - build a new execution using the success fin limit order trigger setter
   - save execution against the vault id
   - remove limit order cache as the execute trigger flow has now ended
   - remove cache as the execute trigger flow has now ended
   - send bank message

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
