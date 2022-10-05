# DCA
## Vault life-cycles
### FIN limit order triggers
#### Creation
1. Vault creation (pt.1)
    - load config and increment vault counter to generate a new vault id
    - create time trigger configuration (to be used after fin limit order trigger)
    - create fin limit order trigger configuration (to be updated once fin limit order is sucessfully submitted)
    - create vault with no trigger reference (trigger reference to be updated once fin limit order is successfully submitted)
    - calculate coins to send with limit order message
    - create fin limit order sub message
    - save time trigger configuration against vault id (to be retrieved when the trigger is ready to be created)
    - save fin limit order trigger against vault id (to be retrieved when the trigger is ready to be created)
    - save vault in active vaults against owner and vault id
    - save empty executions array against vault id
    - save vault id and owner in cache (to be used for finding the vault by id and owner in the fin submit order reply handler)
    - send fin limit order sub message
2. Vault creation (pt.2)
    - reply handler to continue on after fin limit order sub message has replied
    - parse fin limit order reply result
    - find wasm event containing the fin order_idx (to be used to index the fin limit order trigger)
    - load cache to get a reference to the vault id and owner associated to this reply message
    - load fin limit order trigger configuration using vault id
    - create the fin limit order trigger from the fin limit order configuration and update the id. vault id, owner and order idx
    - update the vault to reference the fin limit order trigger
    - save the fin limit order trigger against the trigger id
    - save the fin limit order trigger id against the order idx (order idx is how these trigger will be resolved when being executed)
    - remove the fin limit order configuration from storage as it is now contained in the trigger
    - remove the cache as the create vault flow has now finished
#### Trigger Execution
1. Off-chain execution
    - off-chain components query all orders under the contract address and find limit orders that have been fullfilled (fin limit order triggers)
    - the order idx of a fullfilled limit order can be used to execute a fin limit order trigger
2. On-chain execution (pt.1)
    - load fin limit order trigger by id using the given order idx
    - load fin limit order trigger by trigger id retrieved in previous step
    - load vault using the fin limit order trigger owner and vault id fields
    - use on chain querier to validate the fin limit order has completed by ensuring the 'offer_amount' field is 0 (there is no more tokens to swap the order is fullfilled)
    - save the amount of coins sent and received in the limit order to the limit order cache (used to update the vault balance after the fin limit order has been successfuly withdrawn)
    - create fin withdraw message using the given order idx and vault pair address
    - save vault id and owner in cache (to be used for finding the vault by id and owner in the fin withdraw order reply handler)
    - send fin withdraw order sub message
3. On-chain execution (pt.2)
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