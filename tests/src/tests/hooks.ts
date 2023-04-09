import dotenv from 'dotenv';
import { fetchConfig } from '../shared/config';
import { createAdminCosmWasmClient, execute, getWallet, uploadAndInstantiate } from '../shared/cosmwasm';
import { coin } from '@cosmjs/proto-signing';
import { createCosmWasmClientForWallet, createWallet } from './helpers';
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { cosmos, FEES, osmosis } from 'osmojs';
import { getPoolsPricesPairs } from '@cosmology/core';
import { find, reverse } from 'ramda';
import { Pair } from '../types/dca/response/get_pairs';
import Long from 'long';

const calcSwapFee = 0.0005;
const automationFee = 0.0075;
const osmosisSwapFee = 0.001;
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

  const dcaContractAddress = await instantiateDCAContract(
    cosmWasmClient,
    queryClient,
    adminContractAddress,
    feeCollectorAddress,
  );

  const stakingRouterContractAddress = await instantiateStakingRouterContract(
    cosmWasmClient,
    adminContractAddress,
    dcaContractAddress,
  );

  const userWallet = await createWallet(config);
  const userWalletAddress = (await userWallet.getAccounts())[0].address;
  const userCosmWasmClient = await createCosmWasmClientForWallet(
    config,
    cosmWasmClient,
    adminContractAddress,
    userWallet,
  );

  await cosmWasmClient.sendTokens(
    adminContractAddress,
    userWalletAddress,
    [coin(100000000, 'stake'), coin(100000000, 'uion'), coin(100000000, 'uosmo')],
    FEES.osmosis.swapExactAmountIn('medium'),
  );

  const contractPairs = (
    await cosmWasmClient.queryContractSmart(dcaContractAddress, {
      get_pairs: {},
    })
  ).pairs;

  const pair = find((pair: Pair) => pair.base_denom == 'stake' && pair.quote_denom == 'uion', reverse(contractPairs));

  return {
    beforeAll(this: Mocha.Context) {
      const context = {
        config,
        cosmWasmClient,
        userCosmWasmClient,
        queryClient,
        dcaContractAddress,
        calcSwapFee,
        automationFee,
        adminContractAddress,
        feeCollectorAddress,
        userWalletAddress,
        stakingRouterContractAddress,
        osmosisSwapFee,
        pair,
        validatorAddress,
        swapAdjustment,
      };

      Object.assign(this, context);
    },
  };
};

const instantiateDCAContract = async (
  cosmWasmClient: SigningCosmWasmClient,
  queryClient: any,
  adminContractAddress: string,
  feeCollectorAdress: string,
): Promise<string> => {
  const dcaContractAddress = await uploadAndInstantiate(
    '../artifacts/dca.wasm',
    cosmWasmClient,
    adminContractAddress,
    {
      admin: adminContractAddress,
      delegation_fee_percent: `${automationFee}`,
      fee_collectors: [{ address: feeCollectorAdress, allocation: '1.0' }],
      page_limit: 1000,
      paused: false,
      staking_router_address: adminContractAddress,
      swap_fee_percent: `${calcSwapFee}`,
      dca_plus_escrow_level: '0.05',
    },
    'dca',
  );

  const { pools } = await getPoolsPricesPairs(queryClient);

  for (const pool of pools) {
    await execute(cosmWasmClient, adminContractAddress, dcaContractAddress, {
      create_pair: {
        address: pool.address,
        base_denom: pool.poolAssets[0].token.denom,
        quote_denom: pool.poolAssets[1].token.denom,
        route: [pool.id.low],
      },
    });
  }

  for (const position_type of ['enter', 'exit']) {
    await execute(cosmWasmClient, adminContractAddress, dcaContractAddress, {
      update_swap_adjustments: {
        position_type,
        adjustments: [
          [30, `${swapAdjustment}`],
          [35, `${swapAdjustment}`],
          [40, `${swapAdjustment}`],
          [45, `${swapAdjustment}`],
          [50, `${swapAdjustment}`],
          [55, `${swapAdjustment}`],
          [60, `${swapAdjustment}`],
          [70, `${swapAdjustment}`],
          [80, `${swapAdjustment}`],
          [90, `${swapAdjustment}`],
        ],
      },
    });
  }

  return dcaContractAddress;
};

const instantiateStakingRouterContract = async (
  cosmWasmClient: SigningCosmWasmClient,
  adminContractAddress: string,
  dcaContractAddress: string,
): Promise<string> => {
  const address = await uploadAndInstantiate(
    '../artifacts/staking_router.wasm',
    cosmWasmClient,
    adminContractAddress,
    {
      admin: adminContractAddress,
      allowed_z_callers: [dcaContractAddress],
    },
    'staking-router',
  );

  await execute(cosmWasmClient, adminContractAddress, dcaContractAddress, {
    update_config: {
      staking_router_address: address,
    },
  });

  return address;
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
