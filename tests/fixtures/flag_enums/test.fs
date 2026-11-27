module Test

open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let rw = Perms.Read ||| Perms.Write
check (rw = Perms.ReadWrite) "combine"
check (rw.HasFlag(Perms.Read)) "has read"
check (rw.HasFlag(Perms.Write)) "has write"
check (not (rw.HasFlag(Perms.Exec))) "no exec"

let re = combinePerms (Perms.Read, Perms.Exec)
check (hasPerm (re, Perms.Read) && hasPerm (re, Perms.Exec) && not (hasPerm (re, Perms.Write))) "js roundtrip"
check (defaultPerms () = (Perms.Read ||| Perms.Exec)) "default none"
check (defaultPerms Perms.Write = Perms.Write) "default some"

printfn "%d checks passed" passed
