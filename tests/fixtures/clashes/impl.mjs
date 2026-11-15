globalThis.Gizmo = class {
    isTransforming() { return true; }
    fooBar() { return 1; }
    foo_bar() { return 2; }
    static isTransforming = () => false;
};
globalThis.shape = () => ({ area: () => 3 });
globalThis.Shape_area = s => s.area() * 10;
globalThis.point = x => ({ x, create: y => ({ x: y, create: null }) });
globalThis.ns = { Node: class { id() { return 1; } } };
globalThis.ns_Node = class { id() { return 2; } };
