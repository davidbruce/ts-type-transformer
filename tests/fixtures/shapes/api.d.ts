interface Named {
    readonly name: string;
    describe(): string;
}
interface Shape extends Named {
    readonly kind: "circle" | "square";
    size: number;
    parent: Shape | null;
    area(): number;
    scale(factor: number): Shape;
    scale(factor: number, name: string): Shape;
    resize(size: number, round?: boolean): void;
    label(value: string | number): string;
}
declare var Shape: {
    prototype: Shape;
    new(kind: "circle" | "square", size: number): Shape;
    unit(): Shape;
    readonly SIDES: 4;
};
declare function totalArea(a: Shape, b: Shape): number;
declare function largest(a: Shape, b: Shape | null): Shape | null;
declare function nameOf(n: Named): string;
declare function orUnit(s: Shape | undefined): Shape;
declare function find(name: string): Shape | undefined;
// A type parameter bound to a type: what it is given, it gives back.
declare function adopt<T extends Named>(child: T): T;
// A Named that is no Shape.
declare function plain(): Named;
