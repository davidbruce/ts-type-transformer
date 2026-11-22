module Program

open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Tanstack.TableCore
open type Tanstack.TableCore.Globals

[<Import("storeReactivityBindings", "@tanstack/table-core/store-reactivity-bindings")>]
let private storeReactivityBindings: unit -> TableReactivityBindings = jsNative

type private LedgerRow =
    { id: string
      account: string
      category: string
      units: float
      amount: float }

let private escapeHtml (s: string) =
    s.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;")

let private features =
    TableFeatures.Create(
        coreReactivityFeature = storeReactivityBindings (),
        rowPaginationFeature = rowPaginationFeature,
        coreRowModel = unbox (createCoreRowModel<TableFeatures, LedgerRow> ()),
        paginatedRowModel = unbox (createPaginatedRowModel<TableFeatures, LedgerRow> ())
    )

let private col (key: string) (header: string) : ColumnDefBase_Core<TableFeatures, LedgerRow, CellData> =
    unbox {| accessorKey = key; header = header |}

let private accounts = [| "ops-pool"; "clearing-ch"; "reserve-de" |]
let private cats = [| "compute"; "storage"; "egress"; "audit" |]

let private data =
    ResizeArray [
        for i in 0 .. 17 ->
            { id = sprintf "TX-%03d" (i + 1)
              account = accounts.[i % 3]
              category = cats.[i % 4]
              units = float ((i * 7) % 19 + 4)
              amount = float (((i * 37) % 90 + 15) * 12) }
    ]

let private columns =
    ResizeArray [
        col "id" "Entry"
        col "account" "Account"
        col "category" "Category"
        col "units" "Units"
        col "amount" "Amount (CHF)"
    ]

let private options =
    TableOptions_Core.Create(
        columns = columns,
        features = features,
        data = data,
        initialState = TableState_All.Create(
            pagination = PaginationState.Create(0.0, 5.0)
        )
    )

let private table = constructTable<TableFeatures, LedgerRow> options
let private pag = unbox<Table_RowPagination<TableFeatures, LedgerRow>> table

let private readPagination () : PaginationState =
    let state: obj = (box table.store)?get ()
    unbox state?pagination

let private wireBtn (id: string) (action: unit -> unit) =
    let btn = (document.getElementById id).Value
    btn.addEventListener (HTMLElementEventMapKeys.click, fun _ -> action ())

let private render () =
    let rows = table.getRowModel().rows
    let html =
        [
            yield "<table><thead><tr><th>Entry</th><th>Account</th><th>Category</th><th class=\"num\">Units</th><th class=\"num\">Amount (CHF)</th></tr></thead><tbody>"
            for r in rows do
                let units: float = r.getValue "units"
                let amount: float = r.getValue "amount"
                yield "<tr>"
                yield sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "id"))
                yield sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "account"))
                yield sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "category"))
                yield sprintf "<td class=\"num\">%.0f</td>" units
                yield sprintf "<td class=\"num\">%.0f</td>" amount
                yield "</tr>"
            yield "</tbody></table>"
        ]
        |> String.concat ""
    let wrap = (document.getElementById "table-wrap").Value
    wrap.innerHTML <- html
    let pg = readPagination ()
    let pageCount = int (pag.getPageCount ())
    let total = table.getCoreRowModel().rows.Count
    let meta = (document.getElementById "meta").Value
    meta.textContent <- sprintf "Page %d / %d · pageSize %.0f · showing %d of %d rows" (int pg.pageIndex + 1) pageCount pg.pageSize rows.Count total

let main () =
    wireBtn "btn-0" (fun () ->
        let pg = readPagination ()
        if pg.pageIndex > 0.0 then
            pag.setPageIndex (!^ (pg.pageIndex - 1.0))
        render ()
    )
    wireBtn "btn-1" (fun () ->
        let pg = readPagination ()
        let maxIdx = pag.getPageCount () - 1.0
        if pg.pageIndex < maxIdx then
            pag.setPageIndex (!^ (pg.pageIndex + 1.0))
        render ()
    )
    wireBtn "btn-2" (fun () ->
        pag.setPageIndex 0.0
        pag.setPageSize 5.0
        render ()
    )
    wireBtn "btn-3" (fun () ->
        pag.setPageIndex 0.0
        pag.setPageSize 6.0
        render ()
    )
    wireBtn "btn-4" (fun () ->
        pag.setPageIndex 0.0
        pag.setPageSize 9.0
        render ()
    )
    render ()

main ()
