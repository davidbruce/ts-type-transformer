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
            "columnVisibilityFeature" ==> columnVisibilityFeature
            "columnPinningFeature" ==> columnPinningFeature
            "rowSelectionFeature" ==> rowSelectionFeature
            "coreRowModel" ==> box (createCoreRowModel<TableFeatures, obj> ())
        ]
    )

let private col (key: string) (header: string) : obj =
    createObj [ "accessorKey" ==> key; "header" ==> header ]

let private taskRow (id: string) (task: string) (owner: string) (zone: string) (cpu: float) (status: string) : obj =
    createObj [
        "id" ==> id
        "task" ==> task
        "owner" ==> owner
        "zone" ==> zone
        "cpu" ==> cpu
        "status" ==> status
    ]

let private data =
    ResizeArray<obj> [
        taskRow "T-101" "ingest-wal-sync" "a.meyer" "eu-ch-1" 42.0 "running"
        taskRow "T-102" "index-compact" "b.rossi" "eu-ch-2" 68.0 "running"
        taskRow "T-103" "snapshot-verify" "c.keller" "eu-de-1" 15.0 "queued"
        taskRow "T-104" "ledger-reconcile" "d.weber" "eu-de-2" 79.0 "running"
        taskRow "T-105" "cert-rotation" "e.morel" "eu-fr-1" 24.0 "ready"
        taskRow "T-106" "shard-rebalance" "f.novak" "eu-fr-2" 55.0 "running"
        taskRow "T-107" "backup-archive" "g.steiner" "eu-ch-1" 31.0 "ready"
    ]

let private columns =
    ResizeArray<obj> [
        col "id" "Task ID"
        col "task" "Workload"
        col "owner" "Owner"
        col "zone" "Zone"
        col "cpu" "CPU %"
        col "status" "Status"
    ]

let private options: TableOptions_Core<TableFeatures, obj> =
    unbox (
        createObj [
            "features" ==> features
            "data" ==> data
            "columns" ==> columns
            "initialState" ==> createObj [
                "columnVisibility" ==> createObj [ "zone" ==> false ]
                "columnPinning" ==> createObj [ "start" ==> ResizeArray [ "id" ]; "end" ==> ResizeArray [ "status" ] ]
                "rowSelection" ==> createObj [ "0" ==> true; "2" ==> true; "4" ==> true ]
            ]
        ]
    )

let private table = constructTable<TableFeatures, obj> options
let private vis = unbox<Table_ColumnVisibility<TableFeatures, obj>> table
let private pin = unbox<Table_ColumnPinning<TableFeatures, obj>> table
let private sel = unbox<Table_RowSelection<TableFeatures, obj>> table

let private wireBtn (id: string) (action: unit -> unit) =
    let btn = document.getElementById id
    btn.addEventListener (HTMLElementEventMapKeys.click, fun _ -> action ())

let private render () =
    let startCols = pin.getStartVisibleLeafColumns ()
    let centerCols = pin.getCenterVisibleLeafColumns ()
    let endCols = pin.getEndVisibleLeafColumns ()
    let orderedCols = ResizeArray<string * string * bool>()
    for c in startCols do orderedCols.Add (c.id, string c.columnDef?header, true)
    for c in centerCols do orderedCols.Add (c.id, string c.columnDef?header, false)
    for c in endCols do orderedCols.Add (c.id, string c.columnDef?header, true)

    let sb = ResizeArray<string>()
    sb.Add "<table><thead><tr><th>Sel</th>"
    for (cid, hdr, isPin) in orderedCols do
        let pinTag = if isPin then "<span class=\"pin-tag\">PIN</span>" else ""
        let cls = if cid = "cpu" then " class=\"num\"" else ""
        sb.Add (sprintf "<th%s>%s%s</th>" cls (escapeHtml hdr) pinTag)
    sb.Add "</tr></thead><tbody>"

    let rows = table.getRowModel().rows
    for r in rows do
        let rSel = unbox<Row_RowSelection> r
        let isSel = rSel.getIsSelected ()
        let rowCls = if isSel then " class=\"selected-row\"" else ""
        let boxMark = if isSel then "\u25a0" else "\u25a1"
        sb.Add (sprintf "<tr%s><td>%s</td>" rowCls boxMark)
        for (cid, _, _) in orderedCols do
            if cid = "cpu" then
                sb.Add (sprintf "<td class=\"num\">%.0f%%</td>" (r.getValue<float> "cpu"))
            else
                sb.Add (sprintf "<td>%s</td>" (escapeHtml (string (r.getValue<obj> cid))))
        sb.Add "</tr>"
    sb.Add "</tbody></table>"
    let wrap = document.getElementById "table-wrap"
    wrap.innerHTML <- String.concat "" sb
    let visCount = vis.getVisibleLeafColumns().Count
    let selCount = sel.getSelectedRowModel().rows.Count
    let meta = document.getElementById "meta"
    meta.textContent <- sprintf "%d / %d visible columns · start:id end:status pinned · %d / %d rows selected" visCount (table.getAllLeafColumns().Count) selCount rows.Count

let main () =
    wireBtn "btn-0" (fun () ->
        let state: obj = (box table.store)?get ()
        let cv: obj = state?columnVisibility
        let cur = if isNull cv || jsTypeof cv?zone <> "boolean" then true else unbox<bool> cv?zone
        let next = createObj [
            "zone" ==> not cur
            (if not (isNull cv) && jsTypeof cv?owner = "boolean" then "owner" ==> unbox<bool> cv?owner else "owner" ==> true)
        ]
        vis.setColumnVisibility (U2.Case1 (unbox next))
        render ()
    )
    wireBtn "btn-1" (fun () ->
        let state: obj = (box table.store)?get ()
        let cv: obj = state?columnVisibility
        let cur = if isNull cv || jsTypeof cv?owner <> "boolean" then true else unbox<bool> cv?owner
        let next = createObj [
            "owner" ==> not cur
            (if not (isNull cv) && jsTypeof cv?zone = "boolean" then "zone" ==> unbox<bool> cv?zone else "zone" ==> true)
        ]
        vis.setColumnVisibility (U2.Case1 (unbox next))
        render ()
    )
    wireBtn "btn-2" (fun () ->
        sel.toggleAllRowsSelected true
        render ()
    )
    wireBtn "btn-3" (fun () ->
        sel.toggleAllRowsSelected false
        render ()
    )
    wireBtn "btn-4" (fun () ->
        vis.setColumnVisibility (U2.Case1 (unbox (createObj [ "zone" ==> false; "owner" ==> true ])))
        pin.setColumnPinning (U2.Case1 (unbox (createObj [ "start" ==> ResizeArray [ "id" ]; "end" ==> ResizeArray [ "status" ] ])))
        sel.setRowSelection (U2.Case1 (unbox (createObj [ "0" ==> true; "2" ==> true; "4" ==> true ])))
        render ()
    )
    render ()

main ()
