// Node's fs, path and events from F#: a scratch directory made, written,
// listed and removed; paths taken apart; an emitter listened to.
// Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// path
check (path.join ("a", "b", "c.txt") = "a/b/c.txt") "path.join"
check (path.basename "/x/y/file.txt" = "file.txt") "path.basename"
check (path.basename ("/x/y/file.txt", ".txt") = "file") "path.basename with a suffix"
check (path.dirname "/x/y/file.txt" = "/x/y") "path.dirname"
check (path.extname "file.tar.gz" = ".gz") "path.extname"
check (path.normalize "/a/b/../c/./d" = "/a/c/d") "path.normalize"
check (path.isAbsolute "/a" && not (path.isAbsolute "a")) "path.isAbsolute"
check (path.resolve ("/a/b", "../c") = "/a/c") "path.resolve"
check (path.relative ("/a/b", "/a/c/d") = "../c/d") "path.relative"
let parsed = path.parse "/home/u/file.txt"
check (parsed.root = "/" && parsed.dir = "/home/u" && parsed.``base`` = "file.txt" && parsed.ext = ".txt" && parsed.name = "file") "path.parse"
check (path.format (path.FormatInputPathObject.Create (dir = Some "/d", name = Some "n", ext = Some ".e")) = "/d/n.e") "path.format"
check (path.sep = path.sep && path.delimiter = path.delimiter) "sep and delimiter are bound"
let join (a: string) (b: string) = path.join (a, b)

// events
let emitter = Events.EventEmitter.Create<obj> ()
let mutable calls = 0
let mutable seen: string list = []
// Still open (finding 11): Fable compiles the delegate to a one-parameter function, so `args` is the first
// argument itself, not an array of them; it is read by casting.
let listener =
    NodeJS.AddListenerListener(fun args ->
        calls <- calls + 1
        seen <- seen @ [ unbox<string> (box args) ])
emitter.on ("ping", listener) |> ignore
check (emitter.listenerCount (U2.Case1 "ping") = 1.0) "listenerCount"
check (emitter.emit (U2.Case1 "ping", box "one")) "emit with a listener is true"
check (calls = 1) "the listener ran"
check (seen = [ "one" ]) "the listener read what was emitted"
check (not (emitter.emit (U2.Case1 "nobody"))) "emit without a listener is false"
emitter.emit (U2.Case1 "ping", box "two") |> ignore
check (calls = 2 && seen = [ "one"; "two" ]) "twice"
let mutable onceCalls = 0
emitter.once ("once", NodeJS.AddListenerListener(fun _ -> onceCalls <- onceCalls + 1)) |> ignore
emitter.emit (U2.Case1 "once") |> ignore
emitter.emit (U2.Case1 "once") |> ignore
check (onceCalls = 1) "once"
check (emitter.listenerCount (U2.Case1 "once") = 0.0) "once removes itself"
emitter.removeListener ("ping", listener) |> ignore
check (emitter.listenerCount (U2.Case1 "ping") = 0.0) "removeListener"
emitter.emit (U2.Case1 "ping", box "three") |> ignore
check (calls = 2) "removed listener is not called"
check (emitter.eventNames().Count = 0) "eventNames"
emitter.setMaxListeners 5.0 |> ignore
check (emitter.getMaxListeners () = 5.0) "setMaxListeners"

// fs
let dir = Fs.mkdtempSync "target/e2e/fs-fsharp-"
check (Fs.existsSync dir) "mkdtempSync"
let file = join dir "hello.txt"
Fs.writeFileSync (file, "héllo\nworld\n")
check (Fs.existsSync file) "writeFileSync"
Fs.appendFileSync (file, "more\n")
// FINDING 12: mkdirSync and readdirSync with just a path are ambiguous between overloads (every overload has optional options); an options value picks one
let created = Fs.mkdirSync (join (join dir "sub") "deep", {| recursive = true; mode = None |})
check (created.IsSome && created.Value.EndsWith "sub") "mkdirSync recursive returns the first directory made"
Fs.writeFileSync (join (join dir "sub") "b.txt", "b")
// FINDING 13: an encoding such as "utf8" cannot be given: `global_BufferEncoding` is an empty interface (Bindings.Prelude.fs:115),
// not a string enum. Read without one, a Buffer, turned into a string by its default (utf8).
let buffer: NonSharedBuffer = Fs.readFileSync file
let content = buffer.toString ()
check (content = "héllo\nworld\nmore\n") "readFileSync"
// FINDING 13 too: Buffer inherits `Uint8Array<_>`, also an empty interface (Bindings.Prelude.fs:112): no length, no bytes.
let names = Fs.readdirSync (dir, U2.Case1 {| encoding = None; withFileTypes = None; recursive = None |})
let name (n: StringOrNonSharedBuffer) = match n with StringOrNonSharedBuffer.String s -> s | _ -> "?"
check (names |> Seq.map name |> List.ofSeq |> List.sort = [ "hello.txt"; "sub" ]) "readdirSync"
let stat = Fs.statSync file
check (stat.isFile () && not (stat.isDirectory ())) "statSync: a file"
check (Fs.statSync(join dir "sub").isDirectory ()) "statSync: a directory"
check (stat.size = 18.0) "size in bytes"
Fs.renameSync (file, join dir "renamed.txt")
check (not (Fs.existsSync file) && Fs.existsSync (join dir "renamed.txt")) "renameSync"
Fs.unlinkSync (join dir "renamed.txt")
check (not (Fs.existsSync (join dir "renamed.txt"))) "unlinkSync"
let rm = Fs.RmOptions.Create (recursive = Some true, force = Some true)
Fs.rmSync (dir, rm)
check (not (Fs.existsSync dir)) "rmSync recursive"

printfn "%d checks passed" passed
