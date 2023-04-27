import { coin } from '@cosmjs/proto-signing';
import dayjs from 'dayjs';
import { Context } from 'mocha';
import { createVault } from '../helpers';
import { expect } from '../shared.test';

describe('when fetching dca plus performance', () => {
  describe('for a vault with no executions', () => {
    let deposit = coin(1000000, 'stake');
    let performance: any;

    before(async function (this: Context) {
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
      expect(performance.fee).to.deep.equal(coin(0, 'uion'));
    });

    it('has an even performance factor', async function (this: Context) {
      expect(performance.factor).to.equal('1');
    });
  });

  describe('for a vault with one execution', () => {
    let deposit = coin(1000000, 'stake');
    let performance: any;

    before(async function (this: Context) {
      const vault_id = await createVault(
        this,
        {
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

    it('has a performance fee', async function (this: Context) {
      expect(performance.fee).to.deep.equal(coin(10, 'uion'));
    });

    it('has slightly positive performance factor', async function (this: Context) {
      expect(Number(performance.factor)).to.be.approximately(1.000042002184113573, 0.0001);
    });
  });
});
