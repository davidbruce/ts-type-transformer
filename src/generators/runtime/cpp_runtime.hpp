// Generated.
#pragma once
#ifndef TS_RUNTIME_HPP
#define TS_RUNTIME_HPP
#include <concepts>
#include <coroutine>
#include <cstddef>
#include <cstdint>
#include <initializer_list>
#include <optional>
#include <string>
#include <string_view>
#include <tuple>
#include <type_traits>
#include <utility>
#include <variant>
#include <vector>

#define TS_IMPORT_MOD(mod, name) __attribute__((import_module(mod), import_name(#name)))
#define TS_IMPORT(name) TS_IMPORT_MOD("env", name)
#define TS_EXPORT(name) extern "C" __attribute__((export_name(#name)))

extern "C" {
TS_IMPORT(js_string_len) uint32_t ts_raw_js_string_len(uint32_t);
TS_IMPORT(js_string_read) void ts_raw_js_string_read(uint32_t, char*);
TS_IMPORT(js_from_number) uint32_t ts_raw_js_from_number(double);
TS_IMPORT(js_from_bool) uint32_t ts_raw_js_from_bool(int32_t);
TS_IMPORT(js_from_string) uint32_t ts_raw_js_from_string(const char*, size_t);
TS_IMPORT(js_to_number) double ts_raw_js_to_number(uint32_t);
TS_IMPORT(js_to_bool) int32_t ts_raw_js_to_bool(uint32_t);
TS_IMPORT(js_equals) int32_t ts_raw_js_equals(uint32_t, uint32_t);
TS_IMPORT(js_array_new) uint32_t ts_raw_js_array_new();
TS_IMPORT(js_array_length) uint32_t ts_raw_js_array_length(uint32_t);
TS_IMPORT(js_array_get) uint32_t ts_raw_js_array_get(uint32_t, uint32_t);
TS_IMPORT(js_array_push) void ts_raw_js_array_push(uint32_t, uint32_t);
TS_IMPORT(js_object_new) uint32_t ts_raw_js_object_new();
TS_IMPORT(js_object_get) uint32_t ts_raw_js_object_get(uint32_t, const char*, uint32_t);
TS_IMPORT(js_object_set) void ts_raw_js_object_set(uint32_t, const char*, uint32_t, uint32_t);
TS_IMPORT(js_object_has) int32_t ts_raw_js_object_has(uint32_t, const char*, uint32_t);
TS_IMPORT(js_listener_from) uint32_t ts_raw_js_listener_from(uintptr_t, void*);
TS_IMPORT(js_callback_release) uint32_t ts_raw_js_callback_release(uint32_t);
TS_IMPORT(js_retain) void ts_raw_js_retain(uint32_t);
TS_IMPORT(js_release) void ts_raw_js_release(uint32_t);
TS_IMPORT(js_live) uint32_t ts_raw_js_live();
TS_IMPORT(js_take_error) uint32_t ts_raw_js_take_error();
TS_IMPORT(js_string_take) uint32_t ts_raw_js_string_take(uint32_t, char*, size_t);
TS_IMPORT(js_error_message) uint32_t ts_raw_js_error_message(uint32_t);
TS_IMPORT(js_error_name) uint32_t ts_raw_js_error_name(uint32_t);
TS_IMPORT(js_error_stack) uint32_t ts_raw_js_error_stack(uint32_t);
TS_IMPORT(js_error_new) uint32_t ts_raw_js_error_new(const char*, size_t);
TS_IMPORT(js_date_new) uint32_t ts_raw_js_date_new(double);
TS_IMPORT(js_date_now) double ts_raw_js_date_now();
TS_IMPORT(js_date_time) double ts_raw_js_date_time(uint32_t);
TS_IMPORT(js_date_to_iso_string) uint32_t ts_raw_js_date_to_iso_string(uint32_t);
TS_IMPORT(js_bytes_len) uint32_t ts_raw_js_bytes_len(uint32_t);
TS_IMPORT(js_bytes_read) uint32_t ts_raw_js_bytes_read(uint32_t, uint32_t, void*, uint32_t);
TS_IMPORT(js_bytes_write) uint32_t ts_raw_js_bytes_write(uint32_t, uint32_t, const void*, uint32_t);
TS_IMPORT(js_bytes_new) uint32_t ts_raw_js_bytes_new(uint32_t, const void*, uint32_t);
TS_IMPORT(js_bytes_buffer) uint32_t ts_raw_js_bytes_buffer(uint32_t);
TS_IMPORT(js_bytes_offset) uint32_t ts_raw_js_bytes_offset(uint32_t);
TS_IMPORT(js_promise_then) void ts_raw_js_promise_then(uint32_t, uintptr_t, void*);
TS_IMPORT(js_iterate) uint32_t ts_raw_js_iterate(uint32_t);
TS_IMPORT(js_iterator_next) uint32_t ts_raw_js_iterator_next(uint32_t);
TS_IMPORT(js_async_iterate) uint32_t ts_raw_js_async_iterate(uint32_t);
TS_IMPORT(js_async_iterator_next) uint32_t ts_raw_js_async_iterator_next(uint32_t);
}

