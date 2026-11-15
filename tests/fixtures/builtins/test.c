#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool str_eq(JsString s, const char* expected) {
    char buf[256];
    uint32_t n = JsString_len(s);
    if (n != ts_strlen(expected) || n > sizeof buf) return false;
    JsString_read(s, buf);
    for (uint32_t i = 0; i < n; i++)
        if (buf[i] != expected[i]) return false;
    return true;
}

__attribute__((export_name("run"))) void run(void) {
    js_scope_open();

    // ArrayBuffer: bulk from, read, write, zero-length.
    const uint8_t init_bytes[4] = { 1, 2, 3, 4 };
    ArrayBuffer ab = ArrayBuffer_from(init_bytes, 4);
    CHECK(ArrayBuffer_byteLength(ab) == 4);
    ArrayBuffer ab2 = roundTripBytes(ab);
    uint8_t out_bytes[4] = {0};
    CHECK(ArrayBuffer_read(ab2, 0, out_bytes, 4) == 4);
    CHECK(out_bytes[0] == (1 ^ 0x5a) && out_bytes[3] == (4 ^ 0x5a));
    uint8_t mid[2] = {0};
    CHECK(ArrayBuffer_read(ab2, 1, mid, 2) == 2);
    CHECK(mid[0] == (2 ^ 0x5a) && mid[1] == (3 ^ 0x5a));
    const uint8_t patch[2] = { 99, 100 };
    CHECK(ArrayBuffer_write(ab, 2, patch, 2) == 2);
    CHECK(ArrayBuffer_read(ab, 0, out_bytes, 4) == 4);
    CHECK(out_bytes[0] == 1 && out_bytes[2] == 99 && out_bytes[3] == 100);
    ArrayBuffer ab_empty = ArrayBuffer_new(0);
    CHECK(ArrayBuffer_byteLength(ab_empty) == 0);
    CHECK(ArrayBuffer_read(ab_empty, 0, out_bytes, 4) == 0);

    // SharedArrayBuffer: in-place mutation visible both ways.
    const uint8_t sab_init[3] = { 5, 10, 15 };
    SharedArrayBuffer sab = SharedArrayBuffer_from(sab_init, 3);
    CHECK(SharedArrayBuffer_byteLength(sab) == 3);
    CHECK(mutateShared(sab) == 30.0);
    uint8_t sab_after[3] = {0};
    CHECK(SharedArrayBuffer_read(sab, 0, sab_after, 3) == 3);
    CHECK(sab_after[0] == 15 && sab_after[1] == 10 && sab_after[2] == 15);

    // ArrayBufferView with byteOffset > 0.
    ArrayBufferView sub = makeSubView();
    CHECK(ArrayBufferView_byteLength(sub) == 4);
    CHECK(ArrayBufferView_byteOffset(sub) == 2);
    uint8_t sub_buf[4] = {0};
    CHECK(ArrayBufferView_read(sub, 0, sub_buf, 4) == 4);
    CHECK(sub_buf[0] == 12 && sub_buf[3] == 15);
    JsValue backing = ArrayBufferView_buffer(sub);
    CHECK(backing.h != 0);

    // DataView.
    const uint8_t dv_in[3] = { 10, 20, 30 };
    DataView dv = DataView_from(dv_in, 3);
    DataView dv_out = flipDataView(dv);
    uint8_t dv_bytes[3] = {0};
    CHECK(DataView_read(dv_out, 0, dv_bytes, 3) == 3);
    CHECK(dv_bytes[0] == 30 && dv_bytes[1] == 20 && dv_bytes[2] == 10);

    // Uint8Array & Uint8ClampedArray.
    Uint8Array u8 = makeUint8(5);
    CHECK(Uint8Array_length(u8) == 5);
    uint8_t u8_buf[5] = {0};
    CHECK(Uint8Array_read(u8, 0, u8_buf, 5) == 5);
    CHECK(u8_buf[0] == 3 && u8_buf[4] == 31);

    const uint8_t clamp_in[3] = { 10, 100, 200 };
    Uint8ClampedArray cl = clampBytes(Uint8ClampedArray_from(clamp_in, 3));
    uint8_t clamp_out[3] = {0};
    CHECK(Uint8ClampedArray_read(cl, 0, clamp_out, 3) == 3);
    CHECK(clamp_out[0] == 210 && clamp_out[1] == 255 && clamp_out[2] == 255);

    // Int16Array, Int32Array, Uint32Array.
    const int16_t i16_in[2] = { -123, 456 };
    int16_t i16_out[2] = {0};
    Int16Array i16 = echoInt16(Int16Array_from(i16_in, 2));
    CHECK(Int16Array_read(i16, 0, i16_out, 2) == 2);
    CHECK(i16_out[0] == -123 && i16_out[1] == 456);

    const int32_t i32_in[2] = { 0x01020304, -5 };
    int32_t i32_out[2] = {0};
    Int32Array i32 = xorInts(Int32Array_from(i32_in, 2));
    CHECK(Int32Array_read(i32, 0, i32_out, 2) == 2);
    CHECK(i32_out[0] == (0x01020304 ^ 0x12345678));
    // Out-of-bounds elem_offset that would wrap 32-bit `elem_offset * 4` to 0:
    int32_t oob_i32[2] = {0};
    const int32_t oob_write_val = 999;
    CHECK(Int32Array_read(i32, 0x40000000u, oob_i32, 2) == 0);
    CHECK(Int32Array_write(i32, 0x40000000u, &oob_write_val, 1) == 0);
    CHECK(Int32Array_read(i32, 0, i32_out, 2) == 2 && i32_out[0] == (0x01020304 ^ 0x12345678));

    const uint32_t u32_in[2] = { 100u, 4000000000u };
    uint32_t u32_out[2] = {0};
    Uint32Array u32 = echoUint32(Uint32Array_from(u32_in, 2));
    CHECK(Uint32Array_read(u32, 0, u32_out, 2) == 2);
    CHECK(u32_out[0] == 100u && u32_out[1] == 4000000000u);

    // Float32Array & Float64Array, including UNALIGNED wasm pointer!
    const float f32_in[3] = { 1.5f, 2.25f, -0.75f };
    Float32Array f32 = Float32Array_from(f32_in, 3);
    CHECK(Float32Array_length(f32) == 3 && Float32Array_byteLength(f32) == 12);
    CHECK(sumFloats(f32) == 3.0);
    // Unaligned dst buffer (offset +1 byte):
    uint8_t unaligned[16] = {0};
    CHECK(Float32Array_read(f32, 0, (float*)(void*)(unaligned + 1), 3) == 3);
    float f32_check = 0.0f;
    __builtin_memcpy(&f32_check, unaligned + 1, sizeof(float));
    CHECK(f32_check == 1.5f);

    const double f64_in[2] = { 2.5, -4.0 };
    Float64Array f64_scaled = scaleDoubles(Float64Array_from(f64_in, 2), 3.0);
    double f64_out[2] = {0};
    CHECK(Float64Array_read(f64_scaled, 0, f64_out, 2) == 2);
    CHECK(f64_out[0] == 7.5 && f64_out[1] == -12.0);

    // BigInt64Array & BigUint64Array.
    const int64_t b64_in[2] = { 1000000000LL, -250000000LL };
    CHECK(sumBigInts(BigInt64Array_from(b64_in, 2)) == 750000000.0);
    const uint64_t bu64_in[2] = { 42ULL, 9007199254740993ULL };
    uint64_t bu64_out[2] = {0};
    BigUint64Array bu64 = echoBigUint64(BigUint64Array_from(bu64_in, 2));
    CHECK(BigUint64Array_read(bu64, 0, bu64_out, 2) == 2);
    CHECK(bu64_out[0] == 42ULL && bu64_out[1] == 9007199254740993ULL);

    // Error: caught exception & returned/constructed Error handles.
    CHECK(mayThrow(false) == 42.0 && !js_failed());
    CHECK(mayThrow(true) == 0.0 && js_failed());
    JsValue caught = js_take_error();
    CHECK(caught.h != 0);
    CHECK(str_eq(js_error_name(caught), "TypeError"));
    CHECK(str_eq(js_error_message(caught), "bad input"));
    CHECK(JsString_len(js_error_stack(caught)) > 0);

    Error re = makeRangeError("out of range");
    CHECK(str_eq(Error_name(re), "RangeError"));
    CHECK(str_eq(Error_message(re), "out of range"));
    CHECK(JsString_len(Error_stack(re)) > 0);

    Error mine = Error_new("from wasm");
    CHECK(isJsError(mine, "from wasm"));

    // Date: fromTime, now, time, toISOString.
    Date d = Date_fromTime(1700000000000.0);
    CHECK(Date_time(d) == 1700000000000.0);
    CHECK(str_eq(Date_toISOString(d), "2023-11-14T22:13:20.000Z"));
    CHECK(Date_now() > 1700000000000.0);
    Date d2 = addOneDay(makeDate(1700000000000.0));
    CHECK(Date_time(d2) == 1700086400000.0);

    js_scope_close();
    CHECK(js_live_handles() == 0);
}
