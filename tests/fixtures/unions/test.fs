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
let options = jsOptions<Options> (fun o -> o.verbose <- true)
check (describe (Value.String "text") = "string text") "string case in"
check (describe (Value.Number 42.0) = "number 42") "number case in"
check (describe (Value.Widget widget) = "widget w") "class case in"
check (describe (Value.Options options) = "options verbose=true") "option bag case in"

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
check (setup (boolean_or_Options.Bool true) = "bool true") "inline union, bool"
check (setup (boolean_or_Options.Options options) = "options verbose=true") "inline union, options"
check ((match lookup "w" with number_or_Widget.Widget w -> w.name | number_or_Widget.Number _ -> "") = "w") "inline union out, class"
check ((match lookup "abc" with number_or_Widget.Number n -> n | number_or_Widget.Widget _ -> 0.0) = 3.0) "inline union out, number"

printfn "%d checks passed" passed
