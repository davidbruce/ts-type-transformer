// URL and URLSearchParams from F#: parsed, taken apart, rebuilt and resolved.
// Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let throws (f: unit -> unit) =
    try
        f ()
        false
    with _ ->
        true

// Parse and read.
let u = URL.Create "https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag"
check (u.protocol = "https:") "protocol"
check (u.host = "example.com:8080") "host"
check (u.hostname = "example.com") "hostname"
check (u.port = "8080") "port"
check (u.pathname = "/a/b/c.html") "pathname"
check (u.search = "?x=1&y=two") "search"
check (u.hash = "#frag") "hash"
check (u.username = "user" && u.password = "pw") "credentials"
check (u.origin = "https://example.com:8080") "origin"
check (u.href = "https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag") "href"
check (u.toString () = u.href) "toString"
check (u.toJSON () = u.href) "toJSON"

// Defaults and normalisation.
let d = URL.Create "HTTP://Example.COM:80/a/../b/./c?q#"
check (d.href = "http://example.com/b/c?q#") "normalised: host lower-cased, default port dropped, dots resolved"
check (d.port = "") "default port is empty"
check (d.protocol = "http:") "protocol lower-cased"
check ((URL.Create "https://example.com").pathname = "/") "empty path is /"
check ((URL.Create "https://example.com/a b").pathname = "/a%20b") "spaces are escaped"
check ((URL.Create "file:///tmp/x.txt").pathname = "/tmp/x.txt") "file URL"
check ((URL.Create "mailto:ada@example.com").pathname = "ada@example.com") "mailto"
check ((URL.Create "https://example.com/?a=1").hash = "") "no hash"

// Build by setting parts.
let b = URL.Create "https://example.com/"
b.pathname <- "/search"
b.search <- "?q=rust"
b.hash <- "top"
b.port <- "8443"
check (b.href = "https://example.com:8443/search?q=rust#top") "built from parts"
b.protocol <- "http"
check (b.protocol = "http:") "protocol set"
b.hostname <- "localhost"
check (b.host = "localhost:8443") "hostname set"
b.host <- "other.test"
check (b.port = "" || b.port = "8443") "host set keeps or drops the port"
b.href <- "https://reset.example/path"
check (b.hostname = "reset.example" && b.pathname = "/path") "href set"
b.username <- "me"
check (b.href = "https://me@reset.example/path") "username set"

// Relative resolution.
let baseUrl = "https://example.com/dir/sub/page.html?old=1#h"
check ((URL.Create ("other.html", baseUrl)).href = "https://example.com/dir/sub/other.html") "sibling"
check ((URL.Create ("../up.html", baseUrl)).href = "https://example.com/dir/up.html") "parent"
check ((URL.Create ("/root.html", baseUrl)).href = "https://example.com/root.html") "absolute path"
check ((URL.Create ("?new=2", baseUrl)).href = "https://example.com/dir/sub/page.html?new=2") "query only"
check ((URL.Create ("#other", baseUrl)).href = "https://example.com/dir/sub/page.html?old=1#other") "fragment only"
check ((URL.Create ("//cdn.test/x.js", baseUrl)).href = "https://cdn.test/x.js") "scheme-relative"
check ((URL.Create ("http://abs.test/", baseUrl)).host = "abs.test") "an absolute URL ignores the base"
check ((URL.Create u).href = u.href) "a URL object as input"

// canParse and parse.
check (URL.canParse "https://example.com") "canParse absolute"
check (not (URL.canParse "/relative")) "canParse a relative path without base"
check (URL.canParse ("/relative", "https://example.com")) "canParse with base"
check (not (URL.canParse "http://")) "canParse empty host"
check (not (URL.canParse "")) "canParse empty"
check (URL.parse "nonsense").IsNone "parse failure is null"
check ((URL.parse "https://ok.test/x").Value.pathname = "/x") "parse"
check (throws (fun () -> URL.Create "not a url" |> ignore)) "the constructor throws on a bad URL"

// searchParams is live: it follows the URL's search.
let sp = u.searchParams
check (sp.get "x" = Some "1" && sp.get "y" = Some "two") "searchParams read"
check (sp.size = 2.0) "searchParams size"
sp.append ("z", "3 4")
check (u.search = "?x=1&y=two&z=3+4") "appending updates the URL"
u.search <- "?k=v"
check (sp.get "k" = Some "v" && not (sp.has "x")) "setting search updates the params"

// URLSearchParams on their own.
let p = URLSearchParams.Create "b=2&a=1&a=3&c=%20spaced%26"
check (p.get "a" = Some "1") "get returns the first"
check ((List.ofSeq (p.getAll "a")) = [ "1"; "3" ]) "getAll"
check (p.get "nope").IsNone "get of a missing key is null"
check (p.get "c" = Some " spaced&") "percent-decoding"
check (p.has "b" && not (p.has "z")) "has"
check (p.has ("a", "3") && not (p.has ("a", "9"))) "has with a value"
p.append ("a", "4")
check (p.getAll("a").Count = 3) "append"
p.set ("a", "only")
check (p.getAll("a").Count = 1 && p.get "a" = Some "only") "set collapses"
p.delete "b"
check (not (p.has "b")) "delete"
p.append ("b", "x")
p.append ("b", "y")
p.delete ("b", "x")
check ((List.ofSeq (p.getAll "b")) = [ "y" ]) "delete with a value"
p.sort ()
check ([ for (k, _) in p -> k ] = [ "a"; "b"; "c" ]) "sort orders by key"
check (p.toString () = "a=only&b=y&c=+spaced%26") "toString encodes"
check (p.size = 3.0) "size"

// Iteration forms.
let pairs = [ for (k, v) in p -> k + "=" + v ]
check (pairs = [ "a=only"; "b=y"; "c= spaced&" ]) "iterate pairs"
let keys = [ for k in p.keys () -> k ]
check (keys = [ "a"; "b"; "c" ]) "keys"
let values = [ for v in p.values () -> v ]
check (values = [ "only"; "y"; " spaced&" ]) "values"
let seen = ResizeArray<string>()
p.forEach (fun v k _ -> seen.Add(k + ":" + v))
check (seen.Count = 3 && seen.[0] = "a:only") "forEach"

// Built from other inits.
let fromPairs = URLSearchParams.Create (ResizeArray [ ResizeArray [ "p"; "1" ]; ResizeArray [ "q"; "2" ] ])
check (fromPairs.toString () = "p=1&q=2") "from pairs"
let fromParams = URLSearchParams.Create fromPairs
fromParams.append ("r", "3")
check (fromParams.toString () = "p=1&q=2&r=3" && fromPairs.size = 2.0) "from another, a copy"
check ((URLSearchParams.Create ()).size = 0.0) "empty"
check ((URLSearchParams.Create "?lead=1").get "lead" = Some "1") "a leading ? is dropped"

// Encoding of reserved characters when set through the URL.
let e = URL.Create "https://example.com/"
e.searchParams.set ("q", "a&b=c d")
check (e.search = "?q=a%26b%3Dc+d") "reserved characters encoded"
e.searchParams.delete "q"
check (e.search = "" && e.href = "https://example.com/") "deleting the last param drops the ?"

// The page's own location.
let here = URL.Create document.location.href
check (here.protocol = "http:" || here.protocol = "https:") "the page URL"
check ((URL.Create ("./index.html", document.baseURI)).pathname.EndsWith "/index.html") "resolved against the document base"

printfn "%d checks passed" passed
