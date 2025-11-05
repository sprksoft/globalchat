import { Snowflake } from "./gctime";
import { Message } from "./mesg";
import { User } from "./user";
import { ProtoError } from "./protoerr";
import { Ban } from "./ban";
import { PacketC2SId, PacketId } from "./packets";
import { type LocalId, Role } from "./user";
import { Reader, Writer } from "./rw";
import { WFTag } from './wf';
export { ProtoError };

export declare const WEBSOCKET_URL: string;
export declare const ROLE: Role;


export type ApiVersion = number;

export class GCClient {
  on_message: ((sender_id: LocalId, message: Message) => void) | null = null;
  on_message_del: ((snowflake: Snowflake) => void) | null = null;
  on_leave: ((data: string | Ban, protoerr: ProtoError) => void) | null = null;
  on_join: ((apiVersion: ApiVersion) => void) | null = null;

  #ws: WebSocket | null = null;
  #localId: LocalId = -1;
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
          const tag = WFTag.fromNum(reader.getUint8(0));
          const len = reader.getUint16(0);
          content.push({ tag: tag, word: reader.getString(len) })
        }

        let sender = this.#users[sender_id];
        if (!sender) {
          console.error("sender id not found");
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
        const apiVersion = reader.getUint16(0) as ApiVersion;

        this.#localId = reader.getUint16();
        this.#users[this.#localId] = new User(
          this.#username,
          this.#modBadge,
          ROLE,
        );

        this.on_join?.(apiVersion);
        break;

      case PacketId.MODJOIN:
      case PacketId.USERJOIN:
        let id = reader.getUint16(0);
        let role = reader.getUint8(0);
        let username = reader.getString();
        this.#users[id] = new User(
          username,
          packetId === PacketId.MODJOIN,
          role,
        );
        break;
      case PacketId.MESSAGE_DEL:
        const msgId = reader.getSnowflake(0);
        this.on_message_del?.(msgId);
        break;

      default:
        console.error(
          "PROTOCOL_ERROR: Invalid subid (" + packetId + ") packet recieved",
        );
        break;
    }
  }

  localUser(): User {
    // local user always exists
    return this.#users[this.#localId] as User;
  }
  localId(): LocalId {
    return this.#localId;
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
    this.#ws = new WebSocket(fullurl);
    this.#ws.binaryType = "arraybuffer";

    this.#ws.onclose = async (e) => {
      this.#users = {};
      let protoerr = e.reason as ProtoError;

      let data;
      if (
        this.#user_wants_leave ||
        (protoerr == ProtoError.Ok && e.code == 1000)
      ) {
        // Normal Closure or the user wants to leave
        data = "";
      } else if (e.code == 1006 && protoerr == ProtoError.Ok) {
        protoerr = ProtoError.Retry;
        data = "Kon niet verbinden met de server.";
      } else if (protoerr.startsWith("err_banned:")) {
        data = Ban.parse(e.reason);
        protoerr = ProtoError.Banned;
        if (data == null) {
          console.error("Can't parse ban received from api");
          protoerr = ProtoError.Protocol;
          data = ProtoError.humanize(protoerr);
        }
      } else {
        data = ProtoError.humanize(protoerr);
      }

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

  wfMarkWord(word: string, good: boolean): boolean {
    if (!this.#ws || this.#ws.readyState !== WebSocket.OPEN) {
      return false;
    }
    const writer = new Writer(1 + word.length);
    writer.setUint8(good ? PacketC2SId.WF_MARKGOOD : PacketC2SId.WF_MARKBAD);
    writer.setString(word);
    this.#ws.send(writer.finish());

    return true;
  }
  wfCommit() {
    if (!this.#ws || this.#ws.readyState !== WebSocket.OPEN) {
      return false;
    }
    const writer = new Writer(1);
    writer.setUint8(PacketC2SId.WF_COMMIT);
    this.#ws.send(writer.finish());
  }

  sendString(message: string): boolean {
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
