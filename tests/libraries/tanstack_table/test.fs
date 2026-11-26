// TanStack Table (v9 core) from F#, in Node: a table of people built from a
// column helper, row models, sorting, and state changes. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

type Person = { name: string; age: float }

// FINDING 16: a table needs `coreReactivityFeature`, which only the package's subpath export
// "@tanstack/table-core/store-reactivity-bindings" has; subpaths are not bound, so it is imported by hand.
[<Import("storeReactivityBindings", "@tanstack/table-core/store-reactivity-bindings")>]
let storeReactivityBindings: unit -> TableReactivityBindings = jsNative

// FINDING 17: `TableFeatures.Create (coreRowModel = createCoreRowModel (), ...)` does not work: the factory is typed
// as an F# function (and does not even fit the field's type: "Type constraint mismatch"), so Fable wraps it in a
// new function (`uncurry2 (arg => f arg)`), losing the `feature` property the library reads: "table._rowModels.coreRowModel
// is not a function". The features are an object made by hand, the factories boxed.
let features: TableFeatures =
    unbox (
        createObj [
            "coreReactivityFeature" ==> storeReactivityBindings ()
            "rowSortingFeature" ==> rowSortingFeature
            "coreRowModel" ==> box (createCoreRowModel<obj> ())
            "sortedRowModel" ==> box (createSortedRowModel<obj> ())
        ]
    )

// Columns from the helper: a string accessor, with a header.
let helper = createColumnHelper<obj> ()
let named (header: string) =
    U6.Case2 (ColumnHelperAccessorColumnHeaderString.Create (header = header))
let nameColumn = helper.accessor<obj> (U2.Case2 "name", named "Name")
let ageColumn = helper.accessor<obj> (U2.Case2 "age", named "Age")
// An accessor function with an explicit id.
let doubleColumn =
    helper.accessor<obj> (
        U2.Case1 (System.Func<_, _, _> (fun (p: obj) (_: float) -> AccessorFnResult.CellData (box ((p?age: float) * 2.0)))),
        U6.Case1 (ColumnHelperAccessorColumnHeaderAccessorResultFooter.Create (id = "double", header = U2.Case1 "Double"))
    )

let people =
    ResizeArray<obj> [ box {| name = "Bo"; age = 30.0 |}; box {| name = "Al"; age = 25.0 |}; box {| name = "Cy"; age = 41.0 |} ]

// FINDING 18: TableOptions_Core (an intersection of four interfaces) has no Create and its fields are
// read-only, and the helper's columns are not ColumnDefBase_Core: the options are an object made by hand.
let options (initialSorting: ColumnSort list) (extra: (string * obj) list) : TableOptions_Core<obj> =
    unbox (
        createObj (
            [ "features" ==> features
              "data" ==> people
              "columns" ==> ResizeArray<obj> [ box nameColumn; box ageColumn; box doubleColumn ]
              "initialState" ==> createObj [ "sorting" ==> ResizeArray<ColumnSort> initialSorting ] ]
            @ extra
        )
    )

// FINDING 19: `Table` is only the core (`Table_Core`): the feature APIs (`setSorting` of Table_RowSorting)
// are on separate interfaces the table is not, so a table is cast to use them; and `table.store`
// is an empty interface (the type is from @tanstack/store), so the state is read by hand.
let sorting (t: Table<obj>) = unbox<Table_RowSorting<obj>> t
let stateOf (t: Table<obj>) : obj = (box t.store)?get ()
let sortedIds (t: Table<obj>) : string list =
    [ for s in (stateOf t)?sorting |> unbox<ResizeArray<ColumnSort>> -> s.id + (if s.desc then " desc" else " asc") ]

let table = constructTable<obj> (options [ ColumnSort.Create (false, "age") ] [])
check (table.getCoreRowModel().rows.Count = 3) "the core row model has a row per datum"
let names (rows: ResizeArray<Row<obj>>) = [ for r in rows -> r.getValue<string> "name" ]
check (names (table.getCoreRowModel().rows) = [ "Bo"; "Al"; "Cy" ]) "the core model keeps the data's order"
check (names (table.getRowModel().rows) = [ "Al"; "Bo"; "Cy" ]) "the row model is sorted by age, ascending"
check (sortedIds table = [ "age asc" ]) "the state holds the initial sorting"

