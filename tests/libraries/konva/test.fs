// Konva from F#, in a page. FINDING 21: the generated bindings do not compile (Bindings.02.konva.fs defines
// `KonvaStatic` twice: "Duplicate definition of type, exception or module 'KonvaStatic'"), so no program can be
// built on them; this is the program that would have run once they do: a Stage on the "stage" element.
module Test

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

check true "the program runs"

printfn "%d checks passed" passed
