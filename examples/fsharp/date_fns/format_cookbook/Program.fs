module Demos.DateFns.FormatCookbook

open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open DateFns
open type DateFns.Globals

let private el (tag: string) (cls: string) (text: string) =
    let node = document.createElement tag
    if cls <> "" then node.className <- cls
    if text <> "" then node.textContent <- text
    node

let private wrap (tag: string) (cls: string) (children: Dom.HTMLElement list) =
    let node = el tag cls ""
    for ch in children do
        node.appendChild ch |> ignore
    node

let private app = (document.getElementById "app").Value

app.appendChild (wrap "div" "head" [ el "h1" "" "Date-fns Format & Parse Cookbook (F#)"; el "span" "badge" "2024-03-15T14:30:45" ]) |> ignore

let sample: JS.Date = parseISO "2024-03-15T14:30:45"
let parsed: JS.Date = parse ("15/03/2024", "dd/MM/yyyy", !^sample)

let rows =
    [ "formatDate(\"yyyy-MM-dd\")", formatDate (sample, "yyyy-MM-dd"), "ISO date"
      "formatDate(\"EEEE, do MMMM yyyy\")", formatDate (sample, "EEEE, do MMMM yyyy"), "Ordinal full date"
      "formatDate(\"EEE, d MMM yy\")", formatDate (sample, "EEE, d MMM yy"), "Compact short date"
      "formatDate(\"HH:mm:ss\")", formatDate (sample, "HH:mm:ss"), "24-hour clock"
      "formatDate(\"h:mm a\")", formatDate (sample, "h:mm a"), "12-hour clock"
      "formatDate(\"'Q'Q · 'W'I\")", formatDate (sample, "'Q'Q · 'W'I"), "Quarter & ISO week"
      "formatDate(\"DDD 'of' yyyy\")", formatDate (sample, "DDD 'of' yyyy"), "Day of year"
      "lightFormat(\"yyyy/MM/dd HH:mm\")", lightFormat (sample, "yyyy/MM/dd HH:mm"), "Zero-locale light formatter"
      "formatISO(sample)", formatISO sample, "ISO 8601 timestamp"
      "parse(\"15/03/2024\", \"dd/MM/yyyy\")", formatDate (parsed, "yyyy-MM-dd"), "Custom parse roundtrip" ]

let tbl =
    wrap "table" "" [
        wrap "thead" "" [ wrap "tr" "" [ for h in [ "API / Pattern"; "Output"; "Notes" ] -> el "th" "" h ] ]
        wrap "tbody" "" [
            for api, output, note in rows ->
                wrap "tr" "" [ wrap "td" "" [ el "code" "" api ]; el "td" "" output; el "td" "" note ]
        ]
    ]

app.appendChild (wrap "div" "card" [ el "div" "label" "Format Tokens & ISO Output"; tbl ]) |> ignore

let goodIso: JS.Date = parseISO "2024-03-15"
let badIso: JS.Date = parseISO "not-a-date"
let checks =
    [ "\"2024-03-15\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", (isValid goodIso && isMatch ("2024-03-15", "yyyy-MM-dd")), "VALID"
      "\"not-a-date\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", (isValid badIso && isMatch ("not-a-date", "yyyy-MM-dd")), "INVALID"
      "(2024, 1, 29)", "isExists(2024, 1, 29) leap Feb 29", isExists (2024.0, 1.0, 29.0), "EXISTS"
      "(2023, 1, 29)", "isExists(2023, 1, 29) non-leap Feb 29", isExists (2023.0, 1.0, 29.0), "INVALID" ]

let vtbl =
    wrap "table" "" [
        wrap "thead" "" [ wrap "tr" "" [ for h in [ "Input"; "Check"; "Result" ] -> el "th" "" h ] ]
        wrap "tbody" "" [
            for input, checkName, ok, labelText in checks ->
                let pillCls = if ok then "pill" else "pill-muted"
                wrap "tr" "" [ wrap "td" "" [ el "code" "" input ]; el "td" "" checkName; wrap "td" "" [ el "span" pillCls labelText ] ]
        ]
    ]

app.appendChild (wrap "div" "card" [ el "div" "label" "Validation & Calendar Existence"; vtbl ]) |> ignore
