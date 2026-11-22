// Generated. The runtime: handles, conversions, arrays, callbacks, promises.
use core::ffi::c_void;
use core::marker::PhantomData;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

#[allow(dead_code)]
pub(crate) mod rt {
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
        pub fn js_object_new() -> u32;
        pub fn js_object_get(h: u32, p: *const u8, n: usize) -> u32;
        pub fn js_object_set(h: u32, p: *const u8, n: usize, v: u32);
        pub fn js_object_has(h: u32, p: *const u8, n: usize) -> i32;
        pub fn js_listener_from(f: usize, ctx: *mut c_void) -> u32;
        pub fn js_callback_release(h: u32) -> u32;
        pub fn js_retain(h: u32);
        pub fn js_release(h: u32);
        pub fn js_live() -> u32;
        pub fn js_attempt_enter();
        pub fn js_attempt_leave();
        pub fn js_take_error() -> u32;
        pub fn js_error_message(h: u32) -> u32;
        pub fn js_error_name(h: u32) -> u32;
        pub fn js_error_stack(h: u32) -> u32;
        pub fn js_error_new(p: *const u8, n: usize) -> u32;
        pub fn js_date_new(ms: f64) -> u32;
        pub fn js_date_now() -> f64;
        pub fn js_date_time(h: u32) -> f64;
        pub fn js_date_to_iso_string(h: u32) -> u32;
        pub fn js_bytes_len(h: u32) -> u32;
        pub fn js_bytes_read(h: u32, off: usize, dst: *mut u8, len: usize) -> u32;
        pub fn js_bytes_write(h: u32, off: usize, src: *const u8, len: usize) -> u32;
        pub fn js_bytes_new(kind: u32, src: *const u8, byte_len: usize) -> u32;
        pub fn js_bytes_buffer(h: u32) -> u32;
        pub fn js_bytes_offset(h: u32) -> u32;
        pub fn js_promise_then(p: u32, f: usize, ctx: *mut c_void);
        pub fn js_iterate(h: u32) -> u32;
        pub fn js_iterator_next(it: u32) -> u32;
        pub fn js_async_iterate(h: u32) -> u32;
        pub fn js_async_iterator_next(it: u32) -> u32;
    }
}

// A JavaScript value, counted: `Clone` retains it, `Drop` releases it. Zero is null.
#[repr(transparent)]
pub struct JsHandle(pub u32);
impl Clone for JsHandle {
    fn clone(&self) -> Self {
        if self.0 != 0 {
            unsafe { rt::js_retain(self.0) }
        }
        JsHandle(self.0)
    }
}
impl Drop for JsHandle {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe { rt::js_release(self.0) }
        }
    }
}
impl core::fmt::Debug for JsHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "JsHandle({})", self.0)
    }
}

