declare const enum Color { Red, Green = 4, Blue = "blue" }
type Partialish<T> = { readonly [K in keyof T]?: T[K] };
type Unwrap<T> = T extends Promise<infer U> ? U : T;
type EventName<T extends string> = `on${Capitalize<T>}`;
declare function parse(s: string): number;
declare function parse(s: string, radix: number): number;
declare namespace Geo.Shapes { interface Point { x: number; y: number } }
interface Box<T = string> { value: T; [Symbol.iterator](): Iterator<T>; "quoted-name": [first: T, ...rest: number[]] }
interface Box<T> { extra?: Geo.Shapes.Point; isBox(x: unknown): x is Box<T> }
