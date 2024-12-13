const leavebtn = document.getElementById("leavebtn");
const sendbtn = document.getElementById("sendbtn");
const username_field = document.getElementById("name-input");
const connectbtn = document.getElementById("connectbtn");
const constatus = document.getElementById("connection-status");
const err_mesg = document.getElementById("err-mesg");

const login_popup=document.getElementById("login");

let login_showed=false;

function ui_show_login(show) {
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
function ui_show_constatus(show){
  if (show){
    constatus.style="";
  }else{
    constatus.style="display:none";
  }
}

async function ui_add_message(message, sender, timestamp, scroll=false){
  let top_el = document.createElement("div");
  top_el.classList.add("message_top");
  mksender(sender, top_el);
  mkspace(top_el);
  mktime(timestamp, top_el);

  let content_el = document.createElement("div");
  content_el.classList.add("content");
  mkcontent(message, content_el);

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


// Er is geen betere manier om dit te doen denk ik.
function ui_has_virtkb(){
  return /Mobi|Android|iPad|iPhone|Tablet|Touch/i.test(navigator.userAgent);
}
