import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Coin, DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice, Attribute, Event } from '@cosmjs/stargate';
import dayjs from 'dayjs';
import { reduce, assoc } from 'ramda';
import { Config } from './config';
import RelativeTime from 'dayjs/plugin/relativeTime';
import fs from 'fs';
import { getOfflineSignerProto as getOfflineSigner } from 'cosmjs-utils';
import { ExecuteMsg } from '../types/dca/execute';
dayjs.extend(RelativeTime);

export const getWallet = async (mnemonic: string, prefix: string): Promise<DirectSecp256k1HdWallet> => {
  return await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix: prefix,
  });
};

export const createAdminCosmWasmClient = async (config: Config): Promise<SigningCosmWasmClient> => {
  const signer = await getOfflineSigner({
    mnemonic: config.adminContractMnemonic,
    chain: {
      bech32_prefix: config.bech32AddressPrefix,
      slip44: 118,
    },
  });

  return await SigningCosmWasmClient.connectWithSigner(config.netUrl, signer, {
    prefix: config.bech32AddressPrefix,
    gasPrice: GasPrice.fromString(`${config.gasPrice}${config.feeDenom}`),
  });
};

export const execute = async (
  cosmWasmClient: SigningCosmWasmClient,
  senderAddress: string,
  contractAddress: string,
  message: ExecuteMsg,
  funds: Coin[] = [],
): Promise<Record<string, unknown>> => {
  const response = await cosmWasmClient.execute(senderAddress, contractAddress, message, 'auto', 'memo', funds);
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
  const { codeId } = await cosmWasmClient.upload(adminAddress, fs.readFileSync(binaryFilePath), 'auto');
  const { contractAddress } = await cosmWasmClient.instantiate(adminAddress, codeId, initMsg, label, 'auto', {
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
  const { codeId } = await cosmWasmClient.upload(adminAddress, fs.readFileSync(binaryFilePath), 'auto');
  await cosmWasmClient.migrate(adminAddress, contractAddress, codeId, migrateMsg, 'auto');
};
