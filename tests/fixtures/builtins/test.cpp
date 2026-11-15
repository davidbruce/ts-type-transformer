#include "bindings.hpp"
#include <cstring>

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    {
        // ArrayBuffer
        std::vector<uint8_t> init_bytes = {1, 2, 3, 4};
        auto ab = ArrayBuffer::from(init_bytes);
        CHECK(ab.byte_length() == 4);
        auto ab2 = roundTripBytes(ab);
        auto out = ab2.to_vector();
        CHECK(out.size() == 4 && out[0] == (1 ^ 0x5a) && out[3] == (4 ^ 0x5a));
        uint8_t patch[2] = {99, 100};
        CHECK(ab.write(patch, 2, 2) == 2);
        CHECK(ab.to_vector()[2] == 99 && ab.to_vector()[3] == 100);
        auto empty_ab = ArrayBuffer::create(0);
        CHECK(empty_ab.byte_length() == 0 && empty_ab.to_vector().empty());

        // SharedArrayBuffer
        std::vector<uint8_t> sab_init = {5, 10, 15};
        auto sab = SharedArrayBuffer::from(sab_init);
        CHECK(mutateShared(sab) == 30.0);
        CHECK(sab.to_vector()[0] == 15);

        // ArrayBufferView
        auto sub = makeSubView();
        CHECK(sub.byte_length() == 4 && sub.byte_offset() == 2);
        CHECK(sub.to_vector()[0] == 12 && sub.to_vector()[3] == 15);
        CHECK(!sub.buffer().is_null());

        // DataView
        std::vector<uint8_t> dv_in = {10, 20, 30};
        auto dv_out = flipDataView(DataView<ArrayBufferLike>::from(dv_in));
        CHECK(dv_out.to_vector() == std::vector<uint8_t>({30, 20, 10}));

        // TypedArrays
        auto u8 = makeUint8(5);
        CHECK(u8.size() == 5 && u8.to_vector()[0] == 3 && u8.to_vector()[4] == 31);

        auto cl = clampBytes(Uint8ClampedArray<ArrayBufferLike>::from({10, 100, 200}));
        CHECK(cl.to_vector() == std::vector<uint8_t>({210, 255, 255}));

        auto i16 = echoInt16(Int16Array<ArrayBufferLike>::from({-123, 456}));
        CHECK(i16.to_vector() == std::vector<int16_t>({-123, 456}));

        auto i32 = xorInts(Int32Array<ArrayBufferLike>::from({0x01020304, -5}));
        CHECK(i32.to_vector()[0] == (0x01020304 ^ 0x12345678));
        // Out-of-bounds elem_offset that would wrap 32-bit `elem_offset * 4` to 0:
        int32_t oob_i32[2] = {};
        const int32_t oob_write_val = 999;
        CHECK(i32.read(oob_i32, 2, 0x40000000u) == 0);
        CHECK(i32.write(&oob_write_val, 1, 0x40000000u) == 0);
        CHECK(i32.to_vector()[0] == (0x01020304 ^ 0x12345678));

        auto u32 = echoUint32(Uint32Array<ArrayBufferLike>::from({100u, 4000000000u}));
        CHECK(u32.to_vector()[1] == 4000000000u);

        auto f32 = Float32Array<ArrayBufferLike>::from({1.5f, 2.25f, -0.75f});
        CHECK(sumFloats(f32) == 3.0);
        uint8_t unaligned[16] = {};
        CHECK(f32.read(reinterpret_cast<float*>(unaligned + 1), 3) == 3);
        float f32_val = 0.0f;
        std::memcpy(&f32_val, unaligned + 1, sizeof(float));
        CHECK(f32_val == 1.5f);

        auto f64 = scaleDoubles(Float64Array<ArrayBufferLike>::from({2.5, -4.0}), 3.0);
        CHECK(f64.to_vector()[0] == 7.5 && f64.to_vector()[1] == -12.0);

        CHECK(sumBigInts(BigInt64Array<ArrayBufferLike>::from({1000000000LL, -250000000LL})) == 750000000.0);
        auto bu64 = echoBigUint64(BigUint64Array<ArrayBufferLike>::from({42ULL, 9007199254740993ULL}));
        CHECK(bu64.to_vector()[1] == 9007199254740993ULL);

        // Error
        CHECK(mayThrow(false) == 42.0 && !ts::take_error().has_value());
        CHECK(mayThrow(true) == 0.0);
        auto err = ts::take_error();
        CHECK(err.has_value() && err->name() == "TypeError" && err->message() == "bad input" && !err->stack().empty());

        auto re = makeRangeError("out of range");
        CHECK(re.name() == "RangeError" && re.message() == "out of range" && !re.stack().empty());
        CHECK(isJsError(Error::create("from wasm"), "from wasm"));

        // Date
        auto d = Date::from_time(1700000000000.0);
        CHECK(d.time() == 1700000000000.0 && d.to_iso_string() == "2023-11-14T22:13:20.000Z");
        CHECK(Date::now() > 1700000000000.0);
        CHECK(addOneDay(makeDate(1700000000000.0)).time() == 1700086400000.0);
    }
    CHECK(ts::live_handles() == 0);
}
