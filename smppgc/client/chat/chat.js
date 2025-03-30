import * as proto from './protocol.js';
import * as utils from './../utils.js';
import * as mk from './mkels.js';
import * as disclaimer from './disclaimer.js';
import * as general from './../general.js';
import * as sflake from './snowflake.js';
import { Message, createMessage } from './mesg.js';

import './../general.css'
import './../buttons.css'
import './css/chat.css'
import './css/login_popup.css'

const mesgs = document.getElementById("mesgs");
const sendinput = document.getElementById("send-input");
const username_field = document.getElementById("name-input");
const leavebtn = document.getElementById("leavebtn");
const sendbtn = document.getElementById("sendbtn");
const showModBadgeCheck = document.getElementById("show-mod-badge");
const connectbtn = document.getElementById("connectbtn");
const constatus = document.getElementById("connection-status");
const err_mesg = document.getElementById("err-mesg");
const login_popup = document.getElementById("login");
const profaneMessageDialog = document.getElementById("profane-message-dialog");
const profaneMessage = document.getElementById("profane-message");
const profaneMessageOk = document.getElementById("profane-message-ok");
const profaneMessageCountdown = document.getElementById("profane-message-countdown");
const profaneMessageBadWord = document.getElementById("badword");



disclaimer.checkbox.addEventListener("change", (e) => {
  connectbtn.disabled = !e.target.checked;
});
connectbtn.disabled=!disclaimer.checkbox.checked;

let profanityCoolDown = 0;
let profanityCoolDownInterval;

export let socketmgr = new proto.SocketMgr();

function add_message(message, scroll=false) {
  let msgEl = createMessage(message, controls=true, highlight=null);

  let should_scroll = Math.abs(mesgs.scrollHeight - mesgs.clientHeight - mesgs.scrollTop) <= 3 || scroll;
  mesgs.appendChild(msgEl);
  if (should_scroll){
    msgEl.scrollIntoView();
  }
}

local_commands.push(["/clearkey", function () {
    localStorage.removeItem("key");
    add_message(new Message("key cleared.", "system"));
    return true;
}]);
local_commands.push(["/leave", function () {
  socketmgr.leave();
  return true;
}]);


// Are we currently trying to reconnect in the background
let background_reconnect=false;

socketmgr.on_join = () => {
  constatus.close();
  background_reconnect=false;
}

let last_retry = 0;
let in_cooldown=false;
function cool_down(time){
  if (!login_popup.open){
    login_popup.showModal();
  }
  in_cooldown=true;

  let oldVal = connectbtn.disabled;
  connectbtn.disabled=true;
  setTimeout(() => {
  connectbtn.disabled=oldVal;
    in_cooldown=false;
  }, time);
}

function mesgEasterEgg(content) {
  if (content.includes("<script>") || (content.includes("alert(1)") && content.includes("<"))){
    add_message(new Message("I see the xss-er has joined. Vewie pwo hweker :3", "system"));
  }
  if (content.includes("SELECT") && content.includes("FROM") && content.includes("WHERE")){
    add_message(new Message("Sql injection? Why? Messages aren't even stored?", "system"));
  }
}

let last_message_snowflake=null;
socketmgr.on_message = (me, sender_id, message) => {
  last_message_snowflake=message.snowflake;
  utils.log(`Got message from ${sender_id} (${message.snowflake}) mod: ${message.mod_badge}: ${message.content}`);
  add_message(message, scroll=me); // scroll if the message comes from me

  if (me) {
    mesgEasterEgg(message.content);
  }

}


function getMessage(snowflake) {
  return document.querySelector(`.message[data-snowflake="${snowflake}"]`);
}

socketmgr.on_message_censor = (snowflake) => {
  let mesgEl = getMessage(snowflake);
  if (mesgEl) {
    mesgEl.classList.add("prof-message");
  }
}

socketmgr.on_message_del = (snowflake) => {
  let mesgEl = getMessage(snowflake);
  if (mesgEl) {
    mesgEl.remove();
  }
}

