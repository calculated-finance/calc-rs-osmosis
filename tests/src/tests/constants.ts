import { StdFee } from '@cosmjs/stargate';

export const FEE: StdFee = {
  amount: [
    {
      denom: 'uosmo',
      amount: '62500',
    },
  ],
  gas: '25000000',
};
