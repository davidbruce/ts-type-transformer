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
declare function largest<T extends Item>(values: T[]): T;

// Generic bases with a member of the same name.
interface Source<T> { label: string; take(): T }
interface Sink<T> { label: string; put(value: T): void }
interface Pipe extends Source<number>, Sink<number> {}
declare function pipe(): Pipe;