// Sentinel passed to optional union parameters to omit the argument (`None`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JsNone;

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
    pub fn into_raw(self) -> (u32, bool) {
        let h = self.h;
        let own = self.own;
        core::mem::forget(self);
        (h, own)
    }
    pub fn from_raw(h: u32, own: bool) -> Self {
        Lent { h, own }
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

// What an optional union argument may be: any member, as the union is made
// from it, or an Option of the union (each union implements it for that).
pub trait Optional<U> {
    fn into_option(self) -> Option<U>;
}
impl<U, T: Into<U>> Optional<U> for T {
    fn into_option(self) -> Option<U> {
        Some(self.into())
    }
}

// A nullable argument: the value by reference (`&opts`, `&text`), `Some` of
// it, or `None`.
pub trait Nullable<T: ?Sized> {
    fn arg(&self) -> Option<&T>;
}
impl<T: ?Sized, U: AsRef<T> + ?Sized> Nullable<T> for &U {
    fn arg(&self) -> Option<&T> {
        Some((**self).as_ref())
    }
}
impl<T: ?Sized> Nullable<T> for Option<&T> {
    fn arg(&self) -> Option<&T> {
        *self
    }
}

// `T: Is<B>` says a T is a B: itself, a subtype of it, a union of such, or
// any value. It is what a type parameter's `extends` becomes.
pub trait Is<B> {}
impl<B> Is<B> for JsValue {}

// A handle JavaScript keeps: what a callback returns.
#[doc(hidden)]
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

impl<T: ToJs + ?Sized> ToJs for &T {
    fn lend(&self) -> Lent {
        (**self).lend()
    }
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

// Declares a handle type, generic or not. `==` is whether two handles are
// one JavaScript value.
macro_rules! handle {
    (@common [$($meta:tt)*] $name:ident $(<$($p:ident $(= $d:ty)?),*>)?) => {
        $($meta)*
        #[repr(transparent)]
        pub struct $name $(<$($p $(= $d)?),*>)? (pub $crate::runtime::JsHandle $(, pub core::marker::PhantomData<fn() -> ($($p,)*)>)?);
        impl $(<$($p),*>)? AsRef<$name $(<$($p),*>)?> for $name $(<$($p),*>)? {
            fn as_ref(&self) -> &Self {
                self
            }
        }
        impl $(<$($p),*>)? Clone for $name $(<$($p),*>)? {
            fn clone(&self) -> Self {
                $name(self.0.clone() $(, core::marker::PhantomData::<fn() -> ($($p,)*)>)?)
            }
        }
        impl $(<$($p),*>)? AsRef<$crate::runtime::JsHandle> for $name $(<$($p),*>)? {
            fn as_ref(&self) -> &$crate::runtime::JsHandle {
                &self.0
            }
        }
        impl $(<$($p),*>)? PartialEq for $name $(<$($p),*>)? {
            fn eq(&self, o: &Self) -> bool {
                $crate::runtime::js::same(&self.0, &o.0)
            }
        }
        impl $(<$($p),*>)? $crate::runtime::ToJs for $name $(<$($p),*>)? {
            fn lend(&self) -> $crate::runtime::Lent {
                $crate::runtime::Lent::borrow(self.0 .0)
            }
        }
        impl $(<$($p),*>)? $crate::runtime::FromJs for $name $(<$($p),*>)? {
            fn from_handle(h: u32) -> Self {
                $name($crate::runtime::JsHandle(h) $(, core::marker::PhantomData::<fn() -> ($($p,)*)>)?)
            }
        }
    };
    (@jsvalue $name:ident) => {
        handle!(@common [] $name);
        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}(#{})", stringify!($name), self.0 .0)
            }
        }
    };
    (@custom_debug $name:ident) => {
        handle!(@common [] $name);
        impl From<$name> for $crate::runtime::JsValue {
            fn from(x: $name) -> Self {
                $crate::runtime::JsValue(x.0)
            }
        }
    };
    ($(#[$meta:meta])* $name:ident $(<$($p:ident $(= $d:ty)?),*>)?) => {
        handle!(@common [$(#[$meta])*] $name $(<$($p $(= $d)?),*>)?);
        impl $(<$($p),*>)? From<$name $(<$($p),*>)?> for $crate::runtime::JsValue {
            fn from(x: $name $(<$($p),*>)?) -> Self {
                $crate::runtime::JsValue(x.0)
            }
        }
        impl $(<$($p),*>)? core::fmt::Debug for $name $(<$($p),*>)? {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}(#{})", stringify!($name), self.0 .0)
            }
        }
    };
}
pub(crate) use handle;

// A JavaScript value of no particular type, made from a number, a flag, a
// string, or any object: `f(42.0)`, `f("text")`, `f(&element)`.
handle!(@jsvalue JsValue);
impl JsValue {
    pub fn object() -> Self {
        JsValue(JsHandle(unsafe { rt::js_object_new() }))
    }
    pub fn get(&self, key: &str) -> JsValue {
        JsValue(JsHandle(unsafe { rt::js_object_get(self.0 .0, key.as_ptr(), key.len()) }))
    }
    pub fn set(&self, key: &str, v: impl Into<JsValue>) {
        let v: JsValue = v.into();
        unsafe { rt::js_object_set(self.0 .0, key.as_ptr(), key.len(), v.0 .0) };
    }
    pub fn with(self, key: &str, v: impl Into<JsValue>) -> Self {
        self.set(key, v);
        self
    }
    pub fn unchecked_into<T: FromJs>(self) -> T {
        let id = self.0 .0;
        core::mem::forget(self);
        T::from_handle(id)
    }
    pub fn unchecked_as<T: FromJs>(&self) -> T {
        js::unchecked_cast(self)
    }
    pub fn has(&self, key: &str) -> bool {
        unsafe { rt::js_object_has(self.0 .0, key.as_ptr(), key.len()) != 0 }
    }
    pub fn as_array<T>(&self) -> JsArray<T> {
        JsArray(self.0.clone(), PhantomData)
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
    // `null`, for an argument of any type given nothing.
    pub fn null() -> Self {
        JsValue(JsHandle(0))
    }
}
impl From<f64> for JsValue {
    fn from(x: f64) -> Self {
        JsValue(JsHandle(unsafe { rt::js_from_number(x) }))
    }
}
impl From<bool> for JsValue {
    fn from(x: bool) -> Self {
        JsValue(JsHandle(unsafe { rt::js_from_bool(x as i32) }))
    }
}
impl From<&str> for JsValue {
    fn from(s: &str) -> Self {
        JsValue(JsHandle(unsafe { rt::js_from_string(s.as_ptr(), s.len()) }))
    }
}
impl From<String> for JsValue {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}
// `Some(&v)` is `v`, and `None` is null.
impl From<Option<&JsValue>> for JsValue {
    fn from(x: Option<&JsValue>) -> Self {
        x.cloned().unwrap_or_else(JsValue::null)
    }
}
impl From<&String> for JsValue {
    fn from(s: &String) -> Self {
        s.as_str().into()
    }
}
impl<T: AsRef<JsHandle>> From<&T> for JsValue {
    fn from(x: &T) -> Self {
        JsValue(x.as_ref().clone())
    }
}

// A JavaScript array of T; `JsArray` of any values. Made from an array, a
// slice, a Vec or an iterator of what T is made from: `["a", "b"]`.
pub struct JsArray<T = JsValue>(pub JsHandle, PhantomData<fn() -> T>);
impl<T> Clone for JsArray<T> {
    fn clone(&self) -> Self {
        JsArray(self.0.clone(), PhantomData)
    }
}
impl<T> AsRef<JsHandle> for JsArray<T> {
    fn as_ref(&self) -> &JsHandle {
        &self.0
    }
}
impl<T> AsRef<JsArray<T>> for JsArray<T> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<T> PartialEq for JsArray<T> {
    fn eq(&self, o: &Self) -> bool {
        js::same(&self.0, &o.0)
    }
}
impl<T> core::fmt::Debug for JsArray<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "JsArray(#{})", self.0 .0)
    }
}
impl<T> From<JsArray<T>> for JsValue {
    fn from(a: JsArray<T>) -> Self {
        JsValue(a.0)
    }
}
impl<T> ToJs for JsArray<T> {
    fn lend(&self) -> Lent {
        Lent::borrow(self.0 .0)
    }
}
impl<T> FromJs for JsArray<T> {
    fn from_handle(h: u32) -> Self {
        JsArray(JsHandle(h), PhantomData)
    }
}
impl<T: JsType> JsArray<T> {
    pub fn new() -> Self {
        JsArray(JsHandle(unsafe { rt::js_array_new() }), PhantomData)
    }
    pub fn len(&self) -> usize {
        unsafe { rt::js_array_length(self.0 .0) as usize }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn get(&self, i: usize) -> T {
        T::from_handle(unsafe { rt::js_array_get(self.0 .0, i as u32) })
    }
    pub fn try_get(&self, i: usize) -> Option<T> {
        if i < self.len() { Some(self.get(i)) } else { None }
    }
    pub fn push(&self, v: impl Into<T>) {
        let v: T = v.into();
        unsafe { rt::js_array_push(self.0 .0, v.lend().h) }
    }
    // Each element in turn.
    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        (0..self.len()).map(|i| self.get(i))
    }
    // The same array, untyped.
    pub fn any(&self) -> JsArray {
        JsArray(self.0.clone(), PhantomData)
    }
}
impl<T: JsType> Default for JsArray<T> {
    fn default() -> Self {
        Self::new()
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
impl<T: JsType, A: Into<T>, const N: usize> From<[A; N]> for JsArray<T> {
    fn from(items: [A; N]) -> Self {
        items.into_iter().collect()
    }
}
impl<T: JsType, A: Into<T>> From<Vec<A>> for JsArray<T> {
    fn from(items: Vec<A>) -> Self {
        items.into_iter().collect()
    }
}
impl<T: JsType, A: Into<T> + Clone> From<&[A]> for JsArray<T> {
    fn from(items: &[A]) -> Self {
        items.iter().cloned().collect()
    }
}
impl<T> From<&JsArray<T>> for JsArray<T> {
    fn from(a: &JsArray<T>) -> Self {
        a.clone()
    }
}

// Arrays of fixed length and element types: `[number, string]` is `JsTuple2<f64, String>`.
macro_rules! tuple {
    ($name:ident; $($t:ident $get:ident $i:literal),*) => {
        pub struct $name<$($t),*>(pub JsHandle, PhantomData<fn() -> ($($t,)*)>);
        impl<$($t),*> Clone for $name<$($t),*> {
            fn clone(&self) -> Self {
                $name(self.0.clone(), PhantomData)
            }
        }
        impl<$($t),*> AsRef<JsHandle> for $name<$($t),*> {
            fn as_ref(&self) -> &JsHandle {
                &self.0
            }
        }
        impl<$($t),*> AsRef<$name<$($t),*>> for $name<$($t),*> {
            fn as_ref(&self) -> &Self {
                self
            }
        }
        impl<$($t),*> PartialEq for $name<$($t),*> {
            fn eq(&self, o: &Self) -> bool {
                js::same(&self.0, &o.0)
            }
        }
        impl<$($t),*> core::fmt::Debug for $name<$($t),*> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}(#{})", stringify!($name), self.0 .0)
            }
        }
        impl<$($t),*> From<&$name<$($t),*>> for $name<$($t),*> {
            fn from(t: &$name<$($t),*>) -> Self {
                t.clone()
            }
        }
        impl<$($t),*> From<$name<$($t),*>> for JsValue {
            fn from(t: $name<$($t),*>) -> Self {
                JsValue(t.0)
            }
        }
        impl<$($t),*> ToJs for $name<$($t),*> {
            fn lend(&self) -> Lent {
                Lent::borrow(self.0 .0)
            }
        }
        impl<$($t),*> FromJs for $name<$($t),*> {
            fn from_handle(h: u32) -> Self {
                $name(JsHandle(h), PhantomData)
            }
        }
        impl<$($t: JsType),*> $name<$($t),*> {
            pub fn new($($get: impl Into<$t>),*) -> Self {
                let a = unsafe { rt::js_array_new() };
                $(unsafe { rt::js_array_push(a, Into::<$t>::into($get).lend().h) };)*
                $name(JsHandle(a), PhantomData)
            }
            $(pub fn $get(&self) -> $t {
                $t::from_handle(unsafe { rt::js_array_get(self.0 .0, $i) })
            })*
        }
    };
}
tuple!(JsTuple2; A get0 0, B get1 1);
tuple!(JsTuple3; A get0 0, B get1 1, C get2 2);
tuple!(JsTuple4; A get0 0, B get1 1, C get2 2, D get3 3);
impl<A: JsType, B: JsType, A0: Into<A>, B0: Into<B>> From<(A0, B0)> for JsTuple2<A, B> {
    fn from((a, b): (A0, B0)) -> Self {
        JsTuple2::new(a, b)
    }
}
impl<A: JsType, B: JsType, C: JsType, A0: Into<A>, B0: Into<B>, C0: Into<C>> From<(A0, B0, C0)> for JsTuple3<A, B, C> {
    fn from((a, b, c): (A0, B0, C0)) -> Self {
        JsTuple3::new(a, b, c)
    }
}
impl<A: JsType, B: JsType, C: JsType, D: JsType, A0: Into<A>, B0: Into<B>, C0: Into<C>, D0: Into<D>> From<(A0, B0, C0, D0)> for JsTuple4<A, B, C, D> {
    fn from((a, b, c, d): (A0, B0, C0, D0)) -> Self {
        JsTuple4::new(a, b, c, d)
    }
}
impl<T: JsType, A: Into<T>> From<[A; 2]> for JsTuple2<T, T> {
    fn from([a, b]: [A; 2]) -> Self {
        JsTuple2::new(a, b)
    }
}
impl<T: JsType, A: Into<T>> From<[A; 3]> for JsTuple3<T, T, T> {
    fn from([a, b, c]: [A; 3]) -> Self {
        JsTuple3::new(a, b, c)
    }
}
impl<T: JsType, A: Into<T>> From<[A; 4]> for JsTuple4<T, T, T, T> {
    fn from([a, b, c, d]: [A; 4]) -> Self {
        JsTuple4::new(a, b, c, d)
    }
}

// A key of a map whose value has type V: `HTMLElementEventMap::CLICK`.
pub struct JsKey<M, V> {
    pub index: i32,
    _p: PhantomData<fn() -> (M, V)>,
}
impl<M, V> JsKey<M, V> {
    pub const fn new(index: i32) -> Self {
        JsKey { index, _p: PhantomData }
    }
}
impl<M, V> Clone for JsKey<M, V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<M, V> Copy for JsKey<M, V> {}

// A closure kept for JavaScript to call, until released.
#[repr(C)]
struct ClosureHeader {
    drop_fn: unsafe fn(*mut c_void),
}
#[repr(C)]
struct Closure<F> {
    drop_fn: unsafe fn(*mut c_void),
    f: core::cell::RefCell<F>,
}
pub(crate) fn closure<F: 'static>(f: F) -> *mut c_void {
    unsafe fn drop_closure<F>(p: *mut c_void) {
        drop(unsafe { Box::from_raw(p as *mut Closure<F>) });
    }
    Box::into_raw(Box::new(Closure { drop_fn: drop_closure::<F>, f: core::cell::RefCell::new(f) })) as *mut c_void
}
pub(crate) unsafe fn closure_fn<'a, F: 'static>(ctx: *mut c_void) -> core::cell::RefMut<'a, F> {
    unsafe { (*(ctx as *const Closure<F>)).f.borrow_mut() }
}

