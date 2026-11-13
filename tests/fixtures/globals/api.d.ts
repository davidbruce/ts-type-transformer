// Global values.
interface Settings { debug: boolean; level: number }
declare var settings: Settings;
declare var VERSION: string;

// A global with members of its own and no interface.
declare var Clock: { now(): number; readonly ZERO: 0 };

// Namespaces.
declare namespace Geometry {
    interface Point { x: number; y: number }
    function point(x: number, y: number): Point;
    function distance(a: Point, b: Point): number;
    const UNIT: number;
    namespace Deep {
        function twice(n: number): number;
    }
}

// Rest parameters and numeric enums.
declare function join(separator: string, ...parts: string[]): string;
declare const enum Level { Low, High = 5 }
declare function bump(level: Level): Level;
