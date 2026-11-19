// MutationObserver, ResizeObserver, IntersectionObserver and AbortSignal.timeout
// from F#. Callbacks run later, so the checks run as they fire and the report
// comes last. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings

module Async =
    let map f a = async { let! x = a in return f x }

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let same (a: obj) (b: obj) = obj.ReferenceEquals(a, b)

// Wait until a condition holds, polling; fail after two seconds.
let waitFor (what: string) (cond: unit -> bool) =
    async {
        let mutable waited = 0
        while not (cond ()) && waited < 2000 do
            do! Async.Sleep 10
            waited <- waited + 10
        check (cond ()) what
    }

let div (id: string) : HTMLDivElement =
    let d: HTMLDivElement = document.createElement HTMLElementTagNameMapKeys.div
    d.id <- id
    d

async {
    // ---- MutationObserver: childList ----
    let host = div "host"
    document.body.appendChild host |> ignore
    let records = ResizeArray<MutationRecord>()
    let observer = MutationObserver.Create (fun batch _ -> records.AddRange batch)
    observer.observe (host, MutationObserverInit.Create (childList = true))
    let child = div "child1"
    host.appendChild child |> ignore
    check (records.Count = 0) "records are delivered later, not synchronously"
    do! waitFor "childList record delivered" (fun () -> records.Count = 1)
    check (records.[0].``type`` = MutationRecordType.ChildList) "record type childList"
    check (same records.[0].target host) "record target"
    check (records.[0].addedNodes.length = 1.0 && same (records.[0].addedNodes.item 0.0) child) "addedNodes"
    check (records.[0].removedNodes.length = 0.0) "no removedNodes yet"
    host.removeChild child |> ignore
    do! waitFor "removal record" (fun () -> records.Count = 2)
    check (records.[1].removedNodes.length = 1.0) "removedNodes"
    check (records.[1].addedNodes.length = 0.0) "nothing added by a removal"

    // Several mutations arrive in one batch.
    let batches = ResizeArray<int>()
    let batchObserver = MutationObserver.Create (fun batch _ -> batches.Add batch.Count)
    batchObserver.observe (host, MutationObserverInit.Create (childList = true))
    host.appendChild (div "a") |> ignore
    host.appendChild (div "b") |> ignore
    host.appendChild (div "c") |> ignore
    do! waitFor "one batch" (fun () -> batches.Count = 1)
    check (batches.[0] = 3) "three records in one batch"
    batchObserver.disconnect ()
    host.replaceChildren ()
    records.Clear ()

    // ---- attributes, with old values and a filter ----
    let target = div "target"
    document.body.appendChild target |> ignore
    let attrRecords = ResizeArray<MutationRecord>()
    let attrObserver = MutationObserver.Create (fun batch _ -> attrRecords.AddRange batch)
    attrObserver.observe (target, MutationObserverInit.Create (attributes = true, attributeOldValue = true, attributeFilter = ResizeArray [ "title"; "data-x" ]))
    target.setAttribute ("title", "one")
    target.setAttribute ("title", "two")
    target.setAttribute ("data-other", "ignored")
    target.setAttribute ("data-x", "1")
    do! waitFor "attribute records" (fun () -> attrRecords.Count = 3)
    check (attrRecords.[0].``type`` = MutationRecordType.Attributes) "record type attributes"
    check (attrRecords.[0].attributeName = "title") "attributeName"
    check (isNull attrRecords.[0].oldValue) "old value of a new attribute is null"
    check (attrRecords.[1].oldValue = "one") "attributeOldValue"
    check (attrRecords.[2].attributeName = "data-x") "filtered attribute seen"
    check (attrRecords |> Seq.forall (fun r -> r.attributeName <> "data-other")) "the filter hid other attributes"

    // ---- characterData ----
    let text = document.createTextNode "before"
    host.appendChild text |> ignore
    let dataRecords = ResizeArray<MutationRecord>()
    let dataObserver = MutationObserver.Create (fun batch _ -> dataRecords.AddRange batch)
    dataObserver.observe (text, MutationObserverInit.Create (characterData = true, characterDataOldValue = true))
    text.data <- "after"
    do! waitFor "characterData record" (fun () -> dataRecords.Count = 1)
    check (dataRecords.[0].``type`` = MutationRecordType.CharacterData) "record type characterData"
    check (dataRecords.[0].oldValue = "before") "characterDataOldValue"
    check (text.data = "after") "the data changed"

    // ---- subtree ----
    let outer = div "outer"
    let inner = div "inner"
    outer.appendChild inner |> ignore
    document.body.appendChild outer |> ignore
    let deepRecords = ResizeArray<MutationRecord>()
    let deepObserver = MutationObserver.Create (fun batch _ -> deepRecords.AddRange batch)
    deepObserver.observe (outer, MutationObserverInit.Create (childList = true, subtree = true))
    inner.appendChild (div "deep") |> ignore
    do! waitFor "subtree record" (fun () -> deepRecords.Count = 1)
    check (same deepRecords.[0].target inner) "subtree sees the nested target"
    deepObserver.disconnect ()

    // ---- takeRecords and disconnect ----
    let taken = ResizeArray<MutationRecord>()
    let takeObserver = MutationObserver.Create (fun batch _ -> taken.AddRange batch)
    takeObserver.observe (host, MutationObserverInit.Create (childList = true))
    host.appendChild (div "t1") |> ignore
    host.appendChild (div "t2") |> ignore
    let pending = takeObserver.takeRecords ()
    check (pending.Count = 2) "takeRecords returns pending records"
    check (takeObserver.takeRecords().Count = 0) "takeRecords empties the queue"
    do! Async.Sleep 30
    check (taken.Count = 0) "taken records are not delivered"
    host.appendChild (div "t3") |> ignore
    takeObserver.disconnect ()
    do! Async.Sleep 30
    check (taken.Count = 0) "disconnect drops queued records"
    host.appendChild (div "t4") |> ignore
    do! Async.Sleep 30
    check (taken.Count = 0) "a disconnected observer sees nothing"

    // ---- ResizeObserver ----
    let sized = div "sized"
    sized.style.width <- "100px"
    sized.style.height <- "50px"
    document.body.appendChild sized |> ignore
    let resizes = ResizeArray<ResizeObserverEntry>()
    let resizeObserver = ResizeObserver.Create (fun entries _ -> resizes.AddRange entries)
    resizeObserver.observe sized
    do! waitFor "ResizeObserver fired on observe" (fun () -> resizes.Count >= 1)
    check (same resizes.[0].target sized) "resize entry target"
    check (resizes.[0].contentRect.width = 100.0 && resizes.[0].contentRect.height = 50.0) "contentRect"
    check (resizes.[0].contentBoxSize.[0].inlineSize = 100.0 && resizes.[0].contentBoxSize.[0].blockSize = 50.0) "contentBoxSize"
    check (resizes.[0].borderBoxSize.Count = 1) "borderBoxSize"
    let before = resizes.Count
    sized.style.width <- "200px"
    do! waitFor "ResizeObserver fired again after a change" (fun () -> resizes.Count > before)
    check (resizes.[resizes.Count - 1].contentRect.width = 200.0) "new width observed"
    resizeObserver.unobserve sized
    let after = resizes.Count
    sized.style.width <- "300px"
    do! Async.Sleep 100
    check (resizes.Count = after) "unobserve stops the callbacks"
    resizeObserver.disconnect ()

    // ---- IntersectionObserver ----
    let visible = div "visible"
    visible.style.width <- "50px"
    visible.style.height <- "50px"
    document.body.insertBefore (visible, document.body.firstChild) |> ignore
    let hits = ResizeArray<IntersectionObserverEntry>()
    // FINDING 7: IntersectionObserverInit.threshold is typed VibratePattern (Bindings.10.core.fs:4363), a float | float[] union named for another API; usable (`!^ 0.0`), but the name is wrong.
    let intersection = IntersectionObserver.Create ((fun entries _ -> hits.AddRange entries), IntersectionObserverInit.Create (threshold = !^ 0.0))
    check (intersection.thresholds.Count = 1 && intersection.thresholds.[0] = 0.0) "thresholds"
    check (intersection.rootMargin = "0px 0px 0px 0px") "rootMargin"
    intersection.observe visible
    do! waitFor "IntersectionObserver fired" (fun () -> hits.Count >= 1)
    check (same hits.[0].target visible) "intersection target"
    check hits.[0].isIntersecting "an element in view intersects"
    check (hits.[0].intersectionRatio > 0.99) "fully visible"
    check (hits.[0].boundingClientRect.width = 50.0) "boundingClientRect"
    check (hits.[0].intersectionRect.height = 50.0) "intersectionRect"
    // Moved far out of view.
    let seen = hits.Count
    visible.style.position <- "fixed"
    visible.style.top <- "-5000px"
    do! waitFor "IntersectionObserver fired when it left" (fun () -> hits.Count > seen)
    check (not hits.[hits.Count - 1].isIntersecting) "out of view does not intersect"
    intersection.unobserve visible
    intersection.disconnect ()

    // ---- AbortSignal.timeout ----
    let signal = AbortSignal.timeout 40.0
    check (not signal.aborted) "the timeout signal starts live"
    let mutable abortEvents = 0
    signal.addEventListener ("abort", fun (_: Event) -> abortEvents <- abortEvents + 1)
    do! waitFor "the signal aborted after its timeout" (fun () -> signal.aborted)
    do! Async.Sleep 10
    check (abortEvents = 1) "abort event fired once"
    check (signal.reason?name = "TimeoutError") "reason is a TimeoutError"
    let mutable threw = false
    try
        signal.throwIfAborted ()
    with _ ->
        threw <- true
    check threw "throwIfAborted throws"
    check ((AbortSignal.abort ()).aborted) "AbortSignal.abort is already aborted"
    let controller = AbortController.Create ()
    let combined = AbortSignal.any (ResizeArray [ controller.signal ])
    check (not combined.aborted) "AbortSignal.any starts live"
    controller.abort ()
    check combined.aborted "AbortSignal.any follows its sources"

    printfn "%d checks passed" passed
}
|> Async.Catch
|> Async.map (function Choice2Of2 e -> printfn "FAILED: %s" e.Message; raise e | _ -> ())
|> Async.StartImmediate
