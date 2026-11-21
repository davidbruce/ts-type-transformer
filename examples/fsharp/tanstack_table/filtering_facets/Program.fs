module Program

open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Tanstack.TableCore
open type Tanstack.TableCore.Globals

[<Import("storeReactivityBindings", "@tanstack/table-core/store-reactivity-bindings")>]
let private storeReactivityBindings: unit -> TableReactivityBindings = jsNative

type private ServiceRow =
    { id: string
      service: string
      tier: string
      region: string
      requests: float
      errors: float }

let private escapeHtml (s: string) =
    s.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;")

let private features =
    TableFeatures.Create(
        coreReactivityFeature = storeReactivityBindings (),
        columnFilteringFeature = columnFilteringFeature,
        globalFilteringFeature = globalFilteringFeature,
        coreRowModel = unbox (createCoreRowModel<TableFeatures, ServiceRow> ()),
        filteredRowModel = unbox (createFilteredRowModel<TableFeatures, ServiceRow> ()),
        filterFns = JsRecord.Create [
            "includesString", unbox filterFn_includesString
            "equalsString", unbox filterFn_equalsString
            "greaterThanOrEqualTo", unbox filterFn_greaterThanOrEqualTo
        ]
    )

let private col (key: string) (header: string) (filterFn: string option) : ColumnDefBase_Core<TableFeatures, ServiceRow, CellData> =
    match filterFn with
    | Some fnName -> unbox {| accessorKey = key; header = header; filterFn = fnName |}
    | None -> unbox {| accessorKey = key; header = header |}

let private data =
    ResizeArray [
        { id = "S-01"; service = "auth-gateway"; tier = "core"; region = "CH"; requests = 620.0; errors = 2.0 }
        { id = "S-02"; service = "session-cache"; tier = "core"; region = "CH"; requests = 480.0; errors = 1.0 }
        { id = "S-03"; service = "billing-ledger"; tier = "core"; region = "DE"; requests = 510.0; errors = 6.0 }
        { id = "S-04"; service = "oauth-signer"; tier = "core"; region = "FR"; requests = 430.0; errors = 3.0 }
        { id = "S-05"; service = "trace-ingestor"; tier = "edge"; region = "DE"; requests = 290.0; errors = 8.0 }
        { id = "S-06"; service = "cdn-router"; tier = "edge"; region = "FR"; requests = 840.0; errors = 5.0 }
        { id = "S-07"; service = "static-assets"; tier = "edge"; region = "CH"; requests = 710.0; errors = 0.0 }
        { id = "S-08"; service = "mail-relay"; tier = "batch"; region = "DE"; requests = 140.0; errors = 7.0 }
        { id = "S-09"; service = "report-export"; tier = "batch"; region = "FR"; requests = 210.0; errors = 4.0 }
        { id = "S-10"; service = "audit-auth-log"; tier = "core"; region = "US"; requests = 360.0; errors = 9.0 }
    ]

let private columns =
    ResizeArray [
        col "id" "ID" None
        col "service" "Service" None
        col "tier" "Tier" (Some "equalsString")
        col "region" "Region" None
        col "requests" "Req/s" (Some "greaterThanOrEqualTo")
        col "errors" "Errors" (Some "greaterThanOrEqualTo")
    ]

let private options =
    TableOptions_Core.Create(
        columns = columns,
        features = features,
        data = data,
        initialState = TableState_All.Create(
            columnFilters = ResizeArray [
                ColumnFilter.Create("tier", box "core")
                ColumnFilter.Create("requests", box 400.0)
            ]
        )
    )

let private table = constructTable<TableFeatures, ServiceRow> options
let private colFilt = unbox<Table_ColumnFiltering> table
let private globFilt = unbox<Table_GlobalFiltering<TableFeatures, ServiceRow>> table

let private wireBtn (id: string) (action: unit -> unit) =
    let btn = document.getElementById id
    btn.addEventListener (HTMLElementEventMapKeys.click, fun _ -> action ())

let private render (label: string) =
    let rows = table.getRowModel().rows
    let html =
        [
            yield "<table><thead><tr><th>ID</th><th>Service</th><th>Tier</th><th>Region</th><th class=\"num\">Req/s</th><th class=\"num\">Errors</th></tr></thead><tbody>"
            for r in rows do
                let reqs: float = r.getValue "requests"
                let errs: float = r.getValue "errors"
                yield "<tr>"
                yield sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "id"))
                yield sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "service"))
                yield sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "tier"))
                yield sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "region"))
                yield sprintf "<td class=\"num\">%.0f</td>" reqs
                yield sprintf "<td class=\"num\">%.0f</td>" errs
                yield "</tr>"
            yield "</tbody></table>"
        ]
        |> String.concat ""
    let wrap = document.getElementById "table-wrap"
    wrap.innerHTML <- html
    let total = table.getCoreRowModel().rows.Count
    let meta = document.getElementById "meta"
    meta.textContent <- sprintf "%s · showing %d / %d services" label rows.Count total

let main () =
    wireBtn "btn-0" (fun () ->
        globFilt.setGlobalFilter (!^ (box ""))
        colFilt.setColumnFilters (!^ (ResizeArray [ ColumnFilter.Create("tier", box "core"); ColumnFilter.Create("requests", box 400.0) ]))
        render "Filter: tier = core & requests >= 400"
    )
    wireBtn "btn-1" (fun () ->
        globFilt.setGlobalFilter (!^ (box ""))
        colFilt.setColumnFilters (!^ (ResizeArray<ColumnFilter> ()))
        render "Filter: all services"
    )
    wireBtn "btn-2" (fun () ->
        globFilt.setGlobalFilter (!^ (box ""))
        colFilt.setColumnFilters (!^ (ResizeArray [ ColumnFilter.Create("tier", box "edge") ]))
        render "Filter: tier = edge"
    )
    wireBtn "btn-3" (fun () ->
        colFilt.setColumnFilters (!^ (ResizeArray<ColumnFilter> ()))
        globFilt.setGlobalFilter (!^ (box "auth"))
        render "Global search: \"auth\""
    )
    wireBtn "btn-4" (fun () ->
        globFilt.setGlobalFilter (!^ (box ""))
        colFilt.setColumnFilters (!^ (ResizeArray [ ColumnFilter.Create("errors", box 5.0) ]))
        render "Filter: errors >= 5"
    )
    render "Filter: tier = core & requests >= 400"

main ()
