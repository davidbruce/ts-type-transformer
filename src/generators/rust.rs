// XML -> Rust bindings for wasm32-unknown-unknown, in one module.
//
// Every JavaScript object is a value holding a counted handle: `Clone`
// retains it and `Drop` releases it. Inheritance is traits: each type's own
// members are default methods of `XMethods`, implemented for the type and
// each of its subtypes, so `div.setAttribute(..)` works on an
// HTMLDivElement. Where two of a type's traits share a name, the type has an
// inherent method of that name, which Rust prefers. Nullables are `Option`,
// unions enums, arrays `JsArray<T>`, callbacks closures, generic types
// generic structs, keyed methods generic over typed keys:
// `el.addEventListener(HTMLElementEventMapKeys::click, Listener::new(|e: PointerEvent| ..), None)`.

use std::collections::{BTreeMap, BTreeSet};

use super::wasm::{Call, Class, Func, Model, NamedUnion, TParam, Ty, base_args, map_types, subst, with_self_args};

fn ident(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
        "pub", "ref", "return", "static", "struct", "trait", "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become",
        "box", "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
    ];
    let s = super::wasm::ident(name);
    match s.as_str() {
        "self" | "Self" | "super" | "crate" | "_" => format!("{s}_"),
        s if KEYWORDS.contains(&s) => format!("r#{s}"),
        _ => s,
    }
}

thread_local! {
    // The unions of the model being generated that values can be decoded to.
    static DECODABLE: std::cell::RefCell<BTreeSet<String>> = const { std::cell::RefCell::new(BTreeSet::new()) };
    static UNIONS: std::cell::RefCell<BTreeSet<String>> = const { std::cell::RefCell::new(BTreeSet::new()) };
    static CLASSES: std::cell::RefCell<BTreeSet<String>> = const { std::cell::RefCell::new(BTreeSet::new()) };
}

fn decodable(n: &str) -> bool {
    DECODABLE.with(|d| d.borrow().contains(n))
}

fn is_union(n: &str) -> bool {
    UNIONS.with(|d| d.borrow().contains(n))
}

// A type the bindings declare without type parameters: one `Is` is implemented for.
fn is_class(n: &str) -> bool {
    CLASSES.with(|d| d.borrow().contains(n))
}

// The Rust type of a value: owned, as it comes back or as a type argument.
fn ty(t: &Ty) -> String {
    match t {
        Ty::Void => "()".into(),
        Ty::Num => "f64".into(),
        Ty::Bool => "bool".into(),
        Ty::Str => "String".into(),
        Ty::Any | Ty::Union(_) => "runtime::JsValue".into(),
        Ty::Array => "runtime::JsArray".into(),
        Ty::ArrayOf(e) => format!("runtime::JsArray<{}>", ty(e)),
        Ty::Tuple(es) if (2..=4).contains(&es.len()) => format!("Tuple{}<{}>", es.len(), es.iter().map(ty).collect::<Vec<_>>().join(", ")),
        Ty::Tuple(_) => "JsArray".into(),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => n.clone(),
        // In full: a DOM type is named Option.
        Ty::Nullable(inner, _) => format!("::core::option::Option<{}>", ty(inner)),
        Ty::NamedUnion(n) if decodable(n) => n.clone(),
        Ty::NamedUnion(_) => "JsValue".into(),
        // The runtime's, by path: an API may declare its own Listener.
        Ty::Key(m) => format!("runtime::Key<{m}, V>"),
        Ty::Mapped(_) => "V".into(),
        Ty::Listener(ev) => format!("runtime::Listener<{}>", ty(ev)),
        Ty::Param(n) => n.clone(),
        Ty::Generic(n, _) if is_union(n) && !decodable(n) => "JsValue".into(),
        Ty::Generic(n, args) if !args.is_empty() => format!("{n}<{}>", args.iter().map(ty).collect::<Vec<_>>().join(", ")),
        Ty::Generic(n, _) => n.clone(),
    }
}

// The Rust type of a parameter: strings are borrowed, and an object is any
// value that is one: `nameOf(&shape)` where a Named is wanted.
fn ty_in(t: &Ty) -> String {
    match t {
        Ty::Str => "&str".into(),
        Ty::Obj(n) => format!("&impl AsRef<{n}>"),
        Ty::Nullable(inner, _) if **inner == Ty::Str => "::core::option::Option<&str>".into(),
        t => ty(t),
    }
}

// What a value crosses the boundary as.
fn raw(t: &Ty) -> Vec<&'static str> {
    match t {
        Ty::Void => vec![],
        Ty::Num | Ty::NumEnum(_) => vec!["f64"],
        Ty::Bool | Ty::Enum(_) | Ty::Key(_) => vec!["i32"],
        Ty::Str => vec!["*const u8", "usize"],
        Ty::Nullable(inner, _) if **inner == Ty::Str => vec!["*const u8", "usize"],
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => vec!["i32", "f64"],
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => vec!["i32"],
        _ => vec!["u32"],
    }
}

fn raw_ret(t: &Ty) -> &'static str {
    match t {
        Ty::Void => "()",
        Ty::Num | Ty::NumEnum(_) => "f64",
        Ty::Bool | Ty::Enum(_) => "i32",
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => "i32",
        _ => "u32",
    }
}

