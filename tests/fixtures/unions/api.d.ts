// A class: recognised with instanceof.
interface Widget { readonly name: string }
declare var Widget: { prototype: Widget; new(name: string): Widget };

// An option bag: no class, recognised by a property only it has.
interface Options { verbose: boolean; depth?: number }

// Primitives, a class and an option bag.
type Value = string | number | Widget | Options;
declare function describe(value: Value): string;
declare function pick(index: number): Value;

// Told apart by a property: a tagged union.
interface Circle { kind: "circle"; radius: number }
interface Square { kind: "square"; side: number }
type Shape = Circle | Square;
declare function makeShape(kind: string, size: number): Shape;
declare function area(shape: Shape): number;

// A union written inline.
declare function setup(options: boolean | Options): string;
declare function lookup(key: string): number | Widget;
