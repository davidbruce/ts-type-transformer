// The library: what api.d.ts declares, as a module.
export class Figure {
    constructor(size) { this.size = size; }
    grow() { return ++this.size; }
}
export class Badge {
    constructor(label) { this.label = label; }
}
export function area(w, h) { return w * h; }
export default function origin() { return { x: 0, y: 0 }; }
export const LIMIT = 10;
export let counter = 0;
export function bump() { return ++counter; }
export function pick(i) { return i === 0 ? new Figure(2) : new Badge('b'); }
export function locate(i) { return i === 0 ? [3, 4] : { x: 5, y: 6 }; }
export const util = { twice: n => n * 2, half: n => n / 2 };
