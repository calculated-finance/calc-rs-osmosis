import { Coin, coin } from '@cosmjs/stargate';
import { expect } from 'chai';
import dayjs, { Dayjs } from 'dayjs';
import { Context } from 'mocha';
import { execute } from '../../shared/cosmwasm';
import { Vault } from '../../types/dca/response/get_vaults';
import { createVault, getBalances, getExpectedPrice, getPool } from '../helpers';
import { setTimeout } from 'timers/promises';
import { EventData } from '../../types/dca/response/get_events';
import { find, map } from 'ramda';
import { PositionType } from '../../types/dca/execute';

describe('when executing a vault', () => {
  describe('with a ready time trigger', () => {
    let targetTime: Dayjs;
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let balancesBeforeExecution: { [x: string]: { address: string } };
    let balancesAfterExecution: { [x: string]: { address: string } };
    let eventPayloadsBeforeExecution: EventData[];
    let eventPayloadsAfterExecution: EventData[];
    let executionTriggeredEvent: EventData;
    let expectedPrice: number;
    let receivedAmount: number;

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

      balancesBeforeExecution = await getBalances(this, [
        this.userWalletAddress,
        this.dcaContractAddress,
        this.feeCollectorAddress,
      ]);

      expectedPrice = await getExpectedPrice(
        this,
        this.pair,
        coin(vaultBeforeExecution.swap_amount, vaultBeforeExecution.balance.denom),
        vaultBeforeExecution.target_denom,
      );

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

      balancesAfterExecution = await getBalances(this, [
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

      receivedAmount = Math.round(Number(vaultAfterExecution.swap_amount) / expectedPrice);
    });

    it('reduces the vault balance', async function (this: Context) {
      expect(vaultAfterExecution.balance.amount).to.equal(
        `${Number(vaultBeforeExecution.balance.amount) - Number(vaultBeforeExecution.swap_amount)}`,
      );
    });

    it('sends funds back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress][vaultAfterExecution.target_denom]).to.equal(
        balancesBeforeExecution[this.userWalletAddress][vaultAfterExecution.target_denom] +
          Number(vaultAfterExecution.received_amount.amount),
      );
    });

    it('sends fees to the fee collector', async function (this: Context) {
      const totalFees = Math.floor(receivedAmount * this.calcSwapFee);
      expect(balancesAfterExecution[this.feeCollectorAddress][vaultAfterExecution.target_denom]).to.equal(
        balancesBeforeExecution[this.feeCollectorAddress][vaultAfterExecution.target_denom] + totalFees,
      );
    });

    it('updates the vault swapped amount correctly', () =>
      expect(vaultAfterExecution.swapped_amount.amount).to.eql(
        `${Number(vaultBeforeExecution.swapped_amount.amount) + Number(vaultBeforeExecution.swap_amount)}`,
      ));

    it('updates the vault received amount correctly', function (this: Context) {
      expect(Number(vaultAfterExecution.received_amount.amount)).to.be.approximately(
        receivedAmount * (1 - this.calcSwapFee),
        5,
      );
    });

    it('creates a new time trigger', () => {
      const triggerTime = dayjs(
        Number('time' in vaultAfterExecution.trigger && vaultAfterExecution.trigger.time.target_time) / 1000000,
      );
      expect(
        triggerTime.diff(dayjs(Math.round(Number(vaultAfterExecution.started_at) / 1000000)), 'seconds'),
      ).to.be.approximately(3600, 1);
    });

    it('adds the correct number of events', () =>
      expect(eventPayloadsAfterExecution.length).to.eql(eventPayloadsBeforeExecution.length + 2));

    it('has an execution triggered event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_execution_triggered: {
            asset_price:
              'dca_vault_execution_triggered' in executionTriggeredEvent &&
              executionTriggeredEvent.dca_vault_execution_triggered?.asset_price,
            quote_denom: vaultAfterExecution.balance.denom,
            base_denom: vaultAfterExecution.received_amount.denom,
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
          Math.round(receivedAmount * this.calcSwapFee),
          2,
        );
    });

    it('makes the vault active', () =>
      expect(vaultBeforeExecution.status).to.eql('scheduled') && expect(vaultAfterExecution.status).to.eql('active'));
  });

  describe('with a failing destination callback', () => {
    let vault: Vault;
    let balancesBeforeExecution: { [x: string]: { address: string } };
    let balancesAfterExecution: { [x: string]: { address: string } };

    before(async function (this: Context) {
      const targetTime = dayjs().add(5, 'second');

      const vaultId = await createVault(this, {
        target_start_time_utc_seconds: `${targetTime.unix()}`,
        destinations: [
          {
            address: this.userWalletAddress,
            allocation: '0.3',
            msg: null,
          },
          {
            address: this.adminContractAddress,
            allocation: '0.7',
            msg: Buffer.from(
              JSON.stringify({
                deposit: {},
              }),
            ).toString('base64'),
          },
        ],
      });

      vault = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id: vaultId,
          },
        })
      ).vault;

      balancesBeforeExecution = await getBalances(this, [vault.owner]);

      while (dayjs((await this.cosmWasmClient.getBlock()).header.time).isBefore(targetTime.add(2, 'seconds'))) {
        await setTimeout(3000);
      }

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

      balancesAfterExecution = await getBalances(this, [vault.owner]);
    });

    it('sends funds to the vault owner instead', function (this: Context) {
      expect(
        balancesBeforeExecution[vault.owner][vault.target_denom] + Number(vault.received_amount.amount),
      ).to.be.approximately(balancesAfterExecution[vault.owner][vault.target_denom], 1);
    });
  });

  describe('until the vault balance is empty', () => {
    let vault: Vault;
    let deposit: Coin;
    const swap_amount = '300000';

    before(async function (this: Context) {
      deposit = coin(1000000, this.pair.quote_denom);
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

        if (vault.trigger) {
          triggerTime = 'time' in vault.trigger && dayjs(Number(vault.trigger.time.target_time) / 1000000);

          while (blockTime.isBefore(triggerTime)) {
            await setTimeout(3000);
            blockTime = dayjs((await this.cosmWasmClient.getBlock()).header.time);
          }
        }
      }
    });

    it('has no trigger', () => expect(vault.trigger).to.be.null);

    it('is inactive', () => expect(vault.status).to.eql('inactive'));

    it('has a zero balance', () => expect(vault.balance.amount).to.eql('0'));

    it('has a swapped amount equal to the total deposited', () =>
      expect(vault.swapped_amount.amount).to.eql(deposit.amount));
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

  describe('with an exceeded price ceiling', () => {
    let targetTime: Dayjs;
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let balancesBeforeExecution: { [x: string]: { address: string } };
    let balancesAfterExecution: { [x: string]: { address: string } };
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

      balancesBeforeExecution = await getBalances(this, [
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

      balancesAfterExecution = await getBalances(this, [
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
      expect(balancesAfterExecution[this.userWalletAddress][vaultAfterExecution.target_denom]).to.equal(
        balancesBeforeExecution[this.userWalletAddress][vaultAfterExecution.target_denom],
      );
    });

    it('sends no fees to the fee collector', async function (this: Context) {
      expect(balancesAfterExecution[this.feeCollectorAddress][vaultAfterExecution.target_denom]).to.equal(
        balancesBeforeExecution[this.feeCollectorAddress][vaultAfterExecution.target_denom],
      );
    });

    it("doesn't update the vault swapped amount", () =>
      expect(vaultAfterExecution.swapped_amount.amount).to.eql(
        `${Number(vaultBeforeExecution.swapped_amount.amount)}`,
      ));

    it("doesn't update the vault received amount", () =>
      expect(vaultAfterExecution.received_amount).to.eql(vaultBeforeExecution.received_amount));

    it('creates a new time trigger', () => {
      const triggerTime = dayjs(
        Number('time' in vaultAfterExecution.trigger && vaultAfterExecution.trigger.time.target_time) / 1000000,
      );
      expect(
        triggerTime.diff(dayjs(Math.round(Number(vaultAfterExecution.started_at) / 1000000)), 'seconds'),
      ).to.be.approximately(3600, 1);
    });

    it('adds the correct number of events', () =>
      expect(eventPayloadsAfterExecution.length).to.eql(eventPayloadsBeforeExecution.length + 2));

    it('has an execution triggered event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_execution_triggered: {
            asset_price:
              'dca_vault_execution_triggered' in executionTriggeredEvent &&
              executionTriggeredEvent.dca_vault_execution_triggered?.asset_price,
            base_denom: vaultAfterExecution.received_amount.denom,
            quote_denom: vaultAfterExecution.balance.denom,
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
    let balancesBeforeExecution: { [x: string]: { address: string } };
    let balancesAfterExecution: { [x: string]: { address: string } };
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

      balancesBeforeExecution = await getBalances(this, [
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

      balancesAfterExecution = await getBalances(this, [
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
      expect(balancesAfterExecution[this.userWalletAddress][vaultAfterExecution.target_denom]).to.equal(
        balancesBeforeExecution[this.userWalletAddress][vaultAfterExecution.target_denom],
      );
    });

    it('sends no fees to the fee collector', async function (this: Context) {
      expect(balancesAfterExecution[this.feeCollectorAddress][vaultAfterExecution.target_denom]).to.equal(
        balancesBeforeExecution[this.feeCollectorAddress][vaultAfterExecution.target_denom],
      );
    });

    it("doesn't update the vault swapped amount", () =>
      expect(vaultAfterExecution.swapped_amount.amount).to.eql(
        `${Number(vaultBeforeExecution.swapped_amount.amount)}`,
      ));

    it("doesn't update the vault received amount", () =>
      expect(vaultAfterExecution.received_amount).to.eql(vaultBeforeExecution.received_amount));

    it('creates a new time trigger', () => {
      const triggerTime = dayjs(
        Number('time' in vaultAfterExecution.trigger && vaultAfterExecution.trigger.time.target_time) / 1000000,
      );
      expect(
        triggerTime.diff(dayjs(Math.round(Number(vaultAfterExecution.started_at) / 1000000)), 'seconds'),
      ).to.be.approximately(3600, 1);
    });

    it('adds the correct number of events', () =>
      expect(eventPayloadsAfterExecution.length).to.eql(eventPayloadsBeforeExecution.length + 2));

    it('has an execution triggered event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_execution_triggered: {
            asset_price:
              'dca_vault_execution_triggered' in executionTriggeredEvent &&
              executionTriggeredEvent.dca_vault_execution_triggered?.asset_price,
            quote_denom: vaultAfterExecution.balance.denom,
            base_denom: vaultAfterExecution.received_amount.denom,
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

  describe('with risk weighted average swap adjustment strategy', () => {
    let vault: Vault;
    let balancesBeforeExecution: { [x: string]: { address: string } };
    let balancesAfterExecution: { [x: string]: { address: string } };
    let expectedPrice: number;

    before(async function (this: Context) {
      balancesBeforeExecution = await getBalances(
        this,
        [this.userWalletAddress],
        [this.pair.quote_denom, this.pair.base_denom],
      );

      const targetTime = dayjs().add(10, 'seconds');

      const vault_id = await createVault(this, {
        target_start_time_utc_seconds: `${targetTime.unix()}`,
        time_interval: 'every_second',
        swap_adjustment_strategy: {
          risk_weighted_average: {
            base_denom: 'bitcoin',
          },
        },
        performance_assessment_strategy: 'compare_to_standard_dca',
      });

      vault = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id,
          },
        })
      ).vault;

      expectedPrice = await getExpectedPrice(
        this,
        this.pair,
        coin(vault.swap_amount, vault.balance.denom),
        vault.target_denom,
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

      balancesAfterExecution = await getBalances(
        this,
        [this.userWalletAddress],
        [vault.balance.denom, vault.target_denom],
      );
    });

    it('subtracts the escrowed balance from the disbursed amount', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress][vault.target_denom]).to.equal(
        Math.round(
          balancesBeforeExecution[this.userWalletAddress][vault.target_denom] +
            Number(vault.received_amount.amount) -
            Number(vault.escrowed_amount.amount),
        ),
      );
    });

    it('stores the escrowed balance', async function (this: Context) {
      expect(vault.escrowed_amount.amount).to.equal(
        `${Math.floor(Number(vault.received_amount.amount) * Number(vault.escrow_level))}`,
      );
    });

    it('calculates the standard dca swapped amount', async function (this: Context) {
      expect(vault.performance_assessment_strategy.compare_to_standard_dca.swapped_amount.amount).to.equal(
        `${Number(vault.swapped_amount.amount) / this.swapAdjustment}`,
      );
    });

    it('calculates the standard dca received amount', async function (this: Context) {
      expect(
        Number(vault.performance_assessment_strategy.compare_to_standard_dca.received_amount.amount),
      ).to.be.approximately(
        Math.round(
          Number(vault.performance_assessment_strategy.compare_to_standard_dca.swapped_amount.amount) / expectedPrice,
        ),
        2,
      );
    });
  });

  describe('with finished risk weighted average swap adjustment strategy and unfinished standard dca', () => {
    let deposit: Coin;
    let vault: Vault;
    let balancesBeforeExecution: { [x: string]: { address: string } };
    let balancesAfterExecution: { [x: string]: { address: string } };
    const swapAdjustment = 1.8;

    before(async function (this: Context) {
      deposit = coin(1000000, this.pair.quote_denom);

      balancesBeforeExecution = await getBalances(
        this,
        [this.userWalletAddress, this.feeCollectorAddress],
        [this.pair.quote_denom, this.pair.base_denom],
      );

      for (const position_type of ['enter', 'exit']) {
        await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
          update_swap_adjustment: {
            strategy: {
              risk_weighted_average: {
                model_id: 30,
                base_denom: 'bitcoin',
                position_type: position_type as PositionType,
              },
            },
            value: `${swapAdjustment}`,
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
          swap_adjustment_strategy: {
            risk_weighted_average: {
              base_denom: 'bitcoin',
            },
          },
          performance_assessment_strategy: 'compare_to_standard_dca',
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

      balancesAfterExecution = await getBalances(
        this,
        [this.userWalletAddress],
        [vault.target_denom, vault.balance.denom],
      );
    });

    it('subtracts the escrowed balance from the disbursed amount', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress][vault.target_denom]).to.equal(
        Math.round(
          balancesBeforeExecution[this.userWalletAddress][vault.target_denom] +
            Number(vault.received_amount.amount) -
            Number(vault.escrowed_amount.amount),
        ),
      );
    });

    it('stores the escrowed balance', async function (this: Context) {
      expect(vault.escrowed_amount.amount).to.equal(
        `${Math.floor(Number(vault.received_amount.amount) * Number(vault.escrow_level))}`,
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
          this,
          [this.userWalletAddress, this.feeCollectorAddress],
          [vault.balance.denom, vault.target_denom],
        );

        performanceFee = Math.max(
          0,
          Math.floor(
            (Number(vault.received_amount.amount) -
              Number(
                'compare_to_standard_dca' in vault.performance_assessment_strategy &&
                  vault.performance_assessment_strategy.compare_to_standard_dca.received_amount.amount,
              )) *
              0.2,
          ),
        );
      });

      it('empties the escrow balance', () => expect(vault.escrowed_amount.amount).to.equal('0'));

      it('pays out the escrow', function (this: Context) {
        expect(balancesAfterExecution[this.userWalletAddress][vault.target_denom]).to.be.approximately(
          balancesBeforeExecution[this.userWalletAddress][vault.target_denom] +
            Number(vault.received_amount.amount) -
            performanceFee,
          2,
        );
      });

      it('pays out the performance fee', function (this: Context) {
        expect(balancesAfterExecution[this.feeCollectorAddress][vault.target_denom]).to.be.approximately(
          balancesBeforeExecution[this.feeCollectorAddress][vault.target_denom] + performanceFee,
          2,
        );
      });
    });
  });

  describe('with finished standard dca and unfinished risk weighted average swap adjustment strategy', () => {
    let deposit: Coin;
    let vault: Vault;
    let balancesBeforeExecution: { [x: string]: { address: string } };
    let balancesAfterExecution: { [x: string]: { address: string } };
    const swapAdjustment = 0.8;

    before(async function (this: Context) {
      deposit = coin(1000000, this.pair.quote_denom);

      balancesBeforeExecution = await getBalances(
        this,
        [this.userWalletAddress, this.feeCollectorAddress],
        [this.pair.base_denom, this.pair.quote_denom],
      );

      for (const position_type of ['enter', 'exit']) {
        await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
          update_swap_adjustment: {
            strategy: {
              risk_weighted_average: {
                model_id: 30,
                base_denom: 'bitcoin',
                position_type: position_type as PositionType,
              },
            },
            value: `${swapAdjustment}`,
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
          swap_adjustment_strategy: {
            risk_weighted_average: {
              base_denom: 'bitcoin',
            },
          },
          performance_assessment_strategy: 'compare_to_standard_dca',
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

      balancesAfterExecution = await getBalances(
        this,
        [this.userWalletAddress],
        [vault.balance.denom, vault.target_denom],
      );
    });

    it('subtracts the escrowed balance from the disbursed amount', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress][vault.target_denom]).to.equal(
        Math.round(
          balancesBeforeExecution[this.userWalletAddress][vault.target_denom] +
            Number(vault.received_amount.amount) -
            Number(vault.escrowed_amount.amount),
        ),
      );
    });

    it('stores the escrowed balance', async function (this: Context) {
      expect(vault.escrowed_amount.amount).to.equal(
        `${Math.floor(Number(vault.received_amount.amount) * Number(vault.escrow_level))}`,
      );
    });

    it('has swapped all the standard vault balance', () => {
      expect(vault.performance_assessment_strategy.compare_to_standard_dca.swapped_amount.amount).to.equal(
        deposit.amount,
      );
    });

    it('has not swapped all the risk weighted average swap adjustment strategy vault balance', () =>
      expect(Number(vault.swapped_amount.amount)).to.equal(Number(deposit.amount) * swapAdjustment));

    it('vault is still active', () => expect(vault.status).to.equal('active'));

    it('still has a time trigger', () =>
      expect(vault.trigger).to.eql({
        time: { target_time: 'time' in vault.trigger && vault.trigger.time.target_time },
      }));

    describe('once risk weighted average swap adjustment strategy finishes', () => {
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
          this,
          [this.userWalletAddress, this.feeCollectorAddress],
          [vault.target_denom, vault.balance.denom],
        );

        performanceFee = Math.floor(
          Math.max(
            0,
            Number(vault.received_amount.amount) -
              Number(vault.performance_assessment_strategy.compare_to_standard_dca.received_amount.amount),
          ) * 0.2,
        );
      });

      it('has swapped all the balance', () => {
        expect(vault.swapped_amount.amount).to.equal(`${Number(vault.deposited_amount.amount)}`);
      });

      it('empties the escrow balance', () => expect(vault.escrowed_amount.amount).to.equal('0'));

      it('pays out the escrow', function (this: Context) {
        expect(balancesAfterExecution[this.userWalletAddress][vault.target_denom]).to.be.approximately(
          balancesBeforeExecution[this.userWalletAddress][vault.target_denom] +
            Number(vault.received_amount.amount) -
            performanceFee,
          2,
        );
      });

      it('pays out the performance fee', function (this: Context) {
        expect(balancesAfterExecution[this.feeCollectorAddress][vault.target_denom]).to.be.approximately(
          balancesBeforeExecution[this.feeCollectorAddress][vault.target_denom] + performanceFee,
          2,
        );
      });

      it('sets the vault to inactive', () => expect(vault.status).to.equal('inactive'));

      it('does not have a trigger', () => expect(vault.trigger).to.equal(null));
    });
  });
});
