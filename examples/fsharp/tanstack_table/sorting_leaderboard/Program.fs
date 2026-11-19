module Program

open System
open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Tanstack.TableCore
open type Tanstack.TableCore.Globals

[<Import("storeReactivityBindings", "@tanstack/table-core/store-reactivity-bindings")>]
let private storeReactivityBindings: unit -> TableReactivityBindings = jsNative

let private escapeHtml (s: string) =
    s.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;")

let private features: TableFeatures =
    unbox (
        createObj [
            "coreReactivityFeature" ==> storeReactivityBindings ()
            "rowSortingFeature" ==> rowSortingFeature
            "coreRowModel" ==> box (createCoreRowModel<TableFeatures, obj> ())
            "sortedRowModel" ==> box (createSortedRowModel<TableFeatures, obj> ())
            "sortFns" ==> createObj [
                "alphanumeric" ==> box sortFn_alphanumeric
                "basic" ==> box sortFn_basic
                "text" ==> box sortFn_text
            ]
        ]
    )

let private col (key: string) (header: string) : obj =
    createObj [ "accessorKey" ==> key; "header" ==> header ]

let private node (id: string) (host: string) (region: string) (latency: float) (throughput: float) (uptime: float) : obj =
    createObj [
        "id" ==> id
        "host" ==> host
        "region" ==> region
        "latency" ==> latency
        "throughput" ==> throughput
        "uptime" ==> uptime
    ]

let private data =
    ResizeArray<obj> [
        node "N-01" "eu-west-1a" "CH" 12.4 4200.0 99.98
        node "N-02" "eu-west-1b" "CH" 8.7 5100.0 99.99
        node "N-03" "eu-central-1a" "DE" 15.2 3900.0 99.91
        node "N-04" "eu-central-1b" "DE" 6.3 6400.0 99.97
        node "N-05" "eu-north-1a" "SE" 19.8 2800.0 99.85
        node "N-06" "eu-north-1b" "SE" 11.1 4750.0 99.95
        node "N-07" "us-east-1a" "US" 28.5 7100.0 99.94
        node "N-08" "us-east-1b" "US" 21.0 5900.0 99.96
    ]

let private columns =
    ResizeArray<obj> [
        col "id" "Node"
        col "host" "Host"
        col "region" "Region"
        col "latency" "Latency (ms)"
        col "throughput" "Req/s"
        col "uptime" "Uptime %"
    ]

let private options: TableOptions_Core<TableFeatures, obj> =
    unbox (
        createObj [
            "features" ==> features
            "data" ==> data
            "columns" ==> columns
            "initialState" ==> createObj [
                "sorting" ==> ResizeArray [ ColumnSort.Create(false, "region"); ColumnSort.Create(false, "latency") ]
            ]
        ]
    )

let private table = constructTable<TableFeatures, obj> options
let private tableSort = unbox<Table_RowSorting<TableFeatures, obj>> table

let private wireBtn (id: string) (action: unit -> unit) =
    let btn = unbox<HTMLElement> (document.getElementById id)
    btn.addEventListener (HTMLElementEventMapKeys.click, fun _ -> action ())

let private render (label: string) =
    let rows = table.getRowModel().rows
    let cols = table.getAllLeafColumns ()
    let sb = ResizeArray<string>()
    sb.Add "<table><thead><tr>"
    for c in cols do
        let cSort = unbox<Column_RowSorting<TableFeatures, obj>> c
        let mark =
            match cSort.getIsSorted () with
            | BooleanOrSortDirection.SortDirection SortDirection.Asc -> " \u25b2"
            | BooleanOrSortDirection.SortDirection SortDirection.Desc -> " \u25bc"
            | _ -> ""
        let hdr = string c.columnDef?header
        let cls = if c.id = "latency" || c.id = "throughput" || c.id = "uptime" then " class=\"num\"" else ""
        sb.Add (sprintf "<th%s>%s%s</th>" cls (escapeHtml hdr) mark)
    sb.Add "</tr></thead><tbody>"
    for r in rows do
        let latency: float = r.getValue "latency"
        let throughput: float = r.getValue "throughput"
        let uptime: float = r.getValue "uptime"
        sb.Add "<tr>"
        sb.Add (sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "id")))
        sb.Add (sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "host")))
        sb.Add (sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "region")))
        sb.Add (sprintf "<td class=\"num\">%.1f</td>" latency)
        sb.Add (sprintf "<td class=\"num\">%.0f</td>" throughput)
        sb.Add (sprintf "<td class=\"num\">%.2f%%</td>" uptime)
        sb.Add "</tr>"
    sb.Add "</tbody></table>"
    let wrap = unbox<HTMLElement> (document.getElementById "table-wrap")
    wrap.innerHTML <- String.concat "" sb
    let topHost = if rows.Count > 0 then rows.[0].getValue<string> "host" else "-"
    let meta = unbox<HTMLElement> (document.getElementById "meta")
    meta.textContent <- sprintf "%s · %d rows · top node: %s" label rows.Count topHost

let main () =
    wireBtn "btn-0" (fun () ->
        tableSort.setSorting (U2.Case1 (ResizeArray [ ColumnSort.Create(false, "region"); ColumnSort.Create(false, "latency") ]))
        render "Sort: region asc, latency asc"
    )
    wireBtn "btn-1" (fun () ->
        tableSort.setSorting (U2.Case1 (ResizeArray [ ColumnSort.Create(false, "latency") ]))
        render "Sort: latency asc"
    )
    wireBtn "btn-2" (fun () ->
        tableSort.setSorting (U2.Case1 (ResizeArray [ ColumnSort.Create(true, "throughput") ]))
        render "Sort: throughput desc"
    )
    wireBtn "btn-3" (fun () ->
        tableSort.setSorting (U2.Case1 (ResizeArray [ ColumnSort.Create(false, "host") ]))
        render "Sort: host asc"
    )
    wireBtn "btn-4" (fun () ->
        tableSort.setSorting (U2.Case1 (ResizeArray<ColumnSort> ()))
        render "Sort: unsorted (data order)"
    )
    render "Sort: region asc, latency asc"

main ()
