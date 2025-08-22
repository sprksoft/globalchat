import { Snowflake } from "../snowflake.ts";
import { log } from "../../common/utils.js";
import { Message } from "./../mesg.ts";
import { User } from "../user.ts";
import { ProtoError } from "./protoerr.ts";
import { parseBan, type Ban } from "../ban.js";
import { PacketC2SId, PacketId } from "./packets.ts";
import { type LocalId, Role } from "../user.ts";
import { Reader, Writer } from "./rw.ts";
import type { WFTag } from "../wf.ts";
export { ProtoError };

export declare const VERSION_INT: number;
export declare const WEBSOCKET_URL: string;
export declare const ROLE: Role;

function handle_version_check(protocol_ver: number) {
  if (protocol_ver !== VERSION_INT) {
    let last_reload_time = localStorage.getItem("last_client_outdated_reload");
    let now = new Date().getTime();
    if (last_reload_time == null || now - parseInt(last_reload_time) > 1000) {
      localStorage.setItem("last_client_outdated_reload", now.toString());
      console.log("NEW PROTOCOL VERSION. RELOADING PAGE TO UPDATE CLIENT");
      location.reload();
    } else {
      console.log(
        "protocol_ver: " + protocol_ver + " page_ver: " + VERSION_INT,
      );
      console.error("Infinite reload loop detected");
      alert("Alles is kapot aaaaaaaaaaaaa.");
    }
  }
}

export class SocketMgr {
  on_message: ((sender_id: LocalId, message: Message) => void) | null = null;
  on_message_del: ((snowflake: Snowflake) => void) | null = null;
  on_message_censor: ((Snowflake: Snowflake) => void) | null = null;
  on_profanity_warn:
    | ((message: string, badWord: string, start: number, end: number) => void)
    | null = null;
  on_leave: ((data: string | Ban, protoerr: ProtoError) => void) | null = null;
  on_join: (() => void) | null = null;

  #ws: WebSocket | null = null;
  #local_id: LocalId = -1;
  #users: { [id: LocalId]: User } = {};
  #user_wants_leave: boolean = false;
  #modBadge: boolean = false;
  #username: string = "";

  #on_packet(packetId: PacketId, reader: Reader) {
    switch (packetId) {
      case PacketId.MESSAGE:
        const sender_id = reader.getUint16(0);
        const snowflake = reader.getSnowflake(0);

        let content = [];
        while (!reader.end()) {
          const tag = reader.getUint8(0) as WFTag;
          const len = reader.getUint16(0);
          content.push({ wf: tag, word: reader.getString(len) })
        }

        let sender = this.#users[sender_id];
        if (!sender) {
          sender = User.nonExisting();
        }
        let message = new Message(content, sender, snowflake);
        message.mod_badge = sender.modBadge;
        this.on_message?.(sender_id, message);
        break;
      case PacketId.MESSAGE_SYSTEM: {
        let content = reader.getString();
        let message = Message.system(content);
        this.on_message?.(-1, message);
        break;
      }

      case PacketId.SETUP:
        this.on_join?.();
        let version = reader.getUint16(0);
        log("Protocol version: " + version + " My version: " + VERSION_INT);
        handle_version_check(version);

        this.#local_id = reader.getUint16();
        this.#users[this.#local_id] = new User(
          this.#username,
          this.#modBadge,
          ROLE,
        );

        log("Setup packet " + this.#local_id);
        break;

      case PacketId.MODJOIN:
      case PacketId.USERJOIN:
        let id = reader.getUint16(0);
        let role = reader.getUint8(0);
        let username = reader.getString();
        log("user join: " + username + " (" + id + ")" + " role: " + role);
        this.#users[id] = new User(
          username,
          packetId === PacketId.MODJOIN,
          role,
        );
        break;

      case PacketId.PROFANITY_WARN:
        let start = reader.getUint16(0);
        let end = reader.getUint16(0);
        let msgLen = reader.getUint16(0);
        let msg = reader.getString(msgLen);
        let badWord = reader.getString();
        this.on_profanity_warn?.(msg, badWord, start, end);
        break;

      case PacketId.MESSAGE_DEL:
        const msgId = reader.getSnowflake(0);
        this.on_message_del?.(msgId);
        break;
      case PacketId.MESSAGE_CENSOR:
        const message_id = reader.getSnowflake(0);
        this.on_message_censor?.(message_id);
        break;