// Forgets a callback when dropped: JavaScript calling it afterwards is an
// error there, never a call into a closure that is gone.
pub(crate) struct Forget(pub u32);
impl Drop for Forget {
    fn drop(&mut self) {
        unsafe { rt::js_callback_release(self.0) };
    }
}

// A JavaScript function made from a closure. One made with `new` lives until
// `release`, which frees the closure; one made with `scoped` lives for the
// length of a call, and its closure may borrow.
pub trait JsCallback: AsRef<JsHandle> + Sized {
    fn release(self) {
        let ctx = unsafe { rt::js_callback_release(AsRef::<JsHandle>::as_ref(&self).0) };
        if ctx != 0 {
            let p = ctx as usize as *mut c_void;
            unsafe { ((*(p as *const ClosureHeader)).drop_fn)(p) };
        }
    }
}

// A callback taking one object of type E: `JsListener::new(|e: PointerEvent| ..)`.
pub struct JsListener<E>(pub JsHandle, PhantomData<fn() -> E>);
impl<E> Clone for JsListener<E> {
    fn clone(&self) -> Self {
        JsListener(self.0.clone(), PhantomData)
    }
}
impl<E> AsRef<JsHandle> for JsListener<E> {
    fn as_ref(&self) -> &JsHandle {
        &self.0
    }
}
impl<E> AsRef<JsListener<E>> for JsListener<E> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<E> From<JsListener<E>> for JsValue {
    fn from(x: JsListener<E>) -> Self {
        JsValue(x.0)
    }
}
impl<E> core::fmt::Debug for JsListener<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "JsListener(#{})", self.0 .0)
    }
}
impl<E> ToJs for JsListener<E> {
    fn lend(&self) -> Lent {
        Lent::borrow(self.0 .0)
    }
}
impl<E> FromJs for JsListener<E> {
    fn from_handle(h: u32) -> Self {
        JsListener(JsHandle(h), PhantomData)
    }
}
impl<E> JsCallback for JsListener<E> {}
impl<E: JsType> JsListener<E> {
    pub fn new<F: FnMut(E) + 'static>(f: F) -> Self {
        extern "C" fn call<E: JsType, F: FnMut(E) + 'static>(ctx: *mut c_void, ev: u32) {
            (&mut *unsafe { closure_fn::<F>(ctx) })(E::from_handle(ev))
        }
        JsListener(JsHandle(unsafe { rt::js_listener_from(call::<E, F> as *const () as usize, closure(f)) }), PhantomData)
    }
    // A listener for the length of `body`: its closure may borrow.
    pub fn scoped<F: FnMut(E), R>(f: F, body: impl FnOnce(&Self) -> R) -> R {
        extern "C" fn call<E: JsType, F: FnMut(E)>(ctx: *mut c_void, ev: u32) {
            (&mut *unsafe { &*(ctx as *const core::cell::RefCell<F>) }.borrow_mut())(E::from_handle(ev))
        }
        let f = core::cell::RefCell::new(f);
        let l = JsListener(JsHandle(unsafe { rt::js_listener_from(call::<E, F> as *const () as usize, &f as *const core::cell::RefCell<F> as *mut c_void) }), PhantomData);
        let _forget = Forget(l.0 .0);
        body(&l)
    }
}

