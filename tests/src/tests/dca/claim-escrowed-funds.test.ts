import { coin } from '@cosmjs/proto-signing';
import { Context } from 'mocha';
import { map } from 'ramda';
import { execute } from '../../shared/cosmwasm';
import { EventData } from '../../types/dca/response/get_events';
import { Vault } from '../../types/dca/response/get_vault';
import { createVault, getBalances } from '../helpers';
import { expect } from '../shared.test';

describe('when claiming escrowed funds', () => {
  describe('with dca plus & no trigger', () => {
    let deposit = coin(1000000, 'ukuji');
    let vaultBeforeExecution: Vault;
    let vaultAfterExecution: Vault;
    let eventPayloads: EventData[];
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;

    before(async function (this: Context) {
      const vault_id = await createVault(this, { use_dca_plus: true }, [deposit]);

      balancesBeforeExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['udemo']);

      vaultBeforeExecution = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: { vault_id },
        })
      ).vault;

      await execute(this.userCosmWasmClient, this.userWalletAddress, this.dcaContractAddress, {
        claim_escrowed_funds: { vault_id },
      });

      balancesAfterExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['udemo']);

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
      expect(vaultAfterExecution.dca_plus_config.escrowed_balance).to.equal('0');
    });

    it('sends the funds back to the user', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['udemo']).to.equal(
        balancesBeforeExecution[this.userWalletAddress]['udemo'] +
          parseInt(vaultBeforeExecution.dca_plus_config.escrowed_balance),
      );
    });
  });
});
