// The JavaScript every benchmark program calls.
globalThis.add = (a, b) => a + b;
class Vec2 {
    constructor(x, y) { this.x = x; this.y = y; }
    length() { return Math.sqrt(this.x * this.x + this.y * this.y); }
}
globalThis.Vec2 = Vec2;
globalThis.count = text => text.length;
globalThis.greet = name => "hello " + name;
globalThis.each = (n, f) => { for (let i = 0; i < n; i++) f(i); };
