import { expect } from 'chai';
import { Context } from 'mocha';
import { execute } from '../../shared/cosmwasm';
import { Vault } from '../../types/dca/response/get_vault';
import { createVault, getBalances } from '../helpers';

describe('when migrating a price trigger', () => {
  const swapAmount = 100000;
  const targetPrice = 0.5;
  let vaultBeforeMigration: Vault;
  let vaultAfterMigration: Vault;
  let balancesBeforeExecution: Record<string, number>;
  let balancesAfterExecution: Record<string, number>;

  before(async function (this: Context) {
    const vaultId = await createVault(this, {
      swap_amount: `${swapAmount}`,
      target_receive_amount: `${Math.round(swapAmount / targetPrice)}`,
    });

    vaultBeforeMigration = (
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
      migrate_price_trigger: {
        vault_id: vaultId,
      },
    });

    vaultAfterMigration = (
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
  });

  it('updates the DCA contract balance', async function (this: Context) {
    expect(balancesAfterExecution[this.dcaContractAddress]['udemo']).to.equal(
      balancesBeforeExecution[this.dcaContractAddress]['udemo'] + swapAmount - 2,
    );
  });

  it('updates the vault balance', async function (this: Context) {
    expect(vaultBeforeMigration.balance.amount).to.equal(`${parseInt(vaultAfterMigration.balance.amount) + 2}`);
  });

  it('updates the price trigger order idx', async function (this: Context) {
    'fin_limit_order' in vaultBeforeMigration.trigger &&
      'fin_limit_order' in vaultAfterMigration.trigger &&
      expect(vaultBeforeMigration.trigger.fin_limit_order.order_idx).to.not.equal(
        vaultAfterMigration.trigger.fin_limit_order.order_idx,
      );
  });
});
