// Generated.
#pragma once
#ifndef TS_RUNTIME_H
#define TS_RUNTIME_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#define TS_IMPORT_MOD(mod, name) __attribute__((import_module(mod), import_name(#name)))
#define TS_IMPORT(name) TS_IMPORT_MOD("env", name)

// `ts_arity_<name>_<N>`, by the number of arguments: how trailing optional arguments are left out.
// An empty call counts as none: `orUnit()` (C23, or C11 without -pedantic).
#define TS_COUNT(...) TS_COUNT_(0 __VA_OPT__(,) __VA_ARGS__, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0)
#define TS_COUNT_(z, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, n, ...) n
#define TS_PASTE(name, n) TS_PASTE_(name, n)
#define TS_PASTE_(name, n) ts_arity_##name##_##n
#define TS_CALL(name, ...) TS_PASTE(name, TS_COUNT(__VA_ARGS__))(__VA_ARGS__)

// What `U_of(x)` calls where `x` is not a member of the union `U`.
#if __STDC_VERSION__ >= 202311L
void ts_no_member(...) __attribute__((error("not a member of the union")));
#else
#ifdef __clang__
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wstrict-prototypes"
#endif
void ts_no_member() __attribute__((error("not a member of the union")));
#ifdef __clang__
#pragma clang diagnostic pop
#endif
#endif

// Handles to JavaScript values. Zero is null.
typedef struct { uint32_t h; } JsValue;
typedef struct { uint32_t h; } JsString;
typedef struct { uint32_t h; } JsArray;
static inline uint32_t ts_strlen(const char* s) { uint32_t n = 0; while (s[n]) n++; return n; }
typedef struct { const char* ptr; uint32_t len; } TsStr;
static inline TsStr ts_str_id(TsStr s) { return s; }
static inline TsStr ts_str_c(const char* s) { return (TsStr){ s, s ? ts_strlen(s) : 0 }; }
static inline TsStr ts_str_js(JsString s) { return (TsStr){ (const char*)(uintptr_t)s.h, 0xFFFFFFFFu }; }
#define TS_STR(x) _Generic((x), TsStr: ts_str_id, JsString: ts_str_js, default: ts_str_c)(x)
TS_IMPORT(js_string_len) uint32_t ts_raw_js_string_len(uint32_t);
TS_IMPORT(js_string_read) void ts_raw_js_string_read(uint32_t, char*);
TS_IMPORT(js_from_number) uint32_t ts_raw_js_from_number(double);
TS_IMPORT(js_from_bool) uint32_t ts_raw_js_from_bool(int32_t);
TS_IMPORT(js_from_string) uint32_t ts_raw_js_from_string(const char*, uint32_t);
TS_IMPORT(js_to_number) double ts_raw_js_to_number(uint32_t);
TS_IMPORT(js_to_bool) int32_t ts_raw_js_to_bool(uint32_t);
TS_IMPORT(js_equals) int32_t ts_raw_js_equals(uint32_t, uint32_t);
TS_IMPORT(js_array_new) uint32_t ts_raw_js_array_new(void);
TS_IMPORT(js_array_length) uint32_t ts_raw_js_array_length(uint32_t);
TS_IMPORT(js_array_get) uint32_t ts_raw_js_array_get(uint32_t, uint32_t);
TS_IMPORT(js_array_push) void ts_raw_js_array_push(uint32_t, uint32_t);
TS_IMPORT(js_object_new) uint32_t ts_raw_js_object_new(void);
TS_IMPORT(js_object_get) uint32_t ts_raw_js_object_get(uint32_t, const char*, uint32_t);
TS_IMPORT(js_object_set) void ts_raw_js_object_set(uint32_t, const char*, uint32_t, uint32_t);
TS_IMPORT(js_object_has) int32_t ts_raw_js_object_has(uint32_t, const char*, uint32_t);
TS_IMPORT(js_retain) void ts_raw_js_retain(uint32_t);
TS_IMPORT(js_release) void ts_raw_js_release(uint32_t);
TS_IMPORT(js_live) uint32_t ts_raw_js_live(void);
TS_IMPORT(js_callback_release) uint32_t ts_raw_js_callback_release(uint32_t);

