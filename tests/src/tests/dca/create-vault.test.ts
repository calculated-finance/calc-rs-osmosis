import { Coin, coin } from '@cosmjs/proto-signing';
import dayjs from 'dayjs';
import { Context } from 'mocha';
import { find, map, range } from 'ramda';
import { EventData } from '../../types/dca/response/get_events';
import { Vault } from '../../types/dca/response/get_vault';
import { createVault, getBalances, getExpectedPrice } from '../helpers';
import { expect } from '../shared.test';

describe('when creating a vault', () => {
  describe('with no trigger', async () => {
    let deposit: Coin;
    const swapAmount = 100000;
    let vault: Vault;
    let eventPayloads: EventData[];
    let executionTriggeredEvent: EventData;
    let receivedAmount: number;
    let receivedAmountAfterFee: number;

    before(async function (this: Context) {
      deposit = coin(1000000, this.pair.quote_denom);

      const expectedPrice = await getExpectedPrice(
        this,
        this.pair,
        coin(swapAmount, deposit.denom),
        this.pair.base_denom == deposit.denom ? this.pair.quote_denom : this.pair.base_denom,
      );

      const vaultId = await createVault(
        this,
        {
          swap_amount: `${swapAmount}`,
        },
        [deposit],
      );

      vault = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: { vault_id: vaultId },
        })
      ).vault;

      eventPayloads = map(
        (event) => event.data,
        (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_events_by_resource_id: { resource_id: vaultId },
          })
        ).events,
      );

      executionTriggeredEvent = find((event) => 'dca_vault_execution_triggered' in event, eventPayloads) as EventData;

      const receivedAmountBeforeFee = Math.floor(Number(vault.swap_amount) / expectedPrice);
      receivedAmount = Math.floor(receivedAmountBeforeFee);
      receivedAmountAfterFee = Math.floor(receivedAmount - receivedAmount * this.calcSwapFee);
    });

    it('has the correct label', () => expect(vault.label).to.equal('test'));

    it('has the correct swapped amount', () => expect(vault.swapped_amount).to.eql(coin(100000, vault.balance.denom)));

    it('has the correct received amount', () =>
      expect(Number(vault.received_amount.amount)).to.be.approximately(receivedAmountAfterFee, 2));

    it('has a funds deposited event', () =>
      expect(eventPayloads).to.include.deep.members([
        {
          dca_vault_funds_deposited: {
            amount: coin(Number(vault.balance.amount) + Number(vault.swap_amount), vault.balance.denom),
          },
        },
      ]));

    it('has an execution triggered event', function (this: Context) {
      expect(eventPayloads).to.include.deep.members([
        {
          dca_vault_execution_triggered: {
            asset_price:
              'dca_vault_execution_triggered' in executionTriggeredEvent &&
              executionTriggeredEvent.dca_vault_execution_triggered?.asset_price,
            base_denom: vault.received_amount.denom,
            quote_denom: vault.balance.denom,
          },
        },
      ]);
    });

    it('has an execution completed event', function (this: Context) {
      const executionCompletedEvent = find((event) => 'dca_vault_execution_completed' in event, eventPayloads);
      expect(executionCompletedEvent).to.not.be.undefined;
      'dca_vault_execution_completed' in executionCompletedEvent &&
        expect(executionCompletedEvent.dca_vault_execution_completed?.sent.amount).to.equal(vault.swap_amount) &&
        expect(Number(executionCompletedEvent.dca_vault_execution_completed?.received.amount)).to.approximately(
          receivedAmount,
          2,
        ) &&
        expect(Number(executionCompletedEvent.dca_vault_execution_completed?.fee.amount)).to.approximately(
          Math.round(receivedAmount * this.calcSwapFee) - 1,
          2,
        );
    });

    it('has no other events', () => expect(eventPayloads).to.have.lengthOf(3));

    it('has a time trigger', () =>
      expect(vault.trigger).to.eql({
        time: { target_time: 'time' in vault.trigger && vault.trigger.time.target_time },
      }));

    it('is active', () => expect(vault.status).to.equal('active'));
  });

  describe('with a time trigger', async () => {
    const targetTime = dayjs().add(1, 'hour').unix();
    let vault: Vault;
    let eventPayloads: EventData[];

    before(async function (this: Context) {
      const vault_id = await createVault(this, { target_start_time_utc_seconds: `${targetTime}` });

      vault = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id,
          },
        })
      ).vault;

      eventPayloads = map(
        (event) => event.data,
        (
          await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
            get_events_by_resource_id: { resource_id: vault_id },
          })
        ).events,
      );
    });

    it('has the correct label', () => expect(vault.label).to.equal('test'));

    it('has the correct swapped amount', () => expect(vault.swapped_amount).to.eql(coin(0, vault.balance.denom)));

    it('has the correct received amount', function (this: Context) {
      expect(vault.received_amount).to.eql(coin(0, vault.target_denom));
    });

    it('has a funds deposited event', () =>
      expect(eventPayloads).to.include.deep.members([{ dca_vault_funds_deposited: { amount: vault.balance } }]));

    it('has no other events', () => expect(eventPayloads).to.have.lengthOf(1));

    it('has a time trigger', () => expect(vault.trigger).to.eql({ time: { target_time: `${targetTime}000000000` } }));

    it('is scheduled', () => expect(vault.status).to.equal('scheduled'));
  });

  describe('with a time trigger in the past', () => {
    it('fails with the correct error message', async function (this: Context) {
      await expect(
        createVault(this, {
          target_start_time_utc_seconds: `${dayjs().subtract(1, 'hour').unix()}`,
        }),
      ).to.be.rejectedWith(/target_start_time_utc_seconds must be some time in the future/);
    });
  });

  describe("with destination allocations that don't add up to 1", () => {
    it('fails with the correct error message', async function (this: Context) {
      await expect(
        createVault(this, {
          destinations: [
            {
              msg: null,
              address: this.userWalletAddress,
              allocation: '0.1',
            },
          ],
        }),
      ).to.be.rejectedWith(/destination allocations must add up to 1/);
    });
  });

  describe('with more than 10 destinations', () => {
    it('fails with the correct error message', async function (this: Context) {
      await expect(
        createVault(this, {
          destinations: map(
            () => ({
              msg: null,
              address: this.userWalletAddress,
              allocation: '0.1',
            }),
            range(0, 11),
          ),
        }),
      ).to.be.rejectedWith(/no more than 10 destinations can be provided/);
    });
  });

  describe('with a destination allocation equal to 0', () => {
    it('fails with the correct error message', async function (this: Context) {
      await expect(
        createVault(this, {
          destinations: [
            {
              msg: null,
              address: this.userWalletAddress,
              allocation: '0.0',
            },
          ],
        }),
      ).to.be.rejectedWith(/all destination allocations must be greater than 0/);
    });
  });

  describe('with an invalid destination address', () => {
    it('fails with the correct error message', async function (this: Context) {
      await expect(
        createVault(this, {
          destinations: [
            {
              msg: null,
              address: 'notanaddress',
              allocation: '1.0',
            },
          ],
        }),
      ).to.be.rejectedWith(/destination address notanaddress is invalid/);
    });
  });

  describe('with a swap amount <= 50000', () => {
    it('fails with the correct error message', async function (this: Context) {
      await expect(
        createVault(this, {
          swap_amount: '50000',
        }),
      ).to.be.rejectedWith(/swap amount must be greater than 50000/);
    });
  });

  describe('with multiple assets sent', () => {
    it('fails with the correct error message', async function (this: Context) {
      await expect(
        createVault(this, {}, [coin(1000000, this.pair.base_denom), coin(1000000, this.pair.quote_denom)]),
      ).to.be.rejectedWith(/received 2 denoms but required exactly 1/);
    });
  });

  describe('with no assets sent', () => {
    it('fails with the correct error message', async function (this: Context) {
      await expect(createVault(this, {}, [])).to.be.rejectedWith(/received 0 denoms but required exactly 1/);
    });
  });

  describe('with risk weight average swap adjustment strategy & a time trigger', () => {
    let vault: Vault;

    before(async function (this: Context) {
      const vault_id = await createVault(this, {
        target_start_time_utc_seconds: `${dayjs().add(1, 'hour').unix()}`,
        performance_assessment_strategy: 'compare_to_standard_dca',
        swap_adjustment_strategy: {
          risk_weighted_average: {
            base_denom: 'bitcoin',
          },
        },
      });

      vault = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id,
          },
        })
      ).vault;
    });

    it('has an empty escrowed balance', async function (this: Context) {
      expect(vault.escrowed_amount.amount).to.equal('0');
    });

    it('sets the escrow level', async function (this: Context) {
      expect(vault.escrow_level).to.equal('0.05');
    });

    it('has an empty standard dca swapped amount', async function (this: Context) {
      expect(vault.performance_assessment_strategy.compare_to_standard_dca.swapped_amount.amount).to.equal('0');
    });

    it('has an empty standard dca received amount', async function (this: Context) {
      expect(vault.performance_assessment_strategy.compare_to_standard_dca.received_amount.amount).to.equal('0');
    });

    it('has a DCA+ model id', async function (this: Context) {
      expect(
        'risk_weighted_average' in vault.swap_adjustment_strategy &&
          vault.swap_adjustment_strategy.risk_weighted_average.model_id,
      ).to.equal(30);
    });
  });

  describe('with risk weight average swap adjustment strategy & no trigger', () => {
    let vault: Vault;
    let balancesBeforeExecution: { [x: string]: { address: string } };
    let balancesAfterExecution: { [x: string]: { address: string } };
    let expectedPrice: number;

    before(async function (this: Context) {
      const deposit = coin(1000000, this.pair.quote_denom);
      const swapAmount = 100000;

      balancesBeforeExecution = await getBalances(this, [this.userWalletAddress], [this.pair.base_denom]);

      expectedPrice = await getExpectedPrice(
        this,
        this.pair,
        coin(swapAmount, this.pair.quote_denom),
        this.pair.base_denom,
      );

      const vault_id = await createVault(
        this,
        {
          performance_assessment_strategy: 'compare_to_standard_dca',
          swap_adjustment_strategy: {
            risk_weighted_average: {
              base_denom: 'bitcoin',
            },
          },
          swap_amount: `${swapAmount}`,
        },
        [deposit],
      );

      vault = (
        await this.cosmWasmClient.queryContractSmart(this.dcaContractAddress, {
          get_vault: {
            vault_id,
          },
        })
      ).vault;

      balancesAfterExecution = await getBalances(this, [this.userWalletAddress], [vault.target_denom]);
    });

    it('subtracts the escrowed balance from the disbursed amount', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress][vault.target_denom]).to.equal(
        Math.round(
          balancesBeforeExecution[this.userWalletAddress][vault.target_denom] +
            Number(vault.received_amount.amount) -
            Number(vault.escrowed_amount.amount),
        ),
      );
    });

    it('stores the escrowed balance', async function (this: Context) {
      expect(vault.escrowed_amount.amount).to.equal(
        `${Math.floor(Number(vault.received_amount.amount) * Number(vault.escrow_level))}`,
      );
    });

    it('calculates the standard dca swapped amount', async function (this: Context) {
      expect(vault.performance_assessment_strategy.compare_to_standard_dca.swapped_amount.amount).to.equal(
        `${Number(vault.swapped_amount.amount) / this.swapAdjustment}`,
      );
    });

    it('calculates the standard dca received amount', async function (this: Context) {
      expect(
        Number(vault.performance_assessment_strategy.compare_to_standard_dca.received_amount.amount),
      ).to.be.approximately(Math.round(Number(vault.swap_amount) / expectedPrice), 2);
    });
  });
});
