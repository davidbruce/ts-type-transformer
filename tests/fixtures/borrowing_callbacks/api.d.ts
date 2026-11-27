interface Queue {
    push(n: number): void;
    drain(f: (item: number) => void): number;
    drainOwned(f: (item: number) => void): void;
}
declare var Queue: { prototype: Queue; new(): Queue };
declare function runScoped(f: (x: number) => number): number;
declare function runScopedStash(f: (x: number) => void): void;
declare function invokeStashed(): void;
