declare function roundTripBytes(buf: ArrayBuffer): ArrayBuffer;
declare function mutateShared(sab: SharedArrayBuffer): number;
declare function makeSubView(): ArrayBufferView;
declare function flipDataView(dv: DataView): DataView;
declare function makeUint8(len: number): Uint8Array;
declare function clampBytes(a: Uint8ClampedArray): Uint8ClampedArray;
declare function echoInt16(a: Int16Array): Int16Array;
declare function xorInts(a: Int32Array): Int32Array;
declare function echoUint32(a: Uint32Array): Uint32Array;
declare function sumFloats(a: Float32Array): number;
declare function scaleDoubles(a: Float64Array, factor: number): Float64Array;
declare function sumBigInts(a: BigInt64Array): number;
declare function echoBigUint64(a: BigUint64Array): BigUint64Array;

declare function mayThrow(fail: boolean): number;
declare function makeRangeError(msg: string): Error;
declare function isJsError(e: Error, expectedMsg: string): boolean;

declare function makeDate(ms: number): Date;
declare function addOneDay(d: Date): Date;
