import { coin } from '@cosmjs/stargate';
import { expect } from 'chai';
import dayjs, { Dayjs } from 'dayjs';
import { Context } from 'mocha';
import { execute } from '../../shared/cosmwasm';
import { Vault } from '../../types/dca/response/get_vaults';
import { createVault, getBalances, getExpectedPrice, provideAuthGrant } from '../helpers';
import { setTimeout } from 'timers/promises';
import { EventData } from '../../types/dca/response/get_events';
import { find, map } from 'ramda';
import { PositionType } from '../../types/dca/execute';

describe('when executing a vault', () => {
  describe('with a ready time trigger', () => {
    let targetTime: Dayjs;
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;
    let eventPayloadsBeforeExecution: EventData[];
    let eventPayloadsAfterExecution: EventData[];
    let executionTriggeredEvent: EventData;
    let receivedAmount: number;
    let receivedAmountAfterFee: number;

    before(async function (this: Context) {
      targetTime = dayjs().add(5, 'second');

      const vaultId = await createVault(this, {
        target_start_time_utc_seconds: `${targetTime.unix()}`,
      });

      vaultBeforeExecution = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id: vaultId,
          },
        })
      ).vault;

      balancesBeforeExecution = await getBalances(this.cosmWasmClient, [
        this.userWalletAddress,
        this.dcaContractAddress,
        this.feeCollectorAddress,
      ]);

      eventPayloadsBeforeExecution = map(
        (event) => event.data,
        (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_events_by_resource_id: { resource_id: vaultId },
          })
        ).events,
      );

      while (dayjs((await this.cosmWasmClient.getBlock()).header.time).isBefore(targetTime.add(2, 'seconds'))) {
        await setTimeout(3000);
      }

      await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
        execute_trigger: {
          trigger_id: vaultId,
        },
      });

      vaultAfterExecution = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id: vaultId,
          },
        })
      ).vault;

      balancesAfterExecution = await getBalances(this.cosmWasmClient, [
        this.userWalletAddress,
        this.dcaContractAddress,
        this.feeCollectorAddress,
      ]);

      eventPayloadsAfterExecution = map(
        (event) => event.data,
        (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_events_by_resource_id: { resource_id: vaultId },
          })
        ).events,
      );

      executionTriggeredEvent = find(
        (event) => 'dca_vault_execution_triggered' in event,
        eventPayloadsAfterExecution,
      ) as EventData;

      const expectedPrice = await getExpectedPrice(
        this,
        this.pair,
        coin(vaultAfterExecution.swap_amount, 'stake'),
        'uion',
      );
      const receivedAmountBeforeFee = Math.floor(Number(vaultAfterExecution.swap_amount) / expectedPrice);
      receivedAmount = Math.floor(receivedAmountBeforeFee);
      receivedAmountAfterFee = Math.floor(receivedAmount - receivedAmount * this.calcSwapFee);
    });

    it('reduces the vault balance', async function (this: Context) {
      expect(vaultAfterExecution.balance.amount).to.equal(
        `${Number(vaultBeforeExecution.balance.amount) - Number(vaultBeforeExecution.swap_amount)}`,
      );
    });

    it('sends funds back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['uion']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['uion'] + Number(vaultAfterExecution.received_amount.amount),
      );
    });

    it('sends fees to the fee collector', async function (this: Context) {
      const totalFees = Math.floor(receivedAmount * this.calcSwapFee);
      expect(balancesAfterExecution[this.feeCollectorAddress]['uion']).to.equal(
        balancesBeforeExecution[this.feeCollectorAddress]['uion'] + totalFees,
      );
    });

    it('updates the vault swapped amount correctly', () =>
      expect(vaultAfterExecution.swapped_amount.amount).to.eql(
        `${Number(vaultBeforeExecution.swapped_amount.amount) + Number(vaultBeforeExecution.swap_amount)}`,
      ));

    it('updates the vault received amount correctly', () =>
      expect(Number(vaultAfterExecution.received_amount.amount)).to.be.approximately(receivedAmountAfterFee, 5));

    it('creates a new time trigger', () =>
      expect('time' in vaultAfterExecution.trigger && vaultAfterExecution.trigger.time.target_time).to.eql(
        `${targetTime.add(1, 'hour').unix()}000000000`,
      ));

    it('adds the correct number of events', () =>
      expect(eventPayloadsAfterExecution.length).to.eql(eventPayloadsBeforeExecution.length + 2));

    it('has an execution triggered event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_execution_triggered: {
            asset_price:
              'dca_vault_execution_triggered' in executionTriggeredEvent &&
              executionTriggeredEvent.dca_vault_execution_triggered?.asset_price,
            base_denom: vaultAfterExecution.balance.denom,
            quote_denom: vaultAfterExecution.received_amount.denom,
          },
        },
      ]);
    });

    it('has an execution completed event', function (this: Context) {
      const executionCompletedEvent = find(
        (event) => 'dca_vault_execution_completed' in event,
        eventPayloadsAfterExecution,
      );
      expect(executionCompletedEvent).to.not.be.undefined;
      'dca_vault_execution_completed' in executionCompletedEvent &&
        expect(executionCompletedEvent.dca_vault_execution_completed?.sent.amount).to.equal(
          vaultAfterExecution.swap_amount,
        ) &&
        expect(Number(executionCompletedEvent.dca_vault_execution_completed?.received.amount)).to.approximately(
          receivedAmount,
          2,
        ) &&
        expect(Number(executionCompletedEvent.dca_vault_execution_completed?.fee.amount)).to.approximately(
          Math.round(receivedAmount * this.calcSwapFee) - 1,
          2,
        );
    });

    it('makes the vault active', () =>
      expect(vaultBeforeExecution.status).to.eql('scheduled') && expect(vaultAfterExecution.status).to.eql('active'));
  });

  describe('until the vault balance is empty', () => {
    let vault: Vault;
    const deposit = coin(1000000, 'stake');
    const swap_amount = '300000';

    before(async function (this: Context) {
      const vaultId = await createVault(
        this,
        {
          swap_amount,
          time_interval: 'every_second',
        },
        [deposit],
      );

      vault = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id: vaultId,
          },
        })
      ).vault;

      let triggerTime = 'time' in vault.trigger && dayjs(Number(vault.trigger.time.target_time) / 1000000);
      let blockTime = dayjs((await this.cosmWasmClient.getBlock()).header.time);

      while (blockTime.isBefore(triggerTime)) {
        await setTimeout(3000);
        blockTime = dayjs((await this.cosmWasmClient.getBlock()).header.time);
      }

      while (vault.status == 'active') {
        await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
          execute_trigger: {
            trigger_id: vaultId,
          },
        });

        vault = (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_vault: {
              vault_id: vaultId,
            },
          })
        ).vault;

        triggerTime = 'time' in vault.trigger && dayjs(Number(vault.trigger.time.target_time) / 1000000);

        while (blockTime.isBefore(triggerTime)) {
          await setTimeout(3000);
          blockTime = dayjs((await this.cosmWasmClient.getBlock()).header.time);
        }
      }
    });

    it('still has a trigger', () => expect(vault.trigger).to.not.eql(null));

    it('is inactive', () => expect(vault.status).to.eql('inactive'));

    it('has a zero balance', () => expect(vault.balance.amount).to.eql('0'));

    it('has a swapped amount equal to the total deposited', () =>
      expect(vault.swapped_amount.amount).to.eql(deposit.amount));

    it('deletes the final trigger on next execution', async function (this: Context) {
      await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
        execute_trigger: {
          trigger_id: vault.id,
        },
      });

      vault = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id: vault.id,
          },
        })
      ).vault;

      expect(vault.trigger).to.eql(null);
    });
  });

  describe('with a time trigger still in the future', () => {
    let vaultId: number;

    before(async function (this: Context) {
      vaultId = await createVault(this, {
        target_start_time_utc_seconds: `${dayjs().add(1, 'day').unix()}`,
      });
    });

    it('fails to execute with the correct error message', async function (this: Context) {
      await expect(
        execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
          execute_trigger: {
            trigger_id: `${vaultId}`,
          },
        }),
      ).to.be.rejectedWith(/trigger execution time has not yet elapsed/);
    });
  });

  describe('with a provide liquidity post execution action', () => {
    let vaultId: number;

    before(async function (this: Context) {
      await provideAuthGrant(
        this.userCosmWasmClient,
        this.userWalletAddress,
        this.dcaContractAddress,
        '/osmosis.lockup.MsgLockTokens',
      );

      vaultId = await createVault(this, {
        destinations: [
          {
            allocation: '1.0',
            address: this.dcaContractAddress,
            msg: Buffer.from(
              JSON.stringify({
                z_delegate: {
                  delegator_address: this.userWalletAddress,
                  validator_address: this.validatorAddress,
                },
              }),
            ).toString('base64'),
          },
        ],
      });
    });

    it('fails to execute with the correct error message', async function (this: Context) {
      await expect(
        execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
          execute_trigger: {
            trigger_id: `${vaultId}`,
          },
        }),
      ).to.be.rejectedWith(/trigger execution time has not yet elapsed/);
    });
  });

  describe('with an exceeded price ceiling', () => {
    let targetTime: Dayjs;
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;
    let eventPayloadsBeforeExecution: EventData[];
    let eventPayloadsAfterExecution: EventData[];
    let executionTriggeredEvent: EventData;

    before(async function (this: Context) {
      targetTime = dayjs().add(10, 'seconds');
      const swapAmount = 100000;

      const vaultId = await createVault(this, {
        target_start_time_utc_seconds: `${targetTime.unix()}`,
        swap_amount: `${swapAmount}`,
        minimum_receive_amount: `${swapAmount * 20}`,
      });

      vaultBeforeExecution = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id: vaultId,
          },
        })
      ).vault;

      balancesBeforeExecution = await getBalances(this.cosmWasmClient, [
        this.userWalletAddress,
        this.dcaContractAddress,
        this.feeCollectorAddress,
      ]);

      eventPayloadsBeforeExecution = map(
        (event) => event.data,
        (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_events_by_resource_id: { resource_id: vaultId },
          })
        ).events,
      );

      while (dayjs((await this.cosmWasmClient.getBlock()).header.time).isBefore(targetTime.add(2, 'seconds'))) {
        await setTimeout(3000);
      }

      await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
        execute_trigger: {
          trigger_id: vaultId,
        },
      });

      vaultAfterExecution = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id: vaultId,
          },
        })
      ).vault;

      balancesAfterExecution = await getBalances(this.cosmWasmClient, [
        this.userWalletAddress,
        this.dcaContractAddress,
        this.feeCollectorAddress,
      ]);

      eventPayloadsAfterExecution = map(
        (event) => event.data,
        (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_events_by_resource_id: { resource_id: vaultId },
          })
        ).events,
      );

      executionTriggeredEvent = find(
        (event) => 'dca_vault_execution_triggered' in event,
        eventPayloadsAfterExecution,
      ) as EventData;

      eventPayloadsAfterExecution = map(
        (event) => event.data,
        (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_events_by_resource_id: { resource_id: vaultId },
          })
        ).events,
      );
    });

    it("doesn't reduce the vault balance", async () =>
      expect(vaultAfterExecution.balance.amount).to.equal(`${Number(vaultBeforeExecution.balance.amount)}`));

    it('sends no funds back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['stake']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['stake'],
      );
    });

    it('sends no fees to the fee collector', async function (this: Context) {
      expect(balancesAfterExecution[this.feeCollectorAddress]['stake']).to.equal(
        balancesBeforeExecution[this.feeCollectorAddress]['stake'],
      );
    });

    it("doesn't update the vault swapped amount", () =>
      expect(vaultAfterExecution.swapped_amount.amount).to.eql(
        `${Number(vaultBeforeExecution.swapped_amount.amount)}`,
      ));

    it("doesn't update the vault received amount", () =>
      expect(vaultAfterExecution.received_amount).to.eql(vaultBeforeExecution.received_amount));

    it('creates a new time trigger', () =>
      expect('time' in vaultAfterExecution.trigger && vaultAfterExecution.trigger.time.target_time).to.eql(
        `${targetTime.add(1, 'hour').unix()}000000000`,
      ));

    it('adds the correct number of events', () =>
      expect(eventPayloadsAfterExecution.length).to.eql(eventPayloadsBeforeExecution.length + 2));

    it('has an execution triggered event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_execution_triggered: {
            asset_price:
              'dca_vault_execution_triggered' in executionTriggeredEvent &&
              executionTriggeredEvent.dca_vault_execution_triggered?.asset_price,
            quote_denom: vaultAfterExecution.received_amount.denom,
            base_denom: vaultAfterExecution.balance.denom,
          },
        },
      ]);
    });

    it('has an execution skipped event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_execution_skipped: {
            reason: {
              price_threshold_exceeded: {
                price:
                  'dca_vault_execution_triggered' in executionTriggeredEvent &&
                  executionTriggeredEvent.dca_vault_execution_triggered?.asset_price,
              },
            },
          },
        },
      ]);
    });

    it('makes the vault active', () =>
      expect(vaultBeforeExecution.status).to.eql('scheduled') && expect(vaultAfterExecution.status).to.eql('active'));
  });

  describe('with exceeded slippage', () => {
    let targetTime: Dayjs;
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;
    let eventPayloadsBeforeExecution: EventData[];
    let eventPayloadsAfterExecution: EventData[];
    let executionTriggeredEvent: EventData;

    before(async function (this: Context) {
      targetTime = dayjs().add(10, 'seconds');

      const vaultId = await createVault(this, {
        target_start_time_utc_seconds: `${targetTime.unix()}`,
        swap_amount: '10000000',
        slippage_tolerance: '0.0001',
      });

      vaultBeforeExecution = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id: vaultId,
          },
        })
      ).vault;

      balancesBeforeExecution = await getBalances(this.cosmWasmClient, [
        this.userWalletAddress,
        this.dcaContractAddress,
        this.feeCollectorAddress,
      ]);

      eventPayloadsBeforeExecution = map(
        (event) => event.data,
        (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_events_by_resource_id: { resource_id: vaultId },
          })
        ).events,
      );

      while (dayjs((await this.cosmWasmClient.getBlock()).header.time).isBefore(targetTime)) {
        await setTimeout(3000);
      }

      await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
        execute_trigger: {
          trigger_id: vaultId,
        },
      });

      vaultAfterExecution = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id: vaultId,
          },
        })
      ).vault;

      balancesAfterExecution = await getBalances(this.cosmWasmClient, [
        this.userWalletAddress,
        this.dcaContractAddress,
        this.feeCollectorAddress,
      ]);

      eventPayloadsAfterExecution = map(
        (event) => event.data,
        (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_events_by_resource_id: { resource_id: vaultId },
          })
        ).events,
      );

      executionTriggeredEvent = find(
        (event) => 'dca_vault_execution_triggered' in event,
        eventPayloadsAfterExecution,
      ) as EventData;
    });

    it("doesn't reduce the vault balance", async () =>
      expect(vaultAfterExecution.balance.amount).to.equal(`${Number(vaultBeforeExecution.balance.amount)}`));

    it('sends no funds back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['uosmo']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['uosmo'],
      );
    });

    it('sends no fees to the fee collector', async function (this: Context) {
      expect(balancesAfterExecution[this.feeCollectorAddress]['uosmo']).to.equal(
        balancesBeforeExecution[this.feeCollectorAddress]['uosmo'],
      );
    });

    it("doesn't update the vault swapped amount", () =>
      expect(vaultAfterExecution.swapped_amount.amount).to.eql(
        `${Number(vaultBeforeExecution.swapped_amount.amount)}`,
      ));

    it("doesn't update the vault received amount", () =>
      expect(vaultAfterExecution.received_amount).to.eql(vaultBeforeExecution.received_amount));

    it('creates a new time trigger', () =>
      expect('time' in vaultAfterExecution.trigger && vaultAfterExecution.trigger.time.target_time).to.eql(
        `${targetTime.add(1, 'hour').unix()}000000000`,
      ));

    it('adds the correct number of events', () =>
      expect(eventPayloadsAfterExecution.length).to.eql(eventPayloadsBeforeExecution.length + 2));

    it('has an execution triggered event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_execution_triggered: {
            asset_price:
              'dca_vault_execution_triggered' in executionTriggeredEvent &&
              executionTriggeredEvent.dca_vault_execution_triggered?.asset_price,
            base_denom: vaultAfterExecution.balance.denom,
            quote_denom: vaultAfterExecution.received_amount.denom,
          },
        },
      ]);
    });

    it('has an execution skipped event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_execution_skipped: {
            reason: 'slippage_tolerance_exceeded',
          },
        },
      ]);
    });

    it('makes the vault active', () =>
      expect(vaultBeforeExecution.status).to.eql('scheduled') && expect(vaultAfterExecution.status).to.eql('active'));
  });

  describe('with dca plus', () => {
    const deposit = coin(1000000, 'stake');
    const swapAmount = '100000';
    let vault: Vault;
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;
    let expectedPrice: number;

    before(async function (this: Context) {
      balancesBeforeExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['uion']);

      const targetTime = dayjs().add(10, 'seconds');

      const vault_id = await createVault(
        this,
        {
          target_start_time_utc_seconds: `${targetTime.unix()}`,
          time_interval: 'every_second',
          swap_adjustment_strategy: 'dca_plus',
        },
        [deposit],
      );

      while (dayjs((await this.cosmWasmClient.getBlock()).header.time).isBefore(targetTime)) {
        await setTimeout(3000);
      }

      await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
        execute_trigger: {
          trigger_id: vault_id,
        },
      });

      vault = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id,
          },
        })
      ).vault;

      expectedPrice = await getExpectedPrice(this, this.pair, coin(swapAmount, 'stake'), 'uion');

      balancesAfterExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['uion']);
    });

    it('subtracts the escrowed balance from the disbursed amount', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['uion']).to.equal(
        Math.round(
          balancesBeforeExecution[this.userWalletAddress]['uion'] +
            Number(vault.received_amount.amount) -
            Number(
              'dca_plus' in vault.swap_adjustment_strategy &&
                vault.swap_adjustment_strategy.dca_plus.escrowed_balance.amount,
            ),
        ),
      );
    });

    it('stores the escrowed balance', async function (this: Context) {
      expect(
        'dca_plus' in vault.swap_adjustment_strategy && vault.swap_adjustment_strategy.dca_plus.escrowed_balance.amount,
      ).to.equal(
        `${Math.floor(
          Number(vault.received_amount.amount) *
            parseFloat(
              'dca_plus' in vault.swap_adjustment_strategy && vault.swap_adjustment_strategy.dca_plus.escrow_level,
            ),
        )}`,
      );
    });

    it('calculates the standard dca swapped amount', async function (this: Context) {
      expect(
        'dca_plus' in vault.swap_adjustment_strategy &&
          vault.swap_adjustment_strategy.dca_plus.standard_dca_swapped_amount.amount,
      ).to.equal(`${Number(vault.swapped_amount.amount) / this.swapAdjustment}`);
    });

    it('calculates the standard dca received amount', async function (this: Context) {
      expect(
        Number(
          'dca_plus' in vault.swap_adjustment_strategy &&
            vault.swap_adjustment_strategy.dca_plus.standard_dca_received_amount.amount,
        ),
      ).to.be.approximately(Math.round(Number(vault.swap_amount) / expectedPrice) * (1 - this.calcSwapFee), 3);
    });
  });

  describe('with finished dca plus and unfinished standard dca', () => {
    const deposit = coin(1000000, 'stake');
    const swapAdjustment = 1.8;

    let vault: Vault;
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;

    before(async function (this: Context) {
      balancesBeforeExecution = await getBalances(
        this.cosmWasmClient,
        [this.userWalletAddress, this.feeCollectorAddress],
        ['uion'],
      );

      for (const position_type of ['enter', 'exit']) {
        await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
          update_swap_adjustments: {
            position_type: position_type as PositionType,
            adjustments: [
              [30, `${swapAdjustment}`],
              [35, `${swapAdjustment}`],
              [40, `${swapAdjustment}`],
              [45, `${swapAdjustment}`],
              [50, `${swapAdjustment}`],
              [55, `${swapAdjustment}`],
              [60, `${swapAdjustment}`],
              [70, `${swapAdjustment}`],
              [80, `${swapAdjustment}`],
              [90, `${swapAdjustment}`],
            ],
          },
        });
      }

      const targetTime = dayjs().add(10, 'seconds');

      const vault_id = await createVault(
        this,
        {
          target_start_time_utc_seconds: `${targetTime.unix()}`,
          swap_amount: `${Math.round(Number(deposit.amount) * (2 / 3))}`,
          time_interval: 'every_second',
          swap_adjustment_strategy: 'dca_plus',
        },
        [deposit],
      );

      while (dayjs((await this.cosmWasmClient.getBlock()).header.time).isBefore(targetTime)) {
        await setTimeout(3000);
      }

      await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
        execute_trigger: {
          trigger_id: vault_id,
        },
      });

      vault = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id,
          },
        })
      ).vault;

      balancesAfterExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['uion']);
    });

    it('subtracts the escrowed balance from the disbursed amount', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['uion']).to.equal(
        Math.round(
          balancesBeforeExecution[this.userWalletAddress]['uion'] +
            Number(vault.received_amount.amount) -
            Number(
              'dca_plus' in vault.swap_adjustment_strategy &&
                vault.swap_adjustment_strategy.dca_plus.escrowed_balance.amount,
            ),
        ),
      );
    });

    it('stores the escrowed balance', async function (this: Context) {
      expect(
        'dca_plus' in vault.swap_adjustment_strategy && vault.swap_adjustment_strategy.dca_plus.escrowed_balance.amount,
      ).to.equal(
        `${Math.floor(
          Number(vault.received_amount.amount) *
            parseFloat(
              'dca_plus' in vault.swap_adjustment_strategy && vault.swap_adjustment_strategy.dca_plus.escrow_level,
            ),
        )}`,
      );
    });

    it('has swapped all the vault balance', () => {
      expect(vault.balance.amount).to.equal('0');
      expect(vault.swapped_amount.amount).to.equal(deposit.amount);
    });

    it('sets the vault status to inactive', () => expect(vault.status).to.equal('inactive'));

    it('still has a time trigger', () =>
      expect(vault.trigger).to.eql({
        time: { target_time: 'time' in vault.trigger && vault.trigger.time.target_time },
      }));

    describe('once standard dca finishes', () => {
      let performanceFee: number;

      before(async function (this: Context) {
        vault = (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_vault: {
              vault_id: vault.id,
            },
          })
        ).vault;

        const triggerTime = 'time' in vault.trigger && dayjs(Number(vault.trigger.time.target_time) / 1000000);
        let blockTime = dayjs((await this.cosmWasmClient.getBlock()).header.time);

        while (blockTime.isBefore(triggerTime)) {
          await setTimeout(3000);
          blockTime = dayjs((await this.cosmWasmClient.getBlock()).header.time);
        }

        await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
          execute_trigger: {
            trigger_id: vault.id,
          },
        });

        vault = (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_vault: {
              vault_id: vault.id,
            },
          })
        ).vault;

        balancesAfterExecution = await getBalances(
          this.cosmWasmClient,
          [this.userWalletAddress, this.feeCollectorAddress],
          ['uion'],
        );

        performanceFee = Math.max(
          0,
          Math.floor(
            (Number(vault.received_amount.amount) -
              Number(
                'dca_plus' in vault.swap_adjustment_strategy &&
                  vault.swap_adjustment_strategy.dca_plus.standard_dca_received_amount.amount,
              )) *
              0.2,
          ),
        );
      });

      it('empties the escrow balance', () =>
        expect(
          'dca_plus' in vault.swap_adjustment_strategy &&
            vault.swap_adjustment_strategy.dca_plus.escrowed_balance.amount,
        ).to.equal('0'));

      it('pays out the escrow', function (this: Context) {
        expect(balancesAfterExecution[this.userWalletAddress]['uion']).to.be.approximately(
          balancesBeforeExecution[this.userWalletAddress]['uion'] +
            Number(vault.received_amount.amount) -
            performanceFee,
          2,
        );
      });

      it('pays out the performance fee', function (this: Context) {
        expect(balancesAfterExecution[this.feeCollectorAddress]['uion']).to.be.approximately(
          balancesBeforeExecution[this.feeCollectorAddress]['uion'] + performanceFee,
          2,
        );
      });
    });
  });

  describe('with finished standard dca and unfinished dca plus', () => {
    const deposit = coin(1000000, 'stake');
    const swapAdjustment = 0.8;

    let vault: Vault;
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;

    before(async function (this: Context) {
      balancesBeforeExecution = await getBalances(
        this.cosmWasmClient,
        [this.userWalletAddress, this.feeCollectorAddress],
        ['uion'],
      );

      for (const position_type of ['enter', 'exit']) {
        await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
          update_swap_adjustments: {
            position_type: position_type as PositionType,
            adjustments: [
              [30, `${swapAdjustment}`],
              [35, `${swapAdjustment}`],
              [40, `${swapAdjustment}`],
              [45, `${swapAdjustment}`],
              [50, `${swapAdjustment}`],
              [55, `${swapAdjustment}`],
              [60, `${swapAdjustment}`],
              [70, `${swapAdjustment}`],
              [80, `${swapAdjustment}`],
              [90, `${swapAdjustment}`],
            ],
          },
        });
      }

      const targetTime = dayjs().add(10, 'seconds');

      const vault_id = await createVault(
        this,
        {
          target_start_time_utc_seconds: `${targetTime.unix()}`,
          swap_amount: deposit.amount,
          time_interval: 'every_second',
          swap_adjustment_strategy: 'dca_plus',
        },
        [deposit],
      );

      while (dayjs((await this.cosmWasmClient.getBlock()).header.time).isBefore(targetTime)) {
        await setTimeout(3000);
      }

      await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
        execute_trigger: {
          trigger_id: vault_id,
        },
      });

      vault = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id,
          },
        })
      ).vault;

      balancesAfterExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['uion']);
    });

    it('subtracts the escrowed balance from the disbursed amount', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['uion']).to.equal(
        Math.round(
          balancesBeforeExecution[this.userWalletAddress]['uion'] +
            Number(vault.received_amount.amount) -
            Number(
              'dca_plus' in vault.swap_adjustment_strategy &&
                vault.swap_adjustment_strategy.dca_plus.escrowed_balance.amount,
            ),
        ),
      );
    });

    it('stores the escrowed balance', async function (this: Context) {
      expect(
        'dca_plus' in vault.swap_adjustment_strategy && vault.swap_adjustment_strategy.dca_plus.escrowed_balance.amount,
      ).to.equal(
        `${Math.floor(
          Number(vault.received_amount.amount) *
            parseFloat(
              'dca_plus' in vault.swap_adjustment_strategy && vault.swap_adjustment_strategy.dca_plus.escrow_level,
            ),
        )}`,
      );
    });

    it('has swapped all the standard vault balance', () => {
      expect(
        'dca_plus' in vault.swap_adjustment_strategy &&
          vault.swap_adjustment_strategy.dca_plus.standard_dca_swapped_amount.amount,
      ).to.equal(deposit.amount);
    });

    it('has not swapped all the dca plus vault balance', () =>
      expect(Number(vault.swapped_amount.amount)).to.equal(Number(deposit.amount) * swapAdjustment));

    it('vault is still active', () => expect(vault.status).to.equal('active'));

    it('still has a time trigger', () =>
      expect(vault.trigger).to.eql({
        time: { target_time: 'time' in vault.trigger && vault.trigger.time.target_time },
      }));

    describe('once dca plus finishes', () => {
      let performanceFee: number;

      before(async function (this: Context) {
        let triggerTime = 'time' in vault.trigger && dayjs(Number(vault.trigger.time.target_time) / 1000000);

        while (dayjs((await this.cosmWasmClient.getBlock()).header.time).isBefore(triggerTime)) {
          await setTimeout(3000);
        }

        await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
          execute_trigger: {
            trigger_id: vault.id,
          },
        });

        vault = (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_vault: {
              vault_id: vault.id,
            },
          })
        ).vault;

        balancesAfterExecution = await getBalances(
          this.cosmWasmClient,
          [this.userWalletAddress, this.feeCollectorAddress],
          ['uion'],
        );

        performanceFee = Math.floor(
          Math.max(
            0,
            Number(vault.received_amount.amount) -
              Number(
                'dca_plus' in vault.swap_adjustment_strategy &&
                  vault.swap_adjustment_strategy.dca_plus.standard_dca_received_amount.amount,
              ),
          ) * 0.2,
        );
      });

      it('has swapped all the balance', () => {
        expect(vault.swapped_amount.amount).to.equal(`${Number(deposit.amount)}`);
      });

      it('empties the escrow balance', () =>
        expect(
          'dca_plus' in vault.swap_adjustment_strategy &&
            vault.swap_adjustment_strategy.dca_plus.escrowed_balance.amount,
        ).to.equal('0'));

      it('pays out the escrow', function (this: Context) {
        expect(balancesAfterExecution[this.userWalletAddress]['uion']).to.be.approximately(
          balancesBeforeExecution[this.userWalletAddress]['uion'] +
            Number(vault.received_amount.amount) -
            performanceFee,
          2,
        );
      });

      it('pays out the performance fee', function (this: Context) {
        expect(balancesAfterExecution[this.feeCollectorAddress]['uion']).to.be.approximately(
          balancesBeforeExecution[this.feeCollectorAddress]['uion'] + performanceFee,
          2,
        );
      });

      it('sets the vault to inactive', () => expect(vault.status).to.equal('inactive'));

      it('does not have a trigger', () => expect(vault.trigger).to.equal(null));
    });
  });
});
