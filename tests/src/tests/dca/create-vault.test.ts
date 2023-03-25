import { coin } from '@cosmjs/proto-signing';
import dayjs from 'dayjs';
import { Context } from 'mocha';
import { map, range, reduce } from 'ramda';
import { EventData } from '../../types/dca/response/get_events';
import { Vault } from '../../types/dca/response/get_vault';
import { createVault, getBalances, provideAuthGrant } from '../helpers';
import { expect } from '../shared.test';

describe('when creating a vault', () => {
  describe('with no trigger', async () => {
    let vault: Vault;
    let eventPayloads: EventData[];
    let receivedAmount: number;
    let receivedAmountAfterFee: number;

    before(async function (this: Context) {
      const vaultId = await createVault(this);

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

      const receivedAmountBeforeFinFee = parseInt(vault.swap_amount) / this.finBuyPrice;
      receivedAmount = Math.round(receivedAmountBeforeFinFee - receivedAmountBeforeFinFee * this.finTakerFee);
      receivedAmountAfterFee = Math.round(receivedAmount - receivedAmount * this.calcSwapFee);
    });

    it('has the correct label', () => expect(vault.label).to.equal('test'));

    it('has the correct swapped amount', () => expect(vault.swapped_amount).to.eql(coin(100000, vault.balance.denom)));

    it('has the correct received amount', () =>
      expect(vault.received_amount).to.eql(coin(receivedAmountAfterFee, 'ukuji')));

    it('has a vault created event', () => expect(eventPayloads).to.include.deep.members([{ dca_vault_created: {} }]));

    it('has a funds deposited event', () =>
      expect(eventPayloads).to.include.deep.members([
        {
          dca_vault_funds_deposited: {
            amount: coin(parseInt(vault.balance.amount) + parseInt(vault.swap_amount), vault.balance.denom),
          },
        },
      ]));

    it('has an execution triggered event', function (this: Context) {
      expect(eventPayloads).to.include.deep.members([
        {
          dca_vault_execution_triggered: {
            asset_price: `${this.finBuyPrice}`,
            base_denom: 'ukuji',
            quote_denom: vault.balance.denom,
          },
        },
      ]);
    });

    it('has an execution completed event', function (this: Context) {
      expect(eventPayloads).to.include.deep.members([
        {
          dca_vault_execution_completed: {
            sent: coin(vault.swap_amount, vault.balance.denom),
            received: coin(receivedAmount, 'ukuji'),
            fee: coin(Math.round(receivedAmount * this.calcSwapFee), 'ukuji'),
          },
        },
      ]);
    });

    it('has no other events', () => expect(eventPayloads).to.have.lengthOf(4));

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

    it('has the correct received amount', () => expect(vault.received_amount).to.eql(coin(0, 'ukuji')));

    it('has a vault created event', () => expect(eventPayloads).to.include.deep.members([{ dca_vault_created: {} }]));

    it('has a funds deposited event', () =>
      expect(eventPayloads).to.include.deep.members([{ dca_vault_funds_deposited: { amount: vault.balance } }]));

    it('has no other events', () => expect(eventPayloads).to.have.lengthOf(2));

    it('has a time trigger', () => expect(vault.trigger).to.eql({ time: { target_time: `${targetTime}000000000` } }));

    it('is scheduled', () => expect(vault.status).to.equal('scheduled'));
  });

  describe('with a price trigger', async () => {
    const swapAmount = 1000000;
    const targetPrice = 0.5;
    let vault: Vault;
    let eventPayloads: EventData[];

    before(async function (this: Context) {
      const vault_id = await createVault(this, {
        swap_amount: `${swapAmount}`,
        target_receive_amount: `${swapAmount / targetPrice}`,
      });

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

    it('has the correct received amount', () => expect(vault.received_amount).to.eql(coin(0, 'ukuji')));

    it('has a vault created event', () => expect(eventPayloads).to.include.deep.members([{ dca_vault_created: {} }]));

    it('has a funds deposited event', () =>
      expect(eventPayloads).to.include.deep.members([
        { dca_vault_funds_deposited: { amount: coin(`${parseInt(vault.balance.amount) + 2}`, vault.balance.denom) } },
      ]));

    it('has no other events', () => expect(eventPayloads).to.have.lengthOf(2));

    it('has a price trigger', () =>
      expect(
        vault.trigger &&
          'fin_limit_order' in vault.trigger &&
          vault.trigger.fin_limit_order.target_price === `${targetPrice}` &&
          vault.trigger.fin_limit_order.order_idx != null,
      ).to.be.true);

    it('is scheduled', () => expect(vault.status).to.equal('scheduled'));
  });

  describe('with a price trigger and a time trigger', () => {
    it('fails with the correct error message', async function (this: Context) {
      await expect(
        createVault(this, {
          target_receive_amount: `1000000`,
          target_start_time_utc_seconds: `${dayjs().add(1, 'hour').unix()}`,
        }),
      ).to.be.rejectedWith(/cannot provide both a target_start_time_utc_seconds and a target_price/);
    });
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
              action: 'send',
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
              action: 'send',
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
              action: 'send',
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
              action: 'send',
              address: 'notanaddress',
              allocation: '0.1',
            },
          ],
        }),
      ).to.be.rejectedWith(/destination address notanaddress is invalid/);
    });
  });

  describe('with an invalid validator address', () => {
    it('fails with the correct error message', async function (this: Context) {
      await expect(
        createVault(this, {
          destinations: [
            {
              action: 'z_delegate',
              address: 'notanaddress',
              allocation: '0.1',
            },
          ],
        }),
      ).to.be.rejectedWith(/validator notanaddress is invalid/);
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
      await expect(createVault(this, {}, [coin(1000000, 'udemo'), coin(1000000, 'ukuji')])).to.be.rejectedWith(
        /received 2 denoms but required exactly 1/,
      );
    });
  });

  describe('with no assets sent', () => {
    it('fails with the correct error message', async function (this: Context) {
      await expect(createVault(this, {}, [])).to.be.rejectedWith(/received 0 denoms but required exactly 1/);
    });
  });

  describe('with a funds denom not in the pair denoms', () => {
    it('fails with the correct error message', async function (this: Context) {
      await expect(createVault(this, {}, [coin(1000000, 'utest')])).to.be.rejectedWith(
        /send denom utest does not match pair base denom ukuji or quote denom udemo/,
      );
    });
  });

  describe('with non stakeable receive denom and z delegate destination', () => {
    it('fails with the correct error message', async function (this: Context) {
      await expect(
        createVault(
          this,
          {
            destinations: [
              {
                action: 'z_delegate',
                address: this.validatorAddress,
                allocation: '1.0',
              },
            ],
          },
          [coin(1000000, 'ukuji')],
        ),
      ).to.be.rejectedWith(/udemo is not the bond denomination/);
    });
  });

  describe('with dca plus & a time trigger', () => {
    let vault: Vault;

    before(async function (this: Context) {
      const vault_id = await createVault(this, {
        target_start_time_utc_seconds: `${dayjs().add(1, 'hour').unix()}`,
        use_dca_plus: true,
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
      expect(vault.dca_plus_config.escrowed_balance.amount).to.equal('0');
    });

    it('sets the escrow level', async function (this: Context) {
      expect(vault.dca_plus_config.escrow_level).to.equal('0.05');
    });

    it('has an empty standard dca swapped amount', async function (this: Context) {
      expect(vault.dca_plus_config.standard_dca_swapped_amount.amount).to.equal('0');
    });

    it('has an empty standard dca received amount', async function (this: Context) {
      expect(vault.dca_plus_config.standard_dca_received_amount.amount).to.equal('0');
    });

    it('has a DCA+ model id', async function (this: Context) {
      expect(vault.dca_plus_config.model_id).to.equal(30);
    });
  });

  describe('with dca plus & no trigger', () => {
    const deposit = coin(1000000, 'ukuji');
    let vault: Vault;
    let balancesBeforeExecution: Record<string, number>;
    let balancesAfterExecution: Record<string, number>;
    let expectedPrice: number;

    before(async function (this: Context) {
      balancesBeforeExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['udemo']);

      const vault_id = await createVault(
        this,
        {
          use_dca_plus: true,
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

      expectedPrice = await this.cosmWasmClient.queryContractSmart(this.swapContractAddress, {
        get_price: {
          swap_amount: coin(vault.swap_amount, 'ukuji'),
          target_denom: 'udemo',
          price_type: 'actual',
        },
      });

      balancesAfterExecution = await getBalances(this.cosmWasmClient, [this.userWalletAddress], ['udemo']);
    });

    it('subtracts the escrowed balance from the disbursed amount', async function (this: Context) {
      expect(balancesAfterExecution[this.userWalletAddress]['udemo']).to.equal(
        Math.round(
          balancesBeforeExecution[this.userWalletAddress]['udemo'] +
            parseInt(vault.received_amount.amount) -
            parseInt(vault.dca_plus_config.escrowed_balance.amount),
        ),
      );
    });

    it('stores the escrowed balance', async function (this: Context) {
      expect(vault.dca_plus_config.escrowed_balance.amount).to.equal(
        `${Math.floor(parseInt(vault.received_amount.amount) * parseFloat(vault.dca_plus_config.escrow_level))}`,
      );
    });

    it('calculates the standard dca swapped amount', async function (this: Context) {
      expect(vault.dca_plus_config.standard_dca_swapped_amount.amount).to.equal(
        `${parseInt(vault.swapped_amount.amount) / this.swapAdjustment}`,
      );
    });

    it('calculates the standard dca received amount', async function (this: Context) {
      expect(vault.dca_plus_config.standard_dca_received_amount.amount).to.equal(
        `${Math.round((parseInt(vault.swap_amount) / expectedPrice) * (1 - this.calcSwapFee - this.finTakerFee)) + 1}`,
      );
    });
  });
});