// An optional number or flag: `has` says whether `value` is there.
typedef struct { bool has; double value; } JsOptionalDouble;
typedef struct { bool has; bool value; } JsOptionalBool;
static inline JsOptionalDouble js_some_double(double x) { return (JsOptionalDouble){ true, x }; }
static inline JsOptionalBool js_some_bool(bool x) { return (JsOptionalBool){ true, x }; }
static inline JsOptionalDouble js_none_double(void) { return (JsOptionalDouble){ false, 0 }; }
static inline JsOptionalBool js_none_bool(void) { return (JsOptionalBool){ false, false }; }
static inline JsOptionalDouble ts_opt_double_id(JsOptionalDouble x) { return x; }
static inline JsOptionalBool ts_opt_bool_id(JsOptionalBool x) { return x; }
#define TS_OPT_DOUBLE(x) _Generic((x), JsOptionalDouble: ts_opt_double_id, default: js_some_double)(x)
#define TS_OPT_BOOL(x) _Generic((x), JsOptionalBool: ts_opt_bool_id, default: js_some_bool)(x)

// A handle a function returns is the caller's: release it when done with it.
// Arguments, and a callback's arguments, are lent: retain one to keep it.
// What a callback returns is given to JavaScript: retain a handle to return it and keep it.
#define js_release(x) ts_raw_js_release((x).h)
#define js_retain(x) ts_raw_js_retain((x).h)
TS_IMPORT(js_take_error) uint32_t ts_raw_js_take_error(void);
TS_IMPORT(js_error_message) uint32_t ts_raw_js_error_message(uint32_t);
TS_IMPORT(js_error_name) uint32_t ts_raw_js_error_name(uint32_t);
TS_IMPORT(js_error_stack) uint32_t ts_raw_js_error_stack(uint32_t);
TS_IMPORT(js_error_new) uint32_t ts_raw_js_error_new(const char*, uint32_t);
TS_IMPORT(js_date_new) uint32_t ts_raw_js_date_new(double);
TS_IMPORT(js_date_now) double ts_raw_js_date_now(void);
TS_IMPORT(js_date_time) double ts_raw_js_date_time(uint32_t);
TS_IMPORT(js_date_to_iso_string) uint32_t ts_raw_js_date_to_iso_string(uint32_t);
TS_IMPORT(js_bytes_len) uint32_t ts_raw_js_bytes_len(uint32_t);
TS_IMPORT(js_bytes_read) uint32_t ts_raw_js_bytes_read(uint32_t, uint32_t, void*, uint32_t);
TS_IMPORT(js_bytes_write) uint32_t ts_raw_js_bytes_write(uint32_t, uint32_t, const void*, uint32_t);
TS_IMPORT(js_bytes_new) uint32_t ts_raw_js_bytes_new(uint32_t, const void*, uint32_t);
TS_IMPORT(js_bytes_buffer) uint32_t ts_raw_js_bytes_buffer(uint32_t);
TS_IMPORT(js_bytes_offset) uint32_t ts_raw_js_bytes_offset(uint32_t);
TS_IMPORT(js_has_error) int32_t ts_raw_js_has_error(void);
// Whether two handles are one JavaScript value.
static inline bool ts_same(uint32_t a, uint32_t b) { return a == b || ts_raw_js_equals(a, b) != 0; }
#define js_same(a, b) ts_same((a).h, (b).h)
// Whether the last call threw: its exception waits to be taken.
static inline bool js_failed(void) { return ts_raw_js_has_error() != 0; }
// The exception a call threw, taken, or a null handle. A call that throws
// returns a zero value, and an exception not taken by the time control
// returns to JavaScript is reported there.
static inline JsValue js_take_error(void) { return (JsValue){ ts_raw_js_take_error() }; }
// An exception's message, name or stack, the caller's to release.
static inline JsString js_error_message(JsValue e) { return (JsString){ ts_raw_js_error_message(e.h) }; }
static inline JsString js_error_name(JsValue e) { return (JsString){ ts_raw_js_error_name(e.h) }; }
static inline JsString js_error_stack(JsValue e) { return (JsString){ ts_raw_js_error_stack(e.h) }; }
static inline JsValue js_error_new(const char* message) { return (JsValue){ ts_raw_js_error_new(message, ts_strlen(message)) }; }
static inline double js_date_now(void) { return ts_raw_js_date_now(); }
// A JavaScript iterator: see `X_iterate`.
typedef struct { uint32_t h; } JsIterator;
TS_IMPORT(js_iterate) uint32_t ts_raw_js_iterate(uint32_t);
TS_IMPORT(js_iterator_next) uint32_t ts_raw_js_iterator_next(uint32_t);
TS_IMPORT(js_async_iterate) uint32_t ts_raw_js_async_iterate(uint32_t);
TS_IMPORT(js_async_iterator_next) uint32_t ts_raw_js_async_iterator_next(uint32_t);
// A scope: every handle a function returns while it is the innermost open
// scope is released when it closes. Retain one to keep it.
TS_IMPORT(js_scope_open) void js_scope_open(void);
TS_IMPORT(js_scope_close) void js_scope_close(void);
// `js_scope();` opens a scope that closes where the enclosing block is left, by any path.
static inline void ts_scope_exit(char* unused) { (void)unused; js_scope_close(); }
#define TS_UNIQUE(name) TS_PASTE2(name, __COUNTER__)
#define js_scope() __attribute__((cleanup(ts_scope_exit), unused)) char TS_UNIQUE(ts_scope_) = (js_scope_open(), 0)
// How many JavaScript values wasm holds handles to.
static inline uint32_t js_live_handles(void) { return ts_raw_js_live(); }
// Forgets a callback made with `X_from`, and returns its context. Its handle is released as any other.
#define js_forget_callback(x) ((void*)(uintptr_t)ts_raw_js_callback_release((x).h))
static inline uint32_t JsString_len(JsString s) { return ts_raw_js_string_len(s.h); }
static inline void JsString_read(JsString s, char* buf) { ts_raw_js_string_read(s.h, buf); }
TS_IMPORT(js_string_copy) uint32_t ts_raw_js_string_copy(uint32_t, char*, uint32_t);
TS_IMPORT(js_string_equals) int32_t ts_raw_js_string_equals(uint32_t, const char*, uint32_t);
// Copies at most cap - 1 bytes and a terminating NUL; returns the string's whole length in bytes.
static inline uint32_t JsString_copy(JsString s, char* buf, uint32_t cap) {
    if (cap == 0) return JsString_len(s);
    uint32_t n = ts_raw_js_string_copy(s.h, buf, cap - 1);
    buf[n < cap - 1 ? n : cap - 1] = 0;
    return n;
}
// Compares in JavaScript, without copying.
static inline bool ts_string_equals(JsString s, TsStr text) { return ts_raw_js_string_equals(s.h, text.ptr, text.len) != 0; }
#define JsString_equals(s, text) ts_string_equals((s), TS_STR(text))
static inline JsValue js_number(double x) { return (JsValue){ ts_raw_js_from_number(x) }; }
static inline JsValue js_bool(bool x) { return (JsValue){ ts_raw_js_from_bool(x) }; }
static inline JsValue js_string(const char* s) { return (JsValue){ ts_raw_js_from_string(s, ts_strlen(s)) }; }
static inline JsString JsString_from(const char* s) { return (JsString){ ts_raw_js_from_string(s, ts_strlen(s)) }; }
static inline double js_to_number(JsValue v) { return ts_raw_js_to_number(v.h); }
static inline bool js_to_bool(JsValue v) { return ts_raw_js_to_bool(v.h) != 0; }
static inline JsString js_to_string(JsValue v) { return (JsString){ v.h }; }
static inline bool js_equals(JsValue a, JsValue b) { return ts_raw_js_equals(a.h, b.h) != 0; }
static inline JsArray JsArray_new(void) { return (JsArray){ ts_raw_js_array_new() }; }
static inline uint32_t JsArray_length(JsArray a) { return ts_raw_js_array_length(a.h); }
static inline JsValue JsArray_get(JsArray a, uint32_t i) { return (JsValue){ ts_raw_js_array_get(a.h, i) }; }
static inline void JsArray_push(JsArray a, JsValue v) { ts_raw_js_array_push(a.h, v.h); }
static inline JsValue js_object(void) { return (JsValue){ ts_raw_js_object_new() }; }
static inline JsValue js_object_get(JsValue o, const char* key) { return (JsValue){ ts_raw_js_object_get(o.h, key, ts_strlen(key)) }; }
static inline void js_object_set(JsValue o, const char* key, JsValue v) { ts_raw_js_object_set(o.h, key, ts_strlen(key), v.h); }
static inline bool js_object_has(JsValue o, const char* key) { return ts_raw_js_object_has(o.h, key, ts_strlen(key)) != 0; }
static inline JsArray js_to_array(JsValue v) { return (JsArray){ v.h }; }
#define js_value(x) ((JsValue){ (x).h })
static inline JsValue ts_get_prop(uint32_t o, const char* k) { return (JsValue){ ts_raw_js_object_get(o, k, ts_strlen(k)) }; }
static inline void ts_set_str(uint32_t o, const char* k, TsStr v) { uint32_t h = ts_raw_js_from_string(v.ptr, v.len); ts_raw_js_object_set(o, k, ts_strlen(k), h); ts_raw_js_release(h); }
static inline void ts_set_num(uint32_t o, const char* k, double v) { uint32_t h = ts_raw_js_from_number(v); ts_raw_js_object_set(o, k, ts_strlen(k), h); ts_raw_js_release(h); }
static inline void ts_set_bool(uint32_t o, const char* k, bool v) { uint32_t h = ts_raw_js_from_bool(v); ts_raw_js_object_set(o, k, ts_strlen(k), h); ts_raw_js_release(h); }
static inline void ts_set_val(uint32_t o, const char* k, uint32_t v) { ts_raw_js_object_set(o, k, ts_strlen(k), v); }
#define js_get_prop(o, k) ts_get_prop((o).h, (k))
#define js_set_str(o, k, v) ts_set_str((o).h, (k), TS_STR(v))
#define js_set_num(o, k, v) ts_set_num((o).h, (k), (v))
#define js_set_bool(o, k, v) ts_set_bool((o).h, (k), (v))
#define js_set_val(o, k, v) ts_set_val((o).h, (k), (v).h)
static inline void ts_push_str(uint32_t a, TsStr v) { uint32_t h = ts_raw_js_from_string(v.ptr, v.len); ts_raw_js_array_push(a, h); ts_raw_js_release(h); }
static inline void ts_push_num(uint32_t a, double v) { uint32_t h = ts_raw_js_from_number(v); ts_raw_js_array_push(a, h); ts_raw_js_release(h); }
static inline void ts_push_bool(uint32_t a, bool v) { uint32_t h = ts_raw_js_from_bool(v); ts_raw_js_array_push(a, h); ts_raw_js_release(h); }
static inline void ts_push_val(uint32_t a, uint32_t v) { ts_raw_js_array_push(a, v); }
#define js_push_str(a, v) ts_push_str((a).h, TS_STR(v))
#define js_push_num(a, v) ts_push_num((a).h, (v))
#define js_push_bool(a, v) ts_push_bool((a).h, (v))
#define js_push_val(a, v) ts_push_val((a).h, (v).h)

