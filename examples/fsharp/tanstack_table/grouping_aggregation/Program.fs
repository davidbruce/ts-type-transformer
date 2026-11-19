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
            "columnGroupingFeature" ==> columnGroupingFeature
            "rowAggregationFeature" ==> rowAggregationFeature
            "rowExpandingFeature" ==> rowExpandingFeature
            "coreRowModel" ==> box (createCoreRowModel<TableFeatures, obj> ())
            "groupedRowModel" ==> box (createGroupedRowModel<TableFeatures, obj> ())
            "expandedRowModel" ==> box (createExpandedRowModel<TableFeatures, obj> ())
            "aggregationFns" ==> createObj [
                "sum" ==> box aggregationFn_sum
                "mean" ==> box aggregationFn_mean
                "count" ==> box aggregationFn_count
            ]
        ]
    )

let private col (key: string) (header: string) (aggFn: string option) : obj =
    match aggFn with
    | Some fnName -> createObj [ "accessorKey" ==> key; "header" ==> header; "aggregationFn" ==> fnName ]
    | None -> createObj [ "accessorKey" ==> key; "header" ==> header ]

let private hostRow (region: string) (host: string) (role: string) (cpu: float) (mem: float) : obj =
    createObj [
        "region" ==> region
        "host" ==> host
        "role" ==> role
        "cpu" ==> cpu
        "mem" ==> mem
        "nodes" ==> 1.0
    ]

let private data =
    ResizeArray<obj> [
        hostRow "CH" "zurich-api-01" "api" 72.0 64.0
        hostRow "CH" "basel-db-01" "db" 58.0 128.0
        hostRow "CH" "geneva-edge-01" "edge" 35.0 32.0
        hostRow "DE" "munich-api-02" "api" 84.0 64.0
        hostRow "DE" "berlin-db-02" "db" 66.0 128.0
        hostRow "DE" "frankfurt-edge-02" "edge" 42.0 32.0
        hostRow "FR" "paris-api-03" "api" 60.0 64.0
        hostRow "FR" "lyon-db-03" "db" 48.0 128.0
        hostRow "FR" "marseille-edge-03" "edge" 30.0 32.0
    ]

let private columns =
    ResizeArray<obj> [
        col "region" "Region" None
        col "host" "Host" None
        col "role" "Role" None
        col "cpu" "CPU % (mean)" (Some "mean")
        col "mem" "Mem GB (sum)" (Some "sum")
        col "nodes" "Nodes (count)" (Some "count")
    ]

let private options: TableOptions_Core<TableFeatures, obj> =
    unbox (
        createObj [
            "features" ==> features
            "data" ==> data
            "columns" ==> columns
            "initialState" ==> createObj [
                "grouping" ==> ResizeArray [ "region" ]
                "expanded" ==> true
            ]
        ]
    )

let private table = constructTable<TableFeatures, obj> options
let private grp = unbox<Table_ColumnGrouping<TableFeatures, obj>> table
let private exp = unbox<Table_RowExpanding<TableFeatures, obj>> table

let private wireBtn (id: string) (action: unit -> unit) =
    let btn = unbox<HTMLElement> (document.getElementById id)
    btn.addEventListener (HTMLElementEventMapKeys.click, fun _ -> action ())

let private render () =
    let rows = table.getRowModel().rows
    let state: obj = (box table.store)?get ()
    let grpIds = unbox<ResizeArray<string>> state?grouping
    let grpCol = if grpIds.Count > 0 then grpIds.[0] else "none"
    let sb = ResizeArray<string>()
    sb.Add "<table><thead><tr><th>Group / Host</th><th>Region</th><th>Role</th><th class=\"num\">CPU %</th><th class=\"num\">Mem GB</th><th class=\"num\">Nodes</th></tr></thead><tbody>"
    let mutable groupCount = 0
    for r in rows do
        let rGrp = unbox<Row_ColumnGrouping> r
        let rExp = unbox<Row_RowExpanding> r
        let isGrp = rGrp.getIsGrouped ()
        let cpu: float = r.getValue "cpu"
        let mem: float = r.getValue "mem"
        let nodes: float = r.getValue "nodes"
        if isGrp then
            groupCount <- groupCount + 1
            let marker = if rExp.getIsExpanded () then "[-]" else "[+]"
            let gVal = string (r.getValue<obj> grpCol)
            sb.Add (sprintf "<tr class=\"group-row\"><td>%s %s=%s (%d)</td><td>%s</td><td>%s</td><td class=\"num\">%.1f</td><td class=\"num\">%.0f</td><td class=\"num\">%.0f</td></tr>"
                marker (escapeHtml grpCol) (escapeHtml gVal) r.subRows.Count (escapeHtml (string (r.getValue<obj> "region"))) (escapeHtml (string (r.getValue<obj> "role"))) cpu mem nodes)
        else
            sb.Add (sprintf "<tr><td class=\"child-indent\">%s</td><td>%s</td><td>%s</td><td class=\"num\">%.0f</td><td class=\"num\">%.0f</td><td class=\"num\">%.0f</td></tr>"
                (escapeHtml (r.getValue<string> "host")) (escapeHtml (r.getValue<string> "region")) (escapeHtml (r.getValue<string> "role")) cpu mem nodes)
    sb.Add "</tbody></table>"
    let wrap = unbox<HTMLElement> (document.getElementById "table-wrap")
    wrap.innerHTML <- String.concat "" sb
    let meta = unbox<HTMLElement> (document.getElementById "meta")
    meta.textContent <- sprintf "Grouped by %s · %d groups · %d visible rows" grpCol groupCount rows.Count

let main () =
    wireBtn "btn-0" (fun () ->
        exp.toggleAllRowsExpanded true
        render ()
    )
    wireBtn "btn-1" (fun () ->
        exp.toggleAllRowsExpanded false
        render ()
    )
    wireBtn "btn-2" (fun () ->
        grp.setGrouping (U2.Case1 (ResizeArray [ "region" ]))
        exp.setExpanded (U2.Case1 (ExpandedState.Bool true))
        render ()
    )
    wireBtn "btn-3" (fun () ->
        grp.setGrouping (U2.Case1 (ResizeArray [ "role" ]))
        exp.setExpanded (U2.Case1 (ExpandedState.Bool true))
        render ()
    )
    render ()

main ()
