import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Config } from '../shared/config';
import { Addr } from '../types/dca/execute';
import { Pair } from '../types/dca/response/get_pairs';
import * as mocha from 'mocha';

declare module 'mocha' {
  export interface Context {
    config: Config;
    cosmWasmClient: SigningCosmWasmClient;
    userCosmWasmClient: SigningCosmWasmClient;
    dcaContractAddress: Addr;
    calcSwapFee: number;
    automationFee: number;
    adminContractAddress: Addr;
    feeCollectorAddress: Addr;
    userWalletAddress: Addr;
    stakingRouterContractAddress: Addr;
    finPairAddress: Addr;
    finBuyPrice: number;
    finSellPrice: number;
    finMakerFee: number;
    finTakerFee: number;
    pair: Pair;
    validatorAddress: string;
  }
}