socketmgr.on_leave = (code, protoerr, user_wants_leave) => {
  utils.log("disconnected "+code)
  constatus.close();

  error = proto.human_err(protoerr);
  time = 1000;
  if (user_wants_leave || code == 1000){ // Normal Closure or the user wants to leave
    error="";
  }else if (protoerr == "err_ratelimit"){
    time=5000;
  }else if (code == 1006 && protoerr == ""){
      let now = Date.now();
      if (last_retry == 0 || now-last_retry > 10_000){ // join again if we should retry
        last_retry = now;
        connect(true, last_message_snowflake);
        return;
      }
  }
  err_mesg.innerText=error;
  cool_down(time);

  //reset everything
  last_message_snowflake=null;
  mesgs.innerHTML="";
  login_popup.showModal();
  profanityCoolDown = 0;
  clearInterval(profanityCoolDownInterval);
  profaneMessageDialog.close();
}

socketmgr.on_keychange = (key) => {
  localStorage.setItem("key", key);
}
socketmgr.on_profanity_warn = (message, badWord, start, end) => {
  utils.log(`${message} contains the word '${badWord}' at ${start}..${end}'`);

  profanityCoolDown = 10;
  profanityCoolDownInterval = setInterval(() => {
    profanityCoolDown--;
    profaneMessageCountdown.innerText = profanityCoolDown + (profanityCoolDown == 1 ? " seconde" : " seconden");
    if (profanityCoolDown == 0) {
      clearInterval(profanityCoolDownInterval);
      profaneMessageOk.innerText = "Ok";
      profaneMessageOk.disabled=false;
    }
  }, 1000);
  profaneMessageCountdown.innerText = profanityCoolDown + profanityCoolDown == 1 ? "seconde" : "seconden";
  profaneMessageOk.disabled=true;
  profaneMessageBadWord.innerText = badWord;
  let mesgEl = createMessage(new Message(message, get_name()), controls=false, highlight=[start, end])
  utils.setChild(profaneMessage, mesgEl)
  profaneMessageDialog.showModal();
}

async function send_message() {
  let message = sendinput.innerText.trim();
  if (message.length < MIN_MESSAGE_LEN || message.length > MAX_MESSAGE_LEN){
    return false;
  }

  for (const cmd of local_commands){
    if (message == cmd[0]){
      if(cmd[1]()){
        sendinput.innerText="";
        return true;
      }
      return false;
    }
  }
  if (await socketmgr.send(message)){
    sendinput.innerText="";
    return true;
  }
  return false;
}

function get_name() {
  let local_name = username_field.value;
  if (username_field.value == ""){
    local_name = username_field.dataset.default_username;
  }
  return local_name;
}

function connect(background, start_snowflake) {
  let show_mod_badge = showModBadgeCheck?.checked;

  utils.log("connecting... in_background="+background+", mod_badge:"+show_mod_badge);
  let local_name = get_name();
  localStorage.setItem("username", local_name);

  background_reconnect=background;
  socketmgr.join(localStorage.getItem("key"), local_name, start_snowflake, show_mod_badge);

  constatus.showModal();
}

sendinput.addEventListener("keypress", (e)=>{
  if (e.key == "Enter" && !e.shiftKey && !utils.has_virtkb()){
    e.preventDefault();
    send_message();
  }
});
sendbtn.addEventListener("click", ()=>{
  if (send_message()){
    setTimeout(() => {
      sendinput.focus();
    }, 100);
  }
});

constatus.addEventListener("cancel", (e)=>{
  e.preventDefault();
})
leavebtn.addEventListener("click", ()=>{
  socketmgr.leave();
});

login_popup.addEventListener("close", (e)=>{
  connect(false);
})
login_popup.addEventListener("cancel", (e)=>{
  e.preventDefault();
})

connectbtn.addEventListener("click", ()=>{
  login_popup.close();
  sendinput.focus();
});

profaneMessageDialog.addEventListener("close", ()=> {
  if (profanityCoolDown > 0) {
    profaneMessageDialog.showModal();
  }
});
profaneMessageOk.addEventListener("click", () => {
  profaneMessageDialog.close();
})

username_field.value = localStorage.getItem("username");
login_popup.showModal();

