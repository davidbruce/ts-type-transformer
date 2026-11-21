module Program

open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Tanstack.TableCore
open type Tanstack.TableCore.Globals

[<Import("storeReactivityBindings", "@tanstack/table-core/store-reactivity-bindings")>]
let private storeReactivityBindings: unit -> TableReactivityBindings = jsNative

type private HostRow =
    { region: string
      host: string
      role: string
      cpu: float
      mem: float
      nodes: float }

let private escapeHtml (s: string) =
    s.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;")

let private features =
    TableFeatures.Create(
        coreReactivityFeature = storeReactivityBindings (),
        columnGroupingFeature = columnGroupingFeature,
        rowAggregationFeature = rowAggregationFeature,
        rowExpandingFeature = rowExpandingFeature,
        coreRowModel = unbox (createCoreRowModel<TableFeatures, HostRow> ()),
        groupedRowModel = unbox (createGroupedRowModel<TableFeatures, HostRow> ()),
        expandedRowModel = unbox (createExpandedRowModel<TableFeatures, HostRow> ()),
        aggregationFns = JsRecord.Create [
            "sum", unbox aggregationFn_sum
            "mean", unbox aggregationFn_mean
            "count", unbox aggregationFn_count
        ]
    )

let private col (key: string) (header: string) (aggFn: string option) : ColumnDefBase_Core<TableFeatures, HostRow, CellData> =
    match aggFn with
    | Some fnName -> unbox {| accessorKey = key; header = header; aggregationFn = fnName |}
    | None -> unbox {| accessorKey = key; header = header |}

let private data =
    ResizeArray [
        { region = "CH"; host = "zurich-api-01"; role = "api"; cpu = 72.0; mem = 64.0; nodes = 1.0 }
        { region = "CH"; host = "basel-db-01"; role = "db"; cpu = 58.0; mem = 128.0; nodes = 1.0 }
        { region = "CH"; host = "geneva-edge-01"; role = "edge"; cpu = 35.0; mem = 32.0; nodes = 1.0 }
        { region = "DE"; host = "munich-api-02"; role = "api"; cpu = 84.0; mem = 64.0; nodes = 1.0 }
        { region = "DE"; host = "berlin-db-02"; role = "db"; cpu = 66.0; mem = 128.0; nodes = 1.0 }
        { region = "DE"; host = "frankfurt-edge-02"; role = "edge"; cpu = 42.0; mem = 32.0; nodes = 1.0 }
        { region = "FR"; host = "paris-api-03"; role = "api"; cpu = 60.0; mem = 64.0; nodes = 1.0 }
        { region = "FR"; host = "lyon-db-03"; role = "db"; cpu = 48.0; mem = 128.0; nodes = 1.0 }
        { region = "FR"; host = "marseille-edge-03"; role = "edge"; cpu = 30.0; mem = 32.0; nodes = 1.0 }
    ]

let private columns =
    ResizeArray [
        col "region" "Region" None
        col "host" "Host" None
        col "role" "Role" None
        col "cpu" "CPU % (mean)" (Some "mean")
        col "mem" "Mem GB (sum)" (Some "sum")
        col "nodes" "Nodes (count)" (Some "count")
    ]

let private options =
    TableOptions_Core.Create(
        columns = columns,
        features = features,
        data = data,
        initialState = TableState_All.Create(
            grouping = ResizeArray [ "region" ],
            expanded = !^true
        )
    )

let private table = constructTable<TableFeatures, HostRow> options
let private grp = unbox<Table_ColumnGrouping<TableFeatures, HostRow>> table
let private exp = unbox<Table_RowExpanding<TableFeatures, HostRow>> table

let private wireBtn (id: string) (action: unit -> unit) =
    let btn = (document.getElementById id).Value
    btn.addEventListener (HTMLElementEventMapKeys.click, fun _ -> action ())

let private render () =
    let rows = table.getRowModel().rows
    let state: obj = (box table.store)?get ()
    let grpIds = unbox<ResizeArray<string>> state?grouping
    let grpCol = if grpIds.Count > 0 then grpIds.[0] else "none"
    let mutable groupCount = 0
    let html =
        [
            yield "<table><thead><tr><th>Group / Host</th><th>Region</th><th>Role</th><th class=\"num\">CPU %</th><th class=\"num\">Mem GB</th><th class=\"num\">Nodes</th></tr></thead><tbody>"
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
                    yield sprintf "<tr class=\"group-row\"><td>%s %s=%s (%d)</td><td>%s</td><td>%s</td><td class=\"num\">%.1f</td><td class=\"num\">%.0f</td><td class=\"num\">%.0f</td></tr>"
                        marker (escapeHtml grpCol) (escapeHtml gVal) r.subRows.Count (escapeHtml (string (r.getValue<obj> "region"))) (escapeHtml (string (r.getValue<obj> "role"))) cpu mem nodes
                else
                    yield sprintf "<tr><td class=\"child-indent\">%s</td><td>%s</td><td>%s</td><td class=\"num\">%.0f</td><td class=\"num\">%.0f</td><td class=\"num\">%.0f</td></tr>"
                        (escapeHtml (r.getValue<string> "host")) (escapeHtml (r.getValue<string> "region")) (escapeHtml (r.getValue<string> "role")) cpu mem nodes
            yield "</tbody></table>"
        ]
        |> String.concat ""
    let wrap = (document.getElementById "table-wrap").Value
    wrap.innerHTML <- html
    let meta = (document.getElementById "meta").Value
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
        grp.setGrouping (!^ (ResizeArray [ "region" ]))
        let expandAll : ExpandedState = !^true
        exp.setExpanded (!^ expandAll)
        render ()
    )
    wireBtn "btn-3" (fun () ->
        grp.setGrouping (!^ (ResizeArray [ "role" ]))
        let expandAll : ExpandedState = !^true
        exp.setExpanded (!^ expandAll)
        render ()
    )
    render ()

main ()
