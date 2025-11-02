import { SMPPGC_EPOCH } from '../common/gctime.ts'

export type Snowflake = bigint;

export namespace Snowflake {
  export function toTimeString(snowflake: Snowflake) {
    return new Date(Number(snowflake >> 22n) + SMPPGC_EPOCH).toLocaleString(undefined, {
      dateStyle: "short",
      timeStyle: "short",
    });
  }
  export function now(): Snowflake {
    return BigInt(new Date().getTime() - SMPPGC_EPOCH) << 22n;
  }

}



