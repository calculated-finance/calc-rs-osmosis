import { Coin, coin } from '@cosmjs/proto-signing';
import { Context } from 'mocha';
import { map } from 'ramda';
import { execute } from '../../shared/cosmwasm';
import { EventData } from '../../types/dca/response/get_events';
import { Vault } from '../../types/dca/response/get_vault';
import { createVault, getBalances } from '../helpers';
import { expect } from '../shared.test';

describe('when disbursing escrow', () => {
  describe('with risk weighted average swap adjustment strategy & no trigger', () => {
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let eventPayloads: EventData[];
    let balancesBeforeExecution: { [x: string]: { address: string } };
    let balancesAfterExecution: { [x: string]: { address: string } };
    let performanceFee: number;

    before(async function (this: Context) {
      const vault_id = await createVault(this, {
        swap_adjustment_strategy: {
          risk_weighted_average: {
            base_denom: 'bitcoin',
          },
        },
        performance_assessment_strategy: 'compare_to_standard_dca',
      });

      vaultBeforeExecution = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: { vault_id },
        })
      ).vault;

      balancesBeforeExecution = await getBalances(this, [this.userWalletAddress], [vaultBeforeExecution.target_denom]);

      performanceFee = Number(
        (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_vault_performance: { vault_id },
          })
        ).fee.amount,
      );

      await execute(this.cosmWasmClient, this.adminContractAddress, this.dcaContractAddress, {
        disburse_escrow: { vault_id },
      });

      balancesAfterExecution = await getBalances(this, [this.userWalletAddress], [vaultBeforeExecution.target_denom]);

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
      expect(vaultAfterExecution.escrowed_amount.amount).to.equal('0');
    });

    it('sends the funds back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress][vaultAfterExecution.target_denom]).to.equal(
        balancesBeforeExecution[this.userWalletAddress][vaultAfterExecution.target_denom] +
          Number(vaultBeforeExecution.escrowed_amount.amount) -
          performanceFee,
      );
    });
  });
});
