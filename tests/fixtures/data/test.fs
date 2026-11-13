// Calls the generated F# bindings; every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// Option objects: built here, read by JavaScript.
let o =
    jsOptions<Options> (fun o ->
        o.width <- 3.0
        o.mode <- Options_mode.Fill)

check (describe o = "3x? fill") "options in"
o.height <- Some 2.0
o.title <- Some "hi"
check (describe o = "3x2 fill \"hi\"") "optional properties in"

// Built by JavaScript, read here.
let d = defaults ()
check (d.width = 4.0 && d.mode = Options_mode.Fit) "options out"
check d.height.IsNone "missing optional property"

// Partial<Options>.
let patched = patch (d, {| width = Some 9.0; height = None; title = None; mode = None |})
check (patched.width = 9.0 && patched.mode = Options_mode.Fit) "Partial"

// Pick<Options, "width" | "height">.
check (area {| width = 3.0; height = None |} = 9.0) "Pick"
check (area {| width = 3.0; height = Some 5.0 |} = 15.0) "Pick with optional"

// keyof Options.
check (field () = field_result.Title) "keyof"

// Record<string, number>.
let table = createObj [ "a" ==> 7 ]
check (lookup (table, "a") = 7.0) "Record hit"
check (lookup (table, "b") = -1.0) "Record miss"

// An intersection of object types.
check (show {| name = "x"; id = 5.0 |} = "x#5") "intersection"

// Inline object types.
check (norm {| x = 3.0; y = 4.0 |} = 5.0) "inline object in"
check ((origin ()).x = 0.0) "inline object out"

// Arrays.
let r = range 4.0
check (r.Count = 4 && r.[3] = 3.0) "array out"
check (sum r = 6.0) "array round trip"
check (sum (ResizeArray [ 10.0; 5.0 ]) = 15.0) "array in"
check ((words "a bc").[1] = "bc") "array of strings"

// Tuples.
let p = pair (1.0, "one")
check (p = (1.0, "one")) "tuple out"
check (swap p = ("one", 1.0)) "tuple in"

// Arrays of objects.
let all = items ()
check (all.Count = 2 && all.[0].id = 1.0 && all.[0].tags.Count = 2) "array of objects"

// Generics are kept.
let b = wrap 21.0
check (b.value = 21.0) "generic value"
check ((b.map (fun v -> v * 2.0)).value = 42.0) "generic method"
check ((b.map (fun v -> string v)).value = "21") "generic method, other type"
check (first r = Some 0.0) "generic function"
check (first (ResizeArray<float>()) = None) "generic function, undefined"
check ((largest all).id = 2.0) "constrained generic function"

printfn "%d checks passed" passed
