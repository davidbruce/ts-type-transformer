// fetch, Request, Response, Headers and Blob from F#: a page file is fetched
// and read, messages are built directly, and a fetch is aborted. The checks
// run as the promises settle; the report comes last. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings
open type Bindings.Globals

module Async =
    let map f a = async { let! x = a in return f x }

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let wait (p: JS.Promise<'T>) = Async.AwaitPromise p

// Headers built directly.
let headers = Headers.Create (!^ (ResizeArray [ ("X-One", "1"); ("Content-Type", "text/plain") ]))
check (headers.get "x-one" = "1") "header names are case-insensitive"
check (headers.has "Content-Type") "has"
headers.append ("X-One", "2")
check (headers.get "X-One" = "1, 2") "append joins values"
headers.set ("X-One", "3")
check (headers.get "X-One" = "3") "set replaces"
headers.delete "X-One"
check (not (headers.has "X-One")) "delete"
check (isNull (headers.get "X-One")) "a missing header is null"
let names = [ for (k, _) in headers -> k ]
check (names = [ "content-type" ]) "headers iterate, names lower-cased"
let copied = Headers.Create (!^ headers)
check (copied.get "content-type" = "text/plain") "Headers from Headers"

// A Request built directly.
let request =
    Request.Create (
        "https://example.com/api?q=1",
        let init = createEmpty<RequestInit>
        init.``method`` <- Some "POST"
        init.headers <- Some (HeadersInit.Headers headers)
        init
    )
// FINDING 4: RequestInit has no Create (Bindings.13.network.fs:20, a plain class, not Interface + ParamObject), so the init is made with createEmpty and field assignment.
check (request.url = "https://example.com/api?q=1") "request url"
check (request.``method`` = "POST") "request method"
check (request.headers.get "content-type" = "text/plain") "request headers"
check (request.mode = RequestMode.Cors) "default request mode"
check (not request.bodyUsed) "no body used"
let again = request.clone ()
check (again.url = request.url) "clone"
let getRequest = Request.Create "./index.html"
check (getRequest.``method`` = "GET") "default method is GET"
check (getRequest.url.EndsWith "/index.html") "a relative url is resolved"

// A Response built directly.
let made = Response.Create (!^ "hello body", ResponseInit.Create (status = 201.0, statusText = "Made", headers = !^ (Headers.Create (!^ (ResizeArray [ ("X-Made", "yes") ])))))
check (made.status = 201.0) "response status"
check (made.statusText = "Made") "statusText"
check made.ok "201 is ok"
check (made.headers.get "x-made" = "yes") "response headers"
check (made.``type`` = ResponseType.Default) "response type"
let failure = Response.Create (!^ "nope", ResponseInit.Create (status = 404.0))
check (not failure.ok) "404 is not ok"
check (Response.error().``type`` = ResponseType.Error) "Response.error"
let redirect = Response.redirect (!^ "https://example.com/there", 301.0)
check (redirect.status = 301.0 && redirect.headers.get "location" = "https://example.com/there") "Response.redirect"

// Blob made directly.
let blob = Blob.Create (ResizeArray [ BlobPart.String "ab"; BlobPart.String "cde" ])
check (blob.size = 5.0) "blob size"
check (blob.``type`` = "") "blob has no type"
let typed =
    Blob.Create (
        ResizeArray [ BlobPart.String "{\"k\":1}" ],
        let o = createEmpty<BlobPropertyBag>
        o.``type`` <- Some "application/json"
        o
    )
// FINDING 5: BlobPropertyBag has no Create either (Bindings.07.storage_base.fs:7, a plain class), so createEmpty again.
check (typed.``type`` = "application/json") "blob type"
let sliced = blob.slice (1.0, 3.0)
check (sliced.size = 2.0) "blob slice"

async {
    // A Blob's text.
    let! text = wait (blob.text ())
    check (text = "abcde") "blob text"
    let! sliceText = wait (sliced.text ())
    check (sliceText = "bc") "slice text"
    let! bytes = wait (blob.bytes ())
    check (bytes.length = 5 && bytes.[0] = 97uy) "blob bytes"

    // Response bodies.
    let! madeText = wait (made.text ())
    check (madeText = "hello body") "response text"
    check made.bodyUsed "body is used after reading"
    let! fromBlob = wait ((Response.Create (!^ typed)).json ())
    check (fromBlob?k = 1) "json from a blob body"
    let cloneSource = Response.Create (!^ "twice")
    let copy = cloneSource.clone ()
    let! first = wait (cloneSource.text ())
    let! second = wait (copy.text ())
    check (first = "twice" && second = "twice") "a clone reads separately"

    // Response.json.
    let payload = createObj [ "name" ==> "ada"; "tags" ==> [| "a"; "b" |]; "n" ==> 3 ]
    let jsonResponse = Response.json payload
    check (jsonResponse.headers.get "content-type" = "application/json") "Response.json content type"
    let! parsed = wait (jsonResponse.json ())
    check (parsed?name = "ada" && parsed?n = 3) "Response.json body"
    check ((parsed?tags: string[]).Length = 2) "Response.json array"

    // A request's body is read like a response's.
    let post =
        Request.Create (
            "https://example.com/post",
            let init = createEmpty<RequestInit>
            init.``method`` <- Some "POST"
            init.body <- Some (BodyInit.String "sent text")
            init
        )
    check (post.``method`` = "POST") "a POST request"
    let! sent = wait (post.text ())
    check (sent = "sent text") "request body text"
    check post.bodyUsed "request body used"
    let formBody = FormData.Create ()
    formBody.append ("k", "v")
    let! fromForm = wait ((Request.Create ("https://example.com/f", (let i = createEmpty<RequestInit> in i.``method`` <- Some "POST"; i.body <- Some (BodyInit.FormData formBody); i))).formData ())
    check (match fromForm.get "k" with FormDataEntryValue.String v -> v = "v" | _ -> false) "request FormData body"

    // fetch of a page file.
    let! response = wait (fetch "./index.html")
    check response.ok "fetch ok"
    check (response.status = 200.0) "status 200"
    check (response.statusText = "OK") "statusText OK"
    check (response.``type`` = ResponseType.Basic) "same-origin response is basic"
    check (response.url.EndsWith "/index.html") "response url"
    check (not response.redirected) "not redirected"
    check ((response.headers.get "content-type").StartsWith "text/html") "content-type"
    let! body = wait (response.text ())
    check (body.Contains "<body") "page text has a body"
    check (body.Contains "<title>") "page text has a title"
    check response.bodyUsed "body used after text()"

    // Reading twice fails; clone first.
    let! second = wait (fetch (!^ getRequest))
    let dup = second.clone ()
    let! a = wait (second.text ())
    let! b = wait (dup.text ())
    check (a = b && a.Length > 10) "cloned fetch response reads twice"
    let! reuse = Async.Catch (wait (second.text ()))
    check (match reuse with Choice2Of2 _ -> true | _ -> false) "reading a used body rejects"

    // blob() of a fetched file.
    let! third = wait (fetch "./index.html")
    let! asBlob = wait (third.blob ())
    check (asBlob.size = float a.Length || asBlob.size > 0.0) "response blob"
    check (asBlob.``type``.StartsWith "text/html") "response blob type"

    // A 404 resolves with ok = false rather than rejecting.
    let! missing = wait (fetch "/status/404")
    check (not missing.ok && missing.status = 404.0) "fetch 404 resolves not ok"
    let! missingText = wait (missing.text ())
    check (missingText = "status 404") "fetch 404 body"

    // A fetch is aborted by its signal.
    let controller = AbortController.Create ()
    let init = createEmpty<RequestInit>
    init.signal <- Some controller.signal
    let pending = fetch ("./index.html", init)
    controller.abort ()
    let! outcome = Async.Catch (wait pending)
    match outcome with
    | Choice2Of2 e -> check (e.Message.ToLower().Contains "abort") "aborted fetch rejects with an abort error"
    | Choice1Of2 _ -> check false "aborted fetch rejects"
    check controller.signal.aborted "signal aborted"

    // An already-aborted signal rejects at once.
    let init2 = createEmpty<RequestInit>
    init2.signal <- Some (AbortSignal.abort ())
    let! outcome2 = Async.Catch (wait (fetch ("./index.html", init2)))
    check (match outcome2 with Choice2Of2 _ -> true | _ -> false) "pre-aborted signal rejects"

    // A bad URL scheme is a rejection (TypeError), not a status.
    let! bad = Async.Catch (wait (fetch "http://"))
    check (match bad with Choice2Of2 _ -> true | _ -> false) "an unparseable URL rejects"

    printfn "%d checks passed" passed
}
|> Async.Catch
|> Async.map (function Choice2Of2 e -> printfn "FAILED: %s" e.Message; raise e | _ -> ())
|> Async.StartImmediate
