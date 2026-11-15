#![no_main]
use bindings::*;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check { ($e:expr) => { unsafe { check_at($e as i32, line!()) } }; }

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    {
        // ArrayBuffer
        let ab = ArrayBuffer::from_bytes(&[1, 2, 3, 4]);
        check!(ab.byte_len() == 4);
        let ab2 = round_trip_bytes(&ab);
        check!(ab2.to_vec() == vec![1 ^ 0x5a, 2 ^ 0x5a, 3 ^ 0x5a, 4 ^ 0x5a]);
        check!(ab.write(2, &[99, 100]) == 2);
        check!(ab.to_vec() == vec![1, 2, 99, 100]);
        let empty_ab = ArrayBuffer::new(0);
        check!(empty_ab.byte_len() == 0 && empty_ab.to_vec().is_empty());

        // SharedArrayBuffer
        let sab = SharedArrayBuffer::from_bytes(&[5, 10, 15]);
        check!(mutate_shared(&sab) == 30.0);
        check!(sab.to_vec() == vec![15, 10, 15]);

        // ArrayBufferView
        let sub = make_sub_view();
        check!(sub.byte_len() == 4 && sub.byte_offset() == 2);
        check!(sub.to_vec() == vec![12, 13, 14, 15]);
        check!(!sub.buffer().is_null());

        // DataView
        let dv_out = flip_data_view(&DataView::from_bytes(&[10, 20, 30]));
        check!(dv_out.to_vec() == vec![30, 20, 10]);

        // TypedArrays
        let u8a = make_uint8(5.0);
        check!(u8a.len() == 5 && u8a.to_vec() == vec![3, 10, 17, 24, 31]);
        let empty_u8 = Uint8Array::<ArrayBufferLike>::from_slice(&[]);
        check!(empty_u8.is_empty() && empty_u8.to_vec().is_empty());

        let cl = clamp_bytes(&Uint8ClampedArray::from_slice(&[10, 100, 200]));
        check!(cl.to_vec() == vec![210, 255, 255]);

        let i16a = echo_int16(&Int16Array::from_slice(&[-123, 456]));
        check!(i16a.to_vec() == vec![-123, 456]);

        let i32a = xor_ints(&Int32Array::from_slice(&[0x01020304, -5]));
        check!(i32a.to_vec()[0] == (0x01020304 ^ 0x12345678));
        // Out-of-bounds elem_offset that would wrap 32-bit `elem_offset * 4` to 0:
        let mut oob_i32 = [0i32; 2];
        check!(i32a.read(0x4000_0000, &mut oob_i32) == 0);
        check!(i32a.write(0x4000_0000, &[999]) == 0);
        check!(i32a.to_vec()[0] == (0x01020304 ^ 0x12345678));

        let u32a = echo_uint32(&Uint32Array::from_slice(&[100, 4_000_000_000]));
        check!(u32a.to_vec() == vec![100, 4_000_000_000]);

        let f32a = Float32Array::<ArrayBufferLike>::from_slice(&[1.5, 2.25, -0.75]);
        check!(sum_floats(&f32a) == 3.0);
        // Unaligned linear memory read
        let mut unaligned = [0u8; 16];
        check!(f32a.read_bytes(0, &mut unaligned[1..13]) == 12);
        let f32_val = f32::from_le_bytes(unaligned[1..5].try_into().unwrap());
        check!(f32_val == 1.5);

        let f64a = scale_doubles(&Float64Array::from_slice(&[2.5, -4.0]), 3.0);
        check!(f64a.to_vec() == vec![7.5, -12.0]);

        check!(sum_big_ints(&BigInt64Array::from_slice(&[1_000_000_000, -250_000_000])) == 750_000_000.0);
        let bu64 = echo_big_uint64(&BigUint64Array::from_slice(&[42, 9_007_199_254_740_993]));
        check!(bu64.to_vec() == vec![42, 9_007_199_254_740_993]);

        // Error
        check!(js::attempt(|| may_throw(false)).ok() == Some(42.0));
        let err = js::attempt(|| may_throw(true)).unwrap_err();
        check!(err.name() == "TypeError" && err.message() == "bad input" && !err.stack().is_empty());

        let re = make_range_error("out of range");
        check!(re.name() == "RangeError" && re.message() == "out of range" && !re.stack().is_empty());
        check!(is_js_error(&Error::new("from wasm"), "from wasm"));

        // Date
        let d = Date::from_time(1700000000000.0);
        check!(d.time() == 1700000000000.0 && d.to_iso_string() == "2023-11-14T22:13:20.000Z");
        check!(Date::now() > 1700000000000.0 && js::date_now() > 1700000000000.0);
        check!(add_one_day(&make_date(1700000000000.0)).time() == 1700086400000.0);
    }
    check!(js::live_handles() == 0);
}
