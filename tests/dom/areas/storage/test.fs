// localStorage and sessionStorage from F#: items set, read, listed and
// removed; a JSON value stored as a string. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let same (a: obj) (b: obj) = obj.ReferenceEquals(a, b)

// Each store starts empty in a fresh page; clear to be sure.
localStorage.clear ()
sessionStorage.clear ()
check (localStorage.length = 0.0) "local starts empty"
check (sessionStorage.length = 0.0) "session starts empty"

// Set, get, overwrite.
localStorage.setItem ("name", "ada")
check (localStorage.getItem "name" = "ada") "getItem"
check (localStorage.length = 1.0) "length 1"
localStorage.setItem ("name", "grace")
check (localStorage.getItem "name" = "grace") "overwrite"
check (localStorage.length = 1.0) "overwrite keeps the length"
check (isNull (localStorage.getItem "missing")) "a missing key is null"

// Several keys: key(i) lists them.
localStorage.setItem ("b", "2")
localStorage.setItem ("c", "3")
check (localStorage.length = 3.0) "length 3"
let keys = [ for i in 0..2 -> localStorage.key (float i) ] |> List.sort
check (keys = [ "b"; "c"; "name" ]) "key(i) names every key"
check (isNull (localStorage.key 3.0)) "key out of range is null"

// Values are strings.
localStorage.setItem ("n", string 42)
check (localStorage.getItem "n" = "42") "numbers stored as text"
check (float (localStorage.getItem "n") + 1.0 = 43.0) "read back as a number"
localStorage.setItem ("empty", "")
check (localStorage.getItem "empty" = "") "an empty string is not null"
localStorage.setItem ("unicode", "héllo 世界")
check (localStorage.getItem "unicode" = "héllo 世界") "unicode round trip"
let big = System.String('x', 100000)
localStorage.setItem ("big", big)
check (localStorage.getItem("big").Length = 100000) "a long value"

// Property-style access.
check (string localStorage.["name"] = "grace") "indexer read"
localStorage.["viaIndexer"] <- box "yes"
check (localStorage.getItem "viaIndexer" = "yes") "indexer write"

// Removal.
localStorage.removeItem "b"
check (isNull (localStorage.getItem "b")) "removeItem"
localStorage.removeItem "never-set"
check (localStorage.getItem "c" = "3") "other keys survive"

// The two stores are separate.
sessionStorage.setItem ("name", "session-ada")
check (sessionStorage.getItem "name" = "session-ada") "session getItem"
check (localStorage.getItem "name" = "grace") "local unchanged by session"
check (sessionStorage.length = 1.0) "session length"
sessionStorage.removeItem "name"
check (isNull (sessionStorage.getItem "name")) "session removeItem"
check (localStorage.getItem "name" = "grace") "local survives session removal"
check (not (same localStorage sessionStorage)) "distinct objects"

// A JSON value kept as a string.
let record =
    createObj
        [ "id" ==> 7
          "name" ==> "Ada"
          "tags" ==> [| "math"; "engines" |]
          "nested" ==> createObj [ "ok" ==> true; "ratio" ==> 0.5 ]
          "none" ==> null ]
localStorage.setItem ("record", JS.JSON.stringify record)
let raw = localStorage.getItem "record"
check (raw.Contains "\"name\":\"Ada\"") "stored as JSON text"
let loaded = JS.JSON.parse raw
check (loaded?id = 7) "JSON number"
check (loaded?name = "Ada") "JSON string"
check ((loaded?tags: string[]).Length = 2 && (loaded?tags: string[]).[1] = "engines") "JSON array"
check (loaded?nested?ok = true && loaded?nested?ratio = 0.5) "JSON nested"
check (isNull loaded?none) "JSON null"
// Updated and stored again.
loaded?id <- 8
localStorage.setItem ("record", JS.JSON.stringify loaded)
check ((JS.JSON.parse (localStorage.getItem "record"))?id = 8) "JSON updated"

// A list of items, stored as one string.
let items = [| "one"; "two"; "three" |]
sessionStorage.setItem ("items", JS.JSON.stringify items)
let back: string[] = JS.JSON.parse (sessionStorage.getItem "items") |> unbox
check (back.Length = 3 && back.[2] = "three") "an array round trip"

// Quota: a store refuses an enormous value.
let mutable refused = false
try
    localStorage.setItem ("huge", System.String('y', 20 * 1024 * 1024))
with _ ->
    refused <- true
check refused "setItem past the quota throws"
check (isNull (localStorage.getItem "huge")) "the refused value is not stored"

// Clear.
localStorage.clear ()
check (localStorage.length = 0.0) "clear empties local"
check (isNull (localStorage.getItem "name")) "clear removes values"
check (sessionStorage.length = 1.0) "clear leaves session alone"
sessionStorage.clear ()
check (sessionStorage.length = 0.0) "clear empties session"

printfn "%d checks passed" passed
