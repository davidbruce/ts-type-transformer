module Program

open Node

let mutable private passed = 0

let private check (ok: bool) (what: string) =
    if ok then
        passed <- passed + 1
    else
        failwithf "check failed: %s" what

let joined = path.join ("/workspace", "packages", "core", "..", "cli", "index.ts")
check (joined = "/workspace/packages/cli/index.ts") "path.join"

let norm = path.normalize "/var//log/./app/../http/access.log"
check (norm = "/var/log/http/access.log") "path.normalize"

let resolved = path.resolve ("/srv/app", "../shared", "config/prod.json")
check (resolved = "/srv/shared/config/prod.json") "path.resolve"

let rel = path.relative ("/srv/app/src/ui", "/srv/app/dist/assets")
check (rel = "../../dist/assets") "path.relative"

check (path.dirname "/srv/app/archive.tar.gz" = "/srv/app") "path.dirname"
check (path.basename "/srv/app/archive.tar.gz" = "archive.tar.gz") "path.basename"
check (path.basename ("/srv/app/archive.tar.gz", ".gz") = "archive.tar") "path.basename suffix"
check (path.extname "/srv/app/archive.tar.gz" = ".gz") "path.extname"
check (path.isAbsolute "/srv/app" && not (path.isAbsolute "src/app")) "path.isAbsolute"

let parsed = path.parse "/srv/app/archive.tar.gz"
check (parsed.root = "/" && parsed.dir = "/srv/app" && parsed.``base`` = "archive.tar.gz" && parsed.name = "archive.tar" && parsed.ext = ".gz") "path.parse"

let formatted = path.format (path.FormatInputPathObject.Create(dir = "/etc/service", name = "daemon", ext = ".conf"))
check (formatted = "/etc/service/daemon.conf") "path.format"

check (path.matchesGlob ("/src/components/Button.tsx", "/src/**/*.tsx") && not (path.matchesGlob ("/src/components/Button.css", "/src/**/*.tsx"))) "path.matchesGlob"

printfn "join=%s norm=%s" joined norm
printfn "resolve=%s relative=%s format=%s" resolved rel formatted
printfn "parsed=(root=%s dir=%s base=%s ext=%s)" parsed.root parsed.dir parsed.``base`` parsed.ext
printfn "PASS: path_inspector (%d checks)" passed
