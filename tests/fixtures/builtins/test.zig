const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn strEq(s: b.JsString, expected: []const u8) bool {
    var buf: [256]u8 = undefined;
    if (s.len() > buf.len) return false;
    return std.mem.eql(u8, s.read(&buf), expected);
}

export fn run() void {
    const scope = b.JsScope.open();

    // ArrayBuffer
    const ab = b.ArrayBuffer.from(&.{ 1, 2, 3, 4 });
    expect(ab.byteLength() == 4, @src());
    const ab2 = b.roundTripBytes(ab);
    var ab_out: [4]u8 = undefined;
    expect(ab2.read(0, &ab_out) == 4, @src());
    expect(ab_out[0] == (1 ^ 0x5a) and ab_out[3] == (4 ^ 0x5a), @src());
    expect(ab.write(2, &.{ 99, 100 }) == 2, @src());
    expect(ab.read(0, &ab_out) == 4, @src());
    expect(ab_out[2] == 99 and ab_out[3] == 100, @src());
    const ab_empty = b.ArrayBuffer.new(0);
    expect(ab_empty.byteLength() == 0, @src());
    expect(ab_empty.read(0, &ab_out) == 0, @src());

    // SharedArrayBuffer
    const sab = b.SharedArrayBuffer.from(&.{ 5, 10, 15 });
    expect(b.mutateShared(sab) == 30.0, @src());
    var sab_out: [3]u8 = undefined;
    expect(sab.read(0, &sab_out) == 3, @src());
    expect(sab_out[0] == 15, @src());

    // ArrayBufferView
    const sub = b.makeSubView();
    expect(sub.byteLength() == 4 and sub.byteOffset() == 2, @src());
    var sub_out: [4]u8 = undefined;
    expect(sub.read(0, &sub_out) == 4, @src());
    expect(sub_out[0] == 12 and sub_out[3] == 15, @src());
    expect(!sub.buffer().isNull(), @src());

    // DataView
    const dv = b.flipDataView(b.DataView.from(&.{ 10, 20, 30 }));
    var dv_out: [3]u8 = undefined;
    expect(dv.read(0, &dv_out) == 3, @src());
    expect(dv_out[0] == 30 and dv_out[2] == 10, @src());

    // TypedArrays
    const u8a = b.makeUint8(5);
    expect(u8a.len() == 5, @src());
    var u8_out: [5]u8 = undefined;
    expect(u8a.read(0, &u8_out) == 5, @src());
    expect(u8_out[0] == 3 and u8_out[4] == 31, @src());

    const cl = b.clampBytes(b.Uint8ClampedArray.from(&.{ 10, 100, 200 }));
    var cl_out: [3]u8 = undefined;
    expect(cl.read(0, &cl_out) == 3, @src());
    expect(cl_out[0] == 210 and cl_out[1] == 255 and cl_out[2] == 255, @src());

    const i16a = b.echoInt16(b.Int16Array.from(&.{ -123, 456 }));
    var i16_out: [2]i16 = undefined;
    expect(i16a.read(0, &i16_out) == 2 and i16_out[0] == -123 and i16_out[1] == 456, @src());

    const i32a = b.xorInts(b.Int32Array.from(&.{ 0x01020304, -5 }));
    var i32_out: [2]i32 = undefined;
    expect(i32a.read(0, &i32_out) == 2 and i32_out[0] == (0x01020304 ^ 0x12345678), @src());
    // Out-of-bounds elem_offset that would wrap 32-bit `elem_offset * 4` to 0:
    var oob_i32: [2]i32 = .{ 0, 0 };
    expect(i32a.read(0x4000_0000, &oob_i32) == 0, @src());
    expect(i32a.write(0x4000_0000, &.{ 999 }) == 0, @src());
    expect(i32a.read(0, &i32_out) == 2 and i32_out[0] == (0x01020304 ^ 0x12345678), @src());

    const u32a = b.echoUint32(b.Uint32Array.from(&.{ 100, 4000000000 }));
    var u32_out: [2]u32 = undefined;
    expect(u32a.read(0, &u32_out) == 2 and u32_out[1] == 4000000000, @src());

    const f32a = b.Float32Array.from(&.{ 1.5, 2.25, -0.75 });
    expect(b.sumFloats(f32a) == 3.0, @src());
    // Unaligned destination bytes
    var unaligned: [16]u8 = undefined;
    expect(f32a.readBytes(0, unaligned[1..13]) == 12, @src());
    var f32_val: f32 = undefined;
    @memcpy(std.mem.asBytes(&f32_val), unaligned[1..5]);
    expect(f32_val == 1.5, @src());

    const f64a = b.scaleDoubles(b.Float64Array.from(&.{ 2.5, -4.0 }), 3.0);
    var f64_out: [2]f64 = undefined;
    expect(f64a.read(0, &f64_out) == 2 and f64_out[0] == 7.5 and f64_out[1] == -12.0, @src());

    expect(b.sumBigInts(b.BigInt64Array.from(&.{ 1000000000, -250000000 })) == 750000000.0, @src());
    const bu64 = b.echoBigUint64(b.BigUint64Array.from(&.{ 42, 9007199254740993 }));
    var bu64_out: [2]u64 = undefined;
    expect(bu64.read(0, &bu64_out) == 2 and bu64_out[1] == 9007199254740993, @src());

    // Error
    if (b.jsTry(b.mayThrow(false))) |v| expect(v == 42.0, @src()) else |_| expect(false, @src());
    if (b.jsTry(b.mayThrow(true))) |_| expect(false, @src()) else |_| {}
    if (b.jsTakeError()) |err| {
        expect(strEq(b.jsErrorName(err), "TypeError"), @src());
        expect(strEq(b.jsErrorMessage(err), "bad input"), @src());
        expect(b.jsErrorStack(err).len() > 0, @src());
    } else expect(false, @src());

    const re = b.makeRangeError("out of range");
    expect(strEq(re.name(), "RangeError"), @src());
    expect(strEq(re.message(), "out of range"), @src());
    expect(re.stack().len() > 0, @src());
    expect(b.isJsError(b.Error.new("from wasm"), "from wasm"), @src());

    // Date
    const d = b.Date.fromTime(1700000000000.0);
    expect(d.time() == 1700000000000.0, @src());
    expect(strEq(d.toIsoString(), "2023-11-14T22:13:20.000Z"), @src());
    expect(b.Date.now() > 1700000000000.0, @src());
    expect(b.addOneDay(b.makeDate(1700000000000.0)).time() == 1700086400000.0, @src());

    scope.close();
    expect(b.jsLiveHandles() == 0, @src());
}
