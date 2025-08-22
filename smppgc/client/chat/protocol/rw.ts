import type { Snowflake } from "../snowflake";

export class Reader {
  dv: DataView;
  #index: number;
  #tdecoder: TextDecoder;

  constructor(dv: DataView) {
    this.dv = dv;
    this.#index = 0;
    this.#tdecoder = new TextDecoder();
  }

  getString(length: number | null = null): string {
    const len =
      typeof length == "number"
        ? length
        : this.dv.byteLength - this.#index;
    let dv = new DataView(this.dv.buffer, this.#index, len);
    this.#index += len;
    return this.#tdecoder.decode(dv);
  }

  getUint8(offset = 0) {
    let out = this.dv.getUint8(this.#index + offset);
    this.#index += 1;
    return out;
  }
  getUint16(offset = 0) {
    let out = this.dv.getUint16(this.#index + offset, false);
    this.#index += 2;
    return out;
  }
  getUint32(offset = 0) {
    let out = this.dv.getUint32(this.#index + offset, false);
    this.#index += 4;
    return out;
  }
  getSnowflake(offset = 0): Snowflake {
    let out = this.dv.getBigUint64(this.#index + offset, false);
    this.#index += 8;
    return out;
  }

  getDate(offset = 0) {
    return new Date(this.getUint32(offset) * 1000 * 60);
  }

  end() {
    return this.#index >= this.dv.byteLength;
  }
}

export class Writer {
  #index: number = 0;
  #buf: ArrayBuffer;
  #dv: DataView;
  #tencoder: TextEncoder;

  constructor(len: number) {
    this.#buf = new ArrayBuffer(len, { maxByteLength: 50_000 });
    this.#dv = new DataView(this.#buf);
    this.#tencoder = new TextEncoder();
  }

  setUint8(value: number) {
    this.#buf.resize(this.#index + 1);
    this.#dv.setUint8(this.#index, value);
    this.#index += 1;
  }

  setUint32(value: number) {
    this.#buf.resize(this.#index + 4);
    this.#dv.setUint32(this.#index, value);
    this.#index += 4;
  }

  setSnowflake(value: Snowflake) {
    this.#buf.resize(this.#index + 4);
    this.#dv.setBigUint64(this.#index, value);
    this.#index += 4;
  }

  setArray(array: ArrayLike<number>) {
    this.#buf.resize(this.#index + array.length);
    const uint8 = new Uint8Array(this.#dv.buffer);
    uint8.set(array, this.#index);
    this.#index += array.length;
  }

  setString(value: string) {
    const encoded = this.#tencoder.encode(value);
    this.setArray(encoded);
  }

  finish(): ArrayBuffer {
    return this.#buf.transferToFixedLength(this.#buf.byteLength);
  }

}