// An option bag: `X_create(.a = 1, .b = "x")` sets the properties it names and no others. `X_FIELDS` lists the
// fields, each by its owner (the type that declares the setter) and kind: `i` a flag or an enum (-1 is absent),
// `d` a number (one NaN is absent), `s` a string (NULL), `h` a handle (0), `u` a union (tag -1), and `B`, `D`
// for an optional flag or number. A designator after the absent values overrides one.
static inline bool ts_absent_num(double x) { union { double d; uint64_t u; } a = { x }, b = { __builtin_nan("0x7515") }; return a.u == b.u; }
#define TS_NO_i (-1)
#define TS_NO_B (-1)
#define TS_NO_d __builtin_nan("0x7515")
#define TS_NO_D __builtin_nan("0x7515")
#define TS_NO_s 0
#define TS_NO_h { 0 }
#define TS_NO_u { .tag = -1 }
#define TS_HAS_i(x) ((x) >= 0)
#define TS_HAS_B(x) ((x) >= 0)
#define TS_HAS_d(x) (!ts_absent_num(x))
#define TS_HAS_D(x) (!ts_absent_num(x))
#define TS_HAS_s(x) ((x) != 0)
#define TS_HAS_h(x) ((x).h != 0)
#define TS_HAS_u(x) ((x).tag != -1)
#define TS_GET_i(x) (x)
#define TS_GET_B(x) js_some_bool(x)
#define TS_GET_d(x) (x)
#define TS_GET_D(x) js_some_double(x)
#define TS_GET_s(x) ts_str_c(x)
#define TS_GET_h(x) (x)
#define TS_GET_u(x) (x)
#define TS_FIELD(c, k, t, n) t n;
#define TS_FIELD_NO(c, k, t, n) .n = TS_NO_##k,
#define TS_FIELD_SET(c, k, t, n) if (TS_HAS_##k(f.n)) (c##_set_##n)((c){ o.h }, TS_GET_##k(f.n));
// A field whose setter is not `set_` and its name: `F_N(c, k, t, n, setter)`.
#define TS_FIELD_N(c, k, t, n, s) t n;
#define TS_FIELD_NO_N(c, k, t, n, s) .n = TS_NO_##k,
#define TS_FIELD_SET_N(c, k, t, n, s) if (TS_HAS_##k(f.n)) (c##_##s)((c){ o.h }, TS_GET_##k(f.n));
#define TS_BAG_DEF(c) typedef struct { c##_FIELDS(TS_FIELD, c) } c##_fields; \
    static inline c c##_from_fields(c##_fields f) { c o = (c##_create)(); c##_FIELDS(TS_FIELD_SET, c) return o; }
