// Option objects.
interface Options {
    width: number;
    height?: number;
    title?: string;
    mode: "fit" | "fill";
}
declare function describe(o: Options): string;
declare function defaults(): Options;

// Type-level expressions over them.
declare function patch(o: Options, changes: Partial<Options>): Options;
declare function area(size: Pick<Options, "width" | "height">): number;
declare function field(): keyof Options;
declare function lookup(table: Record<string, number>, key: string): number;
type Tagged = { name: string } & { id: number };
declare function show(t: Tagged): string;

// Inline object types.
declare function norm(point: { x: number; y: number }): number;
declare function origin(): { x: number; y: number };

// Arrays and tuples.
declare function range(n: number): number[];
declare function sum(values: number[]): number;
declare function words(text: string): string[];
declare function pair(n: number, s: string): [number, string];
declare function swap(p: [number, string]): [string, number];
interface Item { id: number; tags: string[] }
declare function items(): Item[];

// Generics.
interface Box<T> {
    value: T;
    map<U>(f: (value: T) => U): Box<U>;
    // A type parameter named like its interface's.
    replace<T>(value: T): Box<T>;
}
declare function wrap<T>(value: T): Box<T>;
declare function first<T>(values: T[]): T | undefined;
type Either<T> = T | Box<T>;
declare function unbox<T>(value: Either<T>): T;
declare function either<T>(value: T, boxed: boolean): Either<T>;
// Unions as type arguments.
type Id = number | string;
declare function ids(): Id[];
declare function boxedId(): Box<Id>;
declare function largest<T extends Item>(values: T[]): T;
interface Holder<T extends Item> { held: T }
declare function hold(item: Item): Holder<Item>;

// Generic aliases of an array, a tuple and an interface: each use is the type
// with its arguments (aliases = "expand").
type List<T> = T[];
type Pair<A, B> = [A, B];
type Boxed<T> = Box<T>;
declare function total(values: List<number>): number;
declare function flip(pair: Pair<string, number>): Pair<number, string>;
declare function boxedName(): Boxed<string>;

// Generic bases with a member of the same name.
interface Source<T> { label: string; take(): T }
interface Sink<T> { label: string; put(value: T): void }
interface Pipe extends Source<number>, Sink<number> {}
declare function pipe(): Pipe;

// An option bag whose properties are all optional: what is named is set, and nothing else.
interface Base { id?: number }
interface Marks extends Base { on?: boolean; count?: number; label?: string; mode?: "wide" | "narrow" }
declare function marked(marks: Marks): string;
interface Spot { at?: [number, number, number]; tags?: string[] }
declare function spot(s: Spot): string;

// A nullable string and a nullable object; a parameter of any type.
declare function greeting(name: string | null, options: Options | null): string;
declare function echo(value: any): string;
