import dayjs from 'dayjs';
import { Context } from 'mocha';
import { execute } from '../../shared/cosmwasm';
import { Vault } from '../../types/dca/response/get_vault';
import { createVault } from '../helpers';
import { Coin, coin } from '@cosmjs/proto-signing';
import { expect } from '../shared.test';
import { EventData } from '../../types/dca/response/get_events';
import { map } from 'ramda';

describe('when depositing into a vault', () => {
  describe('with a status of scheduled', async () => {
    const swapAmount = 1000000;
    let deposit: Coin;
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let eventPayloads: EventData[];

    before(async function (this: Context) {
      deposit = coin(1000, this.pair.quote_denom);
      const vault_id = await createVault(this, {
        swap_amount: `${swapAmount}`,
        target_start_time_utc_seconds: `${dayjs().add(1, 'hour').unix()}`,
      });

      vaultBeforeExecution = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id,
          },
        })
      ).vault;

      await execute(
        this.cosmWasmClient,
        this.adminContractAddress,
        this.dcaContractAddress,
        {
          deposit: {
            address: this.userWalletAddress,
            vault_id,
          },
        },
        [deposit],
      );

      vaultAfterExecution = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id,
          },
        })
      ).vault;

      eventPayloads = map(
        (event) => event.data,
        (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_events_by_resource_id: { resource_id: vault_id },
          })
        ).events,
      );
    });

    it('should deposit into the vault', () =>
      expect(Number(vaultAfterExecution.balance.amount)).to.equal(
        Number(vaultBeforeExecution.balance.amount) + Number(deposit.amount),
      ));

    it('has a funds deposited event', () =>
      expect(eventPayloads).to.include.deep.members([{ dca_vault_funds_deposited: { amount: deposit } }]));

    it('should not change the vault status', () =>
      expect(vaultAfterExecution.status).to.equal(vaultBeforeExecution.status));
  });

  describe('with a status of inactive', async () => {
    const swapAmount = 1000000;
    let initialDeposit: Coin;
    let deposit: Coin;
    let vaultBeforeDeposit: Vault;
    let vaultAfterDeposit: Vault;

    before(async function (this: Context) {
      initialDeposit = coin(100, this.pair.quote_denom);
      deposit = coin(10000000, this.pair.quote_denom);
      const vault_id = await createVault(
        this,
        {
          swap_amount: `${swapAmount}`,
          time_interval: 'every_second',
        },
        [initialDeposit],
      );

      vaultBeforeDeposit = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id,
          },
        })
      ).vault;

      await execute(
        this.cosmWasmClient,
        this.adminContractAddress,
        this.dcaContractAddress,
        {
          deposit: {
            address: this.userWalletAddress,
            vault_id,
          },
        },
        [deposit],
      );

      vaultAfterDeposit = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id,
          },
        })
      ).vault;
    });

    it('should change the vault status', () => {
      expect(vaultAfterDeposit.status).to.equal('active');
    });

    it('should not execute the vault', () => {
      expect(Number(vaultBeforeDeposit.swapped_amount.amount)).to.equal(Number(initialDeposit.amount));
      expect(Number(vaultAfterDeposit.swapped_amount.amount)).to.equal(Number(initialDeposit.amount));
    });
  });
});
