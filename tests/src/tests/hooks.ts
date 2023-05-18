import dotenv from 'dotenv';
import { fetchConfig } from '../shared/config';
import { createAdminCosmWasmClient, execute, getWallet, uploadAndInstantiate } from '../shared/cosmwasm';
import { createCosmWasmClientForWallet, createWallet } from './helpers';
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { cosmos, osmosis } from 'osmojs';
import { find, map } from 'ramda';
import { Pair } from '../types/dca/response/get_pairs';
import { PositionType } from '../types/dca/execute';
import { Pool } from 'osmojs/types/codegen/osmosis/gamm/pool-models/balancer/balancerPool';
import { coin } from '@cosmjs/proto-signing';
import { FEE } from './constants';
import Long from 'long';

const calcSwapFee = 0.0005;
const automationFee = 0.0075;
const swapAdjustment = 1.3;

export const mochaHooks = async (): Promise<Mocha.RootHookObject> => {
  dotenv.config();
  const config = await fetchConfig();

  const queryClient = await osmosis.ClientFactory.createRPCQueryClient({ rpcEndpoint: config.netUrl });
  const validatorAddress = (
    await queryClient.cosmos.staking.v1beta1.validators({
      status: cosmos.staking.v1beta1.bondStatusToJSON(cosmos.staking.v1beta1.BondStatus.BOND_STATUS_BONDED),
    })
  ).validators[0].operatorAddress;

  const cosmWasmClient = await createAdminCosmWasmClient(config);
  const adminContractAddress = (
    await (await getWallet(config.adminContractMnemonic, config.bech32AddressPrefix)).getAccounts()
  )[0].address;

  const feeCollectorWallet = await createWallet(config);
  const feeCollectorAddress = (await feeCollectorWallet.getAccounts())[0].address;

  const userWallet = await createWallet(config);
  const userWalletAddress = (await userWallet.getAccounts())[0].address;
  const userCosmWasmClient = await createCosmWasmClientForWallet(config, userWallet);
  await cosmWasmClient.sendTokens(adminContractAddress, userWalletAddress, [coin(10000000, config.feeDenom)], FEE);

  const twapPeriod = 60;

  const dcaContractAddress = await instantiateDCAContract(
    cosmWasmClient,
    adminContractAddress,
    feeCollectorAddress,
    twapPeriod,
  );

  const denoms = ['stake', 'uion'];

  const pools = map(
    (pool: any) => osmosis.gamm.v1beta1.Pool.decode(pool.value) as Pool,
    (await queryClient.osmosis.gamm.v1beta1.pools({})).pools,
  );

  const pool = find((pool: Pool) => {
    const assets = map((asset) => asset.token.denom, pool.poolAssets);
    return assets.length == 2 && assets.includes(denoms[0]) && assets.includes(denoms[1]);
  }, pools);

  const pair: Pair = {
    base_denom: denoms[0],
    quote_denom: denoms[1],
    route: [Number(pool.id)],
  };

  await execute(cosmWasmClient, adminContractAddress, dcaContractAddress, {
    create_pair: pair,
  });

  return {
    beforeAll(this: Mocha.Context) {
      const context = {
        config,
        cosmWasmClient,
        userCosmWasmClient,
        queryClient,
        dcaContractAddress,
        calcSwapFee: 0.0005,
        automationFee: 0.0075,
        adminContractAddress,
        feeCollectorAddress,
        userWalletAddress,
        pair,
        validatorAddress,
        swapAdjustment,
        twapPeriod,
      };

      Object.assign(this, context);
    },
  };
};

const instantiateDCAContract = async (
  cosmWasmClient: SigningCosmWasmClient,
  adminContractAddress: string,
  feeCollectorAdress: string,
  twapPeriod: number,
): Promise<string> => {
  const dcaContractAddress = await uploadAndInstantiate(
    '../artifacts/dca.wasm',
    cosmWasmClient,
    adminContractAddress,
    {
      admin: adminContractAddress,
      executors: [adminContractAddress],
      automation_fee_percent: `${automationFee}`,
      fee_collectors: [{ address: feeCollectorAdress, allocation: '1.0' }],
      default_page_limit: 30,
      paused: false,
      default_slippage_tolerance: '0.05',
      twap_period: twapPeriod,
      swap_fee_percent: `${calcSwapFee}`,
      risk_weighted_average_escrow_level: '0.05',
    },
    'dca',
  );

  for (const position_type of ['enter', 'exit']) {
    await execute(cosmWasmClient, adminContractAddress, dcaContractAddress, {
      update_swap_adjustment: {
        strategy: {
          risk_weighted_average: {
            model_id: 30,
            base_denom: 'bitcoin',
            position_type: position_type as PositionType,
          },
        },
        value: `${swapAdjustment}`,
      },
    });
  }

  return dcaContractAddress;
};

export const instantiateFundCoreContract = async (
  cosmWasmClient: SigningCosmWasmClient,
  routerContractAddress: string,
  swapContractAddress: string,
  baseAsset: string = 'uusk',
): Promise<string> =>
  uploadAndInstantiate(
    '../artifacts/fund_core.wasm',
    cosmWasmClient,
    routerContractAddress,
    {
      router: routerContractAddress,
      swapper: swapContractAddress,
      base_denom: baseAsset,
    },
    'fund-core',
  );
