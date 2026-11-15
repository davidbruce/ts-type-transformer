// A mutable cell, for tests to see what callbacks did: Gleam has no other.
pub type Cell(a)

@external(javascript, "./cell_ffi.mjs", "make")
pub fn new(value: a) -> Cell(a)

@external(javascript, "./cell_ffi.mjs", "get")
pub fn get(cell: Cell(a)) -> a

@external(javascript, "./cell_ffi.mjs", "set")
pub fn set(cell: Cell(a), value: a) -> Nil