// Rows: ids, indexes, the original datum, values by column id.
let rows = table.getRowModel().rows
check (rows.[0].id = "1" && rows.[0].index = 1.0) "a row's id and index are those of the data"
check ((rows.[0].original?name: string) = "Al") "original"
check (rows.[0].getValue<float> "age" = 25.0) "a value by column id"
check (rows.[0].getValue<float> "double" = 50.0) "a value from an accessor function"
check (rows.[2].getValue<float> "double" = 82.0) "another"
check (rows.[0].depth = 0.0 && rows.[0].subRows.Count = 0) "no sub rows"
check (table.getRow("2").getValue<string> "name" = "Cy") "getRow by id"

// Columns and cells.
let columns = table.getAllColumns ()
check ([ for c in columns -> c.id ] = [ "name"; "age"; "double" ]) "column ids"
check (table.getColumn("age").IsSome && table.getColumn("nope").IsNone) "getColumn"
check (table.getAllLeafColumns().Count = 3) "leaf columns"
let cells = rows.[0].getAllCells ()
check (cells.Count = 3 && cells.[0].id = "1_name") "cells"
check (string (cells.[1].getValue ()) = "25") "a cell's value"
check (obj.ReferenceEquals (cells.[0].row, rows.[0])) "a cell knows its row"
check (columns.[0].columnDef?header = "Name") "the column's header"

// Headers.
let groups = table.getHeaderGroups ()
check (groups.Count = 1 && groups.[0].headers.Count = 3) "header groups"
check (groups.[0].headers.[1].column.id = "age") "a header's column"

// Sorting through the table; the state follows.
(sorting table).setSorting (U2.Case1 (ResizeArray [ ColumnSort.Create (true, "age") ]))
check (names (table.getRowModel().rows) = [ "Cy"; "Bo"; "Al" ]) "sorted descending"
check (sortedIds table = [ "age desc" ]) "the state follows"
(sorting table).setSorting (U2.Case1 (ResizeArray [ ColumnSort.Create (false, "name") ]))
check (names (table.getRowModel().rows) = [ "Al"; "Bo"; "Cy" ]) "sorted by name"
(sorting table).setSorting (U2.Case1 (ResizeArray [ ColumnSort.Create (true, "double") ]))
check (names (table.getRowModel().rows) = [ "Cy"; "Bo"; "Al" ]) "sorted by an accessor function's column"
(sorting table).setSorting (U2.Case2 (fun (old: ResizeArray<ColumnSort>) -> ResizeArray [ ColumnSort.Create (false, "age") ]))
check (names (table.getRowModel().rows) = [ "Al"; "Bo"; "Cy" ]) "an updater function"
(sorting table).setSorting (U2.Case1 (ResizeArray<ColumnSort> ()))
check (names (table.getRowModel().rows) = [ "Bo"; "Al"; "Cy" ]) "unsorted: the data's order"
check (table.getCoreRowModel().rows.Count = 3) "the core model is unaffected"

// A column sorts itself.
let age = (table.getColumn "age").Value
let ageSorting = unbox<Column_RowSorting<obj>> age
check (ageSorting.getCanSort ()) "getCanSort"
ageSorting.toggleSorting (false)
check (sortedIds table = [ "age asc" ]) "toggleSorting"
check (names (table.getRowModel().rows) = [ "Al"; "Bo"; "Cy" ]) "sorted by the column"

// onStateChange: an option the table calls instead of changing its own state (controlled).
let changes = ResizeArray<string>()
let onSortingChange: Updater<SortingState> -> unit =
    let f =
        fun (u: Updater<SortingState>) ->
            match u with
            | U2.Case1 v -> changes.Add (string v.Count)
            | U2.Case2 _ -> changes.Add "fn"
    f
let controlled = constructTable<obj> (options [] [ "onSortingChange" ==> onSortingChange ])
check (names (controlled.getRowModel().rows) = [ "Bo"; "Al"; "Cy" ]) "no initial sorting"
(sorting controlled).setSorting (U2.Case1 (ResizeArray [ ColumnSort.Create (false, "age") ]))
check (changes.Count = 1 && changes.[0] = "fn") "onSortingChange was called, with an updater function"
check (names (controlled.getRowModel().rows) = [ "Bo"; "Al"; "Cy" ]) "and the table did not change itself"
check (sortedIds controlled = []) "its state is as it was"

printfn "%d checks passed" passed
