import { StdFee } from '@cosmjs/stargate';

export const FEE: StdFee = {
  amount: [
    {
      denom: 'uosmo',
      amount: '70000',
    },
  ],
  gas: '25000000',
};
