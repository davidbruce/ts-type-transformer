// The CSS object model from F#: inline style, computed style, a style sheet's
// rules, media queries, and classes that change what is computed. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let same (a: obj) (b: obj) = obj.ReferenceEquals(a, b)

let box1: HTMLDivElement = document.createElement HTMLElementTagNameMapKeys.div
box1.id <- "box"
box1.textContent <- "styled"
document.body.appendChild box1 |> ignore

// Inline style: set, read back, remove.
check (box1.style.color = "") "no inline colour yet"
box1.style.color <- "red"
check (box1.style.color = "red") "style.color"
box1.style.setProperty ("background-color", "rgb(0, 128, 0)")
check (box1.style.getPropertyValue "background-color" = "rgb(0, 128, 0)") "getPropertyValue"
check (box1.style.backgroundColor = "rgb(0, 128, 0)") "camelCase property sees it"
box1.style.setProperty ("margin-top", "4px", "important")
check (box1.style.getPropertyPriority "margin-top" = "important") "priority"
check (box1.style.length = 3.0) "style length"
check (box1.style.item 0.0 = "color") "style item"
let removed = box1.style.removeProperty "background-color"
check (removed = "rgb(0, 128, 0)") "removeProperty returns the old value"
check (box1.style.getPropertyValue "background-color" = "") "removed"
check (box1.style.cssText.Contains "color: red") "cssText"
box1.style.cssText <- "width: 120px; height: 40px"
check (box1.style.color = "") "cssText replaces everything"
check (box1.getAttribute "style" = "width: 120px; height: 40px;") "style attribute reflects"
box1.style.display <- "block"
box1.style.padding <- "5px"

// Computed style resolves units and layout.
let computed = Globals.getComputedStyle box1
check (computed.width = "120px") "computed width"
check (computed.height = "40px") "computed height"
check (computed.paddingLeft = "5px") "computed padding"
check (computed.getPropertyValue "display" = "block") "computed display"
check (box1.offsetWidth = 130.0) "layout agrees: width plus padding"
box1.style.color <- "blue"
check (computed.color = "rgb(0, 0, 255)") "computed colour is rgb, and live"
box1.style.fontSize <- "2em"
check (computed.fontSize <> "2em" && computed.fontSize.EndsWith "px") "computed font size in px"

// A <style> element and its sheet.
let styleEl: HTMLStyleElement = document.createElement HTMLElementTagNameMapKeys.style
styleEl.textContent <- ".big { font-size: 30px; } .red { color: red; } #box.marked { border-left: 7px solid black; }"
document.head.appendChild styleEl |> ignore
let sheet = styleEl.sheet
check (not (isNull sheet)) "sheet exists once attached"
check (sheet.cssRules.length = 3.0) "three rules parsed"
let rule = sheet.cssRules.item 0.0 :?> CSSStyleRule
check (rule.selectorText = ".big") "selectorText"
check (rule.style.fontSize = "30px") "rule style"
check (rule.cssText.StartsWith ".big") "rule cssText"
check (rule.``type`` = rule.STYLE_RULE) "rule type"
check (same rule.parentStyleSheet sheet) "parentStyleSheet"
rule.style.fontSize <- "31px"
check (rule.style.getPropertyValue "font-size" = "31px") "rule style set"
rule.selectorText <- ".bigger"
check ((sheet.cssRules.item 0.0).cssText.StartsWith ".bigger") "selectorText set"
rule.selectorText <- ".big"
rule.style.fontSize <- "30px"

// Rules inserted and deleted.
let at = sheet.insertRule (".hidden { display: none; }", 1.0)
check (at = 1.0) "insertRule returns the index"
check (sheet.cssRules.length = 4.0) "four rules"
check ((sheet.cssRules.item 1.0 :?> CSSStyleRule).selectorText = ".hidden") "inserted at the index"
sheet.deleteRule 1.0
check (sheet.cssRules.length = 3.0) "deleteRule"
let rules = [ for r in sheet.cssRules -> r.cssText ]
check (rules.Length = 3) "rules iterate"
let mutable threw = false
try
    sheet.insertRule "not css at all" |> ignore
with _ ->
    threw <- true
check threw "a bad rule throws"

// Classes drive the computed style through the sheet.
let other: HTMLDivElement = document.createElement HTMLElementTagNameMapKeys.div
document.body.appendChild other |> ignore
let before = (Globals.getComputedStyle other).fontSize
other.classList.add "big"
check ((Globals.getComputedStyle other).fontSize = "30px") "class applies a rule"
check (before <> "30px") "before the class it was something else"
other.classList.add "red"
check ((Globals.getComputedStyle other).color = "rgb(255, 0, 0)") "second class"
other.classList.remove "big"
check ((Globals.getComputedStyle other).fontSize = before) "removing the class restores"
// An inline value beats the sheet.
other.style.color <- "green"
check ((Globals.getComputedStyle other).color = "rgb(0, 128, 0)") "inline beats a class"
// A compound selector through the id.
box1.classList.add "marked"
check ((Globals.getComputedStyle box1).borderLeftWidth = "7px") "id and class selector"

// Rules added later take effect at once.
sheet.insertRule ("p.late { margin-left: 13px }", sheet.cssRules.length) |> ignore
let late: HTMLParagraphElement = document.createElement HTMLElementTagNameMapKeys.p
late.className <- "late"
document.body.appendChild late |> ignore
check ((Globals.getComputedStyle late).marginLeft = "13px") "a rule inserted later applies"

// Media queries.
let wide = Globals.matchMedia "(min-width: 1px)"
check wide.matches "min-width 1px matches"
check (wide.media = "(min-width: 1px)") "media text"
check (not (Globals.matchMedia "(max-width: 0px)").matches) "max-width 0 does not"
check ((Globals.matchMedia "(min-width: 99999px)").matches = false) "huge min-width does not"
check ((Globals.matchMedia "screen").matches || (Globals.matchMedia "print").matches = false) "media type"
let mutable changes = 0
let listener: MediaQueryListEvent -> unit =
    let f = fun (_: MediaQueryListEvent) -> changes <- changes + 1
    f
wide.addEventListener (MediaQueryListEventMapKeys.change, listener)
wide.removeEventListener (MediaQueryListEventMapKeys.change, listener)
check (changes = 0) "no change fired"

// A media rule inside a sheet.
sheet.insertRule ("@media (min-width: 1px) { .m { width: 11px } }", 0.0) |> ignore
let mediaRule = sheet.cssRules.item 0.0 :?> CSSMediaRule
check (mediaRule.cssRules.length = 1.0) "media rule holds a rule"
check ((mediaRule.cssRules.item 0.0 :?> CSSStyleRule).selectorText = ".m") "nested rule"
let m: HTMLDivElement = document.createElement HTMLElementTagNameMapKeys.div
m.className <- "m"
document.body.appendChild m |> ignore
check ((Globals.getComputedStyle m).width = "11px") "media rule applies"

// A constructed sheet.
let adopted = CSSStyleSheet.Create ()
adopted.replaceSync ".z { z-index: 9 }"
check (adopted.cssRules.length = 1.0) "constructed sheet"

printfn "%d checks passed" passed
