// The JavaScript the generated bindings call.
class Widget {
    constructor(name) { this.name = name; }
}
globalThis.Widget = Widget;
globalThis.describe = v =>
    typeof v === "string" ? `string ${v}`
    : typeof v === "number" ? `number ${v}`
    : v instanceof Widget ? `widget ${v.name}`
    : `options verbose=${v.verbose}`;
const values = ["text", 42, new Widget("w"), { verbose: true }];
globalThis.pick = i => values[i];
globalThis.makeShape = (kind, size) => (kind === "circle" ? { kind, radius: size } : { kind: "square", side: size });
globalThis.area = s => (s.kind === "circle" ? 3 * s.radius * s.radius : s.side * s.side);
globalThis.setup = o => (typeof o === "boolean" ? `bool ${o}` : `options verbose=${o.verbose}`);
globalThis.lookup = key => (key === "w" ? new Widget(key) : key.length);
globalThis.peek = slot => (slot.value === undefined ? "empty" : globalThis.describe(slot.value));
globalThis.loose = v => `loose ${JSON.stringify(v)}`;
globalThis.anyLoose = () => ({ b: 2 });
globalThis.looseAll = values => values.map(globalThis.loose).join(";");