// Expressions turning a Rust value into what crosses. A lent handle lives
// until the end of the statement that makes the call.
fn lower(t: &Ty, x: &str) -> Vec<String> {
    match t {
        Ty::Num => vec![x.into()],
        Ty::NumEnum(_) => vec![format!("{x} as i32 as f64")],
        Ty::Bool | Ty::Enum(_) => vec![format!("{x} as i32")],
        Ty::Key(_) => vec![format!("{x}.index")],
        Ty::Str => vec![format!("{x}.as_ptr()"), format!("{x}.len()")],
        Ty::Nullable(inner, _) if **inner == Ty::Str => vec![format!("{x}.map_or(core::ptr::null(), |s| s.as_ptr())"), format!("{x}.map_or(0, |s| s.len())")],
        Ty::Nullable(inner, _) if **inner == Ty::Num => vec![format!("{x}.is_some() as i32"), format!("{x}.unwrap_or(0.0)")],
        Ty::Nullable(inner, _) if matches!(**inner, Ty::NumEnum(_)) => vec![format!("{x}.is_some() as i32"), format!("{x}.map_or(0.0, |v| v as i32 as f64)")],
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => vec![format!("{x}.map_or(-1, |v| v as i32)")],
        Ty::Obj(n) => vec![format!("AsRef::<{n}>::as_ref({x}).lend().h")],
        _ => vec![format!("{x}.lend().h")],
    }
}

// Expression turning what came back, `r`, into a Rust value.
fn raise(t: &Ty, r: &str) -> String {
    match t {
        Ty::Void => "()".into(),
        Ty::Num => r.into(),
        Ty::NumEnum(e) => format!("{e}::from_value({r} as i32)"),
        Ty::Bool => format!("{r} != 0"),
        Ty::Enum(e) => format!("{e}::from_index({r})"),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("if {r} < 0 {{ None }} else {{ Some({r} != 0) }}"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("if {r} < 0 {{ None }} else {{ Some({}::from_index({r})) }}", ty(inner)),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::NumEnum(_)) => format!("::core::option::Option::<f64>::from_handle({r}).map(|v| {}::from_value(v as i32))", ty(inner)),
        t => format!("<{}>::from_handle({r})", ty(t)),
    }
}

// What a callback argument or result crosses as.
fn raw_callback(t: &Ty) -> &'static str {
    raw_ret(t)
}