namespace ts {

// Tag for making a value from a handle.
struct handle_t {
    explicit handle_t() = default;
};
inline constexpr handle_t handle{};

// A JavaScript object. Zero is null. A handle is a counted reference:
// copies retain it, and the last one to go frees it on the JavaScript side.
struct Handle {
    uint32_t h = 0;
    Handle() = default;
    // Takes over a reference: what JavaScript returned.
    Handle(handle_t, uint32_t h) : h(h) {}
    Handle(const Handle& o) : h(o.h) {
        if (h) ts_raw_js_retain(h);
    }
    Handle(Handle&& o) noexcept : h(std::exchange(o.h, 0)) {}
    Handle& operator=(Handle o) noexcept {
        std::swap(h, o.h);
        return *this;
    }
    ~Handle() {
        if (h) ts_raw_js_release(h);
    }
    explicit operator bool() const noexcept { return h != 0; }
    bool is_null() const noexcept { return h == 0; }
    template <class T>
        requires std::derived_from<T, Handle>
    T unsafe_as() const {
        if (h) ts_raw_js_retain(h);
        return T(handle, h);
    }
    // Whether two handles are one JavaScript value.
    friend bool operator==(const Handle& a, const Handle& b) { return a.h == b.h || ts_raw_js_equals(a.h, b.h) != 0; }
};

// How many JavaScript values wasm holds handles to.
inline uint32_t live_handles() { return ts_raw_js_live(); }

template <class T>
inline constexpr bool is_handle = std::is_base_of_v<Handle, std::decay_t<T>>;

// Whether the arguments are one object of `X`, or of a type made from it:
// that is a copy or an upcast of the handle, never a new JavaScript object.
template <class X, class... A>
inline constexpr bool is_self_arg = sizeof...(A) == 1 && (std::is_base_of_v<X, std::remove_cvref_t<A>> && ...);

// T, as a type that depends on D: checked only once D is known.
template <class D, class T>
struct dep_ {
    using type = T;
};
template <class D, class T>
using dep = typename dep_<D, T>::type;

inline std::string read_string(uint32_t h) {
    std::string s(ts_raw_js_string_len(h), '\0');
    ts_raw_js_string_read(h, s.data());
    return s;
}

// A string from a handle JavaScript returned, which is released: in one
// call, when it fits a small buffer.
inline std::string take_string(uint32_t h) {
    char buf[256];
    uint32_t n = ts_raw_js_string_take(h, buf, sizeof buf);
    if (n <= sizeof buf) return std::string(buf, n);
    std::string s(n, '\0');
    ts_raw_js_string_read(h, s.data());
    ts_raw_js_release(h);
    return s;
}

// Which enum of strings a type is, for its values to cross as handles: -1 for
// an enum of numbers.
template <class T>
inline constexpr int32_t enum_id = -1;
template <class T>
inline uint32_t enum_to_handle(T x) { return ts_raw_js_from_number(double(int32_t(x))); }
template <class T>
inline T enum_from_handle(uint32_t h) { return T(int32_t(ts_raw_js_to_number(h))); }

template <class U>
struct In;
template <class T>
struct Opt;
template <class U>
struct Exact;
template <class T>
struct OptExact;

// A value lent to JavaScript as a handle, for one call: an object's own
// handle, or a new one for a number, flag or string, released after the call.
struct Arg {
    uint32_t h = 0;
    bool own = false;
    Arg(const std::string& s) : h(ts_raw_js_from_string(s.data(), s.size())), own(true) {}
    Arg(std::string_view s) : h(ts_raw_js_from_string(s.data(), s.size())), own(true) {}
    Arg(const char* s) : Arg(std::string_view(s)) {}
    Arg(std::nullopt_t) noexcept {}
    Arg(double x) : h(ts_raw_js_from_number(x)), own(true) {}
    template <class I, std::enable_if_t<std::is_integral_v<I> && !std::is_same_v<I, bool>, int> = 0>
    Arg(I x) : h(ts_raw_js_from_number(double(x))), own(true) {}
    Arg(bool x) : h(ts_raw_js_from_bool(x)), own(true) {}
    template <class H, std::enable_if_t<is_handle<H>, int> = 0>
    Arg(const H& v) : h(v.h) {}
    template <class E, std::enable_if_t<std::is_enum_v<E>, int> = 0>
    Arg(E x) : h(enum_to_handle(x)), own(true) {}
    template <class... T>
    Arg(const std::variant<T...>& v) {
        std::visit([this](const auto& x) { take(Arg(x)); }, v);
    }
    template <class T>
    Arg(const std::optional<T>& v) {
        if (v) take(Arg(*v));
    }
    template <class U>
    Arg(const In<U>& v);
    template <class T>
    Arg(const Opt<T>& v);
    template <class U>
    Arg(const Exact<U>& v);
    template <class T>
    Arg(const OptExact<T>& v);
    Arg(const Arg&) = delete;
    Arg& operator=(const Arg&) = delete;
    ~Arg() {
        if (own) ts_raw_js_release(h);
    }
    // A reference for JavaScript to keep: what a callback returns.
    uint32_t give() {
        if (!own && h) ts_raw_js_retain(h);
        own = false;
        return h;
    }

