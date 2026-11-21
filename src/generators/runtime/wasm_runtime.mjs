// Generated. Implements the wasm imports by calling the JavaScript API.
//
// createRuntime(getInstance, options): shared handle table, memory/string helpers,
// callbacks, and runtime env imports. Multiple generated bindings share one runtime
// via `makeEnv(getInstance, { ...options, runtime: baseEnv.$runtime })`.
//
// makeEnv(getInstance, options): `options.root` is where the API's names
// are, when not all are globals: a library's module, say; `options.modules`,
// with several packages, each one's module by its name. `options.check`
// reports mistakes with handles (one used after its release, or released
// twice), at some cost in speed; `options.unchecked(e)` is called with an
// exception no one took, and by default writes it to the console.
export function createRuntime(getInstance, options = {}) {
  const CHECK = !!options.check;
  const RELEASED = Symbol('released');
  // Values by handle. Each `ref` is a reference wasm owns, in a slot of its
  // own; the slot is freed when its count reaches zero. Zero is null.
  const objs = [null];
  const refs = [0];
  const gens = [0];
  const free = [];
  let live = 0;
  const ref = v => {
    if (v === null || v === undefined) return 0;
    // When checking, a slot is never reused: a stale handle stays stale.
    const h = free.length ? free.pop() : objs.length;
    objs[h] = v;
    refs[h] = 1;
    if (gens[h] === undefined) gens[h] = 0;
    live++;
    return h;
  };
  // Open scopes: each holds (handle, generation) pairs given to wasm while it
  // is innermost, released when it closes unless already freed.
  const scopes = [];
  const put = v => {
    const h = ref(v);
    if (h !== 0 && scopes.length !== 0) scopes[scopes.length - 1].push(h, gens[h]);
    return h;
  };
  const release = h => {
    if (h === 0) return;
    if (!(refs[h] > 0)) return fail(new Error(`handle ${h} released twice`));
    if (--refs[h] === 0) {
      live--;
      gens[h]++;
      if (CHECK) objs[h] = RELEASED;
      else {
        objs[h] = undefined;
        free.push(h);
      }
    }
  };
  const get = CHECK
    ? h => {
        const v = objs[h];
        if (v === RELEASED) throw new Error(`handle ${h} used after its release`);
        return v;
      }
    : h => objs[h];
  // A handle wasm gave up: what a callback returns.
  const take = h => {
    const v = get(h);
    release(h);
    return v;
  };
  // The exception a call threw, until wasm takes it: a handle, or 0. One that
  // is not taken by the time control returns to JavaScript, or before the
  // next one, is reported.
  let error = 0;
  let attemptDepth = 0;
  const trapUncaught = options.trapUncaught ?? /*__TRAP_UNCAUGHT__*/false;
  const unchecked = options.unchecked ?? (e => console.error('Uncaught in a wasm call:', e));
  const fail = e => {
    if (trapUncaught && attemptDepth === 0) throw e;
    if (error !== 0) {
      const oldH = error;
      const old = objs[oldH];
      error = 0;
      release(oldH);
      unchecked(old);
    }
    const mine = (error = ref(e));
    queueMicrotask(() => {
      if (error === mine && objs[mine] === e) {
        error = 0;
        release(mine);
        unchecked(e);
      }
    });
    return 0;
  };
  // Runtime calls that can throw, with up to four arguments.
  const guard = f => (a, b, c, d) => {
    try {
      return f(a, b, c, d);
    } catch (e) {
      return fail(e);
    }
  };
  // The module's memory, as bytes: a new view when it has grown.
  let bytes = new Uint8Array(0);
  const mem = () => (bytes.byteLength !== 0 ? bytes : (bytes = new Uint8Array(getInstance().exports.memory.buffer)));
  const table = () => getInstance().exports.__indirect_function_table;
  const enc = new TextEncoder(), dec = new TextDecoder();
  // A string from memory. Short ones are usually ASCII, read faster by hand.
  const decode = (m, ptr, len) => {
    if (len <= 16) {
      let s = '';
      for (let i = 0; i < len; i++) {
        const c = m[ptr + i];
        if (c > 127) return dec.decode(m.subarray(ptr, ptr + len));
        s += String.fromCharCode(c);
      }
      return s;
    }
    return dec.decode(m.subarray(ptr, ptr + len));
  };
  // Strings seen at an address, reused while the bytes there are the same:
  // most strings a program passes are its literals, passed again and again.
  const seen = new Map();
  const str = (ptr, len) => {
    if (len === -1) return text(ptr >>> 0);
    const m = mem();
    const key = ptr * 65536 + len;
    const e = seen.get(key);
    if (e !== undefined) {
      const b = e.bytes;
      let i = 0;
      while (i < len && m[ptr + i] === b[i]) i++;
      if (i === len) return e.s;
      // A buffer that keeps changing is not worth remembering.
      if (e.misses++ > 2) return decode(m, ptr, len);
    }
    const s = decode(m, ptr, len);
    if (len > 256) return s;
    if (seen.size > 1024) seen.clear();
    seen.set(key, { s, bytes: m.slice(ptr, ptr + len), misses: e === undefined ? 0 : e.misses });
    return s;
  };
  // The length of a string in UTF-8.
  const utf8Length = s => {
    const n = s.length;
    for (let i = 0; i < n; i++) if (s.charCodeAt(i) > 127) return enc.encode(s).length;
    return n;
  };
  // Writes at most `cap` bytes of a string to memory; returns its whole length in UTF-8.
  const write = (s, ptr, cap) => {
    const m = mem();
    const n = s.length;
    const fits = n <= cap ? n : cap;
    let i = 0;
    for (; i < fits; i++) {
      const c = s.charCodeAt(i);
      if (c > 127) break;
      m[ptr + i] = c;
    }
    if (i === n) return n;
    if (i === fits && n > cap) return utf8Length(s);
    const r = enc.encodeInto(s, m.subarray(ptr, ptr + cap));
    return r.read === n ? r.written : utf8Length(s);
  };
  const text = h => String(get(h) ?? '');
  const viewOf = (h, off, len) => {
    const v = get(h);
    if (v == null) return new Uint8Array(0);
    const buf = ArrayBuffer.isView(v) ? v.buffer : v;
    const base = ArrayBuffer.isView(v) ? v.byteOffset : 0;
    const total = v.byteLength >>> 0;
    const start = Math.min(off >>> 0, total);
    const count = len === undefined ? total - start : Math.min(len >>> 0, total - start);
    return new Uint8Array(buf, base + start, count);
  };
  const bufferCtors = [
    null,
    typeof SharedArrayBuffer !== 'undefined' ? SharedArrayBuffer : null,
    Int8Array,
    Uint8Array,
    Uint8ClampedArray,
    Int16Array,
    Uint16Array,
    Int32Array,
    Uint32Array,
    Float32Array,
    Float64Array,
    typeof BigInt64Array !== 'undefined' ? BigInt64Array : null,
    typeof BigUint64Array !== 'undefined' ? BigUint64Array : null,
    DataView,
  ];
  // One JavaScript function per wasm function and context, so that a
  // callback passed twice is the same function (removeEventListener).
  const callbacks = new Map();
  // Callbacks made here: their key and context, until released or GC'd.
  const owned = new WeakMap();
  const cbFinalizer = typeof FinalizationRegistry !== 'undefined'
    ? new FinalizationRegistry(state => {
        if (callbacks.get(state.key)?.deref() === undefined) callbacks.delete(state.key);
        if (state.alive) {
          state.alive = false;
          if (state.ctx) getInstance()?.exports?.ts_drop_closure?.(state.ctx);
        }
      })
    : null;
  const dead = () => {
    throw new Error('callback called after release');
  };
  const callback = (kind, fn, ctx, make) => {
    const key = `${kind}:${fn}:${ctx}`;
    let f = callbacks.get(key)?.deref();
    if (f === undefined) {
      const state = { key, ctx, alive: true };
      const raw = make(table().get(fn), state);
      f = function (...a) {
        const saved = attemptDepth;
        try {
          return raw.apply(this, a);
        } finally {
          attemptDepth = saved;
        }
      };
      owned.set(f, state);
      callbacks.set(key, new WeakRef(f));
      cbFinalizer?.register(f, state, state);
    }
    return put(f);
  };
  const env = {
    js_string_len: guard(h => utf8Length(text(h))),
    js_string_read: guard((h, ptr) => { write(text(h), ptr, 0x7fffffff); }),
    // At most `cap` bytes of the string; returns its whole length.
    js_string_copy: guard((h, ptr, cap) => write(text(h), ptr, cap)),
    // As js_string_copy, and if it fit, the handle is released.
    js_string_take: guard((h, ptr, cap) => { const n = write(text(h), ptr, cap); if (n <= cap) release(h); return n; }),
    js_string_equals: guard((h, ptr, len) => (text(h) === str(ptr, len) ? 1 : 0)),
    js_from_number: x => put(x),
    js_from_bool: x => put(!!x),
    js_from_string: (ptr, len) => put(str(ptr, len)),
    js_to_number: guard(h => Number(get(h))),
    js_to_bool: guard(h => (get(h) ? 1 : 0)),
    // Whether two handles are one JavaScript value.
    js_equals: guard((a, b) => (a === b || get(a) === get(b) ? 1 : 0)),
    js_array_new: () => put([]),
    js_array_length: guard(h => get(h).length),
    js_array_get: guard((h, i) => put(get(h)[i])),
    js_array_push: guard((h, v) => { get(h).push(get(v)); }),
    js_object_new: () => put({}),
    js_object_get: guard((h, ptr, len) => put(get(h)[str(ptr, len)])),
    js_object_set: guard((h, ptr, len, v) => { get(h)[str(ptr, len)] = get(v); }),
    js_object_has: guard((h, ptr, len) => (Object.prototype.hasOwnProperty.call(get(h), str(ptr, len)) ? 1 : 0)),
    js_listener_from: (fn, ctx) => callback('listener', fn, ctx, (f, s) => ev => { if (!s.alive) dead(); /*__LISTENER__*/ }),
    // Forgets a callback made here; returns its context, for wasm to free, or 0.
    js_callback_release: guard(h => {
      const f = get(h);
      const o = owned.get(f);
      if (!o) return 0;
      owned.delete(f);
      callbacks.delete(o.key);
      cbFinalizer?.unregister(o);
      o.alive = false;
      return o.ctx;
    }),
    js_retain: guard(h => { if (h !== 0 && get(h) !== undefined) refs[h]++; }),
    js_release: release,
    js_scope_open: () => { scopes.push([]); },
    js_scope_close: () => {
      const s = scopes.pop();
      if (!s) return;
      for (let i = 0; i < s.length; i += 2) {
        if (gens[s[i]] === s[i + 1]) release(s[i]);
      }
    },
    // References wasm holds.
    js_live: () => live,
    // Iteration: an iterator over a value, and its next value, or -1 at the end.
    js_iterate: guard(h => put(get(h)[Symbol.iterator]())),
    js_iterator_next: guard(it => { const r = get(it).next(); return r.done ? -1 : put(r.value); }),
    // Asynchronous iteration: a promise of the next value, or of nothing at the end.
    js_async_iterate: guard(h => put(get(h)[Symbol.asyncIterator]())),
    js_async_iterator_next: guard(it => put(Promise.resolve(get(it).next()).then(r => (r.done ? undefined : r.value)))),
    js_attempt_enter: () => { attemptDepth++; },
    js_attempt_leave: () => { if (attemptDepth > 0) attemptDepth--; },
    // Whether an exception waits to be taken; the exception, which wasm now holds, or 0.
    js_has_error: () => (error !== 0 ? 1 : 0),
    js_take_error: () => { const e = error; error = 0; if (e !== 0 && scopes.length !== 0) scopes[scopes.length - 1].push(e, gens[e]); return e; },
    js_error_message: guard(h => { const e = get(h); return put(String(e instanceof Error ? e.message : (e && typeof e === 'object' && 'message' in e ? e.message : e))); }),
    js_error_name: guard(h => { const e = get(h); return put(String(e instanceof Error ? e.name : (e && typeof e === 'object' && 'name' in e ? e.name : typeof e))); }),
    js_error_stack: guard(h => { const e = get(h); return put(String(e && typeof e === 'object' && e.stack != null ? e.stack : '')); }),
    js_error_new: guard((ptr, len) => put(new Error(str(ptr, len)))),
    js_date_new: ms => put(new Date(ms)),
    js_date_now: () => Date.now(),
    js_date_time: guard(h => { const d = get(h); return d instanceof Date ? d.getTime() : Number(d); }),
    js_date_to_iso_string: guard(h => put(get(h).toISOString())),
    js_bytes_len: guard(h => { const v = get(h); return v == null ? 0 : (v.byteLength >>> 0); }),
    js_bytes_read: guard((h, srcOff, dstPtr, len) => { const s = viewOf(h, srcOff, len); if (s.byteLength) mem().set(s, dstPtr >>> 0); return s.byteLength; }),
    js_bytes_write: guard((h, dstOff, srcPtr, len) => { const d = viewOf(h, dstOff, len); const p = srcPtr >>> 0; if (d.byteLength) d.set(mem().subarray(p, p + d.byteLength)); return d.byteLength; }),
    js_bytes_new: guard((kind, srcPtr, byteLen) => {
      const n = byteLen >>> 0;
      const p = srcPtr >>> 0;
      if (kind === 0) {
        const ab = new ArrayBuffer(n);
        if (p !== 0 && n > 0) new Uint8Array(ab).set(mem().subarray(p, p + n));
        return put(ab);
      }
      if (kind === 1) {
        const Ctor = bufferCtors[1];
        if (!Ctor) throw new TypeError('SharedArrayBuffer is not available');
        const sab = new Ctor(n);
        if (p !== 0 && n > 0) new Uint8Array(sab).set(mem().subarray(p, p + n));
        return put(sab);
      }
      const Ctor = bufferCtors[kind];
      if (!Ctor) throw new TypeError(`unsupported buffer kind ${kind}`);
      if (kind === 13) {
        const ab = new ArrayBuffer(n);
        if (p !== 0 && n > 0) new Uint8Array(ab).set(mem().subarray(p, p + n));
        return put(new DataView(ab));
      }
      const count = (n / Ctor.BYTES_PER_ELEMENT) >>> 0;
      const ta = new Ctor(count);
      if (p !== 0 && count > 0) new Uint8Array(ta.buffer, ta.byteOffset, ta.byteLength).set(mem().subarray(p, p + ta.byteLength));
      return put(ta);
    }),
    js_bytes_buffer: guard(h => { const v = get(h); return put(ArrayBuffer.isView(v) ? v.buffer : v); }),
    js_bytes_offset: guard(h => { const v = get(h); return ArrayBuffer.isView(v) ? (v.byteOffset >>> 0) : 0; }),
    // Calls fn(ctx, ok, value) once the promise settles: ok 1 with its value, 0 with its reason.
    js_promise_then: guard((p, fn, ctx) => {
      const settle = ok => v => { const saved = attemptDepth; try { /*__SETTLE__*/ } finally { attemptDepth = saved; } };
      Promise.resolve(get(p)).then(settle(1), settle(0));
    }),
  };
  const trim = a => { let n = a.length; while (n > 0 && a[n - 1] === undefined) n--; if (n < a.length) a.length = n; return a; };
  return { ref, put, release, get, take, str, mem, table, fail, guard, callback, dead, trim, env };
}
