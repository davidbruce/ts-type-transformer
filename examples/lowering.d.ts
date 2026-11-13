// One small case per lowering switch.
interface Emitter<M> {
    on<K extends keyof M>(type: K, listener: (this: this, ev: M[K]) => void): void;
    off(type: string): void;
}
interface ShapeEvents { resize: UIEvent; close: Event }
interface Shape extends Emitter<ShapeEvents> {
    readonly kind: "circle" | "square";
    area(): number;
    scale(factor: number, origin?: Point | [number, number]): Shape | null;
    scale(factor: { x: number; y: number }): Shape;
    onchange: ((ev: Event) => void) | null;
    style: Partial<Style> & { id: string };
}
declare var Shape: { prototype: Shape; new(kind: "circle" | "square", size?: number): Shape; unit(): Shape };
interface Point { x: number; y: number }
interface Style { fill: string; stroke: string | number }
type Keys = keyof Style;
type Listener<T> = (value: T) => void;