// What a callback returns, given to JavaScript.
fn lower_callback(t: &Ty, x: &str) -> String {
    match t {
        Ty::Void => format!("{x};"),
        Ty::Num => x.into(),
        Ty::NumEnum(_) => format!("{x} as i32 as f64"),
        Ty::Bool | Ty::Enum(_) => format!("{x} as i32"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => format!("{x}.map_or(-1, |v| v as i32)"),
        _ => format!("give(&{x})"),
    }
}

// A type parameter with its bounds: `T: JsType`, and for `T extends Node`,
// `T: JsType + Is<Node>`.
fn bound(p: &TParam) -> String {
    match &p.constraint {
        Some(Ty::Obj(n)) if is_class(n) => format!("{}: JsType + Is<{n}>", p.name),
        _ => format!("{}: JsType", p.name),
    }
}

// `<T: JsType, U: JsType>`, or nothing.
fn generics(params: &[TParam]) -> String {
    if params.is_empty() { String::new() } else { format!("<{}>", params.iter().map(bound).collect::<Vec<_>>().join(", ")) }
}

// `<T, U>`, or nothing.
fn args_of(params: &[TParam]) -> String {
    if params.is_empty() { String::new() } else { format!("<{}>", params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")) }
}

// `PhantomData<(T, U)>` for a generic struct.
fn phantom(params: &[String]) -> String {
    if params.is_empty() { String::new() } else { format!(", PhantomData<fn() -> ({},)>", params.join(", ")) }
}

// One method or function: its signature and body. `this` is the handle of
// the object it is called on, for members.
fn function(f: &Func, imports: &mut BTreeMap<String, String>, this: Option<&str>) -> (String, String) {
    let mut raw_params: Vec<String> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    if let Some(this) = this {
        raw_params.push("this: u32".into());
        args.push(this.into());
    }
    let mut params = Vec::new();
    for (name, t) in &f.params {
        let name = format!("{}_", super::wasm::ident(name));
        params.push(format!("{name}: {}", ty_in(t)));
        for (i, r) in raw(t).into_iter().enumerate() {
            raw_params.push(format!("{name}{i}: {r}"));
        }
        args.extend(lower(t, &name));
    }
    let import = match &f.shared {
        Some((shared, k)) => {
            let at = usize::from(this.is_some());
            raw_params.insert(at, "k: i32".into());
            args.insert(at, k.to_string());
            shared.clone()
        }
        None => f.import.clone(),
    };
    imports.entry(import.clone()).or_insert_with(|| format!("        pub fn {import}({}) -> {};\n", raw_params.join(", "), raw_ret(&f.ret)));
    let mut type_params: Vec<String> = f.type_params.iter().map(bound).collect();
    if f.params.iter().any(|p| matches!(p.1, Ty::Key(_))) {
        type_params.insert(0, "V: JsType".into());
    }
    let generics = if type_params.is_empty() { String::new() } else { format!("<{}>", type_params.join(", ")) };
    let self_param = if this.is_some() { "&self".to_string() } else { String::new() };
    let all_params: Vec<String> = std::iter::once(self_param).filter(|s| !s.is_empty()).chain(params).collect();
    let ret = ty(&f.ret);
    let sig = format!("fn {}{generics}({}) -> {ret}", ident(&f.name), all_params.join(", "));
    let body = format!("{{ let r = unsafe {{ raw::{import}({}) }}; {} }}", args.join(", "), raise(&f.ret, "r"));
    (sig, body)
}

const RUNTIME: &str = r#"// Generated. The runtime: handles, conversions, arrays, callbacks, promises.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code, unused_imports, unused_unsafe, clippy::all)]

use core::ffi::c_void;
use core::marker::PhantomData;
use std::any::Any;

mod rt {
    use core::ffi::c_void;
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        pub fn js_string_len(h: u32) -> u32;
        pub fn js_string_read(h: u32, p: *mut u8);
        pub fn js_string_take(h: u32, p: *mut u8, cap: usize) -> u32;
        pub fn js_from_number(x: f64) -> u32;
        pub fn js_from_bool(x: i32) -> u32;
        pub fn js_from_string(p: *const u8, n: usize) -> u32;
        pub fn js_to_number(h: u32) -> f64;
        pub fn js_to_bool(h: u32) -> i32;
        pub fn js_equals(a: u32, b: u32) -> i32;
        pub fn js_array_new() -> u32;
        pub fn js_array_length(h: u32) -> u32;
        pub fn js_array_get(h: u32, i: u32) -> u32;
        pub fn js_array_push(h: u32, v: u32);
        pub fn js_listener_from(f: usize, ctx: *mut c_void) -> u32;
        pub fn js_callback_release(h: u32) -> u32;
        pub fn js_retain(h: u32);
        pub fn js_release(h: u32);
        pub fn js_live() -> u32;
        pub fn js_take_error() -> u32;
        pub fn js_error_message(h: u32) -> u32;
        pub fn js_promise_then(p: u32, f: usize, ctx: *mut c_void);
        pub fn js_iterate(h: u32) -> u32;
        pub fn js_iterator_next(it: u32) -> u32;
        pub fn js_async_iterate(h: u32) -> u32;
        pub fn js_async_iterator_next(it: u32) -> u32;
    }
}

// A JavaScript value, counted: `Clone` retains it, `Drop` releases it. Zero is null.
pub struct Handle(pub u32);
impl Clone for Handle {
    fn clone(&self) -> Self {
        if self.0 != 0 {
            unsafe { rt::js_retain(self.0) }
        }
        Handle(self.0)
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe { rt::js_release(self.0) }
        }
    }
}

// How many references to JavaScript values wasm holds.
pub fn live_handles() -> u32 {
    unsafe { rt::js_live() }
}

// Whether two handles are one JavaScript value: what `==` means for every handle type.
pub fn same(a: &Handle, b: &Handle) -> bool {
    a.0 == b.0 || unsafe { rt::js_equals(a.0, b.0) != 0 }
}

// A value lent to JavaScript for one call: an object's own handle, or a new
// one for a number, flag or string, released after the call.
pub struct Lent {
    pub h: u32,
    own: bool,
}
impl Lent {
    pub fn borrow(h: u32) -> Self {
        Lent { h, own: false }
    }
    pub fn owned(h: u32) -> Self {
        Lent { h, own: true }
    }
}
impl Drop for Lent {
    fn drop(&mut self) {
        if self.own && self.h != 0 {
            unsafe { rt::js_release(self.h) }
        }
    }
}

// A value that crosses as a handle, going in and coming back. `from_handle`
// takes the handle over. KIND tells a bare type parameter apart in a union.
pub trait ToJs {
    fn lend(&self) -> Lent;
}
pub trait FromJs: Sized {
    const KIND: i32 = 0;
    fn from_handle(h: u32) -> Self;
}
pub trait JsType: FromJs + ToJs + Clone + 'static {}
impl<T: FromJs + ToJs + Clone + 'static> JsType for T {}

// A handle JavaScript keeps: what a callback returns.
pub fn give<T: ToJs + ?Sized>(v: &T) -> u32 {
    let l = v.lend();
    let h = l.h;
    if !l.own && h != 0 {
        unsafe { rt::js_retain(h) }
    }
    core::mem::forget(l);
    h
}

fn read_string(h: u32) -> String {
    let n = unsafe { rt::js_string_len(h) } as usize;
    let mut v = vec![0u8; n];
    unsafe { rt::js_string_read(h, v.as_mut_ptr()) };
    String::from_utf8(v).unwrap_or_default()
}

impl ToJs for f64 {
    fn lend(&self) -> Lent {
        Lent::owned(unsafe { rt::js_from_number(*self) })
    }
}
impl FromJs for f64 {
    const KIND: i32 = 1;
    fn from_handle(h: u32) -> Self {
        let x = unsafe { rt::js_to_number(h) };
        unsafe { rt::js_release(h) };
        x
    }
}
impl ToJs for bool {
    fn lend(&self) -> Lent {
        Lent::owned(unsafe { rt::js_from_bool(*self as i32) })
    }
}
impl FromJs for bool {
    const KIND: i32 = 3;
    fn from_handle(h: u32) -> Self {
        let x = unsafe { rt::js_to_bool(h) } != 0;
        unsafe { rt::js_release(h) };
        x
    }
}
impl ToJs for str {
    fn lend(&self) -> Lent {
        Lent::owned(unsafe { rt::js_from_string(self.as_ptr(), self.len()) })
    }
}
impl ToJs for &str {
    fn lend(&self) -> Lent {
        (**self).lend()
    }
}
impl ToJs for String {
    fn lend(&self) -> Lent {
        self.as_str().lend()
    }
}
impl FromJs for String {
    const KIND: i32 = 2;
    // In one call, when it fits a small buffer.
    fn from_handle(h: u32) -> Self {
        let mut buf = [0u8; 256];
        let n = unsafe { rt::js_string_take(h, buf.as_mut_ptr(), buf.len()) } as usize;
        if n <= buf.len() {
            return String::from_utf8(buf[..n].to_vec()).unwrap_or_default();
        }
        let s = read_string(h);
        unsafe { rt::js_release(h) };
        s
    }
}
impl ToJs for () {
    fn lend(&self) -> Lent {
        Lent::borrow(0)
    }
}
impl FromJs for () {
    fn from_handle(h: u32) -> Self {
        if h != 0 {
            unsafe { rt::js_release(h) }
        }
    }
}
impl<T: ToJs> ToJs for Option<T> {
    fn lend(&self) -> Lent {
        match self {
            Some(x) => x.lend(),
            None => Lent::borrow(0),
        }
    }
}
impl<T: FromJs> FromJs for Option<T> {
    fn from_handle(h: u32) -> Self {
        if h == 0 { None } else { Some(T::from_handle(h)) }
    }
}

// Declares a handle type, generic or not.
macro_rules! handle {
    ($name:ident $(<$($p:ident),*>)?) => {
        #[repr(transparent)]
        pub struct $name $(<$($p),*>)? (pub Handle $(, PhantomData<fn() -> ($($p,)*)>)?);
        impl $(<$($p),*>)? AsRef<$name $(<$($p),*>)?> for $name $(<$($p),*>)? {
            fn as_ref(&self) -> &Self {
                self
            }
        }
        impl $(<$($p),*>)? Clone for $name $(<$($p),*>)? {
            fn clone(&self) -> Self {
                $name(self.0.clone() $(, PhantomData::<fn() -> ($($p,)*)>)?)
            }
        }
        impl $(<$($p),*>)? AsRef<Handle> for $name $(<$($p),*>)? {
            fn as_ref(&self) -> &Handle {
                &self.0
            }
        }
        impl $(<$($p),*>)? PartialEq for $name $(<$($p),*>)? {
            fn eq(&self, o: &Self) -> bool {
                same(&self.0, &o.0)
            }
        }
        impl $(<$($p),*>)? ToJs for $name $(<$($p),*>)? {
            fn lend(&self) -> Lent {
                Lent::borrow(self.0 .0)
            }
        }
        impl $(<$($p),*>)? FromJs for $name $(<$($p),*>)? {
            fn from_handle(h: u32) -> Self {
                $name(Handle(h) $(, PhantomData::<fn() -> ($($p,)*)>)?)
            }
        }
    };
}
pub(crate) use handle;

// `T: Is<B>` says a T is a B: itself, a subtype of it, a union of such, or
// any value. It is what a type parameter's `extends` becomes.
pub trait Is<B> {}
impl<B> Is<B> for JsValue {}

// A JavaScript value of no particular type.
handle!(JsValue);
impl JsValue {
    pub fn number(x: f64) -> Self {
        JsValue(Handle(unsafe { rt::js_from_number(x) }))
    }
    pub fn boolean(x: bool) -> Self {
        JsValue(Handle(unsafe { rt::js_from_bool(x as i32) }))
    }
    pub fn string(s: &str) -> Self {
        JsValue(Handle(unsafe { rt::js_from_string(s.as_ptr(), s.len()) }))
    }
    pub fn as_f64(&self) -> f64 {
        unsafe { rt::js_to_number(self.0 .0) }
    }
    pub fn as_bool(&self) -> bool {
        unsafe { rt::js_to_bool(self.0 .0) != 0 }
    }
    pub fn as_string(&self) -> String {
        read_string(self.0 .0)
    }
    pub fn is_null(&self) -> bool {
        self.0 .0 == 0
    }
}

// A JavaScript array of T; `JsArray` of any values.
pub struct JsArray<T = JsValue>(pub Handle, PhantomData<fn() -> T>);
impl<T> Clone for JsArray<T> {
    fn clone(&self) -> Self {
        JsArray(self.0.clone(), PhantomData)
    }
}
impl<T> ToJs for JsArray<T> {
    fn lend(&self) -> Lent {
        Lent::borrow(self.0 .0)
    }
}
impl<T> FromJs for JsArray<T> {
    fn from_handle(h: u32) -> Self {
        JsArray(Handle(h), PhantomData)
    }
}
impl<T: JsType> JsArray<T> {
    pub fn new() -> Self {
        JsArray(Handle(unsafe { rt::js_array_new() }), PhantomData)
    }
    pub fn len(&self) -> usize {
        unsafe { rt::js_array_length(self.0 .0) as usize }
    }
    pub fn get(&self, i: usize) -> T {
        T::from_handle(unsafe { rt::js_array_get(self.0 .0, i as u32) })
    }
    pub fn push(&self, v: impl Into<T>) {
        let v: T = v.into();
        unsafe { rt::js_array_push(self.0 .0, v.lend().h) }
    }
    // The same array, untyped.
    pub fn any(&self) -> JsArray {
        JsArray(self.0.clone(), PhantomData)
    }
}
impl<T: JsType, A: Into<T>> FromIterator<A> for JsArray<T> {
    fn from_iter<I: IntoIterator<Item = A>>(items: I) -> Self {
        let a = JsArray::new();
        for x in items {
            a.push(x);
        }
        a
    }
}

// Arrays of fixed length and element types: `[number, string]` is `Tuple2<f64, String>`.
macro_rules! tuple {
    ($name:ident; $($t:ident $get:ident $i:literal),*) => {
        pub struct $name<$($t),*>(pub Handle, PhantomData<fn() -> ($($t,)*)>);
        impl<$($t),*> Clone for $name<$($t),*> {
            fn clone(&self) -> Self {
                $name(self.0.clone(), PhantomData)
            }
        }
        impl<$($t),*> ToJs for $name<$($t),*> {
            fn lend(&self) -> Lent {
                Lent::borrow(self.0 .0)
            }
        }
        impl<$($t),*> FromJs for $name<$($t),*> {
            fn from_handle(h: u32) -> Self {
                $name(Handle(h), PhantomData)
            }
        }
        impl<$($t: JsType),*> $name<$($t),*> {
            pub fn new($($get: impl Into<$t>),*) -> Self {
                let a = unsafe { rt::js_array_new() };
                $(unsafe { rt::js_array_push(a, Into::<$t>::into($get).lend().h) };)*
                $name(Handle(a), PhantomData)
            }
            $(pub fn $get(&self) -> $t {
                $t::from_handle(unsafe { rt::js_array_get(self.0 .0, $i) })
            })*
        }
    };
}
tuple!(Tuple2; A get0 0, B get1 1);
tuple!(Tuple3; A get0 0, B get1 1, C get2 2);
tuple!(Tuple4; A get0 0, B get1 1, C get2 2, D get3 3);

// A key of a map whose value has type V: `HTMLElementEventMapKeys::click`.
pub struct Key<M, V> {
    pub index: i32,
    _p: PhantomData<fn() -> (M, V)>,
}
impl<M, V> Key<M, V> {
    pub const fn new(index: i32) -> Self {
        Key { index, _p: PhantomData }
    }
}
impl<M, V> Clone for Key<M, V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<M, V> Copy for Key<M, V> {}

// A closure kept for JavaScript to call, until released.
pub fn closure<F: 'static>(f: F) -> *mut c_void {
    Box::into_raw(Box::new(Box::new(f) as Box<dyn Any>)) as *mut c_void
}
pub unsafe fn closure_fn<'a, F: 'static>(ctx: *mut c_void) -> &'a mut F {
    unsafe { (*(ctx as *mut Box<dyn Any>)).downcast_mut::<F>().unwrap() }
}

