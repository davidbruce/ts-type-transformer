type Listener = (value: number) => void;
interface Counter {
    readonly count: number;
    onchange: ((value: number) => void) | null;
    subscribe(listener: Listener): void;
    unsubscribe(listener: Listener): void;
    increment(by?: number): number;
    fold(f: (total: number, label: string) => number): number;
    visit(f: (c: Counter) => boolean): boolean;
}
declare var Counter: { prototype: Counter; new(): Counter };
declare function later(f: () => void): void;
declare function flush(): number;
// A callback taking a union.
type Id = number | string;
declare function eachId(f: (id: Id) => void): number;
