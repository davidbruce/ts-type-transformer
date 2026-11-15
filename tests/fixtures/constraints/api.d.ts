// Constraints TypeScript checks by shape, lowered for targets that check by name.
declare class Stream { close(): void }
declare class Readable extends Stream { read(): string }
interface Shaped { close(): void }
// Kept: every use names a type declared to extend Stream.
interface Kept<T extends Stream> { item: T }
declare function kept(): Kept<Readable>;
// Dropped: Shaped extends Stream only by shape.
interface ByShape<T extends Stream> { item: T }
declare function byShape(): ByShape<Shaped>;
// A type parameter meets Stream through Readable: nominal keeps it, exact does not.
interface Options<T extends Stream> { stream: T }
interface ReadableOptions<T extends Readable> extends Options<T> { encoding: string }