// A JavaScript function made from a closure. It lives until `release`, which
// frees the closure; JavaScript must not call it after that.
pub trait Callback: AsRef<Handle> + Sized {
    fn release(self) {
        let ctx = unsafe { rt::js_callback_release(AsRef::<Handle>::as_ref(&self).0) };
        if ctx != 0 {
            drop(unsafe { Box::from_raw(ctx as usize as *mut Box<dyn Any>) });
        }
    }
}

// A callback taking one object of type E: `Listener::new(|e: PointerEvent| ..)`.
pub struct Listener<E>(pub Handle, PhantomData<fn() -> E>);
impl<E> Clone for Listener<E> {
    fn clone(&self) -> Self {
        Listener(self.0.clone(), PhantomData)
    }
}
impl<E> AsRef<Handle> for Listener<E> {
    fn as_ref(&self) -> &Handle {
        &self.0
    }
}
impl<E> ToJs for Listener<E> {
    fn lend(&self) -> Lent {
        Lent::borrow(self.0 .0)
    }
}
impl<E> FromJs for Listener<E> {
    fn from_handle(h: u32) -> Self {
        Listener(Handle(h), PhantomData)
    }
}
impl<E> Callback for Listener<E> {}
impl<E: JsType> Listener<E> {
    pub fn new<F: FnMut(E) + 'static>(f: F) -> Self {
        extern "C" fn call<E: JsType, F: FnMut(E) + 'static>(ctx: *mut c_void, ev: u32) {
            (unsafe { closure_fn::<F>(ctx) })(E::from_handle(ev))
        }
        Listener(Handle(unsafe { rt::js_listener_from(call::<E, F> as *const () as usize, closure(f)) }), PhantomData)
    }
}