  private:
    void take(Arg&& a) {
        h = a.h;
        own = std::exchange(a.own, false);
    }
};

// What `from_handle` turns into a union: specialized for each union a value can be decoded to.
template <class T>
struct decoder {};

// What a type argument is, for telling a bare type parameter apart in a union:
// 1 a number, 2 a string, 3 a flag, 0 anything else.
template <class T>
inline constexpr int32_t kind_of = std::is_same_v<T, double> ? 1 : std::is_same_v<T, std::string> ? 2 : std::is_same_v<T, bool> ? 3 : 0;

template <class T>
struct is_optional : std::false_type {};
template <class T>
struct is_optional<std::optional<T>> : std::true_type {};

// A value of type T from a handle JavaScript returned: a number, a flag, a
// string, a union or an object. Takes over the handle.
template <class T>
T from_handle(uint32_t h) {
    if constexpr (is_optional<T>::value) {
        if (!h) return std::nullopt;
        return from_handle<typename T::value_type>(h);
    } else if constexpr (requires { decoder<T>::from(h); }) {
        return decoder<T>::from(h);
    } else if constexpr (std::is_same_v<T, std::string>) {
        return take_string(h);
    } else if constexpr (std::is_enum_v<T>) {
        T x = enum_from_handle<T>(h);
        ts_raw_js_release(h);
        return x;
    } else if constexpr (std::is_same_v<T, double> || std::is_same_v<T, bool>) {
        T x;
        if constexpr (std::is_same_v<T, double>) x = ts_raw_js_to_number(h);
        else x = ts_raw_js_to_bool(h) != 0;
        ts_raw_js_release(h);
        return x;
    } else return T(handle, h);
}

template <class T = struct Value>
struct Array;

// A JavaScript value of no particular type.
struct Value : Handle {
    using Handle::Handle;
    Value() = default;
    Value(const Value&) = default;
    Value(Value&&) = default;
    Value& operator=(const Value&) = default;
    Value& operator=(Value&&) = default;
    template <class H>
        requires (is_handle<H> && !std::same_as<std::decay_t<H>, Value>)
    Value(const H& x) : Handle(handle, x.h) {
        if (h) ts_raw_js_retain(h);
    }
    Value(double x) : Handle(handle, ts_raw_js_from_number(x)) {}
    template <class I>
        requires (std::is_integral_v<I> && !std::same_as<I, bool>)
    Value(I x) : Handle(handle, ts_raw_js_from_number(double(x))) {}
    Value(bool x) : Handle(handle, ts_raw_js_from_bool(x)) {}
    Value(std::string_view s) : Handle(handle, ts_raw_js_from_string(s.data(), s.size())) {}
    Value(const std::string& s) : Value(std::string_view(s)) {}
    Value(const char* s) : Value(s ? std::string_view(s) : std::string_view()) {}
    Value(std::nullptr_t) = delete;
    static Value number(double x) { return {handle, ts_raw_js_from_number(x)}; }
    static Value boolean(bool x) { return {handle, ts_raw_js_from_bool(x)}; }
    static Value string(std::string_view s) { return {handle, ts_raw_js_from_string(s.data(), s.size())}; }
    static Value object() { return {handle, ts_raw_js_object_new()}; }
    static Value of(const Handle& x) {
        if (x.h) ts_raw_js_retain(x.h);
        return {handle, x.h};
    }
    Value get(std::string_view key) const { return {handle, ts_raw_js_object_get(h, key.data(), key.size())}; }
    template <class U>
    const Value& set(std::string_view key, const U& v) const {
        ts_raw_js_object_set(h, key.data(), key.size(), Arg(v).h);
        return *this;
    }
    bool has(std::string_view key) const { return ts_raw_js_object_has(h, key.data(), key.size()) != 0; }
    template <class T = Value>
    Array<T> to_array() const;
    double to_number() const { return ts_raw_js_to_number(h); }
    bool to_bool() const { return ts_raw_js_to_bool(h) != 0; }
    std::string to_string() const { return read_string(h); }
};

// The direct bases of a type, declared beside it in `ts_types.hpp`: what
// `extends` follows. A constraint is then checked of types that are only
// declared, as a template's arguments are when the template is named.
template <class... T>
struct list {};
template <class T>
struct bases {
    using type = list<>;
};
template <class T>
inline constexpr bool is_callback = false;
template <class C>
struct Scoped;
template <class C>
struct bases<Scoped<C>> {
    using type = list<C>;
};

// TypeScript's `T extends U`, for a constraint. Anything extends `any`, and
// `any` anything; a union, if each of its members does.
template <class T, class U>
struct extends_t;
template <class U, class L>
struct any_extends : std::false_type {};
template <class U, class B, class... R>
struct any_extends<U, list<B, R...>> : std::bool_constant<extends_t<B, U>::value || any_extends<U, list<R...>>::value> {};
template <class T>
inline constexpr bool is_variant = false;
template <class... Ts>
inline constexpr bool is_variant<std::variant<Ts...>> = true;
template <class T, class U>
struct extends_t : std::bool_constant<std::same_as<T, U> || std::same_as<T, Value> || std::same_as<U, Value> || any_extends<U, typename bases<T>::type>::value> {};
template <template <class...> class G, class... A, class... B>
    requires (!is_variant<G<A...>> && sizeof...(A) == sizeof...(B) && (extends_t<A, B>::value && ...))
struct extends_t<G<A...>, G<B...>> : std::true_type {};
template <class... A, class U>
struct extends_t<std::variant<A...>, U> : std::bool_constant<(extends_t<A, U>::value && ...)> {};
// A type extends a union it is, or extends, a member of; a union, one whose
// members each do.
template <class T, class... B>
struct extends_t<T, std::variant<B...>> : std::bool_constant<(extends_t<T, B>::value || ...)> {};
template <class... A, class... B>
struct extends_t<std::variant<A...>, std::variant<B...>> : std::bool_constant<(extends_t<A, std::variant<B...>>::value && ...)> {};
template <class T, class U>
concept extends = extends_t<T, U>::value;

template <class X>
inline constexpr bool is_number_arg = std::is_arithmetic_v<std::decay_t<X>> && !std::is_same_v<std::decay_t<X>, bool>;
template <class X>
inline constexpr bool is_string_arg =
    std::same_as<std::decay_t<X>, std::string> ||
    std::same_as<std::decay_t<X>, std::string_view> ||
    std::same_as<std::decay_t<X>, const char*> ||
    std::same_as<std::decay_t<X>, char*>;
template <class F>
inline constexpr bool callable_candidate =
    !is_handle<F> &&
    !is_number_arg<F> &&
    !std::same_as<std::decay_t<F>, bool> &&
    !is_string_arg<F> &&
    !std::is_enum_v<std::decay_t<F>> &&
    !is_variant<std::decay_t<F>> &&
    !is_optional<std::decay_t<F>>::value &&
    !std::same_as<std::decay_t<F>, std::nullopt_t> &&
    !std::same_as<std::decay_t<F>, std::nullptr_t>;

template <class M, class X>
constexpr bool in_match_v() {
    using D = std::decay_t<X>;
    if constexpr (is_variant<D>) {
        return []<size_t... Is>(std::index_sequence<Is...>) {
            return (in_match_v<M, const std::variant_alternative_t<Is, D>&>() && ...);
        }(std::make_index_sequence<std::variant_size_v<D>>{});
    } else if constexpr (is_variant<M>) {
        return []<size_t... Is>(std::index_sequence<Is...>) {
            return (in_match_v<std::variant_alternative_t<Is, M>, X>() || ...);
        }(std::make_index_sequence<std::variant_size_v<M>>{});
    } else if constexpr (std::same_as<M, double>) {
        return is_number_arg<X>;
    } else if constexpr (std::same_as<M, bool>) {
        return std::same_as<D, bool>;
    } else if constexpr (std::same_as<M, std::string>) {
        return is_string_arg<X>;
    } else if constexpr (std::is_enum_v<M>) {
        return std::same_as<D, M>;
    } else if constexpr (is_handle<X>) {
        return std::same_as<M, Value> || std::same_as<M, Handle> || extends<D, M>;
    } else if constexpr (is_callback<M> && callable_candidate<X>) {
        return std::constructible_from<M, X>;
    } else {
        return false;
    }
}

template <class U, class X>
concept in_accepts = in_match_v<U, X>();

template <class M, class X>
uint32_t in_make_cb(X&& x) {
    if constexpr (is_variant<M>) {
        return [&]<class... Ms>(std::type_identity<std::variant<Ms...>>) -> uint32_t {
            uint32_t res = 0;
            bool done = false;
            auto try_one = [&]<class Alt>() {
                if constexpr (in_match_v<Alt, X>()) {
                    if (!done) {
                        res = in_make_cb<Alt>(std::forward<X>(x));
                        done = true;
                    }
                }
            };
            (try_one.template operator()<Ms>(), ...);
            return res;
        }(std::type_identity<M>{});
    } else if constexpr (is_callback<M> && std::constructible_from<M, X>) {
        M cb(std::forward<X>(x));
        return std::exchange(cb.h, 0);
    } else {
        return 0;
    }
}

template <class U>
struct In {
    uint32_t h = 0;
    bool own = false;
    In(const In&) = delete;
    In& operator=(const In&) = delete;
    template <class X>
        requires (!std::same_as<std::decay_t<X>, In> && in_accepts<U, X>)
    In(X&& x) {
        using D = std::decay_t<X>;
        if constexpr (is_variant<D>) {
            std::visit([this](const auto& v) {
                In tmp(v);
                h = tmp.h;
                own = std::exchange(tmp.own, false);
            }, x);
        } else if constexpr (is_string_arg<D>) {
            if constexpr (std::is_pointer_v<D>) {
                std::string_view sv = x ? std::string_view(x) : std::string_view();
                h = ts_raw_js_from_string(sv.data(), sv.size());
            } else {
                std::string_view sv(x);
                h = ts_raw_js_from_string(sv.data(), sv.size());
            }
            own = true;
        } else if constexpr (std::same_as<D, bool>) {
            h = ts_raw_js_from_bool(x);
            own = true;
        } else if constexpr (is_number_arg<D>) {
            h = ts_raw_js_from_number(double(x));
            own = true;
        } else if constexpr (std::is_enum_v<D>) {
            h = enum_to_handle(x);
            own = true;
        } else if constexpr (is_handle<D>) {
            h = x.h;
            own = false;
        } else {
            h = in_make_cb<U>(std::forward<X>(x));
            own = true;
        }
    }
    ~In() {
        if (own) ts_raw_js_release(h);
    }
};

template <class T, class X>
concept opt_elem_accepts =
    (is_variant<T> && in_accepts<T, X>) ||
    (!is_variant<T> && is_handle<X> && extends<std::decay_t<X>, T>) ||
    (is_callback<T> && callable_candidate<X> && std::constructible_from<T, X>);

template <class T>
struct Opt {
    uint32_t h = 0;
    bool own = false;
    Opt(const Opt&) = delete;
    Opt& operator=(const Opt&) = delete;
    Opt(std::nullopt_t) noexcept {}
    template <class X>
        requires opt_elem_accepts<T, const X&>
    Opt(const std::optional<X>& o) {
        if (o) init(*o);
    }
    template <class X>
        requires (is_variant<T> && !std::same_as<std::decay_t<X>, Opt> && !is_optional<std::decay_t<X>>::value && !std::same_as<std::decay_t<X>, std::nullopt_t> && in_accepts<T, X>)
    Opt(X&& x) {
        In<T> tmp(std::forward<X>(x));
        h = tmp.h;
        own = std::exchange(tmp.own, false);
    }
    template <class H>
        requires (!is_variant<T> && !std::same_as<std::decay_t<H>, Opt> && is_handle<H> && extends<std::decay_t<H>, T>)
    Opt(const H& x) noexcept : h(x.h) {}
    template <class F>
        requires (is_callback<T> && callable_candidate<F> && std::constructible_from<T, F>)
    Opt(F&& f) {
        T cb(std::forward<F>(f));
        h = std::exchange(cb.h, 0);
        own = true;
    }
    ~Opt() {
        if (own) ts_raw_js_release(h);
    }

