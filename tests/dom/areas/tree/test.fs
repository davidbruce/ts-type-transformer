// The DOM tree from F#: nodes made, attached, moved, cloned, searched and
// walked. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let same (a: obj) (b: obj) = obj.ReferenceEquals(a, b)

let body = document.body

// A list built from elements and text.
let list: HTMLUListElement = document.createElement HTMLElementTagNameMapKeys.ul
list.id <- "list"
for word in [ "one"; "two"; "three" ] do
    let item: HTMLLIElement = document.createElement HTMLElementTagNameMapKeys.li
    item.appendChild (document.createTextNode word) |> ignore
    item.setAttribute ("data-word", word)
    list.appendChild item |> ignore
body.appendChild list |> ignore
check (list.childElementCount = 3.0) "childElementCount"
check (list.textContent = "onetwothree") "textContent of a list"
check (list.firstElementChild.Value.textContent = "one") "firstElementChild"
check (list.lastElementChild.Value.getAttribute "data-word" = Some "three") "lastElementChild attribute"
check (same (document.getElementById "list").Value list) "getElementById finds the list"
check list.isConnected "list is connected"

// Parents, siblings and containment.
let two = (list.children.item 1.0).Value
check (same two.parentElement.Value list) "parentElement"
check (same two.parentNode.Value list) "parentNode"
check (two.previousElementSibling.Value.textContent = "one") "previousElementSibling"
check (two.nextElementSibling.Value.textContent = "three") "nextElementSibling"
check (body.contains two) "body contains"
check (two.matches "li[data-word=two]") "matches"
// FINDING 1: the value `Option` (the HTML <option> constructor) hides F#'s Option module.
check ((two.closest<Element> "ul").Value.id = "list") "closest"
check ((two.closest<Element> "table").IsNone) "closest finds nothing"
let fc = two.firstChild.Value
check (fc.nodeType = two.TEXT_NODE) "first child is text"
check fc.nextSibling.IsNone "text has no next sibling"
check (two.nodeName = "LI") "nodeName"

// Moved: appending a node that is in the tree moves it.
list.appendChild two |> ignore
check (list.lastElementChild.Value.textContent = "two") "append moves"
check (list.textContent = "onethreetwo") "order after move"

// insertBefore.
let half: HTMLLIElement = document.createElement HTMLElementTagNameMapKeys.li
half.textContent <- "half"
list.insertBefore (half, list.firstChild.Value) |> ignore
check (list.firstElementChild.Value.textContent = "half") "insertBefore"
check (list.childElementCount = 4.0) "four children"

// Removed and replaced.
let first = list.firstElementChild.Value
let zero: HTMLLIElement = document.createElement HTMLElementTagNameMapKeys.li
zero.textContent <- "zero"
let old = list.replaceChild (zero, first)
check (same old first) "replaceChild returns the old node"
check first.parentNode.IsNone "replaced node has no parent"
check (list.firstElementChild.Value.textContent = "zero") "replaced"
two.remove ()
check (list.childElementCount = 3.0) "remove"
check two.parentNode.IsNone "removed node has no parent"
list.removeChild (list.lastElementChild.Value) |> ignore
check (list.childElementCount = 2.0) "removeChild"
list.appendChild (document.createElement HTMLElementTagNameMapKeys.li) |> ignore
(list.lastElementChild.Value :?> HTMLElement).textContent <- "three"

// A fragment's children move in at once; it is left empty.
let fragment = document.createDocumentFragment ()
for word in [ "a"; "b" ] do
    let item: HTMLLIElement = document.createElement HTMLElementTagNameMapKeys.li
    item.textContent <- word
    fragment.appendChild item |> ignore
check (fragment.childElementCount = 2.0) "fragment holds two"
list.appendChild fragment |> ignore
check (list.childElementCount = 5.0) "fragment moved in"
check (fragment.childNodes.length = 0.0) "fragment emptied"

// A deep clone copies the subtree; a shallow one does not.
let deep = list.cloneNode true
let shallow = list.cloneNode false
check (deep.textContent = Some list.textContent) "deep clone text"
check (shallow.childNodes.length = 0.0) "shallow clone is empty"
check (deep.isEqualNode list && not (deep.isSameNode list)) "clone is equal, not same"

// Markup parsed and written.
let host: HTMLDivElement = document.createElement HTMLElementTagNameMapKeys.div
host.innerHTML <- "<p class=\"x\">hi <b>there</b></p>"
check ((host.querySelector<Element> "b").Value.textContent = "there") "querySelector"
check (host.innerHTML = "<p class=\"x\">hi <b>there</b></p>") "innerHTML"
let p = host.firstElementChild.Value
check (p.outerHTML.StartsWith "<p class=\"x\">") "outerHTML"
check (p.classList.contains "x") "classList.contains"
p.classList.toggle "y" |> ignore
check (p.getAttribute "class" = Some "x y") "classList.toggle"
p.classList.add ("z", "w")
check (p.classList.length = 4.0) "classList.add many"
p.classList.remove "w"
check (p.className = "x y z") "className"
check ((host.querySelectorAll<Element> "p, b").length = 2.0) "querySelectorAll"

// Positions compared: `zero` precedes `a`.
let a = (list.children.item 3.0).Value
let position = zero.compareDocumentPosition a
check (int position &&& 4 <> 0) "compareDocumentPosition following"
check (int (a.compareDocumentPosition zero) &&& 2 <> 0) "compareDocumentPosition preceding"
check (int (list.compareDocumentPosition zero) &&& 16 <> 0) "contained by"

// A walk over the list's elements.
let walker = document.createTreeWalker (list, NodeFilter.SHOW_ELEMENT)
let seen = ResizeArray<string>()
let mutable node = walker.nextNode ()
while node.IsSome do
    seen.Add node.Value.textContent.Value
    node <- walker.nextNode ()
check (List.ofSeq seen = [ "zero"; "one"; "three"; "a"; "b" ]) "tree walker"
check (same walker.root list) "walker root"
check (list.hasChildNodes ()) "hasChildNodes"

// Whole text and normalize.
let t = document.createTextNode "ab"
host.appendChild t |> ignore
host.appendChild (document.createTextNode "cd") |> ignore
host.normalize ()
check (host.lastChild.Value.textContent = Some "abcd") "normalize merges text"

printfn "%d checks passed" passed
