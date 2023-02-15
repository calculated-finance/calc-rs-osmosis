import { coin } from '@cosmjs/proto-signing';
import { Context } from 'mocha';
import { flatten, map, range } from 'ramda';
import { execute } from '../../shared/cosmwasm';
import { instantiateFinPairContract, instantiateSwapContract } from '../hooks';
import { expect } from '../shared.test';

describe('when fetching prices', () => {
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

  it('returns an actual quote price', async function (this: Context) {
    const price = await this.cosmWasmClient.queryContractSmart(swapContractAddress, {
      get_price: {
        swap_amount: coin(300, quoteDenom),
        target_denom: baseDenom,
        price_type: 'actual',
      },
    });

    expect(price).to.equal(`${300 / (200 / 2 + 100 / 4)}`);
  });

  it('returns a belief quote price', async function (this: Context) {
    const price = await this.cosmWasmClient.queryContractSmart(swapContractAddress, {
      get_price: {
        swap_amount: coin(300, quoteDenom),
        target_denom: baseDenom,
        price_type: 'belief',
      },
    });

    expect(price).to.equal(`${300 / (300 / 2)}`);
  });

  it('returns an actual base price', async function (this: Context) {
    const price = await this.cosmWasmClient.queryContractSmart(swapContractAddress, {
      get_price: {
        swap_amount: coin(300, baseDenom),
        target_denom: quoteDenom,
        price_type: 'actual',
      },
    });

    expect(price).to.equal(`${300 / (200 / 2 + 100 / 4)}`);
  });

  it('returns an belief base price', async function (this: Context) {
    const price = await this.cosmWasmClient.queryContractSmart(swapContractAddress, {
      get_price: {
        swap_amount: coin(300, baseDenom),
        target_denom: quoteDenom,
        price_type: 'belief',
      },
    });

    expect(price).to.equal(`${300 / (300 / 2)}`);
  });
});
