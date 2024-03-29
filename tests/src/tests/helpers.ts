import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { GenericAuthorization } from 'cosmjs-types/cosmos/authz/v1beta1/authz';
import { coin, Coin, DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice } from '@cosmjs/stargate';
import dayjs, { Dayjs } from 'dayjs';
import { Context } from 'mocha';
import { indexBy, map, mergeAll, prop, reverse, sum } from 'ramda';
import { Config } from '../shared/config';
import { execute } from '../shared/cosmwasm';
import { Addr } from '../types/dca/execute';
import { EventsResponse } from '../types/dca/response/get_events';
import { Pair } from '../types/dca/response/get_pairs';
import Long from 'long';
import { FEES, osmosis } from 'osmojs';
import { Pool } from 'osmojs/types/codegen/osmosis/gamm/pool-models/balancer/balancerPool';

export const createWallet = async (config: Config) =>
  await DirectSecp256k1HdWallet.generate(12, {
    prefix: config.bech32AddressPrefix,
  });

export const createCosmWasmClientForWallet = async (
  config: Config,
  userWallet: DirectSecp256k1HdWallet,
): Promise<SigningCosmWasmClient> =>
  await SigningCosmWasmClient.connectWithSigner(config.netUrl, userWallet, {
    prefix: config.bech32AddressPrefix,
    gasPrice: GasPrice.fromString(`${config.gasPrice}${config.feeDenom}`),
  });

export const createVault = async (
  context: Context,
  overrides: Record<string, unknown> = {},
  deposit: Coin[] = [coin(1000000, context.pair.quote_denom)],
) => {
  if (deposit.length > 0) {
    await context.cosmWasmClient.sendTokens(
      context.adminContractAddress,
      context.userWalletAddress,
      deposit,
      FEES.osmosis.swapExactAmountIn('medium'),
    );
  }

  const response = await execute(
    context.userCosmWasmClient,
    context.userWalletAddress,
    context.dcaContractAddress,
    {
      create_vault: {
        label: 'test',
        swap_amount: '100000',
        target_denom: context.pair.base_denom,
        time_interval: 'hourly',
        ...overrides,
      },
    },
    deposit,
  );

  return response['wasm']['vault_id'];
};

export const getBalances = async (
  context: Context,
  addresses: Addr[],
  denoms: string[] = [context.pair.base_denom, context.pair.quote_denom],
) => {
  return indexBy(
    prop('address'),
    await Promise.all(
      map(
        async (address) => ({
          address,
          ...mergeAll(
            await Promise.all(
              map(
                async (denom) => ({
                  [denom]: Number((await context.cosmWasmClient.getBalance(address, denom)).amount),
                }),
                denoms,
              ),
            ),
          ),
        }),
        addresses,
      ),
    ),
  );
};

export const getVaultLastUpdatedTime = async (
  cosmWasmClient: SigningCosmWasmClient,
  dcaContractAddress: Addr,
  vaultId: string,
): Promise<Dayjs> => {
  const response = (await cosmWasmClient.queryContractSmart(dcaContractAddress, {
    get_events_by_resource_id: {
      resource_id: vaultId,
    },
  })) as EventsResponse;

  return dayjs(Number(response.events.pop().timestamp) / 1000000);
};

export const provideAuthGrant = async (
  client: SigningCosmWasmClient,
  granter: string,
  grantee: string,
  msg: string,
  authType: string = '/cosmos.authz.v1beta1.GenericAuthorization',
) => {
  const secondsInOneYear = 31536000;
  const message = {
    typeUrl: '/cosmos.authz.v1beta1.MsgGrant',
    value: {
      granter,
      grantee,
      grant: {
        authorization: {
          typeUrl: authType,
          value: GenericAuthorization.encode({ msg }).finish(),
        },
        expiration: dayjs().add(secondsInOneYear, 'seconds').toDate(),
      },
    },
  };

  return await client.signAndBroadcast(
    granter,
    [message],
    FEES.osmosis.joinSwapExternAmountIn('high'),
    'creating authz grant',
  );
};

export const sendTokens = async (
  cosmWasmClient: SigningCosmWasmClient,
  fromAddess: string,
  toAddress: string,
  tokens: Coin[],
) => {
  for (const token of tokens) {
    await cosmWasmClient.sendTokens(fromAddess, toAddress, [token], 'auto');
  }
};

export const isWithinPercent = (total: number, actual: number, expected: number, percent: number) =>
  Math.abs(actual / total - expected / total) * 100 <= percent;

export const getPool = async (context: Context, poolId: number): Promise<Pool> => {
  const { pool } = await context.queryClient.osmosis.gamm.v1beta1.pool({ poolId: Long.fromNumber(poolId, true) });
  return osmosis.gamm.v1beta1.Pool.decode(pool.value) as Pool;
};

export const getTwapToNow = async (context: Context, pair: Pair, swapDenom: string, period: number) => {
  const pool = await getPool(context, pair.route[0]);
  const fee = Number(pool.poolParams.swapFee) / 10e17;

  const block = await context.cosmWasmClient.getBlock();
  const blockTime = dayjs(block.header.time);

  const twapAtomics = Number(
    (
      await context.queryClient.osmosis.twap.v1beta1.arithmeticTwapToNow({
        poolId: Long.fromNumber(pair.route[0], true),
        baseAsset: pair.base_denom == swapDenom ? pair.quote_denom : pair.base_denom,
        quoteAsset: swapDenom,
        startTime: blockTime.subtract(period, 'seconds').toDate(),
      })
    ).arithmeticTwap,
  );

  return (twapAtomics / 10e17) * (1 + fee);
};

export const getSpotPrice = async (context: Context, pair: Pair, swapDenom: string) => {
  const pool = await getPool(context, pair.route[0]);
  const fee = Number(pool.poolParams.swapFee) / 10e17;

  const spotPrice = Number(
    (
      await context.queryClient.osmosis.gamm.v2.spotPrice({
        poolId: Long.fromNumber(pair.route[0], true),
        baseAssetDenom: pair.base_denom == swapDenom ? pair.quote_denom : pair.base_denom,
        quoteAssetDenom: swapDenom,
      })
    ).spotPrice,
  );

  return spotPrice * (1 + fee);
};

export const getExpectedPrice = async (
  context: Context,
  pair: Pair,
  swapAmount: Coin,
  tokenOutDenom: string,
): Promise<number> =>
  sum(
    await Promise.all(
      map(
        async (poolId: any) => {
          poolId = Long.fromNumber(poolId, true);
          return (
            Number(swapAmount.amount) /
            Number(
              (
                await context.queryClient.osmosis.gamm.v1beta1.estimateSwapExactAmountIn({
                  sender: context.dcaContractAddress,
                  poolId,
                  tokenIn: `${swapAmount.amount}${swapAmount.denom}`,
                  routes: [
                    {
                      poolId,
                      tokenOutDenom,
                    },
                  ],
                })
              ).tokenOutAmount,
            )
          );
        },
        swapAmount.denom == pair.quote_denom ? pair.route : reverse(pair.route),
      ),
    ),
  );
