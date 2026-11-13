// The JavaScript the generated bindings call.
globalThis.describe = o => `${o.width}x${o.height ?? "?"} ${o.mode}${o.title === undefined ? "" : ` "${o.title}"`}`;
globalThis.defaults = () => ({ width: 4, mode: "fit" });
globalThis.patch = (o, changes) => {
    const out = { ...o };
    for (const [k, v] of Object.entries(changes)) if (v !== undefined) out[k] = v;
    return out;
};
globalThis.area = size => size.width * (size.height ?? size.width);
globalThis.field = () => "title";
globalThis.lookup = (table, key) => table[key] ?? -1;
globalThis.show = t => `${t.name}#${t.id}`;

globalThis.norm = p => Math.hypot(p.x, p.y);
globalThis.origin = () => ({ x: 0, y: 0 });

globalThis.range = n => Array.from({ length: n }, (_, i) => i);
globalThis.sum = values => values.reduce((a, b) => a + b, 0);
globalThis.words = text => text.split(" ");
globalThis.pair = (n, s) => [n, s];
globalThis.swap = ([n, s]) => [s, n];
globalThis.items = () => [{ id: 1, tags: ["a", "b"] }, { id: 2, tags: [] }];

class Box {
    constructor(value) { this.value = value; }
    map(f) { return new Box(f(this.value)); }
    replace(value) { return new Box(value); }
}
globalThis.wrap = value => new Box(value);
globalThis.first = values => values[0];
globalThis.unbox = v => (v instanceof Box ? v.value : v);
globalThis.either = (value, boxed) => (boxed ? new Box(value) : value);
globalThis.ids = () => [1, "two"];
globalThis.boxedId = () => new Box("three");
globalThis.largest = values => values.reduce((a, b) => (b.id > a.id ? b : a));
globalThis.pipe = () => {
    const queue = [];
    return { label: "pipe", take: () => queue.shift(), put: v => { queue.push(v); } };
};
