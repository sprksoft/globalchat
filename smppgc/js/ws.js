const CLOSED=3;
const SUBID_SETUP=0;
const SUBID_USERJOIN=1;
const KEY_LENGTH=33;

class Reader{
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

  getDate(offset=0){
    return new Date((this.getUint32(offset)*1000*60))
  }

  end(){
    return this.index >= this.dv.byteLength;
  }
}

function into_gcdate(date){
  return (date.getTime()/1000)/60
}

function handle_version_check(protocol_ver, ver){
  if (protocol_ver > ver){
    let last_reload_time = localStorage.getItem("last_client_outdated_reload");
    let now = new Date().getTime();
    if (last_reload_time == null || now-parseInt(last_reload_time) > 1000){
      localStorage.setItem("last_client_outdated_reload", now);
      console.log("NEW PROTOCOL VERSION. RELOADING PAGE TO UPDATE CLIENT");
      location.reload();
    }else{
      console.error("Infinite reload loop detected");
      alert("Alles is kapot aaaaaaaaaaaaa.");
    }
  }
  if (protocol_ver < ver){
    console.error("Server older than me");
    alert("Alles is kapot aaaaaaaaaaaaa.\nOp een of anderen manier is deze versie van global chat nieuwer dan de server");
  }
}

class SocketMgr{
  on_message;
  on_leave;
  on_join;
  on_keychange;

  #local_id;
  #users;
  #user_wants_leave;

  constructor(){
    this.users={};
  }

  #on_special_message(sub_id, reader){
    switch(sub_id){
      case SUBID_SETUP:
        this.on_join();
        let version = reader.getUint16(0);
        if (LOG){
          console.log("Protocol version: "+version+ " My version: "+VERSION_INT);
        }
        handle_version_check(version, VERSION_INT);

        this.local_id = reader.getUint16();

        this.local_key = reader.getString(0, KEY_LENGTH);
        this.on_keychange(this.local_key);

        let client_count = reader.getUint16();
        for (let i = 0; i < client_count; i++){
          let id = reader.getUint16();
          let name_length = reader.getUint8();
          let username = reader.getString(0, name_length);
          this.users[id]=username;
          if (LOG){
            console.log("(hist_user) "+username+" ("+id+")")
          }
        }

        while(!reader.end()){
          let timestamp = reader.getDate();
          let username_length=reader.getUint8();
          let username = reader.getString(0, username_length);
          let mesg_length=reader.getUint8();
          let message = reader.getString(0, mesg_length);
          this.on_message(this.local_id, -1, username, timestamp, message);
        }

        if (LOG){
          console.log("Setup packet "+this.local_id+" "+this.local_key);
        }
        break;
      case SUBID_USERJOIN:
        let id = reader.getUint16(0);
        let username = reader.getString(0)
        if (LOG){
          console.log("user join: "+username+" ("+id+")");
        }
        this.users[id] = username;
        break;
      default:
        console.error("PROTOCOL_ERROR: Invalid subid ("+sub_id+") packet recieved");
        break;
    }

  }

  async join(key, username, start_time){
    this.user_wants_leave=false;
    if (this.ws !== undefined){
      await this.ws.close();
    }
    let encoded_username = encodeURIComponent(username);
    let query=`username=${encoded_username}`;
    if (key !== undefined && key !== null && key !== ""){
      query+="&key="+key;
    }
    if (start_time !== undefined && start_time !== null){
      query+="&start_time="+into_gcdate(start_time);
    }
    let fullurl = WEBSOCKET_URL+"?"+query;
    if (LOG){
      console.log("creating socket: "+fullurl);
    }
    this.ws = new WebSocket(fullurl);
    this.ws.binaryType = "arraybuffer";

    this.ws.onclose = async (e) => {
      this.users={};
      let reason = e.reason;
      if (!e.reason || e.reason.startsWith("INT:")){
        if (e.reason) {
          console.error("Internal Error: "+e.reason);
        }else{
          console.error("Reason empty");
          console.error(e);
        }
        reason="Onverwachte fout.";
      }
      this.on_leave(e.code, reason, this.user_wants_leave);
    }

    this.ws.onmessage = async (e) =>{
      let data = e.data;
      if (data instanceof ArrayBuffer){
        let reader = new Reader(new DataView(data))
        const sender_id = reader.getUint16();
        if (sender_id == 0){ // user 0 is special message
          let sub_id = reader.getUint8();
          this.#on_special_message(sub_id, reader);
        }else{
          const timestamp = reader.getDate();
          let message = reader.getString(0);
          let sender_username = this.users[sender_id];
          let me = this.local_id == sender_id;
          if (me){
            sender_username = username;
          }
          this.on_message(me, sender_id, sender_username, timestamp, message);
        }
      }
    };
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
