import "./chat/css/styles.css.js";
import * as ban from "./chat/ban.js";
import type { Ban } from "./gcapi/ban";
import { execLocalCmd, localCmd } from "./chat/commands";
import { createMessage, Message } from "./chat/message";
import type { Word } from "./gcapi/mesg";
import { Role } from "./gcapi/user";

import { Snowflake } from "./gcapi/nanotime.js";
import { fixTextFields } from "./common/text.js";
import { hasVirtKb, log } from "./common/utils.js";

import { GCClient, type ApiVersion, type ChatConfig } from "./gcapi/protocol";
import { ProtoError } from "./gcapi/protoerr";
import {
  clearProfWarn,
  setupProfWarn,
  setupWFEditor,
  showProfWarn,
} from "./chat/wf";

export declare const WEBSOCKET_URL: string;
export declare const ROLE: Role;
export declare const READONLY: boolean;
export declare const MIN_MESSAGE_LEN: number;
export declare const MAX_MESSAGE_LEN: number;
export declare const VERSION_INT: ApiVersion;

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

export let gcclient = new GCClient(WEBSOCKET_URL);

export let chatConfig: ChatConfig | null = null;

export interface HtmlMessage {
  html: HTMLElement;
  message: Message;
}
export let messages: HtmlMessage[] = [];

let scrollLock: boolean = true;

function addMessage(message: Message, scroll: boolean, showControls: boolean) {
  const controls = [];
  if (showControls) {
    if (ROLE >= Role.Mod) {
      controls.push({ name: "delete", click: onMessageDelete });
      controls.push({
        name: "ban",
        click: message.sender.role >= ROLE ? null : onMessageBan,
      });
    }
  }
  let msgEl = createMessage(message, controls, showControls);

  const belowIndex = getMessageIndexBelow(message.snowflake);
  if (belowIndex !== null) {
    log(`inserting message above ${belowIndex}`);
    const aboveMesg = messages[belowIndex]!;
    messages.splice(belowIndex, 0, { html: msgEl, message: message });
    aboveMesg.html.insertAdjacentElement("beforebegin", msgEl);

    if (aboveMesg.message.snowflake === message.snowflake) {
      log("replaced message");
      delMessage(belowIndex + 1);
    }
  } else {
    log("appending message to end");
    messages.push({ html: msgEl, message: message });
    $("#mesgs").get(0)!.appendChild(msgEl);

    while (messages.length > chatConfig!.maxMessages) {
      delMessage(0);
    }
  }

  requestAnimationFrame(() => {
    const lastMessage =
      belowIndex !== null ? belowIndex === messages.length - 1 : true;
    if ((scrollLock && lastMessage) || scroll) {
      msgEl.scrollIntoView({ block: "start" });
      log(
        `scrolled message (scrollLock: ${scrollLock}, scroll: ${scroll} lastMessage: ${lastMessage})`,
      );
    }
  });
}

function onMessageDelete(_: any, message: Message) {
  gcclient.deleteMessage(message.snowflake);
}
function onMessageBan(_: any, message: Message) {
  ban.showDialog(message.snowflake, message.sender);
}

function addSystemMessage(message: string) {
  addMessage(Message.system(message), true, false);
}

/**
 * Gets a message using the snowflake. If the message doesn't exist returns the message that would be below the snowflake
 */
function getMessageIndexBelow(snowflake: Snowflake): number | null {
  for (let i = 0; i < messages.length; i++) {
    const mesg = messages[i]!;
    if (mesg.message.snowflake >= snowflake) {
      return i;
    }
  }
  return null;
}
function getMessageIndex(snowflake: Snowflake): number | null {
  const index = getMessageIndexBelow(snowflake);
  if (index === null || messages[index]!.message.snowflake != snowflake) {
    return null;
  }
  return index;
}
function delMessage(index: number) {
  if (!messages[index]) {
    return;
  }
  messages[index].html.remove();
  messages.splice(index, 1);
}

localCmd("/clearkey", function () {
  localStorage.removeItem("key");
  addSystemMessage("Key cleared.");
});
localCmd("/leave", function () {
  gcclient.leave();
});
localCmd("/mesgs", function () {
  console.log("messages");
  for (const mesg of messages) {
    console.log(mesg.html);
    console.log(mesg.message.snowflake.toString());
  }
  console.log("end messages");
});

function clearSendInput() {
  if (sendinput) {
    sendinput.innerText = "";
  }
}

// Are we currently trying to reconnect in the background
let background_reconnect = false;

