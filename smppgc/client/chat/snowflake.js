const SMPPGC_EPOCH = (2024 - 1970) * 31557600*1000;

function into_gctime(date) {
  return date.getTime()-SMPPGC_EPOCH;
}

export function now() {
  return BigInt(new Date().getTime()-SMPPGC_EPOCH) << 22n;
}

export function into_time_str(snowflake) {
  if (snowflake == null || snowflake == undefined || snowflake == 0) {
    console.error("invalid snowflake id: "+snowflake);
  }
  return new Date(Number(snowflake >> 22n)+SMPPGC_EPOCH).toLocaleString(undefined, {
    dateStyle:"short",
    timeStyle:"short",
  });
}

