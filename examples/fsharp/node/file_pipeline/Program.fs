module Demos.Node.FilePipeline

open Node

let mutable private passed = 0

let private check (ok: bool) (what: string) =
    if ok then
        passed <- passed + 1
    else
        failwithf "check failed: %s" what

let bus = Events.EventEmitter.Create ()
let mutable stages = 0
let mutable doneCount = 0

bus.on ("stage", NodeJS.AddListenerListener.Create(fun _ -> stages <- stages + 1)) |> ignore
bus.once ("done", NodeJS.AddListenerListener.Create(fun _ -> doneCount <- doneCount + 1)) |> ignore

let root = Fs.mkdtempSync "/tmp/tsrs-node-pipe-"
let inDir = path.join (root, "in")
let outDir = path.join (root, "out")
let mkOpts = {| recursive = true; mode = None |}
Fs.mkdirSync (inDir, mkOpts) |> ignore
Fs.mkdirSync (outDir, mkOpts) |> ignore

// Stage 1: seed
Fs.writeFileSync (path.join (inDir, "a.log"), "100 OK\n")
Fs.writeFileSync (path.join (inDir, "b.log"), "200 OK\n")
Fs.writeFileSync (path.join (inDir, "c.log"), "300 OK\n")
bus.emit ("stage", box "seed") |> ignore
check (stages = 1) "seed stage"

// Stage 2: bundle
let logs =
    Fs.readdirSync inDir
    |> Seq.sort
    |> List.ofSeq

check (logs = [ "a.log"; "b.log"; "c.log" ]) "input logs listed"
let combined = path.join (outDir, "combined.log")
for name in logs do
    check (path.extname name = ".log") "extname is .log"
    let chunk: NonSharedBuffer = Fs.readFileSync (path.join (inDir, name))
    Fs.appendFileSync (combined, chunk.toString ())

bus.emit ("stage", box "bundle") |> ignore
check (stages = 2) "bundle stage"

// Stage 3: verify
let combBuf: NonSharedBuffer = Fs.readFileSync combined
let combText = combBuf.toString ()
check (Fs.statSync(combined).size = 21.0 && combText.Contains("200 OK")) "combined output verified"

let enoentCaught =
    try
        Fs.readFileSync (path.join (outDir, "missing.log")) |> ignore
        false
    with ex ->
        ex.Message.Contains("ENOENT")
check enoentCaught "missing file raised ENOENT"

bus.emit ("stage", box "verify") |> ignore
bus.emit ("done", box combText.Length) |> ignore
check (stages = 3 && doneCount = 1) "pipeline completion events"

let rm = Fs.RmOptions.Create(recursive = true, force = true)
Fs.rmSync (root, rm)
check (not (Fs.existsSync root)) "pipeline workspace removed"

printfn "pipeline_root=%s combined_bytes=%.0f" root (float combText.Length)
printfn "stages=%d done=%d enoent=%b" stages doneCount enoentCaught
printfn "PASS: file_pipeline (%d checks)" passed
