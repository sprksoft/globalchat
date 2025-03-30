import * as utils from './../utils.js';
import { Message } from './mesg.js';


// Range 0-3 is for message packets
const PACKET_SETUP = 4;
const PACKET_USERJOIN = 5;
const PACKET_PROFANITY_WARN = 6;
const PACKET_MESSAGE_DEL = 7;
const PACKET_MESSAGE_CENSOR = 8;


const CLOSED=3;
const KEY_LENGTH=33;


const ERRORS = {
  "err_cmd": "Je bent gekicked door een admin.",
  "err_ratelimit":"Te veel berichten. Typ langzamer.\nJe kunt terug joinen binnen een paar seconden",
  "err_ipratelimit":"Er zijn spammers met het zelfde ip als jou.",
  "err_toomanyusers": "Er worden op dit moment te veel nieuwe gebruikers gemaakt. Dit komt waarschijnlijk door spammers.",
  "err_505": "Stop and wait a sec when you look at me like that my darling what do you expect. In my imagination you're waiting lying on your side with your hands between your theights.",
  "err_full": "De chat zit vol.",
  "err_shutdown": "Globalchat gaat even offline. Sorry voor het ongemak",

  "err_username_invalid":"Gebruikersnaam bevat ongeldige letters.",
  "err_username_length":"Gebruikersnaam is te kort of te lang.",
  "err_username_taken":"Gebruikersnaam is bezet.",
  "err_username_prof":"Gebruikersnaam is ongepast",
}

export function human_err(protoerr) {
  if (protoerr == "err_ratelimit" && Math.floor(Math.random * 505) == 1) {
    return ERRORS["err_505"];
  }
  let herr = ERRORS[protoerr];
  if (!herr){
    herr="Onverwachte fout.";
  }
  return herr;
}

class Reader {
  #dv;
  #index;
  constructor(dv){
    this.dv = dv;
    this.index = 0;
    this.tdecoder = new TextDecoder();
  }

  getString(offset, length) {
    let len = typeof length == 'number' ? length : this.dv.byteLength-(this.index+offset);
    let dv = new DataView(this.dv.buffer, this.index+offset, len);
    this.index+=len;
    return this.tdecoder.decode(dv);
  };

  getUint8(offset=0){
    let out = this.dv.getUint8(this.index+offset, false);
    this.index+=1;
    return out;
  }
  getUint16(offset=0){
    let out = this.dv.getUint16(this.index+offset, false);
    this.index+=2;
    return out;
  }
  getUint32(offset=0){
    let out = this.dv.getUint32(this.index+offset, false);
    this.index+=4;
    return out;
  }
  getSnowflake(offset=0){
    let out = this.dv.getBigUint64(this.index+offset, false);
    this.index+=8;
    return out;
  }

  getDate(offset=0){
    return new Date((this.getUint32(offset)*1000*60))
  }

  end(){
    return this.index >= this.dv.byteLength;
  }
}

function handle_version_check(protocol_ver, ver) {
  if (protocol_ver !== ver){
    let last_reload_time = localStorage.getItem("last_client_outdated_reload");
    let now = new Date().getTime();
    if (last_reload_time == null || now-parseInt(last_reload_time) > 1000){
      localStorage.setItem("last_client_outdated_reload", now);
      console.log("NEW PROTOCOL VERSION. RELOADING PAGE TO UPDATE CLIENT");
      location.reload();
    }else{
      console.log("protocol_ver: "+protocol_ver+" page_ver: "+ver);
      console.error("Infinite reload loop detected");
      alert("Alles is kapot aaaaaaaaaaaaa.");
    }
  }
}

export class SocketMgr {
  on_message;
  on_message_del;
  on_message_sensor;
  on_profanity_warn;
  on_leave;
  on_join;
  on_keychange;

  #local_id;
  #users;
  #user_wants_leave;

