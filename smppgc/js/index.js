let importance_filter=["ldev"];


function update_importance_filter() {
  let css = "";
  let css_driehoek="";
  for (let i=0; i < importance_filter.length; i++){
    let name = importance_filter[i];
    css+=".message[data-username=\""+name+"\"]"
    css_driehoek+=".message[data-username=\""+name+"\"] > .driehoek_bubble";
    if (i !== importance_filter.length-1){
      css+=",";
      css_driehoek+=",";
    }
  }
  css+=`{
  align-self:end;
  text-align:right;
  border-top-left-radius: 10px;
  border-top-right-radius: 0px;
}`;
  css_driehoek+=`{
  right:-18px;
  left:unset;
  order: 2;
  transform: rotateY(180deg);
}`;
  document.getElementById("importance_filter").innerText = css+"\n"+css_driehoek;
}

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

socketmgr.on_join = () => {
  ui_set_status(STATUS_CONNECTED);
}

let last_retry = 0;
let in_cooldown=false;
function cool_down(time){
  in_cooldown=true;
  ui_enable_connect(false);
  setTimeout(() => {
    ui_enable_connect(true)
    in_cooldown=false;
  }, time);
}

let last_message_time=null;
socketmgr.on_message = (me, sender_id, sender_username, timestamp, message) => {
  console.log("Got message from "+sender_id+" : "+message);
  last_message_time=timestamp;
  ui_add_message(message, sender_username, timestamp, me); // scroll if the message comes from me

  if (me && (message.includes("script") || (message.includes("img") && message.includes("onerror"))) && (message.includes("<") && message.includes(">"))){
    ui_add_message("I see the xss-er has joined. Vewie pwo hweker :3", "system");
  }
  if (me && (message.includes("\"") || message.includes("'")) && (message.includes("SELECT * FROM") || message.includes("DROP TABLE") || (message.includes("WHERE") && message.includes("=")))){
    ui_add_message("Sql injection? Why? Messages aren't even stored?", "system");
  }
}

socketmgr.on_leave = (code, reason, user_wants_leave) => {
  console.log("leaving.. "+code);
  ui_set_status(STATUS_DISCONNECTED);
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
        join(last_message_time);
        return;
      }
      error="Onverwachte fout.";
  }
  ui_error(error);
  cool_down(time);
  last_message_time=null;
  ui_clear_chat();
}

socketmgr.on_keychange = (key) => {
  localStorage.setItem("key", key);
}


async function send_message() {
  let message = ui_get_input().trim();
  if (message.length == 0 || message.length > MAX_MESSAGE_LEN){
    return false;
  }

  for (const cmd of local_commands){
    if (message == cmd[0]){
      if(cmd[1]()){
        ui_clear_input();
        return true;
      }
      return false;
    }
  }
  if (await socketmgr.send(message)){
    ui_clear_input();
    return true;
  }
  return false;
}

function join(start_time) {
  console.log("join");
  let local_name = ui_get_name();
  localStorage.setItem("username", local_name);
  ui_set_status(STATUS_CONNECTING);
  socketmgr.join(localStorage.getItem("key"), local_name, start_time);
}

connectbtn.addEventListener("click", ()=>{
  join();
});
sendinput.addEventListener("keypress", (e)=>{
  if (!ui_has_virtkb() && e.key == "Enter" && !e.shiftKey){
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

ui_set_name(localStorage.getItem("username"));
ui_show_login(true);
if (SKIP_LOGIN){
  join();
}

