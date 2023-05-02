import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Coin, DirectSecp256k1HdWallet, GeneratedType, Registry } from '@cosmjs/proto-signing';
import { GasPrice, Attribute, Event } from '@cosmjs/stargate';
import dayjs from 'dayjs';
import { reduce, assoc } from 'ramda';
import { Config } from './config';
import RelativeTime from 'dayjs/plugin/relativeTime';
import fs from 'fs';
import { getOfflineSignerProto } from 'cosmjs-utils';
import { ExecuteMsg } from '../types/dca/execute';
import { cosmosProtoRegistry, cosmwasmProtoRegistry, ibcProtoRegistry, osmosisProtoRegistry } from 'osmojs';
import { FEE } from '../tests/constants';
dayjs.extend(RelativeTime);

export const getWallet = async (mnemonic: string, prefix: string): Promise<DirectSecp256k1HdWallet> => {
  return await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix: prefix,
  });
};

export const createAdminCosmWasmClient = async (config: Config): Promise<SigningCosmWasmClient> => {
  const signer = await getOfflineSignerProto({
    mnemonic: config.adminContractMnemonic,
    chain: {
      bech32_prefix: config.bech32AddressPrefix,
      slip44: 118,
    },
  });

  const protoRegistry: ReadonlyArray<[string, GeneratedType]> = [
    ...cosmosProtoRegistry,
    ...cosmwasmProtoRegistry,
    ...ibcProtoRegistry,
    ...osmosisProtoRegistry,
  ];

  return await SigningCosmWasmClient.connectWithSigner(config.netUrl, signer, {
    prefix: config.bech32AddressPrefix,
    gasPrice: GasPrice.fromString(`${config.gasPrice}${config.feeDenom}`),
    registry: new Registry(protoRegistry),
  });
};

export const execute = async (
  cosmWasmClient: SigningCosmWasmClient,
  senderAddress: string,
  contractAddress: string,
  message: ExecuteMsg,
  funds: Coin[] = [],
): Promise<Record<string, unknown>> => {
  const response = await cosmWasmClient.execute(senderAddress, contractAddress, message, FEE, 'memo', funds);
  return parseEventAttributes(response.logs[0].events);
};

export const parseEventAttributes = (events: readonly Event[]): Record<string, Record<string, string>> =>
  reduce(
    (obj: object, event: Event) => ({
      [event.type]: reduce((obj: object, attr: Attribute) => assoc(attr.key, attr.value, obj), {}, event.attributes),
      ...obj,
    }),
    {},
    events,
  );

export const dayFromCosmWasmUnix = (unix: number) => dayjs(unix / 1000000);

export const uploadAndInstantiate = async (
  binaryFilePath: string,
  cosmWasmClient: SigningCosmWasmClient,
  adminAddress: string,
  initMsg: Record<string, unknown>,
  label: string,
  funds: Coin[] = [],
): Promise<string> => {
  const { codeId } = await cosmWasmClient.upload(adminAddress, fs.readFileSync(binaryFilePath), FEE);
  const { contractAddress } = await cosmWasmClient.instantiate(adminAddress, codeId, initMsg, label, FEE, {
    funds,
    admin: adminAddress,
  });
  return contractAddress;
};

export const uploadAndMigrate = async (
  binaryFilePath: string,
  cosmWasmClient: SigningCosmWasmClient,
  adminAddress: string,
  contractAddress: string,
  migrateMsg: Record<string, unknown>,
): Promise<void> => {
  const { codeId } = await cosmWasmClient.upload(adminAddress, fs.readFileSync(binaryFilePath), FEE);
  await cosmWasmClient.migrate(adminAddress, contractAddress, codeId, migrateMsg, FEE);
};
