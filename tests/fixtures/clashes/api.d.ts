// Names apart in TypeScript that a target would make one identifier of.
declare class Gizmo {
    constructor();
    // A method, and a static property of a function type, hoisted as
    // `Gizmo_isTransforming`: in C, the method's name too.
    isTransforming(): boolean;
    static isTransforming: () => boolean;
    // One `foo_bar` in snake case.
    fooBar(): number;
    foo_bar(): number;
}
// A function named as a type's method is in C: `Shape_area`.
interface Shape { area(): number }
declare function shape(): Shape;
declare function Shape_area(s: Shape): number;
// A property `create` beside the constructor of an option bag: `Point_create`.
interface Point { x: number; create: (x: number) => Point }
declare function point(x: number): Point;
// A namespace's `Node` flattens to `ns_Node`, which a class declares too.
declare namespace ns {
    class Node { constructor(); id(): number }
}
declare class ns_Node { constructor(); id(): number }
