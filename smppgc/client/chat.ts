// @ts-ignore
import "./common/common.css";
// @ts-ignore
import "./common/buttons.css";
// @ts-ignore
import "./common/logo.css";
// @ts-ignore
import "./chat/css/chat.css";
// @ts-ignore
import "./chat/css/login_popup.css";
// @ts-ignore
import "./chat/css/stickers.css";
// @ts-ignore
import "./chat/css/ban.css";

import * as ban from "./chat/ban.js";
import { execLocalCmd, localCmd } from "./chat/commands.ts";
import { createMessage, Message } from "./chat/mesg.ts";
import { Role } from './chat/user.ts';
import type { Control } from "./chat/mesg.ts";
import * as mk from "./chat/mkels.js";

import { Snowflake } from "./chat/snowflake.ts";
import { fixTextFields } from "./common/text.js";
import { hasVirtKb, log } from "./common/utils.js";

import { SocketMgr } from "./chat/protocol/protocol.ts";
import { ProtoError } from "./chat/protocol/protoerr.ts";

declare const ROLE: Role;
declare const IS_MOD: boolean;
declare const READONLY: boolean;
declare const MIN_MESSAGE_LEN: number;
declare const MAX_MESSAGE_LEN: number;

const sendinput = document.getElementById(
  "send-input",
) as HTMLInputElement | null;
const username_field = document.getElementById(
  "name-input",
) as HTMLInputElement;
const leavebtn = document.getElementById("leavebtn") as HTMLButtonElement;
const sendbtn = document.getElementById("sendbtn") as HTMLButtonElement;
const showModBadgeCheck = document.getElementById(
  "show-mod-badge",
) as HTMLInputElement;
const connectbtn = document.getElementById("connectbtn") as HTMLButtonElement;
const constatus = document.getElementById(
  "connection-status",
) as HTMLDialogElement;
const login_popup = document.getElementById("login") as HTMLDialogElement;
const profaneMessageDialog = document.getElementById(
  "profane-message-dialog",
) as HTMLDialogElement;
const profaneMessage = document.getElementById(
  "profane-message",
) as HTMLElement;
const profaneMessageOk = document.getElementById(
  "profane-message-ok",
) as HTMLButtonElement;
const profaneMessageCountdown = document.getElementById(
  "profane-message-countdown",
) as HTMLElement;
const profaneMessageBadWord = document.getElementById("badword") as HTMLElement;

let profanityCoolDown = 0;
let profanityCoolDownInterval: number;

export let socketmgr = new SocketMgr();

function add_message(
  message: Message,
  scroll: boolean,
  adminControls: boolean,
) {
  const controls = [];
  if (adminControls) {
    controls.push({ name: "delete", click: onMessageDelete });
    controls.push({
      name: "ban",
      click: message.sender.role >= ROLE ? null : onMessageBan,
    });
  }
  let msgEl = createMessage(message, controls);

  const mesgs = $("#mesgs").get(0)!;
  const should_scroll =
    Math.abs(mesgs.scrollHeight - mesgs.clientHeight - mesgs.scrollTop) <= 3 ||
    scroll;
  mesgs.appendChild(msgEl);
  if (should_scroll) {
    msgEl.scrollIntoView();
  }
}
function onMessageDelete(e: any, message: Message) {
  socketmgr.deleteMessage(message.snowflake);
}
function onMessageBan(e: any, message: Message) {
  ban.showDialog(message.snowflake, message.sender);
}

function add_system_message(message: string) {
  add_message(Message.system(message), true, false);
}

localCmd("/clearkey", function () {
  localStorage.removeItem("key");
  add_system_message("Key cleared.");
});
localCmd("/leave", function () {
  socketmgr.leave();
});

function clearSendInput() {
  if (sendinput) {
    sendinput.innerText = "";
  }
}

// Are we currently trying to reconnect in the background
let background_reconnect = false;

socketmgr.on_join = () => {
  constatus.close();
  background_reconnect = false;
};

let last_retry = 0;
let in_cooldown = false;
// Create a cool down for 'time' milliseconds on the join button to prevent
// people from spamming the join and leave buttons
function cooldown(time: number) {
  if (!login_popup.open) {
    login_popup.showModal();
  }
  in_cooldown = true;

  let oldVal = connectbtn.disabled;
  connectbtn.disabled = true;
  if (time == -1) {
    return;
  }
  setTimeout(() => {
    connectbtn.disabled = oldVal;
    in_cooldown = false;
  }, time);
}

function mesgEasterEgg(messageContent: string[]) {
  const content = messageContent.join("");
  if (
    content.includes("<script>") ||
    (content.includes("alert(1)") && content.includes("<"))
  ) {
    add_system_message("I see the xss-er has joined. Vewie pwo hweker :3");
  }
  if (
    content.includes("SELECT") &&
    content.includes("FROM") &&
    content.includes("WHERE")
  ) {
    add_system_message("Sql injection? Why? Messages aren't even stored?");
  }
}

let last_message_snowflake: Snowflake | null = null;
socketmgr.on_message = (me: boolean, sender_id: number, message: Message) => {
  last_message_snowflake = message.snowflake;
  log(
    `Got message from ${sender_id} (${message.snowflake}) mod: ${
      message.mod_badge
    }: ${message.content}`,
  );
  const scroll = me || sender_id === -1; // scroll if the message comes from me or system
  const adminControls = IS_MOD && sender_id !== -1;
  add_message(message, scroll, adminControls);

  if (me) {
    mesgEasterEgg(message.content);
  }
};

