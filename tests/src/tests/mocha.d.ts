import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Config } from '../shared/config';
import { Addr } from '../types/dca/execute';
import { Pair } from '../types/dca/response/get_pairs';
import * as mocha from 'mocha';
import { Pair } from '../types/dca/response/get_vault';

declare module 'mocha' {
  export interface Context {
    config: Config;
    cosmWasmClient: SigningCosmWasmClient;
    userCosmWasmClient: SigningCosmWasmClient;
    queryClient: any;
    dcaContractAddress: Addr;
    calcSwapFee: number;
    automationFee: number;
    adminContractAddress: Addr;
    feeCollectorAddress: Addr;
    userWalletAddress: Addr;
    pair: Pair;
    validatorAddress: string;
    swapAdjustment: number;
  }
}