// A JavaScript exception, or the reason a promise was rejected.
handle!(Error);
impl Error {
    pub fn message(&self) -> String {
        String::from_handle(unsafe { rt::js_error_message(self.0 .0) })
    }
}

// The exception a call threw, taken. A call that throws returns a zero
// value, and an exception not taken by the time control returns to
// JavaScript is reported there: see `attempt`.
pub fn take_error() -> Option<Error> {
    let e = unsafe { rt::js_take_error() };
    if e == 0 { None } else { Some(Error(Handle(e))) }
}

// Runs f and gives what it returned, or the exception a call in it threw:
// `let found = attempt(|| el.querySelector(text))?;`.
pub fn attempt<T>(f: impl FnOnce() -> T) -> Result<T, Error> {
    let x = f();
    match take_error() {
        Some(e) => Err(e),
        None => Ok(x),
    }
}

// A JavaScript promise: `then` calls a closure with its result once it settles.
pub struct Promise<T = ()>(pub Handle, PhantomData<fn() -> T>);
pub type PromiseLike<T = ()> = Promise<T>;
impl<T> Clone for Promise<T> {
    fn clone(&self) -> Self {
        Promise(self.0.clone(), PhantomData)
    }
}
impl<T> ToJs for Promise<T> {
    fn lend(&self) -> Lent {
        Lent::borrow(self.0 .0)
    }
}
impl<T> FromJs for Promise<T> {
    fn from_handle(h: u32) -> Self {
        Promise(Handle(h), PhantomData)
    }
}
impl<T: JsType> Promise<T> {
    pub fn then<F: FnOnce(Result<T, Error>) + 'static>(&self, f: F) {
        extern "C" fn settle<T: JsType, F: FnOnce(Result<T, Error>) + 'static>(ctx: *mut c_void, ok: i32, v: u32) {
            let mut b = unsafe { Box::from_raw(ctx as *mut Box<dyn Any>) };
            let f = b.downcast_mut::<Option<F>>().unwrap().take().unwrap();
            f(if ok != 0 { Ok(T::from_handle(v)) } else { Err(Error(Handle(v))) })
        }
        unsafe { rt::js_promise_then(self.0 .0, settle::<T, F> as *const () as usize, closure(Some(f))) }
    }
}

// Walks a JavaScript iterator: `for n in list.iter()`. Each value is the loop's.
pub struct JsIterator<T>(Handle, PhantomData<fn() -> T>);
impl<T: JsType> JsIterator<T> {
    pub fn over(h: u32) -> Self {
        JsIterator(Handle(unsafe { rt::js_iterate(h) }), PhantomData)
    }
}
impl<T: JsType> Iterator for JsIterator<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        let v = unsafe { rt::js_iterator_next(self.0 .0) };
        if v == u32::MAX { None } else { Some(T::from_handle(v)) }
    }
}