  private:
    template <class X>
    void init(const X& x) {
        if constexpr (is_variant<T>) {
            In<T> tmp(x);
            h = tmp.h;
            own = std::exchange(tmp.own, false);
        } else if constexpr (is_handle<X>) {
            h = x.h;
        } else {
            T cb(x);
            h = std::exchange(cb.h, 0);
            own = true;
        }
    }
};

template <class A>
struct array_elem {};
template <class U>
struct array_elem<Array<U>> {
    using type = U;
};

// A field of an aggregate that may be left out: `X({.a = 1})` sets only what it names.
// Made from what a `T` is made from, it holds the value as a handle: a header of hundreds
// of them is then cheap to compile, as it would not be of `std::optional<T>`.
template <class T>
struct Field {
    uint32_t h = 0;
    Field() = default;
    Field(std::nullopt_t) {}
    // From what `T` is made from: one value, or a braced list of the values of a tuple (`{r, g, b, 1.0}`).
    template <class X0, class... X>
        requires (!std::same_as<std::decay_t<X0>, Field> && std::constructible_from<T, X0, X...>)
    Field(X0&& x0, X&&... xs) {
        T v(std::forward<X0>(x0), std::forward<X>(xs)...);
        h = Arg(v).give();
    }
    // A braced list of the elements of an array (`{1, 2, 3}`).
    template <class A = T>
    Field(std::initializer_list<typename array_elem<A>::type> xs) {
        T v(xs);
        h = Arg(v).give();
    }
    Field(const Field&) = delete;
    Field& operator=(const Field&) = delete;
    ~Field() {
        if (h) ts_raw_js_release(h);
    }
    // Sets `o[key]` if given.
    void to(uint32_t o, std::string_view key) const {
        if (h) ts_raw_js_object_set(o, key.data(), key.size(), h);
    }
};

template <class U>
struct Exact {
    uint32_t h = 0;
    bool own = false;
    Exact(const Exact&) = delete;
    Exact& operator=(const Exact&) = delete;
    Exact(const U& x) {
        Arg a(x);
        h = a.h;
        own = std::exchange(a.own, false);
    }
    ~Exact() {
        if (own) ts_raw_js_release(h);
    }
};

template <class T>
struct OptExact {
    uint32_t h = 0;
    bool own = false;
    OptExact(const OptExact&) = delete;
    OptExact& operator=(const OptExact&) = delete;
    OptExact(std::nullopt_t) noexcept {}
    OptExact(const std::optional<T>& x) {
        if (x) {
            Arg a(*x);
            h = a.h;
            own = std::exchange(a.own, false);
        }
    }
    ~OptExact() {
        if (own) ts_raw_js_release(h);
    }
};

template <class U>
inline Arg::Arg(const In<U>& v) : h(v.h) {}
template <class T>
inline Arg::Arg(const Opt<T>& v) : h(v.h) {}
template <class U>
inline Arg::Arg(const Exact<U>& v) : h(v.h) {}
template <class T>
inline Arg::Arg(const OptExact<T>& v) : h(v.h) {}

template <class Elem, class X>
inline constexpr bool rest_accepts_v = [] {
    using D = std::decay_t<X>;
    if constexpr (std::same_as<Elem, Value>) {
        return is_handle<X> || is_string_arg<X> || is_number_arg<X> || std::same_as<D, bool> || std::is_enum_v<D> || is_variant<D> || is_optional<D>::value;
    } else if constexpr (is_variant<Elem>) {
        return in_accepts<Elem, X>;
    } else if constexpr (std::same_as<Elem, std::string>) {
        return is_string_arg<X>;
    } else if constexpr (std::same_as<Elem, double>) {
        return is_number_arg<X>;
    } else if constexpr (std::same_as<Elem, bool>) {
        return std::same_as<D, bool>;
    } else if constexpr (std::is_enum_v<Elem>) {
        return std::same_as<D, Elem>;
    } else {
        return (is_handle<X> && extends<D, Elem>) || (is_callback<Elem> && callable_candidate<X> && std::constructible_from<Elem, X>);
    }
}();
template <class Elem, class X>
concept rest_accepts = rest_accepts_v<Elem, X>;

template <class A>
inline constexpr bool is_array = false;
template <class U>
inline constexpr bool is_array<Array<U>> = true;

// An array given alone is the argument list, of whatever it holds.
template <class Elem, bool CheckExtends, class R0, class... Rest>
concept rest_pack =
    !(sizeof...(Rest) == 0 && (is_array<std::decay_t<R0>> || std::is_base_of_v<Array<Elem>, std::decay_t<R0>> || (CheckExtends && extends<std::decay_t<R0>, Array<Elem>>)))
    && rest_accepts<Elem, R0>
    && (rest_accepts<Elem, Rest> && ...);

template <class T>
struct Iterator;
struct IterationEnd {};

// A JavaScript array of T; `Array<>` of any values. A type that extends
// Array inherits this: it can be indexed and iterated.
template <class T>
struct Array : virtual Handle {
    Array() = default;
    Array(handle_t, uint32_t h) : Handle(handle, h) {}
    Array(std::initializer_list<T> items) : Handle(handle, ts_raw_js_array_new()) {
        for (const T& x : items) push(x);
    }
    // An array of any value takes any array, an array of a union one of what the union takes,
    // and an array of a class one of its subclass, as TypeScript has it (`Sub[]` is a `Base[]`).
    // Not a type that extends Array: it is also taken where another parameter would take it.
    template <class A, class U = typename array_elem<A>::type>
        requires (!std::same_as<U, T> && (std::same_as<T, Value> || (is_variant<T> && in_accepts<T, U>) || std::is_base_of_v<T, U>))
    Array(const A& o) : Handle(o) {}
    static Array create() { return Array(handle, ts_raw_js_array_new()); }
    template <class... Us>
        requires (rest_accepts<T, Us> && ...)
    static Array of(Us&&... xs) {
        Array a = create();
        (a.push(std::forward<Us>(xs)), ...);
        return a;
    }
    uint32_t size() const { return ts_raw_js_array_length(h); }
    T operator[](uint32_t i) const { return from_handle<T>(ts_raw_js_array_get(h, i)); }
    void push(const T& v) const { ts_raw_js_array_push(h, Arg(v).h); }
    template <class U>
        requires (!std::same_as<std::decay_t<U>, T> && (rest_accepts<T, U> || std::convertible_to<const U&, T>))
    void push(U&& v) const {
        if constexpr (std::same_as<T, Value>) ts_raw_js_array_push(h, Arg(v).h);
        else if constexpr (is_variant<T>) ts_raw_js_array_push(h, In<T>(std::forward<U>(v)).h);
        else if constexpr (std::same_as<T, std::string>) {
            using D = std::decay_t<U>;
            if constexpr (std::is_pointer_v<D>) {
                std::string_view sv = v ? std::string_view(v) : std::string_view();
                ts_raw_js_array_push(h, Arg(sv).h);
            } else {
                std::string_view sv(v);
                ts_raw_js_array_push(h, Arg(sv).h);
            }
        } else if constexpr (std::same_as<T, double>) ts_raw_js_array_push(h, Arg(double(v)).h);
        else if constexpr (std::same_as<T, bool>) ts_raw_js_array_push(h, Arg(bool(v)).h);
        else if constexpr (std::is_enum_v<T>) ts_raw_js_array_push(h, Arg(T(v)).h);
        else if constexpr (is_handle<std::decay_t<U>>) ts_raw_js_array_push(h, v.h);
        else ts_raw_js_array_push(h, Arg(T(std::forward<U>(v))).h);
    }
    Iterator<T> begin() const;
    IterationEnd end() const { return {}; }
};

template <class T>
inline Array<T> Value::to_array() const {
    if (h) ts_raw_js_retain(h);
    return Array<T>(handle, h);
}

// A JavaScript exception, or the reason a promise was rejected.
struct Error : Handle {
    using Handle::Handle;
    static Error create(std::string_view msg) { return Error(handle, ts_raw_js_error_new(msg.data(), msg.size())); }
    std::string name() const { return from_handle<std::string>(ts_raw_js_error_name(h)); }
    std::string message() const { return from_handle<std::string>(ts_raw_js_error_message(h)); }
    std::string stack() const { return from_handle<std::string>(ts_raw_js_error_stack(h)); }
};

// The exception a call threw, taken. A call that throws returns a zero
// value, and an exception not taken by the time control returns to
// JavaScript is reported there: see `attempt`.
inline std::optional<Error> take_error() {
    if (uint32_t e = ts_raw_js_take_error()) return Error(handle, e);
    return std::nullopt;
}

// A value, or the error that came instead: what a promise settles with, and
// what `attempt` returns.
template <class T>
struct [[nodiscard]] Result {
    Result(T x) : v(std::in_place_index<0>, std::move(x)) {}
    Result(Error e) : v(std::in_place_index<1>, std::move(e)) {}
    explicit operator bool() const { return v.index() == 0; }
    T& operator*() { return std::get<0>(v); }
    const T& operator*() const { return std::get<0>(v); }
    T* operator->() { return &std::get<0>(v); }
    const T* operator->() const { return &std::get<0>(v); }
    const Error& error() const { return std::get<1>(v); }

