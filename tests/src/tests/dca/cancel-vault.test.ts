import dayjs from 'dayjs';
import { Context } from 'mocha';
import { map } from 'ramda';
import { execute } from '../../shared/cosmwasm';
import { EventData } from '../../types/dca/response/get_events';
import { Vault } from '../../types/dca/response/get_vault';
import { createVault, getBalances } from '../helpers';
import { expect } from '../shared.test';

describe('when cancelling a vault', () => {
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
      expect(balancesAfterExecution[this.userWalletAddress]['stake']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['stake'] + parseInt(vaultBeforeExecution.balance.amount),
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
      expect(balancesAfterExecution[this.userWalletAddress]['stake']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['stake'] + parseInt(vaultBeforeExecution.balance.amount),
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

  // describe('with dca plus', async () => {
  //   const swapAmount = 100000;
  //   let vaultBeforeExecution: Vault;
  //   let vaultAfterExecution: Vault;
  //   let eventPayloadsAfterExecution: EventData[];
  //   let balancesBeforeExecution: Record<string, number>;
  //   let balancesAfterExecution: Record<string, number>;

  //   before(async function (this: Context) {
  //     const vaultId = await createVault(this, {
  //       swap_amount: `${swapAmount}`,
  //       use_dca_plus: true,
  //       time_interval: 'every_second',
  //     });

  //     vaultBeforeExecution = (
  //       await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
  //         get_vault: {
  //           vault_id: vaultId,
  //         },
  //       })
  //     ).vault;

  //     balancesBeforeExecution = await getBalances(this.cosmWasmClient, [
  //       this.userWalletAddress,
  //       this.dcaContractAddress,
  //       this.feeCollectorAddress,
  //     ]);

  //     await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
  //       cancel_vault: {
  //         vault_id: vaultId,
  //       },
  //     });

  //     vaultAfterExecution = (
  //       await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
  //         get_vault: {
  //           vault_id: vaultId,
  //         },
  //       })
  //     ).vault;

  //     balancesAfterExecution = await getBalances(this.cosmWasmClient, [
  //       this.userWalletAddress,
  //       this.dcaContractAddress,
  //       this.feeCollectorAddress,
  //     ]);

  //     eventPayloadsAfterExecution = map(
  //       (event) => event.data,
  //       (
  //         await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
  //           get_events_by_resource_id: { resource_id: vaultId },
  //         })
  //       ).events,
  //     );
  //   });

  //   it('empties the vault balance', async function (this: Context) {
  //     expect(vaultAfterExecution.balance.amount).to.equal('0');
  //   });

  //   it('sends vault balance back to the user', async function (this: Context) {
  //     expect(balancesAfterExecution[this.userWalletAddress]['uion']).to.equal(
  //       balancesBeforeExecution[this.userWalletAddress]['uion'] + parseInt(vaultBeforeExecution.balance.amount),
  //     );
  //   });

  //   it('is cancelled', () => expect(vaultAfterExecution.status).to.equal('cancelled'));

  //   it('has a vault cancelled event', function (this: Context) {
  //     expect(eventPayloadsAfterExecution).to.include.deep.members([
  //       {
  //         dca_vault_cancelled: {},
  //       },
  //     ]);
  //   });

  //   it('removes the trigger', () => expect(vaultAfterExecution.trigger).to.be.null);

  //   it('creates a disburse escrow task', async function (this: Context) {
  //     let tasks = (
  //       await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
  //         get_disburse_escrow_tasks: {},
  //       })
  //     ).vault_ids;

  //     while (isEmpty(tasks)) {
  //       await setTimeout(1000);

  //       tasks = (
  //         await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
  //           get_disburse_escrow_tasks: {},
  //         })
  //       ).vault_ids;
  //     }

  //     expect(tasks).to.include(vaultBeforeExecution.id);
  //   });

  //   describe('after the disburse escrow task is completed', () => {
  //     let amountDisbursed: number;
  //     let performanceFee: number;

  //     before(async function (this: Context) {
  //       balancesBeforeExecution = await getBalances(this.cosmWasmClient, [
  //         this.userWalletAddress,
  //         this.dcaContractAddress,
  //         this.feeCollectorAddress,
  //       ]);

  //       await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
  //         disburse_escrow: {
  //           vault_id: vaultBeforeExecution.id,
  //         },
  //       });

  //       vaultAfterExecution = (
  //         await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
  //           get_vault: {
  //             vault_id: vaultBeforeExecution.id,
  //           },
  //         })
  //       ).vault;

  //       balancesAfterExecution = await getBalances(this.cosmWasmClient, [
  //         this.userWalletAddress,
  //         this.dcaContractAddress,
  //         this.feeCollectorAddress,
  //       ]);

  //       let escrow_disbursed_event = filter(
  //         (event: Event) => 'dca_vault_escrow_disbursed' in event.data,
  //         (
  //           await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
  //             get_events_by_resource_id: { resource_id: vaultBeforeExecution.id },
  //           })
  //         ).events,
  //       )[0].data.dca_vault_escrow_disbursed;

  //       amountDisbursed = parseInt(escrow_disbursed_event.amount_disbursed.amount);
  //       performanceFee = parseInt(escrow_disbursed_event.performance_fee.amount);
  //     });

  //     it('empties the escrow balance', () =>
  //       expect(vaultAfterExecution.dca_plus_config.escrowed_balance.amount).to.equal('0'));

  //     it('pays out the escrow', function (this: Context) {
  //       expect(balancesAfterExecution[this.userWalletAddress]['uosmo']).to.equal(
  //         balancesBeforeExecution[this.userWalletAddress]['uosmo'] + amountDisbursed,
  //       );
  //     });

  //     it('pays out the performance fee', function (this: Context) {
  //       expect(balancesAfterExecution[this.feeCollectorAddress]['uosmo']).to.equal(
  //         balancesBeforeExecution[this.feeCollectorAddress]['uosmo'] + performanceFee,
  //       );
  //     });
  //   });
  // });
});
