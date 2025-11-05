import { Snowflake } from "./gctime.ts";
import { User } from "./user.ts";
import { WFTag } from "./wf.ts";


export interface Word {
  tag: WFTag,
  word: string,
}

export class Message {
  content: Word[];
  sender: User;
  snowflake: Snowflake;
  mod_badge: boolean;

  constructor(
    content: Word[] | string[],
    sender: User,
    snowflake: Snowflake | null = null,
  ) {
    this.content = [];
    for (const word of content) {
      if (typeof word === "string") {
        this.content.push({ tag: WFTag.Good, word: word })
      } else {
        this.content.push(word);
      }
    }
    this.sender = sender;

    if (snowflake == null) {
      this.snowflake = Snowflake.now();
    } else {
      this.snowflake = snowflake as Snowflake;
    }
    this.mod_badge = false;
  }
}
export namespace Message {
  export function system(content: string): Message {
    return new Message([content], User.system(), Snowflake.now());
  }
  export function stringContent(mesg: Word[] | Message): string {
    const content = mesg instanceof Message ? mesg.content : mesg;
    return content.map((w) => w.word).join("");
  }

  export function containsUnknown(mesg: Word[] | Message): boolean {
    if (mesg instanceof Message) {
      return containsUnknown(mesg.content);
    } else {
      let unknown = false;
      for (const word of mesg) {
        if (word.tag == WFTag.Bad) {
          return false;
        }
        if (word.tag == WFTag.Unknown) {
          unknown = true;
        }
      }
      return unknown;
    }
  }

  export function containsProf(mesg: Word[] | Message): boolean {
    if (mesg instanceof Message) {
      return containsProf(mesg.content);
    } else {
      for (let word of mesg) {
        if (word.tag == WFTag.Bad || word.tag == WFTag.Unknown) {
          return true;
        }
      }
      return false;
    }
  }
}