  private:
    std::variant<T, Error> v;
};
template <>
struct [[nodiscard]] Result<void> {
    Result() = default;
    Result(Error e) : e(std::move(e)) {}
    explicit operator bool() const { return !e; }
    const Error& error() const { return *e; }

  private:
    std::optional<Error> e;
};

// Runs f and gives what it returned, or the exception a call in it threw:
// `auto found = ts::attempt([&] { return el.querySelector(text); });`.
template <class F>
auto attempt(F f) -> Result<std::invoke_result_t<F>> {
    using T = std::invoke_result_t<F>;
    if constexpr (std::is_void_v<T>) {
        f();
        if (auto e = take_error()) return Result<void>(std::move(*e));
        return Result<void>();
    } else {
        T x = f();
        if (auto e = take_error()) return Result<T>(std::move(*e));
        return Result<T>(std::move(x));
    }
}

// A JavaScript promise. `then` calls a function with its Result once it
// settles; in a coroutine, `co_await` waits for it: `auto r = co_await fetch(url);`.
template <class T>
struct Promise : virtual Handle {
    Promise() = default;
    Promise(handle_t, uint32_t h) : Handle(handle, h) {}

    template <class F>
    void then(F f) const {
        ts_raw_js_promise_then(h, reinterpret_cast<uintptr_t>(&settle<F>), new F(std::move(f)));
    }

