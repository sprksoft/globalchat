import * as mk from "./mkels.js";
import * as sflake from "./snowflake.js";
import * as chat from "./../chat.js";

const messageTemplate = document.getElementById("message-template");

export class Message {
  content;
  sender;
  snowflake;
  profanity;
  mod_badge;

  constructor(content, sender, snowflake = null) {
    this.content = content;
    this.sender = sender;
    if (snowflake == null) {
      this.snowflake = sflake.now();
    } else {
      this.snowflake = snowflake;
    }
    this.profanity = false;
    this.mod_badge = false;
  }
}

export function createMessage(message, onControl = null, highlight = null) {
  const msgFrag = messageTemplate.content.cloneNode(true);
  let msgEl = msgFrag.querySelector(".message");
  if (message.profanity) {
    msgEl.classList.add("prof-message");
    msgEl.addEventListener("click", (e) => {
      if (msgEl.classList.contains("prof-message-show")) {
        msgEl.classList.remove("prof-message-show");
      } else {
        msgEl.classList.add("prof-message-show");
      }
    });
  }

  msgEl.dataset.username = message.sender;
  msgEl.dataset.snowflake = message.snowflake;
  let userEl = msgEl.querySelector(".user");
  userEl.innerText = message.sender;
  if (message.mod_badge) {
    userEl.classList.add("mod-badge");
  }
  msgEl.querySelector(".timestamp").innerText = sflake.into_time_str(
    message.snowflake,
  );

  for (const control of msgEl.querySelectorAll(".message-actions button")) {
    if (onControl) {
      control.addEventListener("click", (e) => {
        onControl(e, message);
      });
    } else {
      control.remove();
    }
  }
  mk.mkcontent(message.content, highlight, msgEl.querySelector(".content"));

  return msgEl;
}