// A JavaScript exception, or the reason a promise was rejected.
handle!(@custom_debug JsError);
impl JsError {
    pub fn new(msg: &str) -> Self {
        JsError(JsHandle(unsafe { rt::js_error_new(msg.as_ptr(), msg.len()) }))
    }
    pub fn name(&self) -> String {
        String::from_handle(unsafe { rt::js_error_name(self.0 .0) })
    }
    pub fn message(&self) -> String {
        String::from_handle(unsafe { rt::js_error_message(self.0 .0) })
    }
    pub fn stack(&self) -> String {
        String::from_handle(unsafe { rt::js_error_stack(self.0 .0) })
    }
}
impl core::fmt::Debug for JsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "JsError({}: {:?})", self.name(), self.message())
    }
}
impl core::fmt::Display for JsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.name(), self.message())
    }
}
impl core::error::Error for JsError {}

// What is not a type's: errors, handles, tasks.
pub mod js {
    use super::*;

    pub const NONE: JsNone = JsNone;

    pub fn unchecked_cast<T: FromJs>(v: &impl AsRef<JsHandle>) -> T {
        let h = v.as_ref().clone();
        let id = h.0;
        core::mem::forget(h);
        T::from_handle(id)
    }

    pub fn date_now() -> f64 {
        unsafe { rt::js_date_now() }
    }

