const mesgs = document.getElementById("mesgs");
const sendinput = document.getElementById("send-input");

const LOG = localStorage.getItem("LOG") == "true";

let socketmgr = new SocketMgr();

local_commands.push(["/clearkey", function () {
    localStorage.removeItem("key");
    ui_add_message("key cleared.", "system");
    return true;
}]);
local_commands.push(["/leave", function () {
  socketmgr.leave();
  return true;
}]);

let background_reconnect=false;

socketmgr.on_join = () => {
  if (!background_reconnect){
    sendinput.focus();
    ui_show_login(false);
  }
  ui_show_constatus(false);
  background_reconnect=false;
}

let last_retry = 0;
let in_cooldown=false;
function cool_down(time){
  if (!login_showed){
    ui_show_login(true);
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
  if (LOG) {
    console.log("Got message from "+sender_id+" : "+message);
  }
  ui_add_message(message, sender_username, timestamp, me); // scroll if the message comes from me

  if (me && (message.includes("script") || (message.includes("img") && message.includes("onerror"))) && (message.includes("<") && message.includes(">"))){
    ui_add_message("I see the xss-er has joined. Vewie pwo hweker :3", "system");
  }
  if (me && (message.includes("\"") || message.includes("'")) && (message.includes("SELECT * FROM") || message.includes("DROP TABLE") || (message.includes("WHERE") && message.includes("=")))){
    ui_add_message("Sql injection? Why? Messages aren't even stored?", "system");
  }
}

socketmgr.on_leave = (code, reason, user_wants_leave) => {
  if (LOG){
    console.log("disconnected "+code);
  }
  ui_show_constatus(false);

  error = reason
  time = 1000;
  if (user_wants_leave || code == 1000){ // Normal Closure or the user wants to leave
    error="";
  }else if (code == 1008){ // Policy (kick for spamming)
    time=5000;
  }else if (code == 1006){
      let now = Date.now();
      if (last_retry == 0 || now-last_retry > 10_000){ // join again if we should retry
        last_retry = now;
        connect(true, last_message_time);
        return;
      }
      error="Onverwachte fout.";
  }
  err_mesg.innerText=error;
  cool_down(time);

  //reset everything
  last_message_time=null;
  mesgs.innerHTML="";
  ui_show_login(true);
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
  if (LOG){
    console.log("connecting... in_background="+background);
  }
  let local_name = get_name();
  localStorage.setItem("username", local_name);

  background_reconnect=background;
  socketmgr.join(localStorage.getItem("key"), local_name, start_time);

  ui_show_login(false);
  ui_show_constatus(true);
}

connectbtn.addEventListener("click", ()=>{
  connect(false);
});
sendinput.addEventListener("keypress", (e)=>{
  if (e.key == "Enter" && !e.shiftKey && !ui_has_virtkb()){
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
ui_show_login(true);
if (SKIP_LOGIN){
  connect(false);
}