  constructor(){
    this.users={};
  }
  #on_packet(packetId, reader) {
    //utils.log("Got packet "+packetId);
    if (packetId >= 0 && packetId < 4) { // Message packet
      let mod_badge = (packetId >> 1) & 0b0000_0001;
      let contains_prof = packetId & 0b0000_0001;
      const sender_id = reader.getUint16(0);
      const snowflake = reader.getSnowflake(0);
      let content = reader.getString(0);
      let sender_username = this.users[sender_id];
      let message = new Message(content, sender_username, snowflake);
      message.profanity = contains_prof;
      message.mod_badge = mod_badge;
      this.on_message(this.local_id == sender_id, sender_id, message);
      return;
    }

    switch(packetId) {
      case PACKET_SETUP:
        this.on_join();
        let version = reader.getUint16(0);
        utils.log("Protocol version: "+version+ " My version: "+VERSION_INT);
        handle_version_check(version, VERSION_INT);

        this.local_id = reader.getUint16();
        this.users[this.local_id] = this.username;

        this.local_key = reader.getString(0, KEY_LENGTH);
        this.on_keychange(this.local_key);

        utils.log("Setup packet "+this.local_id+" "+this.local_key);
        break;

      case PACKET_USERJOIN:
        let id = reader.getUint16(0);
        let username = reader.getString(0)
        utils.log("user join: "+username+" ("+id+")");
        this.users[id] = username;
        break;

      case PACKET_PROFANITY_WARN:
        let start = reader.getUint16(0);
        let end = reader.getUint16(0);
        let msgLen = reader.getUint16(0);
        let message = reader.getString(0, msgLen);
        let badWord = reader.getString(0);
        this.on_profanity_warn(message, badWord,start,end);
        break;

      case PACKET_MESSAGE_DEL:
        const msgId = reader.getSnowflake(0);
        this.on_message_del(msgId);
        break;
      case PACKET_MESSAGE_CENSOR:
        const message_id = reader.getSnowflake(0);
        this.on_message_censor(message_id);
        break;

      default:
        console.error("PROTOCOL_ERROR: Invalid subid ("+packetId+") packet recieved");
        break;
    }

  }


  async join(key, username, start_snowflake, show_admin_badge){
    this.user_wants_leave=false;
    this.username = username;
    if (this.ws !== undefined){
      await this.ws.close();
    }
    let encoded_username = encodeURIComponent(username);
    let query=`username=${encoded_username}`;
    if (key !== undefined && key !== null && key !== "") {
      query+="&key="+key;
    }
    if (start_snowflake !== undefined && start_snowflake !== null){
      query+="&start_time="+start_snowflake;
    }
    if (show_admin_badge) {
      query+="&mod_badge=true";
    }
    let fullurl = WEBSOCKET_URL+"?"+query;
    utils.log("creating socket: "+fullurl);
    this.ws = new WebSocket(fullurl);
    this.ws.binaryType = "arraybuffer";

    this.ws.onclose = async (e) => {
      this.users={};
      console.error("disconnect reason: "+e.reason)
      this.on_leave(e.code, e.reason, this.user_wants_leave);
    }

    this.ws.onmessage = async (e) =>{
      let data = e.data;
      if (data instanceof ArrayBuffer){
        let reader = new Reader(new DataView(data))
        let packetId = reader.getUint8();
        this.#on_packet(packetId, reader);
      }
    };
  }

  async deleteMessage(snowflake) {
    if (this.ws.readyState !== WebSocket.OPEN){
      return false;
    }
    await this.ws.send("%admin /delmsg "+snowflake);
  }

  async send(message){
    if (this.ws.readyState !== WebSocket.OPEN){
      return false;
    }
    if (this.ws.bufferedAmount > 2){
      return false;
    }
    await this.ws.send(message);
    return true;
  }

  async leave(){
    this.user_wants_leave=true;
    await this.ws.close(1000, "Dag dag ik ga je missen. xxx");
  }

}