function getMessage(snowflake: Snowflake) {
  return document.querySelector(
    `.message[data-snowflake="${snowflake.toString()}"]`,
  );
}

socketmgr.on_message_censor = (snowflake: Snowflake) => {
  let mesgEl = getMessage(snowflake);
  if (mesgEl) {
    mesgEl.classList.add("prof-message");
  }
};

socketmgr.on_message_del = (snowflake: Snowflake) => {
  let mesgEl = getMessage(snowflake);
  if (mesgEl) {
    mesgEl.remove();
  }
};

socketmgr.on_leave = (data: string | any, protoerr: ProtoError) => {
  constatus.close();

  let time = 1000;
  if (protoerr == ProtoError.err_ratelimit) {
    time = 5000;
  } else if (protoerr == ProtoError.retry) {
    let now = Date.now();
    if (last_retry == 0 || now - last_retry > 10_000) {
      // join again if we should retry
      last_retry = now;
      connect(true, last_message_snowflake);
      return;
    }
  } else if (protoerr == ProtoError.err_banned) {
    time = -1;
    ban.setBan(data);
  }
  if (typeof data === "string") {
    $("#err-mesg").text(data);
  }
  cooldown(time);

  // reset everything
  last_message_snowflake = null;
  $("#mesgs").empty();
  ban.reset();

  login_popup.showModal();
  profanityCoolDown = 0;
  clearInterval(profanityCoolDownInterval);
  profaneMessageDialog.close();

  if (protoerr == ProtoError.err_no_session) {
    log("Got no session error. Redirecting to login page...");
    location.href = "/login?redirect=/v1";
  }
};

socketmgr.on_profanity_warn = (
  message: string,
  badWord: string,
  start: number,
  end: number,
) => {
  log(`${message} contains the word '${badWord}' at ${start}..${end}'`);

  profanityCoolDown = 10;
  profanityCoolDownInterval = setInterval(() => {
    profanityCoolDown--;
    profaneMessageCountdown.innerText =
      profanityCoolDown + (profanityCoolDown == 1 ? " seconde" : " seconden");
    if (profanityCoolDown == 0) {
      clearInterval(profanityCoolDownInterval);
      profaneMessageOk.innerText = "Ok";
      profaneMessageOk.disabled = false;
    }
  }, 1000);
  profaneMessageCountdown.innerText =
    profanityCoolDown + profanityCoolDown == 1 ? "seconde" : "seconden";
  profaneMessageOk.disabled = true;
  profaneMessageBadWord.innerText = badWord;
  let mesgEl = createMessage(
    new Message([message], socketmgr.get_local_user()),
    [],
  );
  profaneMessage.innerHTML = "";
  profaneMessage.appendChild(mesgEl);
  profaneMessageDialog.showModal();
};

async function send_message() {
  let message = sendinput?.innerText.trim();
  if (!message) {
    message = "";
  }
  if (message.length < MIN_MESSAGE_LEN || message.length > MAX_MESSAGE_LEN) {
    return false;
  }

  if (execLocalCmd(message)) {
    clearSendInput();
    return true;
  }
  if (await socketmgr.send(message)) {
    clearSendInput();
    return true;
  }
  return false;
}

function get_name(): string {
  let local_name = username_field.value;
  const default_name = username_field.dataset.default_username;
  if (local_name == "" && default_name) {
    local_name = default_name;
  }
  return local_name;
}

function connect(
  background: boolean,
  start_snowflake: Snowflake | null = null,
) {
  let show_mod_badge = showModBadgeCheck?.checked;

  log(
    "connecting... in_background=" +
      background +
      ", mod_badge:" +
      show_mod_badge,
  );
  let local_name: string | null = null;
  if (!READONLY) {
    local_name = get_name();
    localStorage.setItem("username", local_name);
  }

  background_reconnect = background;
  socketmgr.join(local_name, start_snowflake, show_mod_badge);

  constatus.showModal();
}

fixTextFields();

sendinput?.addEventListener("keypress", async (e) => {
  if (e.key == "Enter" && !e.shiftKey && !hasVirtKb()) {
    e.preventDefault();
    await send_message();
  }
});
sendbtn?.addEventListener("click", async () => {
  if (await send_message()) {
    setTimeout(() => {
      sendinput?.focus();
    }, 100);
  }
});

constatus.addEventListener("cancel", (e) => {
  e.preventDefault();
});
leavebtn.addEventListener("click", () => {
  socketmgr.leave();
});

login_popup.addEventListener("close", (e) => {
  connect(false);
});
login_popup.addEventListener("cancel", (e) => {
  e.preventDefault();
});

connectbtn.addEventListener("click", () => {
  login_popup.close();
  sendinput?.focus();
});

profaneMessageDialog.addEventListener("close", () => {
  if (profanityCoolDown > 0) {
    profaneMessageDialog.showModal();
  }
});
profaneMessageOk.addEventListener("click", () => {
  profaneMessageDialog.close();
});

if (!READONLY) {
  username_field.value! = localStorage.getItem("username")!;
}
login_popup.showModal();
