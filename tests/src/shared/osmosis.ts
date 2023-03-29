import { getSigningOsmosisClient, osmosis } from 'osmojs';
import { Config } from './config';
import { getOfflineSignerProto as getOfflineSigner } from 'cosmjs-utils';
import { GasPrice, SigningStargateClient } from '@cosmjs/stargate';
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';

export const createAdminOsmosisClient = async (config: Config): Promise<SigningCosmWasmClient> => {
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
  // return (await getSigningOsmosisClient({ rpcEndpoint: config.netUrl, signer })) as unknown as SigningStargateClient;
};
