import * as chat from "./../chat.js";
import { log } from "./../common/utils.js";
import * as mk from "./mkels.js";
import * as sflake from "./snowflake.js";
import { User } from './user.js';

const messageTemplate = document.getElementById("message-template");

export class Message {
  content;
  sender;
  snowflake;
  profanity;
  mod_badge;

  constructor(content, sender, snowflake = null) {
    this.content = content;
    if (typeof sender === "string") {
      this.sender = new User(sender);
    } else {
      this.sender = sender;
    }
    if (snowflake == null) {
      this.snowflake = sflake.now();
    } else {
      this.snowflake = snowflake;
    }
    this.profanity = false;
    this.mod_badge = false;
  }
}

function setupActionBtn(btn, action, message) {
  if (action == "disabled") {
    btn.disabled = true;
  } else if (action == null) {
    btn.remove();
  } else {
    btn.addEventListener("click", (e) => {
      action(e, message);
    });
  }
}

function role_to_string(role) {
  switch (role) {
    case 0:
      return "";
    case 1:
      return "mod";
    case 2:
      return "admin";
    case 3:
      return "owner";
    default:
      return undefined;
  }
}

export function createMessage(
  message,
  delAction = null,
  banAction = null,
  highlight = null,
) {
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

  msgEl.dataset.username = message.sender.username;
  msgEl.dataset.snowflake = message.snowflake;
  let userEl = msgEl.querySelector(".user");
  userEl.innerText = message.sender.username;
  const roleEl = msgEl.querySelector(".role");
  roleEl.innerText = role_to_string(message.sender.role);
  if (message.mod_badge) {
    roleEl.classList.add("mod-badge");
  }

  msgEl.querySelector(".timestamp").innerText = sflake.into_time_str(
    message.snowflake,
  );

  setupActionBtn(msgEl.querySelector(".banbtn"), banAction, message);
  setupActionBtn(msgEl.querySelector(".delbtn"), delAction, message);

  mk.mkcontent(message.content, highlight, msgEl.querySelector(".content"));

  return msgEl;
}
