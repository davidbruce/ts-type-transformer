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
