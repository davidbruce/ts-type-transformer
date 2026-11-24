module Demos.Node.DirectoryTree

open Node

let mutable private passed = 0

let private check (ok: bool) (what: string) =
    if ok then
        passed <- passed + 1
    else
        failwithf "check failed: %s" what

let root = Fs.mkdtempSync "/tmp/tsrs-node-tree-"
check (Fs.existsSync root) "temp tree created"

let mkOpts = {| recursive = true; mode = None |}
Fs.mkdirSync (path.join (root, "src", "core"), mkOpts) |> ignore
Fs.mkdirSync (path.join (root, "src", "cli"), mkOpts) |> ignore
Fs.mkdirSync (path.join (root, "docs"), mkOpts) |> ignore

Fs.writeFileSync (path.join (root, "README.md"), "node tree demo")
Fs.writeFileSync (path.join (root, "src", "core", "engine.txt"), "engine core module!\n")
Fs.writeFileSync (path.join (root, "src", "cli", "args.txt"), "cli args parser\n")
Fs.writeFileSync (path.join (root, "docs", "guide.md"), "monochrome documentation\n")

let mutable dirCount = 0
let mutable fileCount = 0
let mutable totalBytes = 0.0

let rec walk (dir: string) =
    for name in Fs.readdirSync dir |> Seq.sort do
        let full = path.join (dir, name)
        let st = Fs.statSync full
        if st.isDirectory () then
            dirCount <- dirCount + 1
            walk full
        elif st.isFile () then
            fileCount <- fileCount + 1
            totalBytes <- totalBytes + st.size

walk root

check (dirCount = 4) "dir count = 4"
check (fileCount = 4) "file count = 4"
check (totalBytes = 75.0) "total bytes = 75"

let rm = Fs.RmOptions.Create(recursive = true, force = true)
Fs.rmSync (root, rm)
check (not (Fs.existsSync root)) "tree cleaned up"
check (dirCount + fileCount = 8) "total entries = 8"
check (true) "walk complete"

printfn "root=%s" root
printfn "dirs=%d files=%d total_bytes=%.0f" dirCount fileCount totalBytes
printfn "PASS: directory_tree (%d checks)" passed