#if defined(__clang__)
#define TS_OVERRIDE_ON _Pragma("clang diagnostic push") _Pragma("clang diagnostic ignored \"-Winitializer-overrides\"")
#define TS_OVERRIDE_OFF _Pragma("clang diagnostic pop")
#else
#define TS_OVERRIDE_ON _Pragma("GCC diagnostic push") _Pragma("GCC diagnostic ignored \"-Woverride-init\"")
#define TS_OVERRIDE_OFF _Pragma("GCC diagnostic pop")
#endif
// No designators is the empty object, made by the function itself.
#define TS_BAG(c, ...) TS_PASTE2(TS_BAG_, TS_FILLED(__VA_ARGS__))(c, __VA_ARGS__)
#define TS_FILLED(...) TS_FILLED_(__VA_OPT__(1,) 0, 0, 0)
#define TS_FILLED_(a, b, ...) a
#define TS_PASTE2(a, b) TS_PASTE2_(a, b)
#define TS_PASTE2_(a, b) a##b
#define TS_BAG_0(c, ...) (c##_create)()
#define TS_BAG_1(c, ...) TS_OVERRIDE_ON c##_from_fields((c##_fields){ c##_FIELDS(TS_FIELD_NO, c) __VA_ARGS__ }) TS_OVERRIDE_OFF

#endif
