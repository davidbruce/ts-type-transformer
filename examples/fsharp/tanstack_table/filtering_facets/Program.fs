module Program

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
            "columnFilteringFeature" ==> columnFilteringFeature
            "globalFilteringFeature" ==> globalFilteringFeature
            "coreRowModel" ==> box (createCoreRowModel<TableFeatures, obj> ())
            "filteredRowModel" ==> box (createFilteredRowModel<TableFeatures, obj> ())
            "filterFns" ==> createObj [
                "includesString" ==> box filterFn_includesString
                "equalsString" ==> box filterFn_equalsString
                "greaterThanOrEqualTo" ==> box filterFn_greaterThanOrEqualTo
            ]
        ]
    )

let private col (key: string) (header: string) (filterFn: string option) : obj =
    match filterFn with
    | Some fnName -> createObj [ "accessorKey" ==> key; "header" ==> header; "filterFn" ==> fnName ]
    | None -> createObj [ "accessorKey" ==> key; "header" ==> header ]

let private svc (id: string) (service: string) (tier: string) (region: string) (requests: float) (errors: float) : obj =
    createObj [
        "id" ==> id
        "service" ==> service
        "tier" ==> tier
        "region" ==> region
        "requests" ==> requests
        "errors" ==> errors
    ]

let private data =
    ResizeArray<obj> [
        svc "S-01" "auth-gateway" "core" "CH" 620.0 2.0
        svc "S-02" "session-cache" "core" "CH" 480.0 1.0
        svc "S-03" "billing-ledger" "core" "DE" 510.0 6.0
        svc "S-04" "oauth-signer" "core" "FR" 430.0 3.0
        svc "S-05" "trace-ingestor" "edge" "DE" 290.0 8.0
        svc "S-06" "cdn-router" "edge" "FR" 840.0 5.0
        svc "S-07" "static-assets" "edge" "CH" 710.0 0.0
        svc "S-08" "mail-relay" "batch" "DE" 140.0 7.0
        svc "S-09" "report-export" "batch" "FR" 210.0 4.0
        svc "S-10" "audit-auth-log" "core" "US" 360.0 9.0
    ]

let private columns =
    ResizeArray<obj> [
        col "id" "ID" None
        col "service" "Service" None
        col "tier" "Tier" (Some "equalsString")
        col "region" "Region" None
        col "requests" "Req/s" (Some "greaterThanOrEqualTo")
        col "errors" "Errors" (Some "greaterThanOrEqualTo")
    ]

let private options: TableOptions_Core<TableFeatures, obj> =
    unbox (
        createObj [
            "features" ==> features
            "data" ==> data
            "columns" ==> columns
            "initialState" ==> createObj [
                "columnFilters" ==> ResizeArray [
                    ColumnFilter.Create("tier", box "core")
                    ColumnFilter.Create("requests", box 400.0)
                ]
            ]
        ]
    )

let private table = constructTable<TableFeatures, obj> options
let private colFilt = unbox<Table_ColumnFiltering> table
let private globFilt = unbox<Table_GlobalFiltering<TableFeatures, obj>> table

let private wireBtn (id: string) (action: unit -> unit) =
    let btn = document.getElementById id
    btn.addEventListener (HTMLElementEventMapKeys.click, fun _ -> action ())

let private render (label: string) =
    let rows = table.getRowModel().rows
    let sb = ResizeArray<string>()
    sb.Add "<table><thead><tr><th>ID</th><th>Service</th><th>Tier</th><th>Region</th><th class=\"num\">Req/s</th><th class=\"num\">Errors</th></tr></thead><tbody>"
    for r in rows do
        let reqs: float = r.getValue "requests"
        let errs: float = r.getValue "errors"
        sb.Add "<tr>"
        sb.Add (sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "id")))
        sb.Add (sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "service")))
        sb.Add (sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "tier")))
        sb.Add (sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "region")))
        sb.Add (sprintf "<td class=\"num\">%.0f</td>" reqs)
        sb.Add (sprintf "<td class=\"num\">%.0f</td>" errs)
        sb.Add "</tr>"
    sb.Add "</tbody></table>"
    let wrap = document.getElementById "table-wrap"
    wrap.innerHTML <- String.concat "" sb
    let total = table.getCoreRowModel().rows.Count
    let meta = document.getElementById "meta"
    meta.textContent <- sprintf "%s · showing %d / %d services" label rows.Count total

let main () =
    wireBtn "btn-0" (fun () ->
        globFilt.setGlobalFilter (U2.Case1 (box ""))
        colFilt.setColumnFilters (U2.Case1 (ResizeArray [ ColumnFilter.Create("tier", box "core"); ColumnFilter.Create("requests", box 400.0) ]))
        render "Filter: tier = core & requests >= 400"
    )
    wireBtn "btn-1" (fun () ->
        globFilt.setGlobalFilter (U2.Case1 (box ""))
        colFilt.setColumnFilters (U2.Case1 (ResizeArray<ColumnFilter> ()))
        render "Filter: all services"
    )
    wireBtn "btn-2" (fun () ->
        globFilt.setGlobalFilter (U2.Case1 (box ""))
        colFilt.setColumnFilters (U2.Case1 (ResizeArray [ ColumnFilter.Create("tier", box "edge") ]))
        render "Filter: tier = edge"
    )
    wireBtn "btn-3" (fun () ->
        colFilt.setColumnFilters (U2.Case1 (ResizeArray<ColumnFilter> ()))
        globFilt.setGlobalFilter (U2.Case1 (box "auth"))
        render "Global search: \"auth\""
    )
    wireBtn "btn-4" (fun () ->
        globFilt.setGlobalFilter (U2.Case1 (box ""))
        colFilt.setColumnFilters (U2.Case1 (ResizeArray [ ColumnFilter.Create("errors", box 5.0) ]))
        render "Filter: errors >= 5"
    )
    render "Filter: tier = core & requests >= 400"

main ()