// A JavaScript async iterator: `next()` is a promise of the next value, or of None at the end.
pub struct AsyncIterator<T>(Handle, PhantomData<fn() -> T>);
impl<T: JsType> AsyncIterator<T> {
    pub fn over(h: u32) -> Self {
        AsyncIterator(Handle(unsafe { rt::js_async_iterate(h) }), PhantomData)
    }
    pub fn next(&self) -> Promise<Option<T>> {
        Promise::from_handle(unsafe { rt::js_async_iterator_next(self.0 .0) })
    }
}
"#;

// Types the runtime declares, which the bindings do not again.
const RUNTIME_TYPES: &[&str] = &["Promise", "PromiseLike", "Error", "JsValue", "JsArray", "Key", "Listener", "Handle", "Lent", "Callback", "JsIterator", "AsyncIterator"];

// A union's variant name: `number`, `string`, `Element`, `array`, or a type parameter's.
fn variant(t: &Ty) -> String {
    match t {
        Ty::Num => "number".into(),
        Ty::Str => "string".into(),
        Ty::Bool => "boolean".into(),
        Ty::Obj(n) | Ty::Param(n) | Ty::Generic(n, _) | Ty::Enum(n) => n.clone(),
        Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => "array".into(),
        t => panic!("Rust generator: a union member it cannot hold: {t:?}"),
    }
}

