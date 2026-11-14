// What the benchmark programs call: each crosses the boundary in one way.
declare function add(a: number, b: number): number;
interface Vec2 { x: number; y: number; length(): number }
declare var Vec2: { prototype: Vec2; new(x: number, y: number): Vec2 };
declare function count(text: string): number;
declare function greet(name: string): string;
declare function each(n: number, f: (i: number) => void): void;
