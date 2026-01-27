import { Message } from "../gcapi/mesg";
import { Snowflake } from "../gcapi/gctime";
import { Role } from "../gcapi/user";
import { mksticker } from "./mkels";
import { WFTag } from "./wf";
import { wfEditor } from "../chat";

export { Message };

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
    const trimmed = word.word.trim();
    if (trimmed.startsWith(":") && trimmed.endsWith(":")) {
      const el = mksticker(trimmed.substring(1, trimmed.length - 1));
      if (el) {
        content.append(el);
        continue;
      }
    }
    const span = $("<span></span>").text(word.word);
    WFTag.assignToElement(word.tag, span.get(0)!)
    if (wfEdit && word.tag !== WFTag.Whitespace) {
      if (wfEditor) {
        span.addClass("editable-word");
      }
      span.on("click", async function() {
        await wfEditor?.toggle(this);
      });
    }

    content.append(span);
  }

  return msgEl.get(0)!;
}
