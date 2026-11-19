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
            "rowPaginationFeature" ==> rowPaginationFeature
            "coreRowModel" ==> box (createCoreRowModel<TableFeatures, obj> ())
            "paginatedRowModel" ==> box (createPaginatedRowModel<TableFeatures, obj> ())
        ]
    )

let private col (key: string) (header: string) : obj =
    createObj [ "accessorKey" ==> key; "header" ==> header ]

let private accounts = [| "ops-pool"; "clearing-ch"; "reserve-de" |]
let private cats = [| "compute"; "storage"; "egress"; "audit" |]

let private data =
    ResizeArray<obj> [
        for i in 0 .. 17 do
            createObj [
                "id" ==> sprintf "TX-%03d" (i + 1)
                "account" ==> accounts.[i % 3]
                "category" ==> cats.[i % 4]
                "units" ==> float ((i * 7) % 19 + 4)
                "amount" ==> float (((i * 37) % 90 + 15) * 12)
            ]
    ]

let private columns =
    ResizeArray<obj> [
        col "id" "Entry"
        col "account" "Account"
        col "category" "Category"
        col "units" "Units"
        col "amount" "Amount (CHF)"
    ]

let private options: TableOptions_Core<TableFeatures, obj> =
    unbox (
        createObj [
            "features" ==> features
            "data" ==> data
            "columns" ==> columns
            "initialState" ==> createObj [
                "pagination" ==> PaginationState.Create(0.0, 5.0)
            ]
        ]
    )

let private table = constructTable<TableFeatures, obj> options
let private pag = unbox<Table_RowPagination<TableFeatures, obj>> table

let private wireBtn (id: string) (action: unit -> unit) =
    let btn = unbox<HTMLElement> (document.getElementById id)
    btn.addEventListener (HTMLElementEventMapKeys.click, fun _ -> action ())

let private render () =
    let rows = table.getRowModel().rows
    let sb = ResizeArray<string>()
    sb.Add "<table><thead><tr><th>Entry</th><th>Account</th><th>Category</th><th class=\"num\">Units</th><th class=\"num\">Amount (CHF)</th></tr></thead><tbody>"
    for r in rows do
        let units: float = r.getValue "units"
        let amount: float = r.getValue "amount"
        sb.Add "<tr>"
        sb.Add (sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "id")))
        sb.Add (sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "account")))
        sb.Add (sprintf "<td>%s</td>" (escapeHtml (r.getValue<string> "category")))
        sb.Add (sprintf "<td class=\"num\">%.0f</td>" units)
        sb.Add (sprintf "<td class=\"num\">%.0f</td>" amount)
        sb.Add "</tr>"
    sb.Add "</tbody></table>"
    let wrap = unbox<HTMLElement> (document.getElementById "table-wrap")
    wrap.innerHTML <- String.concat "" sb
    let state: obj = (box table.store)?get ()
    let pg: PaginationState = unbox state?pagination
    let pageCount = int (pag.getPageCount ())
    let total = table.getCoreRowModel().rows.Count
    let meta = unbox<HTMLElement> (document.getElementById "meta")
    meta.textContent <- sprintf "Page %d / %d · pageSize %.0f · showing %d of %d rows" (int pg.pageIndex + 1) pageCount pg.pageSize rows.Count total

let main () =
    wireBtn "btn-0" (fun () ->
        let state: obj = (box table.store)?get ()
        let pg: PaginationState = unbox state?pagination
        if pg.pageIndex > 0.0 then
            pag.setPageIndex (U2.Case1 (pg.pageIndex - 1.0))
        render ()
    )
    wireBtn "btn-1" (fun () ->
        let state: obj = (box table.store)?get ()
        let pg: PaginationState = unbox state?pagination
        let maxIdx = pag.getPageCount () - 1.0
        if pg.pageIndex < maxIdx then
            pag.setPageIndex (U2.Case1 (pg.pageIndex + 1.0))
        render ()
    )
    wireBtn "btn-2" (fun () ->
        pag.setPageIndex (U2.Case1 0.0)
        pag.setPageSize (U2.Case1 5.0)
        render ()
    )
    wireBtn "btn-3" (fun () ->
        pag.setPageIndex (U2.Case1 0.0)
        pag.setPageSize (U2.Case1 6.0)
        render ()
    )
    wireBtn "btn-4" (fun () ->
        pag.setPageIndex (U2.Case1 0.0)
        pag.setPageSize (U2.Case1 9.0)
        render ()
    )
    render ()

main ()
