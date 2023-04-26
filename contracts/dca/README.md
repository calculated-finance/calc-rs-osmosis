# DCA

## Cache life-cycle

Because cosmos chains implement the actor pattern, we can be certain that anything read from the cache will be relevant to the current transaction. Cache is never read from at the start of a brand new transaction, only ever written to.

## Vaults & Triggers

Vaults store information relating to the overall DCA strategy the user has requested including (but not only):

- `owner`: only the owner can cancel the vault
- `destinations`: the addresses to distribute funds to after vault executions, including customisable callbacks to send funds to other contracts
- `status`: `Active`, `Inactive` or `Cancelled`
- `balance`: the current balance of the vault
- `target_denom`: the resulting denom to be received when the vault is executed
- `swap_amount`: the amount to be swapped
- `time_interval`: the time interval at which the executions should take place once the vault executions have started
- `performance_assessment_strategy`: the strategy to use for assessing the performance of the vault
- `swap_adjustment_strategy`: the strategy to use for adjusting the swap amount (i.e. risk weighted average)

Triggers store the information required decide whether to execute a vault or not. Currently, there is only 1 trigger type:

1. Time triggers - set using the vault time interval and scheduled start date, executed once the trigger `target_time` has passed.

### Create Vault

Vaults are created by users via the CALC frontend application.

#### Validation

- if an owner is provided, it must be a valid address
- only a single asset can be provided in the message funds
- the vault `swap_amount` must be less than or equal to the vault balance
- the number of destinations provided must not exceed the limit set in config
- the sum of all destination allocations must == 1.0
- all destination allocations must be > 0.0
- the vault balance denom and the `target_denom` must be found in a pair on the contract
- if `target_start_time_utc_seconds` is `Some`, it must be set to some timestamp in the future
- if `performance_assessment_strategy` is `Some`, `swap_adjustment_strategy` must also be `Some`, and vice versa

#### Domain Logic

- save a vault using the submitted vault details
- save a vault created event
- save a vault funds deposited event
- save a time trigger with the submitted `target_start_time_utc_seconds` or the block time if `target_start_time_utc_seconds` was `None`
- execute the vault if `target_start_time_utc_seconds` was `None`

#### Assertions

- all vaults should be created with a time trigger
- all vaults should be created in the scheduled status
- all vaults should be created with a balance > 0

### Execute Trigger

Execute trigger accepts a trigger_id. For DCA vaults, the `trigger_id` is equal to the vault `id`. An off chain scheduler obtains `trigger_id`s for triggers that are ready to be executed via the `GetTimeTriggerIds` query for time triggers.

#### Validation

- the vault must not be cancelled
- the vault must have a trigger
- the `target_time` must be in the past
- the current expected price must yeild at least the `minimum_receive_amount`

#### Domain Logic

- delete the current trigger
- if the vault was scheduled
  - make the vault active
  - set the vault started time to the current block time
- if the vault has a performance assessment strategy
  - update any performance assessment data
- if the vault is active OR the vault performance assessment is still active
  - create a new time trigger
- create a execution triggered event
- if the vault has a price threshold & it is exceeded
  - create an execution skipped event
  - finish execution
- if the vault is inactive AND has a performance assessment strategy that is finished && has escrowed funds
  - disburse the escrowed funds
  - finish execution
- execute a swap on the underlying DEX
- if the swap is successful:
  - create an execution completed event
  - escrow any received amount according to the vault escrow level
  - reduce the vault balance by the swapped amount
  - distribute the swap and automation fees to the fee collectors
  - distribute remaining swapped funds to all vault `destinations` based on destination allocations & callbacks
- else
  - create an execution skipped event with reason `SlippageToleranceExceeded`

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

- update the vault to have `status` of `Cancelled`
- update the vault balance to 0
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
- update the vault deposited_amount to include the deposited funds
- if the vault status is inactive:
  - update the vault status to active
- save a vault funds deposited event
- if the vault was inactive and had no trigger, create a new trigger and execute the vault

#### Assertions

- no vault should ever have balance < 0
- every vault that gets topped up should be active afterwards

### Disburse Escrow

#### Validation

- the sender must be the admin address or the contract address

#### Domain Logic

- if the vault has no escrowed funds, return early
- evaluate the fee according to the performance assessment strategy & escrowed balance
- return the fee to the fee collector
- return the remaining escrowed funds to the vault destinations

#### Assertions

- the vault escrowed balance should be disbursed entirely
- the vault escrowed balance should be set to 0