    // The exception a call threw, taken. A call that throws returns a zero
    // value, and an exception not taken by the time control returns to
    // JavaScript is reported there: see `attempt`.
    pub fn take_error() -> Option<JsError> {
        let e = unsafe { rt::js_take_error() };
        if e == 0 { None } else { Some(JsError(JsHandle(e))) }
    }

    // Runs f and gives what it returned, or the exception a call in it threw:
    // `let found = js::attempt(|| el.query_selector(text))?;`.
    pub fn attempt<T>(f: impl FnOnce() -> T) -> Result<T, JsError> {
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                unsafe { rt::js_attempt_leave() };
            }
        }
        unsafe { rt::js_attempt_enter() };
        let _guard = Guard;
        let x = f();
        match take_error() {
            Some(e) => Err(e),
            None => Ok(x),
        }
    }

    // How many references to JavaScript values wasm holds.
    pub fn live_handles() -> u32 {
        unsafe { rt::js_live() }
    }

    // Whether two handles are one JavaScript value: what `==` means for every handle type.
    pub fn same(a: &JsHandle, b: &JsHandle) -> bool {
        a.0 == b.0 || unsafe { rt::js_equals(a.0, b.0) != 0 }
    }

    // Runs a future to its end, across the promises it awaits:
    // `js::spawn(async move { let r = fetch(url, None).await; .. })`.
    pub fn spawn(f: impl Future<Output = ()> + 'static) {
        poll(&Rc::new(Task { future: RefCell::new(Some(Box::pin(f))) }));
    }

    struct Task {
        future: RefCell<Option<Pin<Box<dyn Future<Output = ()>>>>>,
    }

    fn poll(task: &Rc<Task>) {
        let waker = waker(task.clone());
        // A wake during its own poll would find the future borrowed: a promise never does that.
        let Ok(mut slot) = task.future.try_borrow_mut() else { return };
        if let Some(f) = slot.as_mut() {
            if f.as_mut().poll(&mut Context::from_waker(&waker)).is_ready() {
                *slot = None;
            }
        }
    }

    // One thread, so a waker is a counted pointer to its task.
    fn waker(task: Rc<Task>) -> Waker {
        unsafe fn clone(p: *const ()) -> RawWaker {
            unsafe { Rc::increment_strong_count(p as *const Task) };
            RawWaker::new(p, &VTABLE)
        }
        unsafe fn wake(p: *const ()) {
            poll(&unsafe { Rc::from_raw(p as *const Task) });
        }
        unsafe fn wake_by_ref(p: *const ()) {
            let task = unsafe { Rc::from_raw(p as *const Task) };
            poll(&task);
            core::mem::forget(task);
        }
        unsafe fn forget(p: *const ()) {
            drop(unsafe { Rc::from_raw(p as *const Task) });
        }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, forget);
        unsafe { Waker::from_raw(RawWaker::new(Rc::into_raw(task) as *const (), &VTABLE)) }
    }
}