    struct Awaiter {
        Promise p;
        std::optional<Result<T>> r;
        bool await_ready() const { return false; }
        void await_suspend(std::coroutine_handle<> c) {
            p.then([this, c](Result<T> x) {
                r.emplace(std::move(x));
                c.resume();
            });
        }
        Result<T> await_resume() { return std::move(*r); }
    };
    Awaiter operator co_await() const { return Awaiter{*this, std::nullopt}; }

  private:
    template <class F>
    static void settle(void* ctx, int32_t ok, uint32_t v) {
        F* f = static_cast<F*>(ctx);
        if (!ok) (*f)(Result<T>(Error(handle, v)));
        else if constexpr (std::is_void_v<T>) {
            if (v) ts_raw_js_release(v);
            (*f)(Result<T>());
        } else (*f)(Result<T>(from_handle<T>(v)));
        delete f;
    }
};

// Walks a JavaScript async iterator. `next()` is a promise of the next value,
// or of nothing at the end: `while (auto r = co_await it.next()) { if (!*r) break; ... }`.
template <class T>
struct AsyncIterator : Handle {
    using Handle::Handle;
    Promise<std::optional<T>> next() const { return Promise<std::optional<T>>(handle, ts_raw_js_async_iterator_next(h)); }
};

// A coroutine that starts at once and runs to its end, awaiting promises on the way.
struct Task {
    struct promise_type {
        Task get_return_object() { return {}; }
        std::suspend_never initial_suspend() noexcept { return {}; }
        std::suspend_never final_suspend() noexcept { return {}; }
        void return_void() {}
        void unhandled_exception() {}
    };
};

// Walks a JavaScript iterator, for a range-for: `for (Node n : list)`.
template <class T>
struct Iterator {
    explicit Iterator(uint32_t it) : it(handle, it) { ++*this; }
    T operator*() const { return *cur; }
    Iterator& operator++() {
        uint32_t v = ts_raw_js_iterator_next(it.h);
        if (v == 0xFFFFFFFFu) cur.reset();
        else cur.emplace(from_handle<T>(v));
        return *this;
    }
    bool operator==(IterationEnd) const { return !cur; }

