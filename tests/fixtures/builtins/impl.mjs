export const root = {
  roundTripBytes(buf) {
    const out = new ArrayBuffer(buf.byteLength);
    const src = new Uint8Array(buf);
    const dst = new Uint8Array(out);
    for (let i = 0; i < src.length; i++) dst[i] = src[i] ^ 0x5a;
    return out;
  },
  mutateShared(sab) {
    if (!(sab instanceof SharedArrayBuffer)) throw new TypeError('expected SharedArrayBuffer');
    const u8 = new Uint8Array(sab);
    const sum = u8.reduce((a, b) => a + b, 0);
    if (u8.length > 0) u8[0] = (u8[0] + 10) & 0xff;
    return sum;
  },
  makeSubView() {
    const ab = new ArrayBuffer(8);
    const all = new Uint8Array(ab);
    for (let i = 0; i < 8; i++) all[i] = i + 10;
    return new Uint8Array(ab, 2, 4); // [12, 13, 14, 15]
  },
  flipDataView(dv) {
    if (!(dv instanceof DataView)) throw new TypeError('expected DataView');
    const out = new DataView(new ArrayBuffer(dv.byteLength));
    for (let i = 0; i < dv.byteLength; i++) out.setUint8(i, dv.getUint8(dv.byteLength - 1 - i));
    return out;
  },
  makeUint8(len) {
    const a = new Uint8Array(len);
    for (let i = 0; i < len; i++) a[i] = (i * 7 + 3) & 0xff;
    return a;
  },
  clampBytes(a) {
    if (!(a instanceof Uint8ClampedArray)) throw new TypeError('expected Uint8ClampedArray');
    const out = new Uint8ClampedArray(a.length);
    for (let i = 0; i < a.length; i++) out[i] = a[i] + 200; // clamps at 255
    return out;
  },
  echoInt16(a) {
    return new Int16Array(a);
  },
  xorInts(a) {
    const out = new Int32Array(a.length);
    for (let i = 0; i < a.length; i++) out[i] = a[i] ^ 0x12345678;
    return out;
  },
  echoUint32(a) {
    return new Uint32Array(a);
  },
  sumFloats(a) {
    let s = 0;
    for (let i = 0; i < a.length; i++) s += a[i];
    return s;
  },
  scaleDoubles(a, factor) {
    const out = new Float64Array(a.length);
    for (let i = 0; i < a.length; i++) out[i] = a[i] * factor;
    return out;
  },
  sumBigInts(a) {
    let s = 0n;
    for (let i = 0; i < a.length; i++) s += a[i];
    return Number(s);
  },
  echoBigUint64(a) {
    return new BigUint64Array(a);
  },
  mayThrow(fail) {
    if (fail) throw new TypeError('bad input');
    return 42;
  },
  makeRangeError(msg) {
    return new RangeError(msg);
  },
  isJsError(e, expectedMsg) {
    return e instanceof Error && e.message === expectedMsg;
  },
  makeDate(ms) {
    return new Date(ms);
  },
  addOneDay(d) {
    if (!(d instanceof Date)) throw new TypeError('expected Date');
    return new Date(d.getTime() + 86400000);
  },
};

// The API is globals, as declared: for bindings that name them, such as F#'s.
Object.assign(globalThis, root);
