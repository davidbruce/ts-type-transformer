module Program

open Node

let mutable private passed = 0

let private check (ok: bool) (what: string) =
    if ok then
        passed <- passed + 1
    else
        failwithf "check failed: %s" what

let dir = Fs.mkdtempSync "/tmp/tsrs-node-ws-"
check (Fs.existsSync dir && Fs.statSync(dir).isDirectory()) "workspace created"

let notes = path.join (dir, "notes.txt")
Fs.writeFileSync (notes, "alpha\nbeta\n")
let st1 = Fs.statSync notes
check (Fs.existsSync notes && st1.isFile () && st1.size = 11.0) "initial write and size"

Fs.appendFileSync (notes, "gamma\n")
let buf: NonSharedBuffer = Fs.readFileSync notes
let text = buf.toString ()
let st2 = Fs.statSync notes
check (text = "alpha\nbeta\ngamma\n" && st2.size = 17.0) "append and readBack"

let backup = path.join (dir, "backup.txt")
let archive = path.join (dir, "archive.txt")
Fs.copyFileSync (notes, backup)
check (Fs.existsSync backup) "copyFileSync"

Fs.renameSync (backup, archive)
check (not (Fs.existsSync backup) && Fs.existsSync archive) "renameSync"

let archBuf: NonSharedBuffer = Fs.readFileSync archive
check (archBuf.toString () = text) "archive matches notes"

Fs.unlinkSync archive
check (not (Fs.existsSync archive)) "unlinkSync"

let rm = Fs.RmOptions.Create(recursive = true, force = true)
Fs.rmSync (dir, rm)
check (not (Fs.existsSync dir)) "rmSync recursive"
check (true) "workspace cleanup verified"

printfn "workspace=%s" dir
printfn "size_before=%.0f size_after=%.0f" st1.size st2.size
printfn "PASS: temp_workspace (%d checks)" passed
