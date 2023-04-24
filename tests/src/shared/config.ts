export type Config = {
  chainName: string;
  bech32AddressPrefix: string;
  netUrl: string;
  gasPrice: number;
  feeDenom: string;
  adminContractMnemonic: string;
};

export const fetchConfig = async (): Promise<Config> => {
  if (process.env.CHAIN_NAME === undefined) {
    throw new Error('Missing CHAIN_NAME environment variable');
  }
  if (process.env.BECH32_ADDRESS_PREFIX === undefined) {
    throw new Error('Missing BECH32_ADDRESS_PREFIX environment variable');
  }
  if (process.env.NET_URL === undefined) {
    throw new Error('Missing NET_URL environment variable');
  }
  if (process.env.GAS_PRICE === undefined) {
    throw new Error('Missing GAS_PRICE environment variable');
  }
  if (process.env.FEE_DENOM === undefined) {
    throw new Error('Missing FEE_DENOM environment variable');
  }
  if (process.env.ADMIN_CONTRACT_MNEMONIC === undefined) {
    throw new Error('Missing ADMIN_CONTRACT_MNEMONIC environment variable');
  }

  return {
    chainName: process.env.CHAIN_NAME,
    bech32AddressPrefix: process.env.BECH32_ADDRESS_PREFIX,
    netUrl: process.env.NET_URL,
    feeDenom: process.env.FEE_DENOM,
    gasPrice: Number(process.env.GAS_PRICE),
    adminContractMnemonic: process.env.ADMIN_CONTRACT_MNEMONIC,
  };
};
