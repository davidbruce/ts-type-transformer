// A module library across files: what is exported is reached by the name it
// is exported as, a default export as `default`, and constants are not assigned.
import { Shape } from "./shapes";
import Badge from "./badge";
export { internalArea as area, Shape as Figure } from "./shapes";
export { default as Badge } from "./badge";
export interface Point { x: number; y: number }
export default function origin(): Point;
export declare const LIMIT: number;
export declare let counter: number;
export declare function bump(): number;
export declare function pick(i: number): Shape | Badge;
// A tuple is an array: `"toString" in v` would hold for any object.
export type Pair = [number, number];
export declare function locate(i: number): Pair | Point;
// Rollup's namespace of locals, exported by another name: `util.twice`.
declare const util_d_twice: (n: number) => number;
declare function util_d_half(n: number): number;
declare namespace util_d {
    export { util_d_twice as twice, util_d_half as half };
}
export { util_d as util };
