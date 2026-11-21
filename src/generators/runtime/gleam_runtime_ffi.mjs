import { Key } from "./runtime.mjs";

export function $key(raw) {
  return new Key(raw);
}
export function promise_then(p, f) {
  Promise.resolve(p).then((v) => f(new Ok(v)), (e) => f(new GleamError(e)));
}
export function ref_new(v) {
  return { v };
}
export function ref_get(c) {
  return c.v;
}
export function ref_set(c, v) {
  c.v = v;
}
export function ref_update(c, f) {
  c.v = f(c.v);
  return c.v;
}
export function attempt(f) {
  try {
    return new Ok(f());
  } catch (e) {
    return new GleamError(e);
  }
}
export function js_error_message(e) {
  return String(e instanceof Error ? e.message : e?.message ?? e);
}
export function dynamic(v) {
  return v;
}
export function js_object(entries) {
  return Object.fromEntries(entries.toArray());
}
export function js_get(o, k) {
  const v = o?.[k];
  return v === undefined ? new None() : new Some(v);
}
export function js_set(o, k, v) {
  o[k] = v;
}
export function js_has(o, k) {
  return o != null && Object.prototype.hasOwnProperty.call(o, k);
}
export function js_array(items) {
  return items.toArray();
}
export function js_to_list(a) {
  return toList(Array.from(a ?? []));
}
// Codecs: how a type parameter's values cross, each way.
export const $same = { to: (v) => v, from: (v) => v };
export function codec_same() {
  return $same;
}
export function codec_option(c) {
  return { to: (v) => (v == null ? new None() : new Some(c.to(v))), from: (v) => (v instanceof Some ? c.from(v[0]) : undefined) };
}
export function codec_list(c) {
  return { to: (v) => toList(Array.from(v ?? [], (x) => c.to(x))), from: (v) => v.toArray().map((x) => c.from(x)) };
}
export function codec_tuple2(a, b) {
  return { to: (v) => [a.to(v[0]), b.to(v[1])], from: (v) => [a.from(v[0]), b.from(v[1])] };
}
// A Gleam function made a JavaScript function, the same one each time: one
// listener added and removed is one function.
const $wrapped = new WeakMap();
export function $wrap(f, make) {
  if (typeof f !== "function") return f;
  let w = $wrapped.get(f);
  if (!w) $wrapped.set(f, (w = make(f)));
  return w;
}
export function $trim(a) {
  let n = a.length;
  while (n > 0 && a[n - 1] === undefined) n--;
  if (n < a.length) a.length = n;
  return a;
}
