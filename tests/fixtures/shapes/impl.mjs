// The JavaScript the generated bindings call.
class Shape {
    static SIDES = 4;
    static unit() { return new Shape("square", 1); }
    constructor(kind, size) {
        this.kind = kind;
        this.size = size;
        this.name = `${kind}-${size}`;
        this.parent = null;
    }
    describe() { return `${this.name} (${this.area()})`; }
    area() { return this.kind === "circle" ? Math.PI * this.size ** 2 : this.size ** 2; }
    scale(factor, name) {
        const s = new Shape(this.kind, this.size * factor);
        if (name !== undefined) s.name = name;
        s.parent = this;
        return s;
    }
    resize(size, round) { this.size = round ? Math.round(size) : size; }
    label(value) { return `${typeof value}:${value}`; }
    measure(options) { return options.exact ? this.area() : `${this.area()}`; }
}
globalThis.Shape = Shape;
globalThis.totalArea = (a, b) => a.area() + b.area();
globalThis.largest = (a, b) => (b === null ? a : a.area() >= b.area() ? a : b);
globalThis.nameOf = n => n.name;
// Strict on purpose: `null` and `undefined` are not interchangeable.
globalThis.orUnit = s => (s === undefined ? Shape.unit() : s);
globalThis.find = name => (name === "unit" ? Shape.unit() : undefined);
globalThis.adopt = child => child;
globalThis.plain = () => ({ name: "plain", describe: () => "plain" });
globalThis.wrap = v => (typeof v === "object" && v !== null && "name" in v ? `[${v.name}]` : v * 2);
