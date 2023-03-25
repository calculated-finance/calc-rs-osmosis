import { coin } from '@cosmjs/proto-signing';
import { Context } from 'mocha';
import { flatten, map, range } from 'ramda';
import { execute } from '../../shared/cosmwasm';
import { instantiateFinPairContract, instantiateSwapContract } from '../hooks';
import { expect } from '../shared.test';

describe.skip('when fetching paths', () => {
  let finPairAddress: string;
  let swapContractAddress: string;
  let baseDenom = 'ukuji';
  let quoteDenom = 'udemo';

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

  it('returns a path with an accurate quote price', async function (this: Context) {
    const paths = await this.cosmWasmClient.queryContractSmart(swapContractAddress, {
      get_paths: {
        swap_amount: coin(300, quoteDenom),
        target_denom: baseDenom,
      },
    });

    expect(paths[0].price).to.equal(`${300 / (200 / 2 + 100 / 4)}`);
    expect(paths[0].pairs).to.deep.equal([
      { fin: { address: finPairAddress, base_denom: baseDenom, quote_denom: quoteDenom } },
    ]);
  });

  it('returns a path with an accurate base price', async function (this: Context) {
    const paths = await this.cosmWasmClient.queryContractSmart(swapContractAddress, {
      get_paths: {
        swap_amount: coin(300, baseDenom),
        target_denom: quoteDenom,
      },
    });

    expect(paths[0].price).to.equal(`${300 / (200 / 2 + 100 / 4)}`);
    expect(paths[0].pairs).to.deep.equal([
      { fin: { address: finPairAddress, base_denom: baseDenom, quote_denom: quoteDenom } },
    ]);
  });
});
