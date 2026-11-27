globalThis.combinePerms = (a, b) => a | b;
globalThis.hasPerm = (p, bit) => (p & bit) === bit;
globalThis.defaultPerms = p => (p === undefined ? 5 : p);