// The bindings: `runtime.rs`, and `bindings.rs` with every type, trait and
// function, which includes the runtime.
pub fn files(m: &Model) -> Result<Vec<(String, String)>, Vec<String>> {
    DECODABLE.with(|d| *d.borrow_mut() = m.unions.iter().filter(|u| u.decodable()).map(|u| u.name.clone()).collect());
    UNIONS.with(|d| *d.borrow_mut() = m.unions.iter().map(|u| u.name.clone()).collect());
    CLASSES.with(|d| *d.borrow_mut() = m.classes.iter().filter(|c| c.type_params.is_empty()).map(|c| c.name.clone()).chain(m.opaque.iter().filter(|n| !m.opaque_arity.contains_key(*n)).cloned()).collect());
    let mut out = String::from("// Generated.\n#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code, unused_imports, unused_unsafe, unused_parens, unused_variables, clippy::all)]\n\n#[path = \"runtime.rs\"]\nmod runtime;\npub use runtime::*;\nuse runtime::handle;\nuse core::marker::PhantomData;\n\n");
    let mut imports: BTreeMap<String, String> = BTreeMap::new();

    for (name, values) in &m.enums {
        let mut seen = std::collections::HashSet::new();
        let variants: Vec<(String, String)> = values
            .iter()
            .map(|v| {
                let id = ident(if v.is_empty() { "empty" } else { v });
                let id = if seen.insert(id.clone()) { id } else { format!("{id}_{}", seen.len()) };
                (id, v.clone())
            })
            .collect();
        let index: String = variants.iter().enumerate().map(|(i, (v, _))| format!("{i} => {name}::{v}, ")).collect();
        let from: String = variants.iter().map(|(v, s)| format!("{s:?} => {name}::{v}, ")).collect();
        let to: String = variants.iter().map(|(v, s)| format!("{name}::{v} => {s:?}, ")).collect();
        out += &format!(
            "#[derive(Clone, Copy, PartialEq, Eq, Debug)]\n#[repr(i32)]\npub enum {name} {{ {} }}\n\
             impl {name} {{\n    pub fn from_index(i: i32) -> Self {{ match i {{ {index}_ => panic!(\"{name}: no value {{i}}\") }} }}\n    pub fn as_str(self) -> &'static str {{ match self {{ {to}}} }}\n}}\n\
             impl ToJs for {name} {{ fn lend(&self) -> Lent {{ self.as_str().lend() }} }}\n\
             impl FromJs for {name} {{ fn from_handle(h: u32) -> Self {{ let s = String::from_handle(h); match s.as_str() {{ {from}_ => panic!(\"{name}: no value {{s}}\") }} }} }}\n",
            variants.iter().map(|(v, _)| v.as_str()).collect::<Vec<_>>().join(", ")
        );
    }
    for (name, members) in &m.num_enums {
        let variants: Vec<String> = members.iter().map(|(member, value)| format!("{} = {value}", ident(member))).collect();
        let from: String = members.iter().map(|(member, value)| format!("{value} => {name}::{}, ", ident(member))).collect();
        out += &format!(
            "#[derive(Clone, Copy, PartialEq, Eq, Debug)]\n#[repr(i32)]\npub enum {name} {{ {} }}\n\
             impl {name} {{ pub fn from_value(v: i32) -> Self {{ match v {{ {from}_ => panic!(\"{name}: no value {{v}}\") }} }} }}\n\
             impl ToJs for {name} {{ fn lend(&self) -> Lent {{ (*self as i32 as f64).lend() }} }}\n\
             impl FromJs for {name} {{ fn from_handle(h: u32) -> Self {{ Self::from_value(f64::from_handle(h) as i32) }} }}\n",
            variants.join(", ")
        );
    }
    // Types declared elsewhere: handles, generic or not. Promises are the runtime's.
    for name in m.opaque.iter().filter(|n| !RUNTIME_TYPES.contains(&n.as_str())) {
        let arity = m.opaque_arity.get(name).copied().unwrap_or(0);
        let params: Vec<String> = (1..=arity).map(|i| format!("T{i}")).collect();
        out += &format!("handle!({name}{});\n", if params.is_empty() { String::new() } else { format!("<{}>", params.join(", ")) });
        if params.is_empty() {
            out += &format!("impl Is<{name}> for {name} {{}}\n");
        }
    }
    // Unions that can be decoded: enums.
    for u in m.unions.iter().filter(|u| u.decodable()) {
        out += &union_enum(u, &mut imports);
    }
    // A map's keys, each typed by the value it selects.
    for k in &m.maps {
        let keys: Vec<String> = k.keys.iter().zip(&k.values).enumerate().map(|(i, (key, v))| format!("    pub const {}: Key<super::{}, {}> = Key::new({i});\n", ident(key), k.name, ty(v))).collect();
        out += &format!("pub mod {}Keys {{\n    use super::*;\n{}}}\n", k.name, keys.concat());
    }
    for c in &m.callbacks {
        out += &callback(c.name.as_str(), &c.type_params, &c.params, &c.ret, &mut imports);
    }

    // Each class: a struct, a trait of its members, and that trait for each subtype.
    let class_of = |n: &str| m.classes.iter().find(|k| k.name == n);
    let ancestors = |name: &str| -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut todo = vec![name.to_string()];
        while let Some(c) = todo.pop() {
            for a in m.classes.iter().filter(|k| k.name == c).flat_map(|k| &k.ancestors) {
                if !out.contains(a) {
                    out.push(a.clone());
                    todo.insert(0, a.clone());
                }
            }
        }
        out
    };
    let member_names = |c: &Class| -> BTreeSet<String> { c.funcs.iter().filter(|f| Class::has_self(f)).map(|f| f.name.clone()).collect() };
    for c in &m.classes {
        let params: Vec<String> = c.type_params.iter().map(|p| p.name.clone()).collect();
        let (bounds, args) = (generics(&c.type_params), args_of(&c.type_params));
        let me = format!("{}{args}", c.name);
        out += &format!("\nhandle!({}{});\n", c.name, if params.is_empty() { String::new() } else { format!("<{}>", params.join(", ")) });
        if params.is_empty() {
            out += &format!("impl Is<{0}> for {0} {{}}\n", c.name);
        }
        let _ = phantom(&params);
        // The trait of its own members.
        let mut trait_body = String::new();
        for f in c.funcs.iter().filter(|f| Class::has_self(f)) {
            let f = with_self_args(f, c);
            let (sig, body) = function(&f, &mut imports, Some("AsRef::<Handle>::as_ref(self).0"));
            trait_body += &format!("    {sig} {body}\n");
        }
        if let Some(elem) = &c.iterates {
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create, params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone() }, c).ret;
            trait_body += &format!("    fn iter(&self) -> JsIterator<{}> {{ JsIterator::over(AsRef::<Handle>::as_ref(self).0) }}\n", ty(&elem));
        }
        if let Some(elem) = &c.iterates_async {
            trait_body += &format!("    fn iterate_async(&self) -> AsyncIterator<{}> {{ AsyncIterator::over(AsRef::<Handle>::as_ref(self).0) }}\n", ty(elem));
        }
        out += &format!("pub trait {}Methods{bounds}: AsRef<Handle> {{\n{trait_body}}}\n", c.name);
        out += &format!("impl{bounds} {}Methods{args} for {me} {{}}\n", c.name);
        // Statics, constants, and where two traits share a name, the one this type means.
        let mut inherent = String::new();
        for f in c.funcs.iter().filter(|f| !Class::has_self(f)) {
            let f = with_self_args(f, c);
            let (sig, body) = function(&f, &mut imports, None);
            inherent += &format!("    pub {sig} {body}\n");
        }
        for (name, value) in &c.consts {
            let t = if value.starts_with('"') { "&'static str" } else if value == "true" || value == "false" { "bool" } else { "f64" };
            // A whole number, decimal or hexadecimal, as an f64.
            let value = if t == "f64" && (value.starts_with("0x") || value.starts_with("0X")) { format!("{value}i64 as f64") } else if t == "f64" && !value.contains('.') && !value.contains('e') { format!("{value}.0") } else { value.clone() };
            inherent += &format!("    pub const {}: {t} = {value};\n", ident(name));
        }
        let all = ancestors(&c.name);
        let mut holders: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for h in std::iter::once(c.name.clone()).chain(all.iter().cloned()) {
            if let Some(k) = class_of(&h) {
                for n in member_names(k) {
                    holders.entry(n).or_default().push(h.clone());
                }
            }
        }
        for (name, hs) in holders.iter().filter(|(_, hs)| hs.len() > 1) {
            // The nearest holder: the type itself, then its nearest ancestor.
            let h = &hs[0];
            let k = class_of(h).unwrap();
            let args = if *h == c.name { k.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect() } else { base_args(m, c, h).unwrap_or_default() };
            for f in k.funcs.iter().filter(|f| Class::has_self(f) && f.name == *name) {
                let f = with_self_args(f, k);
                let f = map_types(&f, &|t| subst(t, &k.type_params, &args));
                let (sig, body) = function(&f, &mut imports, Some("AsRef::<Handle>::as_ref(self).0"));
                inherent += &format!("    pub {sig} {body}\n");
            }
        }
        // A subclass of Array, as the array it is.
        if let Some(elem) = &c.elements {
            inherent += &format!("    pub fn as_array(&self) -> runtime::JsArray<{}> {{ runtime::JsArray::from_handle(give(self)) }}\n", ty(elem));
        }
        if !inherent.is_empty() {
            out += &format!("impl{bounds} {me} {{\n{inherent}}}\n");
        }
        // The traits of its ancestors, and conversions to them.
        for a in &all {
            let Some(k) = class_of(a) else {
                if params.is_empty() && !m.opaque_arity.contains_key(a) {
                    out += &upcast(&me, a);
                }
                continue;
            };
            let base: Vec<Ty> = base_args(m, c, a).unwrap_or_default();
            let base_args = if k.type_params.is_empty() { String::new() } else { format!("<{}>", (0..k.type_params.len()).map(|i| base.get(i).map(ty).unwrap_or_else(|| "JsValue".into())).collect::<Vec<_>>().join(", ")) };
            out += &format!("impl{bounds} {a}Methods{base_args} for {me} {{}}\n");
            if params.is_empty() && k.type_params.is_empty() {
                out += &upcast(&me, a);
            }
        }
    }
    for f in &m.funcs {
        let (sig, body) = function(f, &mut imports, None);
        out += &format!("pub {sig} {body}\n");
    }
    out += "\nmod raw {\n    use core::ffi::c_void;\n    #[link(wasm_import_module = \"env\")]\n    unsafe extern \"C\" {\n";
    out += &imports.into_values().collect::<String>();
    out += "    }\n}\n";
    Ok(vec![("runtime.rs".into(), RUNTIME.into()), ("bindings.rs".into(), out)])
}

