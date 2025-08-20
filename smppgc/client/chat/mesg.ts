import * as chat from "./../chat.js";
import { log } from "./../common/utils.js";
import { mksticker } from "./mkels.ts";
import { Snowflake } from "./snowflake.ts";
import { User, Role } from "./user.ts";

export class Message {
  content: string[];
  sender: User;
  snowflake: Snowflake;
  profanity: boolean;
  mod_badge: boolean;

  constructor(
    content: string[],
    sender: User,
    snowflake: Snowflake | null = null,
  ) {
    this.content = content;
    this.sender = sender;

    if (snowflake == null) {
      this.snowflake = Snowflake.now();
    } else {
      this.snowflake = snowflake as Snowflake;
    }
    this.profanity = false;
    this.mod_badge = false;
  }
}
export namespace Message {
  export function system(content: string): Message {
    return new Message([content], User.system(), Snowflake.now());
  }
}

export interface Control {
  name: string;
  click: ((e: any, message: Message) => void) | null;
}

export function createMessage(
  message: Message,
  controls: Array<Control> = [],
  wfEdit: boolean,
): HTMLElement {
  const template = $("#message-template").get(0) as HTMLTemplateElement;
  const msgFrag = template.content.cloneNode(true) as HTMLElement;
  const msgEl = $(msgFrag.querySelector(".message") as HTMLElement);

  msgEl.attr("data-snowflake", message.snowflake.toString());
  msgEl.attr("data-username", message.sender.username);
  msgEl.find(".user").text(message.sender.username);
  msgEl.find(".timestamp").text(Snowflake.toTimeString(message.snowflake));

  const role = msgEl.find(".role");
  role.text(Role.toString(message.sender.role));
  if (message.mod_badge) {
    role.addClass("mod-badge");
  }

  const msgActions = msgEl.find(".message-actions");
  for (const control of controls) {
    let ctrl = $("<button class='unimportantbtn'></button>")
      .text(control.name)
      .prop("disabled", control.click === null);

    if (control.click !== null) {
      const handler = control.click;
      ctrl.on("click", (e) => {
        handler(e, message);
      });
    }

    msgActions.append(ctrl);
  }

  const content = msgEl.find(".content");
  for (const word of message.content) {
    const trimmed = word.trim();
    if (trimmed.startsWith(":") && trimmed.endsWith(":")) {
      const el = mksticker(trimmed.substring(1,trimmed.length-1));
      if (el) {
        content.append(el);
        continue;
      }
    }
    content.append($("<span class='word '></span>").text(word).on("click", function () {
      $(this).toggleClass("bad");
      $(this).toggleClass("good");
    }));
  }

  return msgEl.get(0)!;
}
