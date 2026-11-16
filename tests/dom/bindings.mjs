// Checks, in a real browser, that every binding reaches what it names, with
// the right kind: a method is a function, a getter is readable, a setter
// writable, a constructor constructs. `bindings` and `implementers` are a
// generator's manifest; `declared`, every name the declarations have.
//
// A binding is
// - resolved: what it names is there, as it says;
// - wrong: there, but not as it says (a method that is a getter, a setter
//   of what is read-only), or a name the declarations do not have;
// - unimplemented: no class of it is in this browser; absent: its class is,
//   but not the member (lib.dom.d.ts has what some browser has);
// - unchecked: nothing to look up (an option bag made, a callable called).

// Objects that hold their members themselves, not their prototype's
// (`window.document`, `location.href` are unforgeable; Chrome has a style's
// properties on the style).
let objects;
const singletons = () =>
    (objects ??= [globalThis, document, document.documentElement, location, navigator, history, screen, performance, document.body.style, getComputedStyle(document.body), document.createRange(), new Event('x'), document.createElement('input')]);

// Where lib.dom.d.ts and Chrome differ, not the bindings: by `owner.member`.
const differs = new Map([
    ['CSSFontFaceRule.style', 'declared with a setter (PutForwards); read-only in Chrome'],
]);

function at(path) {
    let v = globalThis;
    for (const part of path.split('.')) {
        if (v == null) return undefined;
        v = v[part];
    }
    return v;
}

// The property's descriptor, from the object up its prototypes.
function find(o, name) {
    for (let p = o; p != null; p = Object.getPrototypeOf(p)) {
        const d = Object.getOwnPropertyDescriptor(p, name);
        if (d) return d;
    }
    return undefined;
}

function judge(kind, d) {
    if (!d) return 'absent';
    const accessor = 'get' in d || 'set' in d;
    switch (kind) {
        case 'method':
        case 'static':
            return !accessor && typeof d.value === 'function' ? 'resolved' : 'wrong';
        case 'get':
        case 'staticGet':
            return accessor ? (d.get ? 'resolved' : 'wrong') : typeof d.value === 'function' ? 'wrong' : 'resolved';
        case 'set':
        case 'staticSet':
            return accessor ? (d.set ? 'resolved' : 'wrong') : d.writable ? 'resolved' : 'wrong';
    }
    return 'unchecked';
}

// Each binding's verdict; `wrong` and `undeclared` are failures.
export function verify(bindings, implementers, declared) {
    const names = new Set(declared);
    // An option bag's members are a plain object's: nothing to look up.
    const bags = new Set(bindings.filter(b => b[1] === 'create').map(b => b[2]));
    const counts = { resolved: 0, wrong: 0, undeclared: 0, absent: 0, unimplemented: 0, unchecked: 0 };
    const wrong = [];
    const absent = [];
    for (const [entry, kind, owner, member, cls] of bindings) {
        const say = (verdict, why) => {
            counts[verdict]++;
            if (verdict === 'wrong' || verdict === 'undeclared') wrong.push(`${entry}: ${why}`);
            if (verdict === 'absent') absent.push(entry);
        };
        if (['create', 'invoke', 'construct', 'indexGet', 'indexSet'].includes(kind)) {
            say('unchecked');
            continue;
        }
        // `[Symbol.toStringTag]` is keyed by the symbol.
        const symbol = member.match(/\[Symbol\.(\w+)\]$/)?.[1];
        const last = symbol ? member : member.split('.').pop();
        const key = symbol ? Symbol[symbol] : last;
        if (!names.has(last)) {
            say('undeclared', `${kind} ${member} is in no declaration`);
            continue;
        }
        // A global, or one reached from one: `fetch`, `URL.createObjectURL`.
        if (owner === '' || ['new', 'static', 'staticGet', 'staticSet'].includes(kind)) {
            const parent = member.includes('.') ? at(member.slice(0, member.lastIndexOf('.'))) : globalThis;
            if (kind === 'new') {
                const v = at(member);
                say(v === undefined ? 'absent' : typeof v === 'function' ? 'resolved' : 'wrong', `new ${member}: not a constructor`);
            } else if (parent == null) say('absent');
            else {
                const v = judge(kind, find(parent, key));
                say(v, `${kind} ${member}`);
            }
            continue;
        }
        if (bags.has(owner)) {
            say('unchecked');
            continue;
        }
        // A member: on its constructor's prototype, or those of the classes
        // of its interface; else held by an object of them.
        const ctors = [...new Set(owner.startsWith('interface:') ? implementers[cls] ?? [] : [owner, ...(implementers[cls] ?? [])])].map(at).filter(c => typeof c === 'function');
        // No class of it in this browser: none of its members is.
        if (ctors.length === 0) {
            say(owner.startsWith('interface:') && !(implementers[cls] ?? []).length ? 'unchecked' : 'unimplemented');
            continue;
        }
        let d;
        for (const c of ctors) {
            d = find(c.prototype, key) ?? singletons().filter(s => s instanceof c).map(s => Object.getOwnPropertyDescriptor(s, key)).find(Boolean);
            if (d) break;
        }
        // A member a subclass has as its own, held by an object of its base
        // (Zig repeats `isTrusted` on each event class; it is each event's own).
        d ??= singletons().map(s => Object.getOwnPropertyDescriptor(s, key)).find(Boolean);
        const v = differs.has(`${owner}.${member}`) ? 'absent' : judge(kind, d);
        say(v, `${kind} ${owner}.${member}: ${d ? ('get' in d || 'set' in d ? 'an accessor' + (d.set ? '' : ', read-only') : typeof d.value) : ''}`);
    }
    return { counts, wrong, absent };
}
