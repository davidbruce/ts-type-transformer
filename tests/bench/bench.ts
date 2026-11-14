// The baseline: TypeScript calling the API directly. Every other program
// does the same work through its generated bindings, and must return the
// same sums.
/// <reference path="./api.d.ts" />

// Numbers in, a number out: the cost of a call.
export function calls(n: number): number {
    let s = 0;
    for (let i = 0; i < n; i++) s = add(s, i);
    return s;
}

// An object made, used once and dropped.
export function objects(n: number): number {
    let s = 0;
    for (let i = 0; i < n; i++) s += new Vec2(i, 1).length();
    return s;
}

// A property written and read.
export function properties(n: number): number {
    const v = new Vec2(0, 0);
    let s = 0;
    for (let i = 0; i < n; i++) {
        v.x = i;
        s += v.x;
    }
    return s;
}

// A string going in.
export function strings_in(n: number): number {
    let s = 0;
    for (let i = 0; i < n; i++) s += count("hello world");
    return s;
}

// A string going in that is different each time.
export function strings_fresh(n: number): number {
    let s = 0;
    for (let i = 0; i < n; i++) s += count(String.fromCharCode(97 + (i % 26)) + "ello world");
    return s;
}

// A string coming back, and read.
export function strings_out(n: number): number {
    let s = 0;
    for (let i = 0; i < n; i++) {
        const g = greet("you");
        s += g.length + g.charCodeAt(0);
    }
    return s;
}

// JavaScript calling back n times.
export function callbacks(n: number): number {
    let s = 0;
    each(n, i => { s += i; });
    return s;
}

// No crossing at all: the language itself.
export function compute(n: number): number {
    let x = 1;
    let s = 0;
    for (let i = 0; i < n; i++) {
        x = (Math.imul(x, 1664525) + 1013904223) >>> 0;
        s += x >>> 24;
    }
    return s;
}
