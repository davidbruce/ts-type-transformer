// Calls the generated F# bindings with JavaScript built-ins: buffers, typed arrays, Error, Date.
module Test

open Fable.Core
open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let bytes (a: JS.Uint8Array) = [ for i in 0 .. int a.length - 1 -> int a.[i] ]

// ArrayBuffer: Fable's, read through a view.
let buf = JS.Constructors.ArrayBuffer.Create 4
let view = JS.Constructors.Uint8Array.Create(buf)
for i in 0..3 do view.[i] <- byte (i + 1)
let flipped = roundTripBytes buf
check (bytes (JS.Constructors.Uint8Array.Create(flipped)) = [ 1 ^^^ 0x5a; 2 ^^^ 0x5a; 3 ^^^ 0x5a; 4 ^^^ 0x5a ]) "ArrayBuffer"

// A view of part of a buffer.
let sub = makeSubView ()
check (sub.byteLength = 4 && sub.byteOffset = 2) "ArrayBufferView"

// DataView.
let dvBuf = JS.Constructors.ArrayBuffer.Create 3
let dvBytes = JS.Constructors.Uint8Array.Create(dvBuf)
dvBytes.[0] <- 10uy; dvBytes.[1] <- 20uy; dvBytes.[2] <- 30uy
let flippedView = flipDataView (JS.Constructors.DataView.Create(dvBuf))
check ([ for i in 0..2 -> int (flippedView.getUint8 i) ] = [ 30; 20; 10 ]) "DataView"

// Typed arrays.
check (bytes (makeUint8 5.0) = [ 3; 10; 17; 24; 31 ]) "Uint8Array"
let i32 = xorInts (JS.Constructors.Int32Array.Create(ResizeArray [ 0x01020304; -5 ]))
check (i32.[0] = (0x01020304 ^^^ 0x12345678)) "Int32Array"
let f32 = JS.Constructors.Float32Array.Create(ResizeArray [ 1.5f; 2.25f; -0.75f ])
check (sumFloats f32 = 3.0) "Float32Array"
let f64 = scaleDoubles (JS.Constructors.Float64Array.Create(ResizeArray [ 2.5; -4.0 ]), 3.0)
check (f64.[0] = 7.5 && f64.[1] = -12.0) "Float64Array"

// A JavaScript Error is an F# exception.
check (mayThrow false = 42.0) "no exception"
check ((try mayThrow true |> ignore; "" with e -> e.Message) = "bad input") "an exception caught in F#"
check ((makeRangeError "out of range").Message = "out of range") "an Error returned"
// An F# exception is not a JavaScript Error: jsError makes one.
check (isJsError (jsError "from F#", "from F#")) "an Error made in F#"

// Date.
let d = makeDate 1700000000000.0
check (d.getTime () = 1700000000000.0 && d.toISOString () = "2023-11-14T22:13:20.000Z") "Date"
check ((addOneDay d).getTime () = 1700086400000.0) "Date passed"

printfn "%d checks passed" passed
