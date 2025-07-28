import { log, hasVirtKb } from "./common/utils.js";
import * as common from "./common/text.js";
import * as proto from "./chat/protocol.js";
import * as mk from "./chat/mkels.js";
import * as sflake from "./chat/snowflake.js";
import * as ban from "./chat/ban.js";
import { Message, createMessage } from "./chat/mesg.js";

import "./common/common.css";
import "./common/buttons.css";
import "./common/logo.css";
import "./chat/css/chat.css";
import "./chat/css/login_popup.css";
import "./chat/css/stickers.css";
import "./chat/css/ban.css";
import $ from "./common/jquery.js";

const sendinput = document.getElementById("send-input");
const username_field = document.getElementById("name-input");
const leavebtn = document.getElementById("leavebtn");
const sendbtn = document.getElementById("sendbtn");
const showModBadgeCheck = document.getElementById("show-mod-badge");
const connectbtn = document.getElementById("connectbtn");
const constatus = document.getElementById("connection-status");
const login_popup = document.getElementById("login");
const profaneMessageDialog = document.getElementById("profane-message-dialog");
const profaneMessage = document.getElementById("profane-message");
const profaneMessageOk = document.getElementById("profane-message-ok");
const profaneMessageCountdown = document.getElementById(
  "profane-message-countdown",
);
const profaneMessageBadWord = document.getElementById("badword");

let profanityCoolDown = 0;
let profanityCoolDownInterval;

export let socketmgr = new proto.SocketMgr();

function add_message(message, scroll = false, adminControls = false) {
  let delAction = onMessageDelete;
  let banAction = onMessageBan;
  if (message.sender.role >= ROLE) {
    banAction = "disabled";
  }
  if (!adminControls) {
    delAction = null;
    banAction = null;
  }
  let msgEl = createMessage(message, delAction, banAction, (highlight = null));

  let mesgs = $("#mesgs").get(0);
  let should_scroll =
    Math.abs(mesgs.scrollHeight - mesgs.clientHeight - mesgs.scrollTop) <= 3 ||
    scroll;
  mesgs.appendChild(msgEl);
  if (should_scroll) {
    msgEl.scrollIntoView();
  }
}
function onMessageDelete(e, message) {
  socketmgr.deleteMessage(message.snowflake);
}
function onMessageBan(e, message) {
  ban.showDialog(message.snowflake, message.sender);
}


local_commands.push([
  "/clearkey",
  function () {
    localStorage.removeItem("key");
    add_message(new Message("key cleared.", "system"));
    return true;
  },
]);
local_commands.push([
  "/leave",
  function () {
    socketmgr.leave();
    return true;
  },
]);

// Are we currently trying to reconnect in the background
let background_reconnect = false;

socketmgr.on_join = () => {
  constatus.close();
  background_reconnect = false;
};

let last_retry = 0;
let in_cooldown = false;
// Create a cool down for 'time' milliseconds on the join button to prevent people from spamming the join and leave buttons
function cooldown(time) {
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

function mesgEasterEgg(content) {
  if (
    content.includes("<script>") ||
    (content.includes("alert(1)") && content.includes("<"))
  ) {
    add_message(
      new Message("I see the xss-er has joined. Vewie pwo hweker :3", "system"),
    );
  }
  if (
    content.includes("SELECT") &&
    content.includes("FROM") &&
    content.includes("WHERE")
  ) {
    add_message(
      new Message("Sql injection? Why? Messages aren't even stored?", "system"),
    );
  }
}

let last_message_snowflake = null;
socketmgr.on_message = (me, sender_id, message) => {
  last_message_snowflake = message.snowflake;
  log(
    `Got message from ${sender_id} (${message.snowflake}) mod: ${message.mod_badge}: ${message.content}`,
  );
  add_message(
    message,
    (scroll = me || sender_id === -1),
    (adminControls = IS_MOD && sender_id !== -1),
  ); // scroll if the message comes from me or system

  if (me) {
    mesgEasterEgg(message.content);
  }
};

function getMessage(snowflake) {
  return document.querySelector(`.message[data-snowflake="${snowflake}"]`);
}

socketmgr.on_message_censor = (snowflake) => {
  let mesgEl = getMessage(snowflake);
  if (mesgEl) {
    mesgEl.classList.add("prof-message");
  }
};

socketmgr.on_message_del = (snowflake) => {
  let mesgEl = getMessage(snowflake);
  if (mesgEl) {
    mesgEl.remove();
  }
};

socketmgr.on_leave = (data, protoerr) => {
  constatus.close();

  time = 1000;
  if (protoerr == "err_ratelimit") {
    time = 5000;
  } else if (protoerr == "retry") {
    let now = Date.now();
    if (last_retry == 0 || now - last_retry > 10_000) {
      // join again if we should retry
      last_retry = now;
      connect(true, last_message_snowflake);
      return;
    }
  }else if (protoerr == "err_banned") {
    time = -1;
    ban.setBan(data);
  }
  if (typeof data === "string") {
    $("#err-mesg").text(data);
  }
  cooldown(time);

  //reset everything
  last_message_snowflake = null;
  $("#mesgs").empty();
  ban.reset();

  login_popup.showModal();
  profanityCoolDown = 0;
  clearInterval(profanityCoolDownInterval);
  profaneMessageDialog.close();

  if (protoerr == "err_no_session") {
    log("Got no session error. Redirecting to login page...");
    location = "/login?redirect=/v1";
  }
};

socketmgr.on_profanity_warn = (message, badWord, start, end) => {
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
    new Message(message, socketmgr.get_local_user()),
    null,
    null,
    (highlight = [start, end]),
  );
  profaneMessage.innerHTML="";
  profaneMessage.appendChild(mesgEl);
  profaneMessageDialog.showModal();
};

async function send_message() {
  let message = sendinput.innerText.trim();
  if (message.length < MIN_MESSAGE_LEN || message.length > MAX_MESSAGE_LEN) {
    return false;
  }

  for (const cmd of local_commands) {
    if (message == cmd[0]) {
      if (cmd[1]()) {
        sendinput.innerText = "";
        return true;
      }
      return false;
    }
  }
  if (await socketmgr.send(message)) {
    sendinput.innerText = "";
    return true;
  }
  return false;
}

function get_name() {
  let local_name = username_field.value;
  if (username_field.value == "") {
    local_name = username_field.dataset.default_username;
  }
  return local_name;
}

function connect(background, start_snowflake) {
  let show_mod_badge = showModBadgeCheck?.checked;

  log(
    "connecting... in_background=" +
      background +
      ", mod_badge:" +
      show_mod_badge,
  );
  let local_name = null;
  if (!READONLY) {
    local_name = get_name();
    localStorage.setItem("username", local_name);
  }

  background_reconnect = background;
  socketmgr.join(local_name, start_snowflake, show_mod_badge);

  constatus.showModal();
}

sendinput?.addEventListener("keypress", (e) => {
  if (e.key == "Enter" && !e.shiftKey && !hasVirtKb()) {
    e.preventDefault();
    send_message();
  }
});
sendbtn?.addEventListener("click", () => {
  if (send_message()) {
    setTimeout(() => {
      sendinput.focus();
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
  username_field.value = localStorage.getItem("username");
}
login_popup.showModal();
