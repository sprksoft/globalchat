export enum WFTag {
  Unknown = 0,
  Good = 1,
  Bad = 2,
  Whitespace = 3,
  GoodLocked = 4,
  BadLocked = 5,
}
export namespace WFTag {
  export function assignToElement(tag: WFTag, el: HTMLElement) {
    for (let i = 0; i < el.classList.length; i++) {
      const clas = el.classList.item(i)!;
      if (clas.startsWith("tag-") || clas == "locked") {
        el.classList.remove(clas);
      }

    }

    el.classList.add("tag-" + WFTag.toString(tag).toLowerCase());
    if (WFTag.isLocked(tag)) {
      el.classList.add("locked");
    }
  }

  export function isBad(tag: WFTag): boolean {
    return tag == WFTag.Bad || tag == WFTag.BadLocked;
  }
  export function isUnknown(tag: WFTag): boolean {
    return tag == WFTag.Unknown;
  }

  export function toString(tag: WFTag): string {
    switch (tag) {
      case WFTag.Unknown:
        return "u";
      case WFTag.Good:
        return "g";
      case WFTag.Bad:
        return "b";
      case WFTag.GoodLocked:
        return "G";
      case WFTag.BadLocked:
        return "B";
      case WFTag.Whitespace:
        return "w";
    }
  }
  export function isLocked(tag: WFTag): boolean {
    return tag == WFTag.GoodLocked || tag == WFTag.BadLocked;
  }
  export function fromString(string: string): WFTag {
    switch (string) {
      case "u":
        return WFTag.Unknown;
      case "g":
        return WFTag.Good;
      case "b":
        return WFTag.Bad;
      case "G":
        return WFTag.GoodLocked;
      case "B":
        return WFTag.BadLocked;
      case "w":
        return WFTag.Whitespace;
      default:
        return WFTag.Unknown;
    }
  }
}

