/* tslint:disable */
/**
 * This file was automatically generated by json-schema-to-typescript.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run json-schema-to-typescript to regenerate this file.
 */

export type ExecuteMsg =
  | {
      create_pair: {
        base_denom: string;
        quote_denom: string;
        route: number[];
      };
    }
  | {
      create_vault: {
        destinations?: Destination[] | null;
        label?: string | null;
        minimum_receive_amount?: Uint128 | null;
        owner?: Addr | null;
        performance_assessment_strategy?: PerformanceAssessmentStrategyParams | null;
        position_type?: PositionType | null;
        slippage_tolerance?: Decimal | null;
        swap_adjustment_strategy?: SwapAdjustmentStrategyParams | null;
        swap_amount: Uint128;
        target_denom: string;
        target_start_time_utc_seconds?: Uint64 | null;
        time_interval: TimeInterval;
      };
    }
  | {
      deposit: {
        address: Addr;
        vault_id: Uint128;
      };
    }
  | {
      cancel_vault: {
        vault_id: Uint128;
      };
    }
  | {
      execute_trigger: {
        trigger_id: Uint128;
      };
    }
  | {
      update_config: {
        dca_plus_escrow_level?: Decimal | null;
        delegation_fee_percent?: Decimal | null;
        fee_collectors?: FeeCollector[] | null;
        page_limit?: number | null;
        paused?: boolean | null;
        swap_fee_percent?: Decimal | null;
      };
    }
  | {
      create_custom_swap_fee: {
        denom: string;
        swap_fee_percent: Decimal;
      };
    }
  | {
      remove_custom_swap_fee: {
        denom: string;
      };
    }
  | {
      update_swap_adjustment: {
        strategy: SwapAdjustmentStrategy;
        value: Decimal;
      };
    }
  | {
      disburse_escrow: {
        vault_id: Uint128;
      };
    }
  | {
      z_delegate: {
        delegator_address: Addr;
        validator_address: Addr;
      };
    }
  | {
      z_provide_liquidity: {
        duration: LockableDuration;
        pool_id: number;
        provider_address: Addr;
        slippage_tolerance?: Decimal | null;
      };
    };
/**
 * A human readable address.
 *
 * In Cosmos, this is typically bech32 encoded. But for multi-chain smart contracts no assumptions should be made other than being UTF-8 encoded and of reasonable length.
 *
 * This type represents a validated address. It can be created in the following ways 1. Use `Addr::unchecked(input)` 2. Use `let checked: Addr = deps.api.addr_validate(input)?` 3. Use `let checked: Addr = deps.api.addr_humanize(canonical_addr)?` 4. Deserialize from JSON. This must only be done from JSON that was validated before such as a contract's state. `Addr` must not be used in messages sent by the user because this would result in unvalidated instances.
 *
 * This type is immutable. If you really need to mutate it (Really? Are you sure?), create a mutable copy using `let mut mutable = Addr::to_string()` and operate on that `String` instance.
 */
export type Addr = string;
/**
 * A fixed-point decimal value with 18 fractional digits, i.e. Decimal(1_000_000_000_000_000_000) == 1.0
 *
 * The greatest possible value that can be represented is 340282366920938463463.374607431768211455 (which is (2^128 - 1) / 10^18)
 */
export type Decimal = string;
/**
 * Binary is a wrapper around Vec<u8> to add base64 de/serialization with serde. It also adds some helper methods to help encode inline.
 *
 * This is only needed as serde-json-{core,wasm} has a horrible encoding for Vec<u8>. See also <https://github.com/CosmWasm/cosmwasm/blob/main/docs/MESSAGE_TYPES.md>.
 */
export type Binary = string;
/**
 * A thin wrapper around u128 that is using strings for JSON encoding/decoding, such that the full u128 range can be used for clients that convert JSON numbers to floats, like JavaScript and jq.
 *
 * # Examples
 *
 * Use `from` to create instances of this and `u128` to get the value out:
 *
 * ``` # use cosmwasm_std::Uint128; let a = Uint128::from(123u128); assert_eq!(a.u128(), 123);
 *
 * let b = Uint128::from(42u64); assert_eq!(b.u128(), 42);
 *
 * let c = Uint128::from(70u32); assert_eq!(c.u128(), 70); ```
 */
export type Uint128 = string;
export type PerformanceAssessmentStrategyParams = "compare_to_standard_dca";
export type PositionType = "enter" | "exit";
export type SwapAdjustmentStrategyParams = {
  risk_weighted_average: {
    base_denom: BaseDenom;
  };
};
export type BaseDenom = "bitcoin";
/**
 * A thin wrapper around u64 that is using strings for JSON encoding/decoding, such that the full u64 range can be used for clients that convert JSON numbers to floats, like JavaScript and jq.
 *
 * # Examples
 *
 * Use `from` to create instances of this and `u64` to get the value out:
 *
 * ``` # use cosmwasm_std::Uint64; let a = Uint64::from(42u64); assert_eq!(a.u64(), 42);
 *
 * let b = Uint64::from(70u32); assert_eq!(b.u64(), 70); ```
 */
export type Uint64 = string;
export type TimeInterval =
  | (
      | "every_second"
      | "every_minute"
      | "half_hourly"
      | "hourly"
      | "half_daily"
      | "daily"
      | "weekly"
      | "fortnightly"
      | "monthly"
    )
  | {
      custom: {
        seconds: number;
      };
    };
export type SwapAdjustmentStrategy = {
  risk_weighted_average: {
    base_denom: BaseDenom;
    model_id: number;
    position_type: PositionType;
  };
};
export type LockableDuration = "one_day" | "one_week" | "two_weeks";

export interface Destination {
  address: Addr;
  allocation: Decimal;
  msg?: Binary | null;
}
export interface FeeCollector {
  address: string;
  allocation: Decimal;
}
