// The benchmark through the F# bindings, compiled by Fable.
module Bench

open Bindings
open type Bindings.Globals

let calls (n: int) : float =
    let mutable s = 0.0
    for i in 0 .. n - 1 do
        s <- add (s, float i + 0.5)
    s

let objects (n: int) : float =
    let mutable s = 0.0
    for i in 0 .. n - 1 do
        s <- s + (Vec2.Create (float i, 1.0)).length ()
    s

let properties (n: int) : float =
    let v = Vec2.Create (0.0, 0.0)
    let mutable s = 0.0
    for i in 0 .. n - 1 do
        v.x <- float i
        s <- s + v.x
    s

let strings_in (n: int) : float =
    let mutable s = 0.0
    for _ in 0 .. n - 1 do
        s <- s + count "hello world"
    s

let strings_fresh (n: int) : float =
    let mutable s = 0.0
    for i in 0 .. n - 1 do
        s <- s + count (string (char (97 + i % 26)) + "ello world")
    s

let strings_out (n: int) : float =
    let mutable s = 0.0
    for _ in 0 .. n - 1 do
        let g = greet "you"
        s <- s + float g.Length + float (int g.[0])
    s

let callbacks (n: int) : float =
    let mutable s = 0.0
    each (float n, fun i -> s <- s + i)
    s

let compute (n: int) : float =
    // int, not uint32: Fable multiplies ints exactly, and uint32s as floats.
    let mutable x = 1
    let mutable s = 0.0
    for _ in 0 .. n - 1 do
        x <- x * 1664525 + 1013904223
        s <- s + float ((uint32 x) >>> 24)
    s
