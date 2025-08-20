const SMPPGC_EPOCH = (2024 - 1970) * 31557600*1000;

export type Snowflake = bigint;

export namespace Snowflake {
  export function toTimeString(snowflake: Snowflake) {
    return new Date(Number(snowflake >> 22n)+SMPPGC_EPOCH).toLocaleString(undefined, {
      dateStyle:"short",
      timeStyle:"short",
    });
  }
  export function now(): Snowflake {
    return BigInt(new Date().getTime()-SMPPGC_EPOCH) << 22n;
  }

}



