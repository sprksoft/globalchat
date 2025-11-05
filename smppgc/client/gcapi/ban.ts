
function splitOnce(str: string, split: string): [string, string] {
  let indexOfSplit = str.indexOf(split);
  if (indexOfSplit == -1) {
    return [str, ""]
  }
  return [str.slice(0, indexOfSplit), str.slice(indexOfSplit + 1)];
}

export interface Ban {
  expirationTime: Date;
  reason: string;
}

export namespace Ban {

  export function parse(str: string): Ban | null {
    const [errBanned, rest] = splitOnce(str, ":");
    if (errBanned != "err_banned") {
      return null;
    }

    const [timeStr, reason] = splitOnce(rest, ":");


    const ban = {
      expirationTime: new Date(parseInt(timeStr) * 1000),
      reason: reason,
    };
    return ban;
  }
}
