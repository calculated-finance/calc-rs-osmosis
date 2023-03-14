import { coin } from '@cosmjs/stargate';
import { expect } from 'chai';
import dayjs, { Dayjs } from 'dayjs';
import { Context } from 'mocha';
import { execute } from '../../shared/cosmwasm';
import { Vault } from '../../types/dca/response/get_vaults';
import { createVault, getBalances, getVaultLastUpdatedTime } from '../helpers';
import { setTimeout } from 'timers/promises';
import { EventData } from '../../types/dca/response/get_events';
import { map } from 'ramda';
import { instantiateFinPairContract } from '../hooks';

describe('when executing a vault', () => {
  describe('with a filled fin limit order trigger', () => {
    const swapAmount = 100000;
    const targetPrice = 1.5;
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let eventPayloadsAfterExecution: EventData[];
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;
    let receivedAmount: number;
    let receivedAmountAfterFee: number;

    before(async function (this: Context) {
      const vaultId = await createVault(this, {
        swap_amount: `${swapAmount}`,
        target_receive_amount: `${Math.round(swapAmount / targetPrice)}`,
      });

      vaultBeforeExecution = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id: vaultId,
          },
        })
      ).vault;

      const orders = await this.cosmWasmClient.queryContractSmart(this.finPairAddress, {
        book: {},
      });

      balancesBeforeExecution = await getBalances(this.cosmWasmClient, [
        this.userWalletAddress,
        this.dcaContractAddress,
        this.finPairAddress,
        this.feeCollectorAddress,
      ]);

      const result = await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
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
        this.finPairAddress,
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

      const receivedAmountBeforeTakerFee = Math.floor(swapAmount / parseFloat(orders.base[0].quote_price));
      receivedAmount = Math.floor(receivedAmountBeforeTakerFee - receivedAmountBeforeTakerFee * this.finTakerFee);
      receivedAmountAfterFee = Math.floor(receivedAmount - Math.floor(receivedAmount * this.calcSwapFee));
    });

    it('withdraws the fin limit order', async function (this: Context) {
      expect(
        this.cosmWasmClient.queryContractSmart(this.finPairAddress, {
          order: {
            order_idx:
              'fin_limit_order' in vaultBeforeExecution.trigger &&
              vaultBeforeExecution.trigger.fin_limit_order.order_idx,
          },
        }),
      ).to.be.rejectedWith(/No orders with the specified information exist/);
    });

    it('reduces the vault balance', async function (this: Context) {
      expect(vaultAfterExecution.balance.amount).to.equal(
        `${parseInt(vaultBeforeExecution.balance.amount) - swapAmount}`,
      );
    });

    it('sends funds back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['ukuji']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['ukuji'] + parseInt(vaultAfterExecution.received_amount.amount),
      );
    });

    it('sends fees to the fee collector', async function (this: Context) {
      const totalFees = receivedAmount - receivedAmountAfterFee;
      expect(balancesAfterExecution[this.feeCollectorAddress]['ukuji']).to.equal(
        balancesBeforeExecution[this.feeCollectorAddress]['ukuji'] + totalFees,
      );
    });

    it('updates the vault swapped amount correctly', () =>
      expect(vaultAfterExecution.swapped_amount.amount).to.eql(
        `${parseInt(vaultBeforeExecution.swapped_amount.amount) + parseInt(vaultBeforeExecution.swap_amount)}`,
      ));

    it('updates the vault received amount correctly', async function (this: Context) {
      expect(vaultAfterExecution.received_amount).to.eql(coin(receivedAmountAfterFee + 2, 'ukuji'));
    });

    it('has an execution triggered event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_execution_triggered: {
            asset_price: `${this.finBuyPrice}`,
            base_denom: 'ukuji',
            quote_denom: vaultAfterExecution.balance.denom,
          },
        },
      ]);
    });

    it('has an execution completed event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_execution_completed: {
            sent: coin(vaultAfterExecution.swap_amount, vaultAfterExecution.balance.denom),
            received: coin(`${receivedAmount + 2}`, 'ukuji'),
            fee: coin(Math.floor(receivedAmount * this.calcSwapFee), 'ukuji'),
          },
        },
      ]);
    });

    it('creates a new time trigger', async function (this: Context) {
      const executionTime = await getVaultLastUpdatedTime(
        this.cosmWasmClient,
        this.dcaContractAddress,
        vaultAfterExecution.id,
      );
      expect('time' in vaultAfterExecution.trigger && vaultAfterExecution.trigger.time.target_time).to.eql(
        `${executionTime.add(1, 'hour').unix()}000000000`,
      );
    });

    it('makes the vault active', () =>
      expect(vaultBeforeExecution.status).to.eql('scheduled') && expect(vaultAfterExecution.status).to.eql('active'));
  });

  describe('with a ready time trigger', () => {
    let targetTime: Dayjs;
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;
    let eventPayloadsBeforeExecution: EventData[];
    let eventPayloadsAfterExecution: EventData[];
    let receivedAmount: number;
    let receivedAmountAfterFee: number;

    before(async function (this: Context) {
      targetTime = dayjs().add(10, 'second');

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
        this.finPairAddress,
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

      while (dayjs().isBefore(targetTime)) {
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
        this.finPairAddress,
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

      const swapAmount = parseInt(vaultBeforeExecution.swap_amount);
      receivedAmount = Math.round((swapAmount - swapAmount * this.finTakerFee) / this.finBuyPrice);
      receivedAmountAfterFee = Math.round(receivedAmount - Math.floor(receivedAmount * this.calcSwapFee));
    });

    it('reduces the vault balance', async function (this: Context) {
      expect(vaultAfterExecution.balance.amount).to.equal(
        `${parseInt(vaultBeforeExecution.balance.amount) - parseInt(vaultBeforeExecution.swap_amount)}`,
      );
    });

    it('sends funds back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['ukuji']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['ukuji'] + parseInt(vaultAfterExecution.received_amount.amount),
      );
    });

    it('sends fees to the fee collector', async function (this: Context) {
      const totalFees = Math.floor(receivedAmount * this.calcSwapFee);
      expect(balancesAfterExecution[this.feeCollectorAddress]['ukuji']).to.equal(
        balancesBeforeExecution[this.feeCollectorAddress]['ukuji'] + totalFees,
      );
    });

    it('updates the vault swapped amount correctly', () =>
      expect(vaultAfterExecution.swapped_amount.amount).to.eql(
        `${parseInt(vaultBeforeExecution.swapped_amount.amount) + parseInt(vaultBeforeExecution.swap_amount)}`,
      ));

    it('updates the vault received amount correctly', () =>
      expect(vaultAfterExecution.received_amount).to.eql(coin(receivedAmountAfterFee, 'ukuji')));

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
            asset_price: `${this.finBuyPrice}`,
            base_denom: 'ukuji',
            quote_denom: vaultAfterExecution.balance.denom,
          },
        },
      ]);
    });

    it('has an execution completed event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_execution_completed: {
            sent: coin(vaultAfterExecution.swap_amount, vaultAfterExecution.balance.denom),
            received: coin(`${receivedAmount}`, 'ukuji'),
            fee: coin(Math.round(receivedAmount * this.calcSwapFee), 'ukuji'),
          },
        },
      ]);
    });

    it('makes the vault active', () =>
      expect(vaultBeforeExecution.status).to.eql('scheduled') && expect(vaultAfterExecution.status).to.eql('active'));
  });

  describe('with an unfilled fin limit order trigger', () => {
    let vaultId: number;

    before(async function (this: Context) {
      vaultId = await createVault(this, {
        swap_amount: '1000000',
        target_receive_amount: '300000000',
      });
    });

    it('fails to execute with the correct error message', async function (this: Context) {
      await expect(
        execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
          execute_trigger: {
            trigger_id: vaultId,
          },
        }),
      ).to.be.rejectedWith(/fin limit order has not been completely filled/);
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
            trigger_id: vaultId,
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
        this.finPairAddress,
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

      while (dayjs().isBefore(targetTime)) {
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
        this.finPairAddress,
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
    });

    it("doesn't reduce the vault balance", async () =>
      expect(vaultAfterExecution.balance.amount).to.equal(`${parseInt(vaultBeforeExecution.balance.amount)}`));

    it('sends no funds back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['ukuji']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['ukuji'],
      );
    });

    it('sends no fees to the fee collector', async function (this: Context) {
      expect(balancesAfterExecution[this.feeCollectorAddress]['ukuji']).to.equal(
        balancesBeforeExecution[this.feeCollectorAddress]['ukuji'],
      );
    });

    it("doesn't update the vault swapped amount", () =>
      expect(vaultAfterExecution.swapped_amount.amount).to.eql(
        `${parseInt(vaultBeforeExecution.swapped_amount.amount)}`,
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
            asset_price: `${this.finBuyPrice}`,
            base_denom: 'ukuji',
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
                price: `${this.finBuyPrice}`,
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

    before(async function (this: Context) {
      const finPairAddress = await instantiateFinPairContract(
        this.cosmWasmClient,
        this.adminContractAddress,
        'ukuji',
        'udemo',
        5,
        [
          { price: 1, amount: coin('100000000', 'ukuji') },
          { price: 0.2, amount: coin('1000', 'ukuji') },
          { price: 0.1, amount: coin('100000000', 'ukuji') },
        ],
      );

      const pair = await this.cosmWasmClient.queryContractSmart(finPairAddress, {
        config: {},
      });

      await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
        create_pair: {
          base_denom: pair.denoms[0].native,
          quote_denom: pair.denoms[1].native,
          address: finPairAddress,
        },
      });

      targetTime = dayjs().add(10, 'seconds');

      const vaultId = await createVault(
        this,
        {
          target_start_time_utc_seconds: `${targetTime.unix()}`,
          swap_amount: '100000000',
          slippage_tolerance: '0.0001',
          pair_address: finPairAddress,
        },
        [coin('1000000000', 'udemo')],
      );

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
        this.finPairAddress,
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

      while (dayjs().isBefore(targetTime)) {
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
        this.finPairAddress,
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
    });

    it("doesn't reduce the vault balance", async () =>
      expect(vaultAfterExecution.balance.amount).to.equal(`${parseInt(vaultBeforeExecution.balance.amount)}`));

    it('sends no funds back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['ukuji']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['ukuji'],
      );
    });

    it('sends no fees to the fee collector', async function (this: Context) {
      expect(balancesAfterExecution[this.feeCollectorAddress]['ukuji']).to.equal(
        balancesBeforeExecution[this.feeCollectorAddress]['ukuji'],
      );
    });

    it("doesn't update the vault swapped amount", () =>
      expect(vaultAfterExecution.swapped_amount.amount).to.eql(
        `${parseInt(vaultBeforeExecution.swapped_amount.amount)}`,
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
            asset_price: '0.1',
            base_denom: 'ukuji',
            quote_denom: vaultAfterExecution.balance.denom,
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

    it('updates the vault status', () =>
      expect(vaultBeforeExecution.status).to.eql('scheduled') && expect(vaultAfterExecution.status).to.eql('active'));
  });

  describe('with insufficient funds afterwards', () => {
    let targetTime: Dayjs;
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;
    let eventPayloadsBeforeExecution: EventData[];
    let eventPayloadsAfterExecution: EventData[];
    let receivedAmount: number;
    let receivedAmountAfterFee: number;

    before(async function (this: Context) {
      const swapAmount = 100000;
      targetTime = dayjs().add(10, 'seconds');

      const vaultId = await createVault(
        this,
        {
          target_start_time_utc_seconds: `${targetTime.unix()}`,
          swap_amount: `${swapAmount}`,
          slippage_tolerance: '0.0001',
        },
        [coin('110000', 'udemo')],
      );

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
        this.finPairAddress,
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

      while (dayjs().isBefore(targetTime)) {
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
        this.finPairAddress,
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

      receivedAmount = Math.round((swapAmount - swapAmount * this.finTakerFee) / this.finBuyPrice);
      receivedAmountAfterFee = Math.round(receivedAmount - Math.floor(receivedAmount * this.calcSwapFee));
    });

    it('reduces the vault balance', async function (this: Context) {
      expect(vaultAfterExecution.balance.amount).to.equal(
        `${parseInt(vaultBeforeExecution.balance.amount) - parseInt(vaultBeforeExecution.swap_amount)}`,
      );
    });

    it('sends funds back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['ukuji']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['ukuji'] + parseInt(vaultAfterExecution.received_amount.amount),
      );
    });

    it('sends fees to the fee collector', async function (this: Context) {
      const totalFees = Math.floor(receivedAmount * this.calcSwapFee);
      expect(balancesAfterExecution[this.feeCollectorAddress]['ukuji']).to.equal(
        balancesBeforeExecution[this.feeCollectorAddress]['ukuji'] + totalFees,
      );
    });

    it('updates the vault swapped amount correctly', () =>
      expect(vaultAfterExecution.swapped_amount.amount).to.eql(
        `${parseInt(vaultBeforeExecution.swapped_amount.amount) + parseInt(vaultBeforeExecution.swap_amount)}`,
      ));

    it('updates the vault received amount correctly', () =>
      expect(vaultAfterExecution.received_amount).to.eql(coin(receivedAmountAfterFee, 'ukuji')));

    it("doesn't create a new trigger", () => expect(vaultAfterExecution.trigger === null));

    it('adds the correct number of events', () =>
      expect(eventPayloadsAfterExecution.length).to.eql(eventPayloadsBeforeExecution.length + 2));

    it('has an execution triggered event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_execution_triggered: {
            asset_price: `${this.finBuyPrice}`,
            base_denom: 'ukuji',
            quote_denom: vaultAfterExecution.balance.denom,
          },
        },
      ]);
    });

    it('has an execution completed event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_execution_completed: {
            sent: coin(vaultAfterExecution.swap_amount, vaultAfterExecution.balance.denom),
            received: coin(`${receivedAmount}`, 'ukuji'),
            fee: coin(Math.round(receivedAmount * this.calcSwapFee), 'ukuji'),
          },
        },
      ]);
    });

    it('makes the vault inactive', () =>
      expect(vaultBeforeExecution.status).to.eql('scheduled') && expect(vaultAfterExecution.status).to.eql('inactive'));
  });
});
