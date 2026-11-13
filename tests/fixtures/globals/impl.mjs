// The JavaScript the generated bindings call.
globalThis.settings = { debug: false, level: 2 };
globalThis.VERSION = "1.2.3";
globalThis.Clock = { ZERO: 0, now: () => 1000 };
globalThis.Geometry = {
    UNIT: 1,
    point: (x, y) => ({ x, y }),
    distance: (a, b) => Math.hypot(a.x - b.x, a.y - b.y),
    Deep: { twice: n => n * 2 },
};
globalThis.join = (separator, ...parts) => parts.join(separator);
globalThis.bump = level => (level === 0 ? 5 : level);