// A JavaScript promise. `then` calls a closure with its result once it
// settles; in an async block, `.await` waits for it.
pub struct JsPromise<T = ()>(pub JsHandle, PhantomData<fn() -> T>);
impl<T> Clone for JsPromise<T> {
    fn clone(&self) -> Self {
        JsPromise(self.0.clone(), PhantomData)
    }
}
impl<T> AsRef<JsHandle> for JsPromise<T> {
    fn as_ref(&self) -> &JsHandle {
        &self.0
    }
}
impl<T> AsRef<JsPromise<T>> for JsPromise<T> {
    fn as_ref(&self) -> &Self {
        self
    }
}
impl<T> From<JsPromise<T>> for JsValue {
    fn from(p: JsPromise<T>) -> Self {
        JsValue(p.0)
    }
}
impl<T> core::fmt::Debug for JsPromise<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "JsPromise(#{})", self.0 .0)
    }
}
impl<T> ToJs for JsPromise<T> {
    fn lend(&self) -> Lent {
        Lent::borrow(self.0 .0)
    }
}
impl<T> FromJs for JsPromise<T> {
    fn from_handle(h: u32) -> Self {
        JsPromise(JsHandle(h), PhantomData)
    }
}
impl<T: JsType> JsPromise<T> {
    pub fn then<F: FnOnce(Result<T, JsError>) + 'static>(&self, f: F) {
        extern "C" fn settle<T: JsType, F: FnOnce(Result<T, JsError>) + 'static>(ctx: *mut c_void, ok: i32, v: u32) {
            let b = unsafe { Box::from_raw(ctx as *mut Closure<Option<F>>) };
            let f = b.f.into_inner().unwrap();
            f(if ok != 0 { Ok(T::from_handle(v)) } else { Err(JsError(JsHandle(v))) })
        }
        unsafe { rt::js_promise_then(self.0 .0, settle::<T, F> as *const () as usize, closure(Some(f))) }
    }
}

