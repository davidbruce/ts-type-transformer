module Program

open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Tanstack.TableCore
open type Tanstack.TableCore.Globals

[<Import("storeReactivityBindings", "@tanstack/table-core/store-reactivity-bindings")>]
let private storeReactivityBindings: unit -> TableReactivityBindings = jsNative

type private NodeRow =
    { id: string
      host: string
      region: string
      latency: float
      throughput: float
      uptime: float }

let private escapeHtml (s: string) =
    s.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;")

let private features =
    TableFeatures.Create(
        coreReactivityFeature = storeReactivityBindings (),
        rowSortingFeature = rowSortingFeature,
        coreRowModel = unbox (createCoreRowModel<TableFeatures, NodeRow> ()),
        sortedRowModel = unbox (createSortedRowModel<TableFeatures, NodeRow> ()),
        sortFns = JsRecord.Create [
            "alphanumeric", unbox sortFn_alphanumeric
            "basic", unbox sortFn_basic
            "text", unbox sortFn_text
        ]
    )

let private col (key: string) (header: string) : ColumnDefBase_Core<TableFeatures, NodeRow, CellData> =
    unbox {| accessorKey = key; header = header |}

let private data =
    ResizeArray [
        { id = "N-01"; host = "eu-west-1a"; region = "CH"; latency = 12.4; throughput = 4200.0; uptime = 99.98 }
        { id = "N-02"; host = "eu-west-1b"; region = "CH"; latency = 8.7; throughput = 5100.0; uptime = 99.99 }
        { id = "N-03"; host = "eu-central-1a"; region = "DE"; latency = 15.2; throughput = 3900.0; uptime = 99.91 }
        { id = "N-04"; host = "eu-central-1b"; region = "DE"; latency = 6.3; throughput = 6400.0; uptime = 99.97 }
        { id = "N-05"; host = "eu-north-1a"; region = "SE"; latency = 19.8; throughput = 2800.0; uptime = 99.85 }
        { id = "N-06"; host = "eu-north-1b"; region = "SE"; latency = 11.1; throughput = 4750.0; uptime = 99.95 }
        { id = "N-07"; host = "us-east-1a"; region = "US"; latency = 28.5; throughput = 7100.0; uptime = 99.94 }
        { id = "N-08"; host = "us-east-1b"; region = "US"; latency = 21.0; throughput = 5900.0; uptime = 99.96 }
    ]

let private columns =
    ResizeArray [
        col "id" "Node"
        col "host" "Host"
        col "region" "Region"
        col "latency" "Latency (ms)"
        col "throughput" "Req/s"
        col "uptime" "Uptime %"
    ]

let private options =
    TableOptions_Core.Create(
        columns = columns,
        features = features,
        data = data,
        initialState = TableState_All.Create(
            sorting = ResizeArray [ ColumnSort.Create(false, "region"); ColumnSort.Create(false, "latency") ]
        )
    )

let private table = constructTable<TableFeatures, NodeRow> options
let private tableSort = unbox<Table_RowSorting<TableFeatures, NodeRow>> table

let private wireBtn (id: string) (action: unit -> unit) =
    let btn = document.getElementById id
    btn.addEventListener (HTMLElementEventMapKeys.click, fun _ -> action ())

let private render (label: string) =
    let rows = table.getRowModel().rows
    let cols = table.getAllLeafColumns ()
    let html =
        [
            yield "<table><thead><tr>"
            for c in cols do
                let cSort = unbox<Column_RowSorting<TableFeatures, NodeRow>> c
                let mark =
                    match cSort.getIsSorted () with
                    | BooleanOrSortDirection.SortDirection SortDirection.Asc -> " \u25b2"
                    | BooleanOrSortDirection.SortDirection SortDirection.Desc -> " \u25bc"
                    | _ -> ""
                let hdr = string c.columnDef?header
                let cls = if c.id = "latency" || c.id = "throughput" || c.id = "uptime" then " class=\"num\"" else ""
                yield sprintf "<th%s>%s%s</th>" cls (escapeHtml hdr) mark
            yield "</tr></thead><tbody>"
            for r in rows do
                let latency: float = r.getValue "latency"
                let throughput: float = r.getValue "throughput"
                let uptime: float = r.getValue "uptime"
                yield "<tr>"
                yield sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "id"))
                yield sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "host"))
                yield sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "region"))
                yield sprintf "<td class=\"num\">%.1f</td>" latency
                yield sprintf "<td class=\"num\">%.0f</td>" throughput
                yield sprintf "<td class=\"num\">%.2f%%</td>" uptime
                yield "</tr>"
            yield "</tbody></table>"
        ]
        |> String.concat ""
    let wrap = document.getElementById "table-wrap"
    wrap.innerHTML <- html
    let topHost = if rows.Count > 0 then rows.[0].getValue<string> "host" else "-"
    let meta = document.getElementById "meta"
    meta.textContent <- sprintf "%s · %d rows · top node: %s" label rows.Count topHost

let main () =
    wireBtn "btn-0" (fun () ->
        tableSort.setSorting (!^ (ResizeArray [ ColumnSort.Create(false, "region"); ColumnSort.Create(false, "latency") ]))
        render "Sort: region asc, latency asc"
    )
    wireBtn "btn-1" (fun () ->
        tableSort.setSorting (!^ (ResizeArray [ ColumnSort.Create(false, "latency") ]))
        render "Sort: latency asc"
    )
    wireBtn "btn-2" (fun () ->
        tableSort.setSorting (!^ (ResizeArray [ ColumnSort.Create(true, "throughput") ]))
        render "Sort: throughput desc"
    )
    wireBtn "btn-3" (fun () ->
        tableSort.setSorting (!^ (ResizeArray [ ColumnSort.Create(false, "host") ]))
        render "Sort: host asc"
    )
    wireBtn "btn-4" (fun () ->
        tableSort.setSorting (!^ (ResizeArray<ColumnSort> ()))
        render "Sort: unsorted (data order)"
    )
    render "Sort: region asc, latency asc"

main ()
