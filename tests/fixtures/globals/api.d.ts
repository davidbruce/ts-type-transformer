// Global values.
/**
 * Settings of the page: `<b>` stays text, & so does this.
 *
 *     indented like code
 *
 * ```
 * settings.level = 1
 * ```
 */
interface Settings {
    debug: boolean;
    /**
     * How loud it is. The range is `0 /* 9`, not `* /`, and `*\/` is no comment end.
     * Ends in a backslash, like a path: C:\temp\
     * @default 3
     */
    level: number
}
declare var settings: Settings;
declare var VERSION: string;

// A global with members of its own and no interface.
declare var Clock: {
    /**
     * Milliseconds since the epoch, as <b>text</b> & numbers.
     * @returns The time now.
     */
    now(): number;
    readonly ZERO: 0
};

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
/**
 * Joins `parts` with `separator`: `a * / b`, `/* c`, `<i>d</i>` & more.
 * ```ts
 * join("-", "a", "b") // "a-b"
 * ```
 * Ends in a backslash\
 * Words go on and on without a break so that this one line of documentation is far longer than any reasonable source line, Words go on and on without a break so that this one line of documentation is far longer than any reasonable source line, Words go on and on without a break so that this one line of documentation is far longer than any reasonable source line, Words go on and on without a break so that this one line of documentation is far longer than any reasonable source line, 
 *
 * A second paragraph, which a member leaves out.
 * @param separator Put between the parts \
 * @param parts The strings to join, as `string[]`.
 * @deprecated Use `Array.prototype.join` instead.
 * @template T
 * @internal
 */
declare function join(separator: string, ...parts: string[]): string;
/** How high. */
declare const enum Level {
    /** The floor: `<` & `>` alone. */
    Low,
    High = 5
}
declare function bump(level: Level): Level;