// A subtype as its ancestor: by value with `into()`, by reference with
// `as_ref()`. Both are one handle, transparently.
fn upcast(sub: &str, base: &str) -> String {
    format!(
        "impl From<{sub}> for {base} {{ fn from(x: {sub}) -> Self {{ {base}(x.0) }} }}\n\
         impl AsRef<{base}> for {sub} {{ fn as_ref(&self) -> &{base} {{ unsafe {{ &*(self as *const {sub} as *const {base}) }} }} }}\n\
         impl Is<{base}> for {sub} {{}}\n"
    )
}

// A union as an enum, decoded from a handle by its tag, lent as its member.
fn union_enum(u: &NamedUnion, imports: &mut BTreeMap<String, String>) -> String {
    let n = &u.name;
    let (bounds, args) = (generics(&u.type_params), args_of(&u.type_params));
    let variants: Vec<String> = u.parts.iter().map(|p| format!("{}({})", variant(p), ty(p))).collect();
    // A bare type parameter is told apart by what its argument is.
    let kind = u.parts.iter().find_map(|p| if let Ty::Param(t) = p { Some(format!("{t}::KIND")) } else { None }).unwrap_or("0".into());
    let decode: String = u.parts.iter().enumerate().map(|(i, p)| format!("{i} => {n}::{}(<{}>::from_handle(h)), ", variant(p), ty(p))).collect();
    let lend: String = u.parts.iter().map(|p| format!("{n}::{}(x) => x.lend(), ", variant(p))).collect();
    imports.insert(format!("{n}_tag"), format!("        pub fn {n}_tag(h: u32, k: i32) -> i32;\n"));
    // A union is what each of its members is.
    let is_bounds: Vec<String> = u.parts.iter().map(|p| format!("{}: Is<B_>", ty(p))).collect();
    let is_params = std::iter::once("B_".to_string()).chain(u.type_params.iter().map(bound)).collect::<Vec<_>>().join(", ");
    let is = format!("impl<{is_params}> Is<B_> for {n}{args} where {} {{}}\n", is_bounds.join(", "));
    is + &format!(
        "pub enum {n}{bounds} {{ {} }}\n\
         impl{bounds} Clone for {n}{args} {{ fn clone(&self) -> Self {{ match self {{ {} }} }} }}\n\
         impl{bounds} ToJs for {n}{args} {{ fn lend(&self) -> Lent {{ match self {{ {lend}}} }} }}\n\
         impl{bounds} FromJs for {n}{args} {{ fn from_handle(h: u32) -> Self {{ match unsafe {{ raw::{n}_tag(h, {kind}) }} {{ {decode}_ => unreachable!() }} }} }}\n",
        variants.join(", "),
        u.parts.iter().map(|p| format!("{n}::{0}(x) => {n}::{0}(Clone::clone(x)), ", variant(p))).collect::<String>()
    )
}

// A callback type, made from a closure: `Counter_visit_f::new(|c: Counter| ..)`.
fn callback(name: &str, type_params: &[TParam], params: &[(String, Ty)], ret: &Ty, imports: &mut BTreeMap<String, String>) -> String {
    let (bounds, args) = (generics(type_params), args_of(type_params));
    let tps: Vec<String> = type_params.iter().map(|p| p.name.clone()).collect();
    let arg_types: Vec<String> = params.iter().map(|(_, t)| ty(t)).collect();
    let fn_bound = format!("FnMut({}) -> {}", arg_types.join(", "), ty(ret));
    let raw_args: String = params.iter().enumerate().map(|(i, (_, t))| format!(", a{i}: {}", raw_callback(t))).collect();
    let call_args: Vec<String> = params.iter().enumerate().map(|(i, (_, t))| raise(t, &format!("a{i}"))).collect();
    let call = format!("(unsafe {{ closure_fn::<F>(ctx) }})({})", call_args.join(", "));
    let body = lower_callback(ret, &call);
    let mut tramp_params = tps.iter().map(|p| format!("{p}: JsType")).collect::<Vec<_>>();
    tramp_params.push(format!("F: {fn_bound} + 'static"));
    let tramp_args: Vec<String> = tps.iter().cloned().chain(std::iter::once("F".to_string())).collect();
    imports.insert(format!("{name}_from"), format!("        pub fn {name}_from(f: usize, ctx: *mut c_void) -> u32;\n"));
    let handle_params = if tps.is_empty() { String::new() } else { format!("<{}>", tps.join(", ")) };
    format!(
        "handle!({name}{handle_params});\nimpl{bounds} Callback for {name}{args} {{}}\n\
         impl{bounds} {name}{args} {{\n    pub fn new<F: {fn_bound} + 'static>(f: F) -> Self {{\n        \
         extern \"C\" fn call<{}>(ctx: *mut core::ffi::c_void{raw_args}) -> {} {{ {body} }}\n        \
         Self::from_handle(unsafe {{ raw::{name}_from(call::<{}> as *const () as usize, closure(f)) }})\n    }}\n}}\n",
        tramp_params.join(", "),
        raw_callback(ret),
        tramp_args.join(", ")
    )
}

