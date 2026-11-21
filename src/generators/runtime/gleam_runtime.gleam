/// A JavaScript promise: `promise_then` gives its result to a function.
pub type Promise(a)

/// A JavaScript exception, or the reason a promise was rejected.
pub type JsError

@external(javascript, "./runtime_ffi.mjs", "promise_then")
pub fn promise_then(promise: Promise(a), then: fn(Result(a, JsError)) -> Nil) -> Nil

/// Awaits `promise`, passing `Ok` values to `then` or `Error` to `on_error`.
pub fn promise_try(promise: Promise(a), on_error: fn(JsError) -> Nil, then: fn(a) -> Nil) -> Nil {
promise_then(promise, fn(res) {
case res {
Ok(v) -> then(v)
Error(e) -> on_error(e)
}
})
}

/// A mutable JavaScript reference cell for stateful callbacks and animation loops.
pub type Ref(a)

@external(javascript, "./runtime_ffi.mjs", "ref_new")
pub fn ref_new(value: a) -> Ref(a)

@external(javascript, "./runtime_ffi.mjs", "ref_get")
pub fn ref_get(cell: Ref(a)) -> a

@external(javascript, "./runtime_ffi.mjs", "ref_set")
pub fn ref_set(cell: Ref(a), value: a) -> Nil

@external(javascript, "./runtime_ffi.mjs", "ref_update")
pub fn ref_update(cell: Ref(a), f: fn(a) -> a) -> a

/// Runs `f`, and gives what it returned, or the exception it threw.
@external(javascript, "./runtime_ffi.mjs", "attempt")
pub fn attempt(f: fn() -> a) -> Result(a, JsError)

@external(javascript, "./runtime_ffi.mjs", "js_error_message")
pub fn js_error_message(error: JsError) -> String

/// Any value, untyped: what a value of no particular type is made from.
@external(javascript, "./runtime_ffi.mjs", "dynamic")
pub fn dynamic(value: a) -> Dynamic

/// A plain JavaScript object from key-value entries.
@external(javascript, "./runtime_ffi.mjs", "js_object")
pub fn js_object(entries: List(#(String, Dynamic))) -> Dynamic

/// Reads a property from a JavaScript object.
@external(javascript, "./runtime_ffi.mjs", "js_get")
pub fn js_get(object: Dynamic, key: String) -> Option(Dynamic)

/// Sets a property on a JavaScript object.
@external(javascript, "./runtime_ffi.mjs", "js_set")
pub fn js_set(object: Dynamic, key: String, value: Dynamic) -> Nil

/// Whether a JavaScript object has its own property `key`.
@external(javascript, "./runtime_ffi.mjs", "js_has")
pub fn js_has(object: Dynamic, key: String) -> Bool

/// A plain JavaScript array from a Gleam list.
@external(javascript, "./runtime_ffi.mjs", "js_array")
pub fn js_array(items: List(Dynamic)) -> Dynamic

/// Reads a JavaScript array into a Gleam list.
@external(javascript, "./runtime_ffi.mjs", "js_to_list")
pub fn js_to_list(array: Dynamic) -> List(Dynamic)

/// A typed key into a map of named values (such as DOM tag or event maps).
pub type Key(map, value) {
  Key(raw: String)
}

/// How a type's values cross between Gleam and JavaScript: what a generic
/// function takes for each type parameter whose values cross (`box_value(b,
/// codec_float())`). Each type has one: `shape_codec()`, `mode_codec()`.
pub type Codec(a)

@external(javascript, "./runtime_ffi.mjs", "codec_same")
pub fn codec_float() -> Codec(Float)

@external(javascript, "./runtime_ffi.mjs", "codec_same")
pub fn codec_string() -> Codec(String)

@external(javascript, "./runtime_ffi.mjs", "codec_same")
pub fn codec_bool() -> Codec(Bool)

@external(javascript, "./runtime_ffi.mjs", "codec_same")
pub fn codec_dynamic() -> Codec(Dynamic)

@external(javascript, "./runtime_ffi.mjs", "codec_option")
pub fn codec_option(of: Codec(a)) -> Codec(Option(a))

@external(javascript, "./runtime_ffi.mjs", "codec_list")
pub fn codec_list(of: Codec(a)) -> Codec(List(a))

@external(javascript, "./runtime_ffi.mjs", "codec_tuple2")
pub fn codec_tuple2(a: Codec(a), b: Codec(b)) -> Codec(#(a, b))

