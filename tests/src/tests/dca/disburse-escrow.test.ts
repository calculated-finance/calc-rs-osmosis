import { coin } from '@cosmjs/proto-signing';
import { Context } from 'mocha';
import { map } from 'ramda';
import { execute } from '../../shared/cosmwasm';
import { EventData } from '../../types/dca/response/get_events';
import { Vault } from '../../types/dca/response/get_vault';
import { createVault, getBalances } from '../helpers';
import { expect } from '../shared.test';

describe('when disbursing escrow', () => {
  describe('with dca plus & no trigger', () => {
    let deposit = coin(1000000, 'uosmo');
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let eventPayloads: EventData[];
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;
    let performanceFee: number;

    before(async function (this: Context) {
      const vault_id = await createVault(this, { swap_amount: deposit.amount, use_dca_plus: true }, [deposit]);

      balancesBeforeExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['uion']);

      vaultBeforeExecution = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: { vault_id },
        })
      ).vault;

      performanceFee = Number(
        (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_dca_plus_performance: { vault_id },
          })
        ).fee.amount,
      );

      await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
        disburse_escrow: { vault_id },
      });

      balancesAfterExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['uion']);

      vaultAfterExecution = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: { vault_id },
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

    it('empties the escrowed balance', async function (this: Context) {
      expect(vaultAfterExecution.dca_plus_config.escrowed_balance.amount).to.equal('0');
    });

    it('sends the funds back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['uion']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['uion'] +
          Number(vaultBeforeExecution.dca_plus_config.escrowed_balance.amount) -
          performanceFee,
      );
    });
  });
});
