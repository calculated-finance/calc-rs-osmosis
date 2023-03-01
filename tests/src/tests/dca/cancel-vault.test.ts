import { coin } from '@cosmjs/proto-signing';
import dayjs from 'dayjs';
import { Context } from 'mocha';
import { map } from 'ramda';
import { execute } from '../../shared/cosmwasm';
import { EventData } from '../../types/dca/response/get_events';
import { Vault } from '../../types/dca/response/get_vault';
import { createVault, getBalances, provideAuthGrant } from '../helpers';
import { expect } from '../shared.test';

describe('when cancelling a vault', () => {
  describe('with an unfilled limit order trigger', async () => {
    const swapAmount = 100000;
    const targetPrice = 0.5;
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let eventPayloadsAfterExecution: EventData[];
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;

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

      balancesBeforeExecution = await getBalances(this.cosmWasmClient, [
        this.userWalletAddress,
        this.dcaContractAddress,
        this.finPairAddress,
        this.feeCollectorAddress,
      ]);

      await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
        cancel_vault: {
          vault_id: vaultId,
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

    it('empties the vault balance', async function (this: Context) {
      expect(vaultAfterExecution.balance.amount).to.equal('0');
    });

    it('sends vault balance back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['udemo']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['udemo'] + parseInt(vaultBeforeExecution.balance.amount) + 2,
      );
    });

    it('is cancelled', () => expect(vaultAfterExecution.status).to.equal('cancelled'));

    it('removes the trigger', () => expect(vaultAfterExecution.trigger).to.be.null);

    it('has a vault cancelled event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_cancelled: {},
        },
      ]);
    });
  });

  describe('with a time trigger', async () => {
    const swapAmount = 100000;
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let eventPayloadsAfterExecution: EventData[];
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;

    before(async function (this: Context) {
      const vaultId = await createVault(this, {
        swap_amount: `${swapAmount}`,
        target_start_time_utc_seconds: `${dayjs().add(1, 'hour').unix()}`,
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

      await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
        cancel_vault: {
          vault_id: vaultId,
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

    it('empties the vault balance', async function (this: Context) {
      expect(vaultAfterExecution.balance.amount).to.equal('0');
    });

    it('sends vault balance back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['udemo']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['udemo'] + parseInt(vaultBeforeExecution.balance.amount),
      );
    });

    it('is cancelled', () => expect(vaultAfterExecution.status).to.equal('cancelled'));

    it('removes the trigger', () => expect(vaultAfterExecution.trigger).to.be.null);

    it('has a vault cancelled event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_cancelled: {},
        },
      ]);
    });
  });

  describe('with no trigger', async () => {
    const swapAmount = 100000;
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let eventPayloadsAfterExecution: EventData[];
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;

    before(async function (this: Context) {
      const vaultId = await createVault(this, {
        swap_amount: `${swapAmount}`,
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

      await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
        cancel_vault: {
          vault_id: vaultId,
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

    it('empties the vault balance', async function (this: Context) {
      expect(vaultAfterExecution.balance.amount).to.equal('0');
    });

    it('sends vault balance back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['udemo']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['udemo'] + parseInt(vaultBeforeExecution.balance.amount),
      );
    });

    it('is cancelled', () => expect(vaultAfterExecution.status).to.equal('cancelled'));

    it('has a vault cancelled event', function (this: Context) {
      expect(eventPayloadsAfterExecution).to.include.deep.members([
        {
          dca_vault_cancelled: {},
        },
      ]);
    });
  });
});
