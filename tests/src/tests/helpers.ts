import { CosmWasmClient, SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { GenericAuthorization } from 'cosmjs-types/cosmos/authz/v1beta1/authz';
import { MsgGrant } from 'cosmjs-types/cosmos/authz/v1beta1/tx';
import { coin, Coin, DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice, QueryClient } from '@cosmjs/stargate';
import dayjs, { Dayjs } from 'dayjs';
import { Context } from 'mocha';
import { indexBy, map, mergeAll, omit, pipe, prop, reduce, reverse, sum } from 'ramda';
import { Config } from '../shared/config';
import { execute } from '../shared/cosmwasm';
import { Addr } from '../types/dca/execute';
import { EventsResponse } from '../types/dca/response/get_events';
import { Timestamp } from 'cosmjs-types/google/protobuf/timestamp';
import { Pair } from '../types/dca/response/get_pairs';
import Long from 'long';

export const createWallet = async (config: Config) =>
  await DirectSecp256k1HdWallet.generate(12, {
    prefix: config.bech32AddressPrefix,
  });

export const createCosmWasmClientForWallet = async (
  config: Config,
  adminCosmWasmClient: SigningCosmWasmClient,
  adminContractAddress: Addr,
  userWallet: DirectSecp256k1HdWallet,
): Promise<SigningCosmWasmClient> => {
  const userCosmWasmClient = await SigningCosmWasmClient.connectWithSigner(config.netUrl, userWallet, {
    prefix: config.bech32AddressPrefix,
    gasPrice: GasPrice.fromString(`${config.gasPrice}${config.feeDenom}`),
  });

  const [userAccount] = await userWallet.getAccounts();
  await adminCosmWasmClient.sendTokens(
    adminContractAddress,
    userAccount.address,
    [coin(1000000, 'uion'), coin(1000000, 'uosmo')],
    'auto',
  );

  return userCosmWasmClient;
};

export const createVault = async (
  context: Context,
  overrides: Record<string, unknown> = {},
  deposit: Coin[] = [coin('1000000', 'stake')],
) => {
  if (deposit.length > 0)
    await context.cosmWasmClient.sendTokens(context.adminContractAddress, context.userWalletAddress, deposit, 'auto');

  const response = await execute(
    context.userCosmWasmClient,
    context.userWalletAddress,
    context.dcaContractAddress,
    {
      create_vault: {
        label: 'test',
        swap_amount: '100000',
        pair_address: context.pair.address,
        time_interval: 'hourly',
        ...overrides,
      },
    },
    deposit,
  );

  return response['wasm']['vault_id'];
};

export const getBalances = async (
  cosmWasmClient: SigningCosmWasmClient,
  addresses: Addr[],
  denoms: string[] = ['uion', 'uosmo', 'stake'],
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
                  [denom]: Number((await cosmWasmClient.getBalance(address, denom)).amount),
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
) => {
  const secondsInOneYear = 31536000;
  const message = {
    typeUrl: '/cosmos.authz.v1beta1.MsgGrant',
    value: {
      granter,
      grantee,
      grant: {
        authorization: {
          typeUrl: '/cosmos.authz.v1beta1.GenericAuthorization',
          value: GenericAuthorization.encode(GenericAuthorization.fromPartial({ msg })).finish(),
        },
        expiration: Timestamp.fromPartial({
          seconds: dayjs().toDate().getTime() / 1000 + secondsInOneYear,
          nanos: 0,
        }),
      },
    } as MsgGrant,
  };

  return await client.signAndBroadcast(granter, [message], 'auto', 'creating authz grant for staking to BOW');
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
