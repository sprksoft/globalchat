export enum WFTag {
  Unknown = 0,
  Good = 1,
  Bad = 2,
  Whitespace = 3,
}
export namespace WFTag {
  export function toString(tag: WFTag): string {
    switch (tag) {
      case WFTag.Unknown:
        return "u";
      case WFTag.Good:
        return "g";
      case WFTag.Bad:
        return "b";
      case WFTag.Whitespace:
        return "w";
    }
  }
  export function fromNum(num: number): WFTag {
    if (num < 0 || num > WFTag.Whitespace) {
      console.error("tried to create a WFTag from an out of range number");
      return WFTag.Unknown;
    }
    return num as WFTag;
  }
}

