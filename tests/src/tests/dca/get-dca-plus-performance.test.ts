import { Coin, coin } from '@cosmjs/proto-signing';
import dayjs from 'dayjs';
import { Context } from 'mocha';
import { createVault } from '../helpers';
import { expect } from '../shared.test';
import { Vault } from '../../types/dca/response/get_vault';

describe('when fetching risk weighted average swap adjustment strategy performance', () => {
  describe('for a vault with no executions', () => {
    let deposit: Coin;
    let performance: any;

    before(async function (this: Context) {
      deposit = coin(1000, this.pair.quote_denom);
      const vault_id = await createVault(
        this,
        {
          target_start_time_utc_seconds: `${dayjs().add(1, 'hour').unix()}`,
          swap_adjustment_strategy: {
            risk_weighted_average: {
              base_denom: 'bitcoin',
            },
          },
          performance_assessment_strategy: 'compare_to_standard_dca',
        },
        [deposit],
      );

      performance = await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
        get_vault_performance: { vault_id },
      });
    });

    it('has an empty performance fee', async function (this: Context) {
      expect(performance.fee).to.deep.equal(coin(0, this.pair.base_denom));
    });

    it('has an even performance factor', async function (this: Context) {
      expect(performance.factor).to.equal('1');
    });
  });

  describe('for a vault with one execution', () => {
    let vault: Vault;
    let performance: any;

    before(async function (this: Context) {
      const vault_id = await createVault(this, {
        swap_adjustment_strategy: {
          risk_weighted_average: {
            base_denom: 'bitcoin',
          },
        },
        performance_assessment_strategy: 'compare_to_standard_dca',
      });

      vault = await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
        get_vault: { vault_id },
      });

      performance = await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
        get_vault_performance: { vault_id },
      });
    });

    it('has a performance fee', () => expect(Number(performance.fee.amount)).to.be.approximately(10, 2));

    it('has slightly positive performance factor', () =>
      expect(Number(performance.factor)).to.be.approximately(1.000042002184113573, 0.0001));
  });
});
