import * as proto from './protocol.js';
import * as utils from './utils.js';
import * as mk from './mkels.js'

import './index.css'

const mesgs = document.getElementById("mesgs");
const sendinput = document.getElementById("send-input");
const username_field = document.getElementById("name-input");
const leavebtn = document.getElementById("leavebtn");
const sendbtn = document.getElementById("sendbtn");
const connectbtn = document.getElementById("connectbtn");
const constatus = document.getElementById("connection-status");
const err_mesg = document.getElementById("err-mesg");

const login_popup=document.getElementById("login");

async function add_message(message, sender, timestamp, scroll=false){
  let top_el = document.createElement("div");
  top_el.classList.add("message_top");
  mk.mksender(sender, top_el);
  mk.mkspace(top_el);
  mk.mktime(timestamp, top_el);

  let content_el = document.createElement("div");
  content_el.classList.add("content");
  mk.mkcontent(message, content_el);

  let user_content_el=document.createElement("div");
  user_content_el.classList.add("user_content");
  user_content_el.appendChild(top_el);
  user_content_el.appendChild(content_el);
  let msg_el = document.createElement("div");
  msg_el.innerHTML=`
<svg class="driehoek_bubble" viewBox="0 0 8 13" height="13" width="8" preserveAspectRatio="xMidYMid meet" class="" version="1.1" x="0px" y="0px" enable-background="new 0 0 8 13"><path fill="currentColor" d="M1.5,2.5L8,11.2V0L2.8,0C1,0,0.5,1.2,1.5,2.6z"></path></svg>`
  msg_el.appendChild(user_content_el);
  msg_el.classList.add("message");
  msg_el.dataset.username=sender;

  let should_scroll = Math.abs(mesgs.scrollHeight - mesgs.clientHeight - mesgs.scrollTop) <= 3 || scroll;
  mesgs.appendChild(msg_el);
  if (should_scroll){
    msg_el.scrollIntoView();
  }
}

let login_showed=false;
function show_login(show) {
  if (show){
    login_showed=true;
    login_popup.className=""
    sendinput.disabled=true;
    username_field.focus();
  }else{
    login_showed=false;
    login_popup.className="hide"; sendinput.disabled=false;
  }
}
function show_constatus(show){
  if (show){
    constatus.style="";
  }else{
    constatus.style="display:none";
  }
}

let socketmgr = new proto.SocketMgr();

local_commands.push(["/clearkey", function () {
    localStorage.removeItem("key");
    add_message("key cleared.", "system");
    return true;
}]);
local_commands.push(["/leave", function () {
  socketmgr.leave();
  return true;
}]);


// Are we currently trying to reconnect in the background
let background_reconnect=false;

socketmgr.on_join = () => {
  if (!background_reconnect){
    sendinput.focus();
    show_login(false);
  }
  show_constatus(false);
  background_reconnect=false;
}

let last_retry = 0;
let in_cooldown=false;
function cool_down(time){
  if (!login_showed){
    show_login(true);
  }
  in_cooldown=true;

  connectbtn.disabled=true;
  setTimeout(() => {
  connectbtn.disabled=false;
    in_cooldown=false;
  }, time);
}

let last_message_time=null;
socketmgr.on_message = (me, sender_id, sender_username, timestamp, message) => {
  last_message_time=timestamp;
    utils.log("Got message from "+sender_id+" : "+message);
  add_message(message, sender_username, timestamp, me); // scroll if the message comes from me

  if (me && (message.includes("script") || (message.includes("img") && message.includes("onerror"))) && (message.includes("<") && message.includes(">"))){
    add_message("I see the xss-er has joined. Vewie pwo hweker :3", "system");
  }
  if (me && (message.includes("\"") || message.includes("'")) && (message.includes("SELECT * FROM") || message.includes("DROP TABLE") || (message.includes("WHERE") && message.includes("=")))){
    add_message("Sql injection? Why? Messages aren't even stored?", "system");
  }
}

socketmgr.on_leave = (code, protoerr, user_wants_leave) => {
  utils.log("disconnected "+code)
  show_constatus(false);

  error = proto.human_err(protoerr);
  time = 1000;
  if (user_wants_leave || code == 1000){ // Normal Closure or the user wants to leave
    error="";
  }else if (protoerr == "err_ratelimit"){
    time=10000;
  }else if (code == 1006 && protoerr == ""){
      let now = Date.now();
      if (last_retry == 0 || now-last_retry > 10_000){ // join again if we should retry
        last_retry = now;
        connect(true, last_message_time);
        return;
      }
  }
  err_mesg.innerText=error;
  cool_down(time);

  //reset everything
  last_message_time=null;
  mesgs.innerHTML="";
  show_login(true);
}

socketmgr.on_keychange = (key) => {
  localStorage.setItem("key", key);
}

async function send_message() {
  let message = sendinput.innerText.trim();
  if (message.length == 0 || message.length > MAX_MESSAGE_LEN){
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

function connect(background, start_time) {
  utils.log("connecting... in_background="+background);
  let local_name = get_name();
  localStorage.setItem("username", local_name);

  background_reconnect=background;
  socketmgr.join(localStorage.getItem("key"), local_name, start_time);

  show_login(false);
  show_constatus(true);
}

// Er is geen betere manier om dit te doen denk ik.
function has_virtkb(){
  return /Mobi|Android|iPad|iPhone|Tablet|Touch/i.test(navigator.userAgent);
}

connectbtn.addEventListener("click", ()=>{
  connect(false);
});
sendinput.addEventListener("keypress", (e)=>{
  if (e.key == "Enter" && !e.shiftKey && !has_virtkb()){
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
leavebtn.addEventListener("click", ()=>{
  socketmgr.leave();
});

username_field.value = localStorage.getItem("username");
show_login(true);
if (SKIP_LOGIN){
  connect(false);
}

