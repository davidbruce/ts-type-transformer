// Calls the generated F# bindings; every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// Values made here, one per case.
let widget = Widget.Create "w"
let options = Options.Create (verbose = true)
// A value of a case's type is the union where an argument is one; `!^`
// makes one elsewhere; a case can be named.
check (describe "text" = "string text") "string case in"
check (describe 42.0 = "number 42") "number case in"
check (describe widget = "widget w") "class case in"
check (describe (Value.Options options) = "options verbose=true") "option bag case in"
let values: Value list = [ !^ "text"; !^ widget ]
check (values |> List.map describe = [ "string text"; "widget w" ]) "erased cast"

// Values made by JavaScript, told apart by matching.
let kind (v: Value) =
    match v with
    | Value.String s -> "string " + s
    | Value.Number n -> "number " + string n
    | Value.Widget w -> "widget " + w.name
    | Value.Options o -> "options " + string o.verbose

check (kind (pick 0.0) = "string text") "match string"
check (kind (pick 1.0) = "number 42") "match number"
check (kind (pick 2.0) = "widget w") "match instanceof"
check (kind (pick 3.0) = "options true") "match by property"

// A tagged union: matched on its `kind` property.
let size (s: Shape) =
    match s with
    | Circle c -> c.radius
    | Square q -> q.side

check (size (makeShape ("circle", 2.0)) = 2.0) "tag circle"
check (size (makeShape ("square", 3.0)) = 3.0) "tag square"
check (area (makeShape ("square", 3.0)) = 9.0) "tagged union in"

// Unions written inline are named after their members.
check (setup true = "bool true") "inline union, bool"
check (setup options = "options verbose=true") "inline union, options"
check ((match lookup "w" with NumberOrWidget.Widget w -> w.name | NumberOrWidget.Number _ -> "") = "w") "inline union out, class"
check ((match lookup "abc" with NumberOrWidget.Number n -> n | NumberOrWidget.Widget _ -> 0.0) = 3.0) "inline union out, number"

printfn "%d checks passed" passed
