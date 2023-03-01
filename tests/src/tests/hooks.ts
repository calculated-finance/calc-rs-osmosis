import dotenv from 'dotenv';
import { fetchConfig } from '../shared/config';
import { createAdminCosmWasmClient, execute, getWallet, uploadAndInstantiate } from '../shared/cosmwasm';
import { Coin, coin } from '@cosmjs/proto-signing';
import { createCosmWasmClientForWallet, createWallet } from './helpers';
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Tendermint34Client } from '@cosmjs/tendermint-rpc';
import { HttpBatchClient } from '@cosmjs/tendermint-rpc/build/rpcclients';
import { kujiraQueryClient } from 'kujira.js';

const calcSwapFee = 0.0165;
const automationFee = 0.0075;
const finTakerFee = 0.0015;
const finMakerFee = 0.00075;
const finBuyPrice = 1.01;
const finSellPrice = 0.99;

export const mochaHooks = async (): Promise<Mocha.RootHookObject> => {
  dotenv.config();

  const config = await fetchConfig();
  const httpClient = new HttpBatchClient(config.netUrl, {
    dispatchInterval: 100,
    batchSizeLimit: 200,
  });
  const tmClient = await Tendermint34Client.create(httpClient);
  const kujiClient = kujiraQueryClient({ client: tmClient });
  const cosmWasmClient = await createAdminCosmWasmClient(config);

  const adminContractAddress = (
    await (await getWallet(config.adminContractMnemonic, config.bech32AddressPrefix)).getAccounts()
  )[0].address;

  const feeCollectorWallet = await createWallet(config);
  const feeCollectorAddress = (await feeCollectorWallet.getAccounts())[0].address;

  const finPairAddress = await instantiateFinPairContract(cosmWasmClient, adminContractAddress);

  const dcaContractAddress = await instantiateDCAContract(cosmWasmClient, adminContractAddress, feeCollectorAddress, [
    finPairAddress,
  ]);

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

  const validatorAddress = (await kujiClient.staking.validators('')).validators[0].operatorAddress;

  return {
    beforeAll(this: Mocha.Context) {
      const context = {
        config,
        cosmWasmClient,
        userCosmWasmClient,
        dcaContractAddress,
        calcSwapFee,
        automationFee,
        adminContractAddress,
        feeCollectorAddress,
        userWalletAddress,
        stakingRouterContractAddress,
        finPairAddress,
        finBuyPrice,
        finSellPrice,
        finMakerFee,
        finTakerFee,
        pair: {
          address: finPairAddress,
          base_denom: 'ukuji',
          quote_denom: 'udemo',
        },
        validatorAddress,
      };

      Object.assign(this, context);
    },
  };
};

const instantiateDCAContract = async (
  cosmWasmClient: SigningCosmWasmClient,
  adminContractAddress: string,
  feeCollectorAdress: string,
  pairAddress: string[] = [],
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

  for (const address of pairAddress) {
    const pair = await cosmWasmClient.queryContractSmart(address, {
      config: {},
    });

    await execute(cosmWasmClient, adminContractAddress, dcaContractAddress, {
      create_pair: {
        base_denom: pair.denoms[0].native,
        quote_denom: pair.denoms[1].native,
        address,
      },
    });
  }

  for (const position_type of ['enter', 'exit']) {
    await execute(cosmWasmClient, adminContractAddress, dcaContractAddress, {
      update_swap_adjustments: {
        position_type,
        adjustments: [
          [30, '1.3'],
          [35, '1.3'],
          [40, '1.3'],
          [45, '1.3'],
          [50, '1.3'],
          [55, '1.3'],
          [60, '1.3'],
          [70, '1.3'],
          [80, '1.3'],
          [90, '1.3'],
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

export const instantiateFinPairContract = async (
  cosmWasmClient: SigningCosmWasmClient,
  adminContractAddress: string,
  baseDenom: string = 'ukuji',
  quoteDenom: string = 'udemo',
  beliefPrice: number = 1.0,
  orders: Record<string, number | Coin>[] = [],
): Promise<string> => {
  const finContractAddress = await uploadAndInstantiate(
    '../artifacts/fin.wasm',
    cosmWasmClient,
    adminContractAddress,
    {
      owner: adminContractAddress,
      denoms: [{ native: baseDenom }, { native: quoteDenom }],
      price_precision: { decimal_places: 3 },
    },
    'fin',
  );

  await execute(cosmWasmClient, adminContractAddress, finContractAddress, {
    launch: {},
  });

  orders =
    (orders.length == 0 && [
      { price: beliefPrice + 0.01, amount: coin('1000000000000', baseDenom) },
      { price: beliefPrice - 0.01, amount: coin('1000000000000', quoteDenom) },
    ]) ||
    orders;

  for (const order of orders) {
    await execute(
      cosmWasmClient,
      adminContractAddress,
      finContractAddress,
      {
        submit_order: { price: `${order.price}` },
      },
      [order.amount as Coin],
    );
  }

  return finContractAddress;
};

export const instantiateSwapContract = async (
  cosmWasmClient: SigningCosmWasmClient,
  adminContractAddress: string,
): Promise<string> =>
  uploadAndInstantiate(
    '../artifacts/swap.wasm',
    cosmWasmClient,
    adminContractAddress,
    {
      admin: adminContractAddress,
    },
    'swap',
  );

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