gcclient.on_join = (config) => {
  chatConfig = config;
  handle_version_check(config.apiVersion);
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

function mesgEasterEgg(messageContent: Word[]) {
  const content = Message.stringContent(messageContent);
  if (
    content.includes("<script>") ||
    (content.includes("alert(1)") && content.includes("<"))
  ) {
    addSystemMessage("I see the xss-er has joined. Vewie pwo hweker :3");
  }
  if (
    content.includes("SELECT") &&
    content.includes("FROM") &&
    content.includes("WHERE")
  ) {
    addSystemMessage("Sql injection? Why? Messages aren't even stored?");
  }
}

let last_message_snowflake: Snowflake | null = null;
gcclient.on_message = (sender_id: number, message: Message) => {
  const me = gcclient.localId() == sender_id;
  let lastMessage = false;
  if (!last_message_snowflake || message.snowflake > last_message_snowflake) {
    lastMessage = true;
    last_message_snowflake = message.snowflake;
  }
  log(
    `Got message from ${sender_id} (${message.snowflake})${me ? " (me)" : ""}${lastMessage ? " (last)" : ""} mod: ${
      message.mod_badge
    }: ${Message.stringContent(message.content)} ${Message.containsProf(message) ? " (prof)" : ""}`,
  );

  if (lastMessage && me && Message.containsProf(message)) {
    if (Message.containsUnknown(message)) {
      showProfWarn(message, 2);
      if (sendinput) {
        sendinput.innerText = Message.stringContent(message);
      }
    } else {
      showProfWarn(message, 10);
    }

    // If we are not a mod don't display the message.
    if (ROLE < Role.Mod) {
      return;
    }
  }

  const scroll = me || sender_id === -1; // scroll if the message comes from me or system
  addMessage(message, scroll, sender_id !== -1); // don't show controls when the message is from system

  if (me) {
    mesgEasterEgg(message.content);
  }
};

gcclient.on_message_del = (snowflake: Snowflake) => {
  const result = getMessageIndex(snowflake);
  if (result == null) {
    return;
  }
  delMessage(result);
};

gcclient.on_leave = (data: string | Ban, protoerr: ProtoError) => {
  log("disconnect reason: " + JSON.stringify(data));
  constatus.close();

  let time = 1000;
  if (protoerr == ProtoError.RateLimit) {
    time = 5000;
  } else if (
    protoerr == ProtoError.Retry ||
    protoerr == ProtoError.AlreadyInChat
  ) {
    let now = Date.now();
    if (last_retry == 0 || now - last_retry > 10_000) {
      // join again if we should retry
      last_retry = now;
      connect(true, last_message_snowflake);
      return;
    }
  } else if (protoerr == ProtoError.Banned) {
    time = -1;
    ban.setBan(data as Ban);
  }

  if (protoerr == ProtoError.Disclaimer) {
    $("#err-mesg").html(
      "De disclaimer is nog niet geaccepteerd. <a href='/login?redirect=/v1'>Accepteer hem hier</a>",
    );
  } else if (typeof data === "string") {
    $("#err-mesg").text(data);
  }
  cooldown(time);

  // reset everything
  last_message_snowflake = null;
  $("#mesgs").empty();
  messages = [];
  ban.reset();
  scrollLock = true;
  clearProfWarn();

  login_popup.showModal();

  if (protoerr === ProtoError.NoSession) {
    log("Got no session error. Redirecting to login page...");
    location.href = "/login?redirect=/v1";
  }
};

gcclient.on_user_count_update = (userCount: number) => {
  $("#user-count-text").text(`${userCount}`);
};

function send_message(): boolean {
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
  if (gcclient.sendString(message)) {
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

function handle_version_check(apiVersion: ApiVersion) {
  if (apiVersion !== VERSION_INT) {
    let last_reload_time = localStorage.getItem("last_client_outdated_reload");
    let now = new Date().getTime();
    if (last_reload_time == null || now - parseInt(last_reload_time) > 1000) {
      localStorage.setItem("last_client_outdated_reload", now.toString());
      console.log("NEW PROTOCOL VERSION. RELOADING PAGE TO UPDATE CLIENT");
      location.reload();
    } else {
      console.log("protocol_ver: " + apiVersion + " page_ver: " + VERSION_INT);
      console.error("Infinite reload loop detected");
      alert("Alles is kapot aaaaaaaaaaaaa.");
    }
  }
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
  gcclient.join(local_name, start_snowflake, show_mod_badge);

  constatus.showModal();
}

fixTextFields();
setupProfWarn();
export const wfEditor = setupWFEditor(ROLE);

sendinput?.addEventListener("keypress", async (e) => {
  if (e.key == "Enter" && !e.shiftKey && !hasVirtKb()) {
    e.preventDefault();
    send_message();
  }
});
sendbtn?.addEventListener("click", async () => {
  if (send_message()) {
    setTimeout(() => {
      sendinput?.focus();
    }, 100);
  }
});

constatus.addEventListener("cancel", (e) => {
  e.preventDefault();
});
leavebtn.addEventListener("click", () => {
  gcclient.leave();
});

login_popup.addEventListener("close", (_) => {
  connect(false);
});
login_popup.addEventListener("cancel", (e) => {
  e.preventDefault();
});

connectbtn.addEventListener("click", () => {
  login_popup.close();
  sendinput?.focus();
});

$("#mesgs").on("scrollend", function () {
  const bottom =
    Math.abs(this.scrollTop + this.clientHeight - this.scrollHeight) < 2;
  scrollLock = bottom;
});

if (!READONLY) {
  username_field.value! = localStorage.getItem("username")!;
}
login_popup.showModal();