  private:
    Handle it;
    std::optional<T> cur;
};

template <class T>
Iterator<T> Array<T>::begin() const { return Iterator<T>(ts_raw_js_iterate(h)); }

// A JavaScript array of fixed length and element types: `[number, string]`
// is `Tuple<double, std::string>`. `get<0>()` reads an element.
template <class... T>
struct Tuple : Handle {
    Tuple() = default;
    Tuple(handle_t, uint32_t h) : Handle(handle, h) {}
    Tuple(const Tuple&) = default;
    Tuple(Tuple&&) = default;
    Tuple& operator=(const Tuple&) = default;
    Tuple& operator=(Tuple&&) = default;
    Tuple(const T&... xs) requires (sizeof...(T) > 0) : Handle(handle, ts_raw_js_array_new()) { (ts_raw_js_array_push(h, Arg(xs).h), ...); }
    template <size_t I>
    std::tuple_element_t<I, std::tuple<T...>> get() const {
        return from_handle<std::tuple_element_t<I, std::tuple<T...>>>(ts_raw_js_array_get(h, I));
    }
};
template <class... T>
Tuple(const T&...) -> Tuple<T...>;

template <class... Ts>
struct Opt<Tuple<Ts...>> {
    uint32_t h = 0;
    bool own = false;
    Opt(const Opt&) = delete;
    Opt& operator=(const Opt&) = delete;
    Opt(std::nullopt_t) noexcept {}
    Opt(const Tuple<Ts...>& v) noexcept : h(v.h) {}
    Opt(const std::optional<Tuple<Ts...>>& o) noexcept {
        if (o) h = o->h;
    }
    Opt(const Ts&... xs) requires (sizeof...(Ts) > 0) : h(ts_raw_js_array_new()), own(true) {
        (ts_raw_js_array_push(h, Arg(xs).h), ...);
    }
    ~Opt() {
        if (own) ts_raw_js_release(h);
    }
};

// A key of a map whose value has type V: `HTMLElementEventMapKeys::click`.
template <class Map, class V>
struct Key {
    int32_t index;
};

// A lambda kept for JavaScript to call, until released.
struct Closure {
    virtual ~Closure() = default;
};
template <class F>
struct ClosureOf : Closure {
    F f;
    explicit ClosureOf(F f) : f(std::move(f)) {}
};
template <class F>
void* closure(F f) { return static_cast<Closure*>(new ClosureOf<F>(std::move(f))); }
template <class F>
F& closure_fn(void* ctx) { return static_cast<ClosureOf<F>*>(static_cast<Closure*>(ctx))->f; }

extern "C" __attribute__((export_name("ts_drop_closure"), weak))
inline void ts_drop_closure(void* ctx) {
    delete static_cast<Closure*>(ctx);
}

// A JavaScript function. One made from a lambda lives until `release()`,
// which frees the lambda; JavaScript must not call it after that. Copies
// share the function: release one of them, once.
struct Callback : virtual Handle {
    void release() {
        if (h) {
            if (uint32_t ctx = ts_raw_js_callback_release(h)) delete static_cast<Closure*>(reinterpret_cast<void*>(uintptr_t(ctx)));
            static_cast<Handle&>(*this) = Handle();
        }
    }
};

// A callback released when it goes out of scope.
template <class C>
struct Scoped : C {
    Scoped() = default;
    template <class F>
        requires (!std::same_as<std::decay_t<F>, Scoped> && !is_handle<F> && std::constructible_from<C, F>)
    Scoped(F&& f) : Handle(C(std::forward<F>(f))) {}
    Scoped(const Scoped&) = delete;
    Scoped& operator=(const Scoped&) = delete;
    Scoped(Scoped&& o) noexcept : Handle(std::move(static_cast<Handle&>(o))), C() {}
    Scoped& operator=(Scoped&& o) noexcept {
        if (this != &o) {
            this->release();
            static_cast<Handle&>(*this) = std::move(static_cast<Handle&>(o));
        }
        return *this;
    }
    ~Scoped() { this->release(); }
};

// A callback taking one object of type E, made from a lambda. Pass the same
// Listener to add and remove it.
template <class E>
struct Listener : Callback {
    Listener() = default;
    Listener(handle_t, uint32_t h) : Handle(handle, h) {}
    template <class F>
        requires (callable_candidate<F> && std::invocable<F&, dep<F, E>>)
    Listener(F f) : Handle(handle, ts_raw_js_listener_from(reinterpret_cast<uintptr_t>(&call<F>), closure(std::move(f)))) {}

  private:
    template <class F>
    static void call(void* ctx, uint32_t ev) { closure_fn<F>(ctx)(E(handle, ev)); }
};
template <class E>
inline constexpr bool is_callback<Listener<E>> = true;

}  // namespace ts
#endif