      default:
        console.error(
          "PROTOCOL_ERROR: Invalid subid (" + packetId + ") packet recieved",
        );
        break;
    }
  }

  local_user(): User {
    // local user always exists
    return this.#users[this.#local_id] as User;
  }
  local_id(): LocalId {
    return this.#local_id;
  }

  async join(
    username: string | null,
    start_snowflake: Snowflake | null,
    show_badge: boolean,
  ) {
    this.#modBadge = show_badge;
    this.#user_wants_leave = false;
    this.#username = username ? username : "";
    if (this.#ws !== null) {
      this.#ws.close();
    }
    let query = "";
    if (username) {
      let encoded_username = encodeURIComponent(username);
      query += `&username=${encoded_username}`;
    }
    if (start_snowflake !== null) {
      query += "&start_time=" + start_snowflake;
    }
    if (show_badge) {
      query += "&mod_badge=true";
    }
    let fullurl = WEBSOCKET_URL + "?" + query.substring(1);
    log("creating socket: " + fullurl);
    this.#ws = new WebSocket(fullurl);
    this.#ws.binaryType = "arraybuffer";

    this.#ws.onclose = async (e) => {
      this.#users = {};
      let protoerr = e.reason as ProtoError;
      log("disconnect protoerr: " + protoerr);

      let data;
      if (
        this.#user_wants_leave ||
        (protoerr == ProtoError.ok && e.code == 1000)
      ) {
        // Normal Closure or the user wants to leave
        data = "";
      } else if (e.code == 1006 && protoerr == ProtoError.ok) {
        protoerr = ProtoError.retry;
        data = "Kon niet verbinden met de server.";
      } else if (protoerr.startsWith("err_banned:")) {
        data = parseBan(e.reason);
        protoerr = ProtoError.err_banned;
      } else {
        data = ProtoError.humanize(protoerr);
      }

      log("disconnect reason: " + JSON.stringify(data));
      this.on_leave?.(data, protoerr);
    };

    this.#ws.onmessage = async (e) => {
      let data = e.data;
      if (data instanceof ArrayBuffer) {
        let reader = new Reader(new DataView(data));
        let packetId = reader.getUint8() as PacketId;
        this.#on_packet(packetId, reader);
      }
    };
  }

  deleteMessage(snowflake: Snowflake) {
    if (!this.#ws || this.#ws.readyState !== WebSocket.OPEN) {
      return false;
    }
    const data = new ArrayBuffer(1 + 8);
    let dv = new DataView(data);
    dv.setUint8(0, PacketC2SId.DELMSG);
    dv.setBigUint64(1, snowflake, false);
    this.#ws.send(data);
  }
  /// duration is in seconds
  banMessageAuthor(
    snowflake: Snowflake,
    duration: number,
    reason: string,
  ): boolean {
    if (!this.#ws || this.#ws.readyState !== WebSocket.OPEN) {
      return false;
    }

    const writer = new Writer(1 + 8 + 4 + reason.length);
    writer.setUint8(PacketC2SId.BANMSGAUTHOR);
    writer.setSnowflake(snowflake);
    writer.setUint32(duration);
    writer.setString(reason);

    this.#ws.send(writer.finish());

    return true;
  }

  markWord(word: string, good: boolean): boolean {
    if (!this.#ws || this.#ws.readyState !== WebSocket.OPEN) {
      return false;
    }
    const writer = new Writer(1 + word.length);
    writer.setUint8(good ? PacketC2SId.WF_MARKGOOD : PacketC2SId.WF_MARKBAD);
    writer.setString(word);
    this.#ws.send(writer.finish());

    return true;
  }

  send(message: string): boolean {
    if (!this.#ws || this.#ws.readyState !== WebSocket.OPEN) {
      return false;
    }
    if (this.#ws.bufferedAmount > 2) {
      return false;
    }
    const tencoder = new TextEncoder();
    const messageData = tencoder.encode(message);
    const data = new ArrayBuffer(messageData.length + 1);
    const array = new Uint8Array(data);
    array.set([PacketC2SId.MESSAGE], 0);
    array.set(messageData, 1);

    this.#ws.send(data);
    return true;
  }

  leave() {
    if (!this.#ws) {
      return;
    }
    this.#user_wants_leave = true;
    this.#ws.close(1000, "Dag dag ik ga je missen. xxx");
  }
}
