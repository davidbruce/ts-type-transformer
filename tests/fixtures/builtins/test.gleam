import bindings as b
import bindings/array_buffer
import bindings/array_buffer_view
import bindings/big_int64_array
import bindings/big_uint64_array
import bindings/data_view
import bindings/date
import bindings/error
import bindings/float32_array
import bindings/float64_array
import bindings/int16_array
import bindings/int32_array
import bindings/runtime as r
import bindings/shared_array_buffer
import bindings/uint32_array
import bindings/uint8_array
import bindings/uint8_clamped_array
import gleam/io

fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  // ArrayBuffer
  let ab = array_buffer.from_bit_array(<<1, 2, 3, 4>>)
  check(array_buffer.byte_length(ab) == 4, "ab len")
  let ab2 = b.round_trip_bytes(ab)
  check(
    array_buffer.to_bit_array(ab2) == <<0x5b, 0x58, 0x59, 0x5e>>,
    "ab xor",
  )
  check(array_buffer.byte_length(array_buffer.new(0)) == 0, "ab empty")

  // SharedArrayBuffer
  let sab = shared_array_buffer.from_bit_array(<<5, 10, 15>>)
  check(b.mutate_shared(sab) == 30.0, "sab sum")
  check(shared_array_buffer.to_bit_array(sab) == <<15, 10, 15>>, "sab mutated")

  // ArrayBufferView
  let sub = b.make_sub_view()
  check(
    array_buffer_view.byte_length(sub) == 4
      && array_buffer_view.byte_offset(sub) == 2,
    "view bounds",
  )
  check(array_buffer_view.to_bit_array(sub) == <<12, 13, 14, 15>>, "view bytes")

  // DataView
  let dv = data_view.from_bit_array(<<10, 20, 30>>) |> b.flip_data_view
  check(data_view.to_bit_array(dv) == <<30, 20, 10>>, "dataview flip")

  // TypedArrays
  let u8 = b.make_uint8(5.0)
  check(
    uint8_array.length(u8) == 5
      && uint8_array.to_list(u8) == [3, 10, 17, 24, 31],
    "u8 array",
  )
  check(uint8_array.to_list(uint8_array.from_list([])) == [], "u8 empty")

  let clamped =
    uint8_clamped_array.from_list([10, 100, 200]) |> b.clamp_bytes
  check(uint8_clamped_array.to_list(clamped) == [210, 255, 255], "clamped")

  let i16 = int16_array.from_list([-123, 456]) |> b.echo_int16
  check(int16_array.to_list(i16) == [-123, 456], "i16")

  let i32 =
    int32_array.from_list([0x01020304, -5])
    |> b.xor_ints
    |> int32_array.to_list
  check(
    case i32 {
      [first, ..] -> first == 0x1336557c
      _ -> False
    },
    "i32 xor",
  )

  let u32 = uint32_array.from_list([100, 4_000_000_000]) |> b.echo_uint32
  check(uint32_array.to_list(u32) == [100, 4_000_000_000], "u32")

  let f32 = float32_array.from_list([1.5, 2.25, -0.75])
  check(b.sum_floats(f32) == 3.0, "f32 sum")

  let f64 =
    float64_array.from_list([2.5, -4.0])
    |> b.scale_doubles(3.0)
    |> float64_array.to_list
  check(f64 == [7.5, -12.0], "f64 scale")

  let bi64 = big_int64_array.from_list([1_000_000_000, -250_000_000])
  check(b.sum_big_ints(bi64) == 750_000_000.0, "bi64 sum")

  let bu64 =
    big_uint64_array.from_list([42, 9_007_199_254_740_991])
    |> b.echo_big_uint64
  check(
    big_uint64_array.to_list(bu64) == [42, 9_007_199_254_740_991],
    "bu64",
  )

  // Error
  check(r.attempt(fn() { b.may_throw(False) }) == Ok(42.0), "may_throw ok")
  case r.attempt(fn() { b.may_throw(True) }) {
    Error(je) -> {
      let e = error.from_js_error(je)
      check(
        error.name(e) == "TypeError"
          && error.message(e) == "bad input"
          && error.stack(e) != "",
        "attempt err",
      )
    }
    Ok(_) -> panic as "expected may_throw error"
  }
  let re = b.make_range_error("out of range")
  check(
    error.name(re) == "RangeError"
      && error.message(re) == "out of range"
      && error.stack(re) != "",
    "range error",
  )
  check(b.is_js_error(error.new("from gleam"), "from gleam"), "is_js_error")

  // Date
  let d = date.from_time(1_700_000_000_000.0)
  check(
    date.time(d) == 1_700_000_000_000.0
      && date.to_iso_string(d) == "2023-11-14T22:13:20.000Z",
    "date iso",
  )
  check(date.now() >. 1_700_000_000_000.0, "date now")
  check(
    date.time(b.add_one_day(b.make_date(1_700_000_000_000.0)))
      == 1_700_086_400_000.0,
    "add one day",
  )

  io.println("checks passed")
}