// What awaiting a promise polls: its result, once the promise has settled.
pub struct JsFuture<T> {
    state: Rc<RefCell<(Option<Result<T, JsError>>, Option<Waker>)>>,
}
impl<T> Future for JsFuture<T> {
    type Output = Result<T, JsError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.borrow_mut();
        match state.0.take() {
            Some(r) => Poll::Ready(r),
            None => {
                state.1 = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
impl<T: JsType> IntoFuture for JsPromise<T> {
    type Output = Result<T, JsError>;
    type IntoFuture = JsFuture<T>;
    fn into_future(self) -> JsFuture<T> {
        let state: Rc<RefCell<(Option<Result<T, JsError>>, Option<Waker>)>> = Rc::new(RefCell::new((None, None)));
        let settled = state.clone();
        self.then(move |r| {
            let waker = {
                let mut state = settled.borrow_mut();
                state.0 = Some(r);
                state.1.take()
            };
            if let Some(w) = waker {
                w.wake();
            }
        });
        JsFuture { state }
    }
}

// Walks a JavaScript iterator: `for n in list.iter()`. Each value is the loop's.
pub struct JsIterator<T>(JsHandle, PhantomData<fn() -> T>);
impl<T: JsType> JsIterator<T> {
    pub fn over(h: u32) -> Self {
        JsIterator(JsHandle(unsafe { rt::js_iterate(h) }), PhantomData)
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
pub struct JsAsyncIterator<T>(JsHandle, PhantomData<fn() -> T>);
impl<T: JsType> JsAsyncIterator<T> {
    pub fn over(h: u32) -> Self {
        JsAsyncIterator(JsHandle(unsafe { rt::js_async_iterate(h) }), PhantomData)
    }
    pub fn next(&self) -> JsPromise<Option<T>> {
        JsPromise::from_handle(unsafe { rt::js_async_iterator_next(self.0 .0) })
    }
}
