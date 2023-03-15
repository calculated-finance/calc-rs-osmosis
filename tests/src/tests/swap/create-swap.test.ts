import { coin } from '@cosmjs/proto-signing';
import { Context } from 'mocha';
import { flatten, map, range } from 'ramda';
import { execute } from '../../shared/cosmwasm';
import { getBalances } from '../helpers';
import { instantiateFinPairContract, instantiateSwapContract } from '../hooks';
import { expect } from '../shared.test';

describe.skip('when creating a swap', () => {
  let finPairAddress: string;
  let swapContractAddress: string;
  let baseDenom = 'utest';
  let quoteDenom = 'udemo';
  let balancesBeforeExecution: Record<string, number>;
  let balancesAfterExecution: Record<string, number>;
  let expectedPrice: number;

  const swapAmount = coin(300, quoteDenom);

  before(async function (this: Context) {
    finPairAddress = await instantiateFinPairContract(
      this.cosmWasmClient,
      this.adminContractAddress,
      baseDenom,
      quoteDenom,
      1.0,
      flatten(
        map(
          (i) => [
            { price: i * 2, amount: coin(100, baseDenom) },
            { price: Math.round((1 / (i * 2)) * 100) / 100, amount: coin(100, quoteDenom) },
          ],
          range(1, 5),
        ),
      ),
    );

    swapContractAddress = await instantiateSwapContract(this.cosmWasmClient, this.adminContractAddress);

    await execute(this.cosmWasmClient, this.adminContractAddress, swapContractAddress, {
      add_path: {
        pair: {
          fin: { address: finPairAddress, base_denom: baseDenom, quote_denom: quoteDenom },
        },
      },
    });
  });

  describe('with no callback provided', () => {
    before(async function (this: Context) {
      await this.cosmWasmClient.sendTokens(this.adminContractAddress, this.userWalletAddress, [swapAmount], 'auto');

      balancesBeforeExecution = await getBalances(this.cosmWasmClient, [
        this.userWalletAddress,
        this.adminContractAddress,
      ]);

      let paths = await this.cosmWasmClient.queryContractSmart(swapContractAddress, {
        get_paths: {
          swap_amount: swapAmount,
          target_denom: baseDenom,
        },
      });

      expectedPrice = paths[0].price;

      await execute(
        this.userCosmWasmClient,
        this.userWalletAddress,
        swapContractAddress,
        {
          create_swap: {
            target_denom: baseDenom,
            slippage_tolerance: null,
            on_complete: null,
          },
        },
        [swapAmount],
      );

      balancesAfterExecution = await getBalances(this.cosmWasmClient, [
        this.userWalletAddress,
        this.adminContractAddress,
      ]);
    });

    it('sends the swap amount', function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress][swapAmount.denom]).to.equal(
        parseInt(balancesBeforeExecution[this.userWalletAddress][swapAmount.denom]) - parseInt(swapAmount.amount),
      );
    });

    it('receives the expected amount', function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress][baseDenom]).to.equal(
        parseInt(balancesBeforeExecution[this.userWalletAddress][baseDenom]) +
          parseInt(swapAmount.amount) / expectedPrice,
      );
    });
  });
});
