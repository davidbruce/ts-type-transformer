module Program

open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Tanstack.TableCore
open type Tanstack.TableCore.Globals

[<Import("storeReactivityBindings", "@tanstack/table-core/store-reactivity-bindings")>]
let private storeReactivityBindings: unit -> TableReactivityBindings = jsNative

type private TaskRow =
    { id: string
      task: string
      owner: string
      zone: string
      cpu: float
      status: string }

let private escapeHtml (s: string) =
    s.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;")

let private features =
    TableFeatures.Create(
        coreReactivityFeature = storeReactivityBindings (),
        columnVisibilityFeature = columnVisibilityFeature,
        columnPinningFeature = columnPinningFeature,
        rowSelectionFeature = rowSelectionFeature,
        coreRowModel = unbox (createCoreRowModel<TableFeatures, TaskRow> ())
    )

let private col (key: string) (header: string) : ColumnDefBase_Core<TableFeatures, TaskRow, CellData> =
    unbox {| accessorKey = key; header = header |}

let private data =
    ResizeArray [
        { id = "T-101"; task = "ingest-wal-sync"; owner = "a.meyer"; zone = "eu-ch-1"; cpu = 42.0; status = "running" }
        { id = "T-102"; task = "index-compact"; owner = "b.rossi"; zone = "eu-ch-2"; cpu = 68.0; status = "running" }
        { id = "T-103"; task = "snapshot-verify"; owner = "c.keller"; zone = "eu-de-1"; cpu = 15.0; status = "queued" }
        { id = "T-104"; task = "ledger-reconcile"; owner = "d.weber"; zone = "eu-de-2"; cpu = 79.0; status = "running" }
        { id = "T-105"; task = "cert-rotation"; owner = "e.morel"; zone = "eu-fr-1"; cpu = 24.0; status = "ready" }
        { id = "T-106"; task = "shard-rebalance"; owner = "f.novak"; zone = "eu-fr-2"; cpu = 55.0; status = "running" }
        { id = "T-107"; task = "backup-archive"; owner = "g.steiner"; zone = "eu-ch-1"; cpu = 31.0; status = "ready" }
    ]

let private columns =
    ResizeArray [
        col "id" "Task ID"
        col "task" "Workload"
        col "owner" "Owner"
        col "zone" "Zone"
        col "cpu" "CPU %"
        col "status" "Status"
    ]

let private options =
    TableOptions_Core.Create(
        columns = columns,
        features = features,
        data = data,
        initialState = TableState_All.Create(
            columnVisibility = JsRecord.Create [ "zone", false ],
            columnPinning = ColumnPinningState.Create(ResizeArray [ "id" ], ResizeArray [ "status" ]),
            rowSelection = JsRecord.Create [ "0", true; "2", true; "4", true ]
        )
    )

let private table = constructTable<TableFeatures, TaskRow> options
let private vis = unbox<Table_ColumnVisibility<TableFeatures, TaskRow>> table
let private pin = unbox<Table_ColumnPinning<TableFeatures, TaskRow>> table
let private sel = unbox<Table_RowSelection<TableFeatures, TaskRow>> table

let private isColVisible (cv: obj) (key: string) : bool =
    if isNull cv || jsTypeof (cv?(key)) <> "boolean" then true else unbox<bool> cv?(key)

let private wireBtn (id: string) (action: unit -> unit) =
    let btn = (document.getElementById id).Value
    btn.addEventListener (HTMLElementEventMapKeys.click, fun _ -> action ())

let private render () =
    let startCols = pin.getStartVisibleLeafColumns ()
    let centerCols = pin.getCenterVisibleLeafColumns ()
    let endCols = pin.getEndVisibleLeafColumns ()
    let orderedCols =
        [
            for c in startCols -> (c.id, string c.columnDef?header, true)
            for c in centerCols -> (c.id, string c.columnDef?header, false)
            for c in endCols -> (c.id, string c.columnDef?header, true)
        ]

    let rows = table.getRowModel().rows
    let html =
        [
            yield "<table><thead><tr><th>Sel</th>"
            for (cid, hdr, isPin) in orderedCols do
                let pinTag = if isPin then "<span class=\"pin-tag\">PIN</span>" else ""
                let cls = if cid = "cpu" then " class=\"num\"" else ""
                yield sprintf "<th%s>%s%s</th>" cls (escapeHtml hdr) pinTag
            yield "</tr></thead><tbody>"
            for r in rows do
                let rSel = unbox<Row_RowSelection> r
                let isSel = rSel.getIsSelected ()
                let rowCls = if isSel then " class=\"selected-row\"" else ""
                let boxMark = if isSel then "\u25a0" else "\u25a1"
                yield sprintf "<tr%s><td>%s</td>" rowCls boxMark
                for (cid, _, _) in orderedCols do
                    if cid = "cpu" then
                        yield sprintf "<td class=\"num\">%.0f%%</td>" (r.getValue<float> "cpu")
                    else
                        yield sprintf "<td>%s</td>" (escapeHtml (string (r.getValue<obj> cid)))
                yield "</tr>"
            yield "</tbody></table>"
        ]
        |> String.concat ""

    let wrap = (document.getElementById "table-wrap").Value
    wrap.innerHTML <- html
    let visCount = vis.getVisibleLeafColumns().Count
    let selCount = sel.getSelectedRowModel().rows.Count
    let meta = (document.getElementById "meta").Value
    meta.textContent <- sprintf "%d / %d visible columns · start:id end:status pinned · %d / %d rows selected" visCount (table.getAllLeafColumns().Count) selCount rows.Count

let main () =
    wireBtn "btn-0" (fun () ->
        let state: obj = (box table.store)?get ()
        let cv: obj = state?columnVisibility
        let curZone = isColVisible cv "zone"
        let curOwner = isColVisible cv "owner"
        vis.setColumnVisibility (!^ (JsRecord.Create [ "zone", not curZone; "owner", curOwner ]))
        render ()
    )
    wireBtn "btn-1" (fun () ->
        let state: obj = (box table.store)?get ()
        let cv: obj = state?columnVisibility
        let curZone = isColVisible cv "zone"
        let curOwner = isColVisible cv "owner"
        vis.setColumnVisibility (!^ (JsRecord.Create [ "owner", not curOwner; "zone", curZone ]))
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
        vis.setColumnVisibility (!^ (JsRecord.Create [ "zone", false; "owner", true ]))
        pin.setColumnPinning (ColumnPinningState.Create(ResizeArray [ "id" ], ResizeArray [ "status" ]))
        sel.setRowSelection (!^ (JsRecord.Create [ "0", true; "2", true; "4", true ]))
        render ()
    )
    render ()

main ()
