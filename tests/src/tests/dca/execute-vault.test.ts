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

    it('makes the vault active', () =>
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
  });

  describe('with dca plus', () => {
    const deposit = coin(1000000, 'ukuji');
    let vault: Vault;
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;
    let expectedPrice: number;

    before(async function (this: Context) {
      balancesBeforeExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['udemo']);

      const targetTime = dayjs().add(10, 'seconds');

      const vault_id = await createVault(
        this,
        {
          target_start_time_utc_seconds: `${targetTime.unix()}`,
          time_interval: 'every_second',
          use_dca_plus: true,
        },
        [deposit],
      );

      while (dayjs().isBefore(targetTime)) {
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

      expectedPrice = await this.cosmWasmClient.queryContractSmart(this.swapContractAddress, {
        get_price: {
          swap_amount: coin(vault.swap_amount, 'ukuji'),
          target_denom: 'udemo',
          price_type: 'actual',
        },
      });

      balancesAfterExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['udemo']);
    });

    it('subtracts the escrowed balance from the disbursed amount', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['udemo']).to.equal(
        Math.round(
          balancesBeforeExecution[this.userWalletAddress]['udemo'] +
            parseInt(vault.received_amount.amount) -
            parseInt(vault.dca_plus_config.escrowed_balance.amount),
        ),
      );
    });

    it('stores the escrowed balance', async function (this: Context) {
      expect(vault.dca_plus_config.escrowed_balance.amount).to.equal(
        `${Math.floor(parseInt(vault.received_amount.amount) * parseFloat(vault.dca_plus_config.escrow_level))}`,
      );
    });

    it('calculates the standard dca swapped amount', async function (this: Context) {
      expect(vault.dca_plus_config.standard_dca_swapped_amount.amount).to.equal(
        `${parseInt(vault.swapped_amount.amount) / this.swapAdjustment}`,
      );
    });

    it('calculates the standard dca received amount', async function (this: Context) {
      expect(vault.dca_plus_config.standard_dca_received_amount.amount).to.equal(
        `${Math.round((parseInt(vault.swap_amount) / expectedPrice) * (1 - this.calcSwapFee - this.finTakerFee))}`,
      );
    });
  });

  describe('with finished dca plus and unfinished standard dca', () => {
    const deposit = coin(1000000, 'ukuji');
    const swapAdjustment = 1.8;

    let vault: Vault;
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;

    before(async function (this: Context) {
      balancesBeforeExecution = await getBalances(
        this.cosmWasmClient,
        [this.userWalletAddress, this.feeCollectorAddress],
        ['udemo'],
      );

      for (const position_type of ['enter', 'exit']) {
        await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
          update_swap_adjustments: {
            position_type,
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
          swap_amount: `${Math.round(parseInt(deposit.amount) * (2 / 3))}`,
          time_interval: 'every_second',
          use_dca_plus: true,
        },
        [deposit],
      );

      while (dayjs().isBefore(targetTime)) {
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

      balancesAfterExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['udemo']);
    });

    it('subtracts the escrowed balance from the disbursed amount', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['udemo']).to.equal(
        Math.round(
          balancesBeforeExecution[this.userWalletAddress]['udemo'] +
            parseInt(vault.received_amount.amount) -
            parseInt(vault.dca_plus_config.escrowed_balance.amount),
        ),
      );
    });

    it('stores the escrowed balance', async function (this: Context) {
      expect(vault.dca_plus_config.escrowed_balance.amount).to.equal(
        `${Math.floor(parseInt(vault.received_amount.amount) * parseFloat(vault.dca_plus_config.escrow_level))}`,
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
          ['udemo'],
        );

        performanceFee = Math.floor(
          (parseInt(vault.received_amount.amount) -
            parseInt(vault.dca_plus_config.standard_dca_received_amount.amount)) *
            0.2,
        );
      });

      it('empties the escrow balance', () => expect(vault.dca_plus_config.escrowed_balance.amount).to.equal('0'));

      it('pays out the escrow', function (this: Context) {
        expect(balancesAfterExecution[this.userWalletAddress]['udemo']).to.equal(
          balancesBeforeExecution[this.userWalletAddress]['udemo'] +
            parseInt(vault.received_amount.amount) -
            performanceFee,
        );
      });

      it('pays out the performance fee', function (this: Context) {
        expect(balancesAfterExecution[this.feeCollectorAddress]['udemo']).to.equal(
          balancesBeforeExecution[this.feeCollectorAddress]['udemo'] + performanceFee,
        );
      });
    });
  });

  describe('with finished standard dca and unfinished dca plus', () => {
    const deposit = coin(1000000, 'ukuji');
    const swapAdjustment = 0.8;

    let vault: Vault;
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;

    before(async function (this: Context) {
      balancesBeforeExecution = await getBalances(
        this.cosmWasmClient,
        [this.userWalletAddress, this.feeCollectorAddress],
        ['udemo'],
      );

      for (const position_type of ['enter', 'exit']) {
        await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
          update_swap_adjustments: {
            position_type,
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
          use_dca_plus: true,
        },
        [deposit],
      );

      while (dayjs().isBefore(targetTime)) {
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

      balancesAfterExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['udemo']);
    });

    it('subtracts the escrowed balance from the disbursed amount', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['udemo']).to.equal(
        Math.round(
          balancesBeforeExecution[this.userWalletAddress]['udemo'] +
            parseInt(vault.received_amount.amount) -
            parseInt(vault.dca_plus_config.escrowed_balance.amount),
        ),
      );
    });

    it('stores the escrowed balance', async function (this: Context) {
      expect(vault.dca_plus_config.escrowed_balance.amount).to.equal(
        `${Math.floor(parseInt(vault.received_amount.amount) * parseFloat(vault.dca_plus_config.escrow_level))}`,
      );
    });

    it('has swapped all the standard vault balance', () => {
      expect(vault.dca_plus_config.standard_dca_swapped_amount.amount).to.equal(deposit.amount);
    });

    it('has not swapped all the dca plus vault balance', () =>
      expect(parseInt(vault.swapped_amount.amount)).to.equal(parseInt(deposit.amount) * swapAdjustment));

    it('vault is still active', () => expect(vault.status).to.equal('active'));

    it('still has a time trigger', () =>
      expect(vault.trigger).to.eql({
        time: { target_time: 'time' in vault.trigger && vault.trigger.time.target_time },
      }));

    describe('once dca plus finishes', () => {
      let performanceFee: number;

      before(async function (this: Context) {
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
          ['udemo'],
        );

        performanceFee = Math.floor(
          (parseInt(vault.received_amount.amount) -
            parseInt(vault.dca_plus_config.standard_dca_received_amount.amount)) *
            0.2,
        );
      });

      it('has swapped all the balance', () => {
        expect(vault.swapped_amount.amount).to.equal(deposit.amount);
      });

      it('empties the escrow balance', () => expect(vault.dca_plus_config.escrowed_balance.amount).to.equal('0'));

      it('pays out the escrow', function (this: Context) {
        expect(balancesAfterExecution[this.userWalletAddress]['udemo']).to.equal(
          balancesBeforeExecution[this.userWalletAddress]['udemo'] +
            parseInt(vault.received_amount.amount) -
            performanceFee,
        );
      });

      it('pays out the performance fee', function (this: Context) {
        expect(balancesAfterExecution[this.feeCollectorAddress]['udemo']).to.equal(
          balancesBeforeExecution[this.feeCollectorAddress]['udemo'] + performanceFee,
        );
      });

      it('sets the vault to inactive', () => expect(vault.status).to.equal('inactive'));

      it('does not have a trigger', () => expect(vault.trigger).to.equal(null));
    });
  });
});
