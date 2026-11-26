// TanStack Table (table-core v9) from Gleam, in Node: the helpers, and a table
// built with reactivity written here, sorted through setSorting.
//
// FINDING 9 (still open, awkward): Atom and ReadonlyAtom are opaque with no
// constructor, so the reactivity's atoms are plain objects taken as Atoms by an
// identity @external to runtime_ffi.mjs's `dynamic`; so are the table options
// object and the table's TableRowSorting view (`coerce`).
import bindings as table
import bindings/column_helper
import bindings/column_helper_accessor_fn_column_header_string as header_options
import bindings/column_sort
import bindings/core_features
import bindings/row_core_properties
import bindings/row_model
import bindings/runtime
import bindings/table_core_properties
import bindings/table_features
import bindings/table_options_core
import bindings/table_reactivity_bindings as reactivity
import bindings/table_row_models_core
import bindings/table_row_sorting
import bindings/types as t
import cell
import gleam/dynamic
import gleam/dynamic/decode
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/string

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

// Atom and the other opaque types have no constructor: a plain object is taken
// as one (an identity in JavaScript).
@external(javascript, "./bindings/runtime_ffi.mjs", "dynamic")
fn atom_of(d: dynamic.Dynamic) -> t.Atom(dynamic.Dynamic)

@external(javascript, "./bindings/runtime_ffi.mjs", "dynamic")
fn readonly_atom_of(d: dynamic.Dynamic) -> t.ReadonlyAtom(dynamic.Dynamic)

@external(javascript, "./bindings/runtime_ffi.mjs", "dynamic")
fn coerce(a: a) -> b

fn field(
  d: dynamic.Dynamic,
  path: List(String),
  decoder: decode.Decoder(a),
) -> a {
  let assert Ok(v) = decode.run(d, decode.at(path, decoder))
  v
}

pub fn main() {
  // ---- features: a registry of the ones a table uses ----
  let features = table_features.create()
  table_features.set_row_sorting_feature(
    features,
    Some(table.get_row_sorting_feature()),
  )
  table_features.set_row_pagination_feature(
    features,
    Some(table.get_row_pagination_feature()),
  )
  let registry = table.table_features(runtime.dynamic(features))
  let state_dyn = fn(s: t.TableState) {
    case s {
      t.TableStateTableStateObject(o) -> runtime.dynamic(o)
      t.TableStateTableStateCellSelection2(o) -> runtime.dynamic(o)
    }
  }
  let state = table.get_initial_table_state(registry)
  let passed =
    check(
      option.is_some(table_features.get_row_sorting_feature(registry)),
      "the sorting feature is registered",
    )
    + check(
      option.is_none(table_features.get_column_filtering_feature(registry)),
      "filtering is not",
    )
    + check(
      field(state_dyn(state), ["sorting"], decode.list(decode.dynamic)) == [],
      "initial sorting state is empty",
    )
    + check(
      field(state_dyn(state), ["pagination", "pageIndex"], decode.int) == 0,
      "initial page index",
    )
    + check(
      field(state_dyn(state), ["pagination", "pageSize"], decode.int) == 10,
      "initial page size",
    )
  let core = core_features.create()
  let passed =
    passed
    + check(
      option.is_none(core_features.get_core_reactivity_feature(core)),
      "the core features have no reactivity of their own",
    )

  // ---- column definitions from a column helper ----
  let helper = table.create_column_helper()
  let name_options = header_options.create()
  header_options.set_header(name_options, "Name")
  let name =
    column_helper.accessor(
      helper,
      t.AccessorFnOrStringString("name"),
      runtime.dynamic(name_options),
      runtime.codec_dynamic(),
    )
  let age_options = header_options.create()
  header_options.set_header(age_options, "Age")
  let age =
    column_helper.accessor(
      helper,
      t.AccessorFnOrStringString("age"),
      runtime.dynamic(age_options),
      runtime.codec_dynamic(),
    )
  let passed =
    passed
    + check(
      field(name, ["accessorKey"], decode.string) == "name",
      "accessorKey",
    )
    + check(field(name, ["header"], decode.string) == "Name", "header")
    + check(
      field(age, ["accessorKey"], decode.string) == "age",
      "a second column",
    )
  let defs = column_helper.columns(helper, [name, age])
  let passed =
    passed + check(list.length(defs) == 2, "columns() keeps the definitions")

  // ---- small helpers ----
  let info = table.get_function_name_info("row_getValue")
  let passed =
    passed
    + check(
      table.get_function_name_info_result_get_fn_key(info) == "getValue",
      "fn key",
    )
    + check(
      table.get_function_name_info_result_get_fn_name(info) == "row.getValue",
      "fn name",
    )
    + check(
      table.get_function_name_info_result_get_parent_name(info) == "row",
      "parent name",
    )
    + check(
      table.functional_update(t.UpdaterT(9.0), 3.0, runtime.codec_float())
        == 9.0,
      "functional_update with a value",
    )
    + check(
      table.functional_update(
        t.UpdaterUpdaterFnCb(fn(x) { x +. 1.0 }),
        3.0,
        runtime.codec_float(),
      )
        == 4.0,
      "functional_update with a function",
    )
    + check(table.is_function(runtime.dynamic(fn() { 1 })), "is_function")
    + check(!table.is_function(runtime.dynamic(1)), "a number is no function")
    + check(
      !table.state_slices_equal(runtime.dynamic("a"), runtime.dynamic("b")),
      "different state slices",
    )
    + check(
      table.state_slices_equal(runtime.dynamic("a"), runtime.dynamic("a")),
      "equal state slices",
    )
    + check(
      table.clone_state([1.0, 2.0], runtime.codec_list(runtime.codec_float()))
        == [1.0, 2.0],
      "clone_state",
    )
  // A tree of numbers: 1 has children 2 and 3, 3 has the child 4.
  let children = fn(n: Float) {
    case n {
      1.0 -> [2.0, 3.0]
      3.0 -> [4.0]
      _ -> []
    }
  }
  let passed =
    passed
    + check(
      table.flatten_by([1.0], children, runtime.codec_float())
        == [1.0, 2.0, 3.0, 4.0],
      "flatten_by walks the tree",
    )
    + check(
      table.flatten_by([2.0, 4.0], children, runtime.codec_float())
        == [2.0, 4.0],
      "flatten_by of leaves",
    )

  // ---- a table ----
  // constructTable needs features.coreReactivityFeature: bindings holding atoms
  // (an atom is a plain object with get, set and subscribe; see FINDING 9).
  let attempt =
    runtime.attempt(fn() { table.construct_table(table_options_core.create()) })
  let passed =
    passed
    + case attempt {
      Ok(_) -> panic as "a table without features should not be made"
      Error(e) ->
        check(
          string.contains(runtime.js_error_message(e), "coreReactivityFeature"),
          "no table without reactivity",
        )
    }

  let bindings = reactivity.create()
  reactivity.set_create_options_store(bindings, False)
  reactivity.set_wrap_external_atoms(bindings, False)
  let atom = fn(
    get: dynamic.Dynamic,
    set: option.Option(fn(dynamic.Dynamic) -> Nil),
  ) {
    let o = table.make_object_map_result_create()
    table.make_object_map_result_set_at(o, "get", get, runtime.codec_dynamic())
    case set {
      Some(f) ->
        table.make_object_map_result_set_at(
          o,
          "set",
          runtime.dynamic(f),
          runtime.codec_dynamic(),
        )
      None -> Nil
    }
    table.make_object_map_result_set_at(
      o,
      "subscribe",
      runtime.dynamic(fn(_) { Nil }),
      runtime.codec_dynamic(),
    )
    runtime.dynamic(o)
  }
  reactivity.set_create_writable_atom(bindings, fn(initial, _) {
    let value = cell.new(initial)
    atom(
      runtime.dynamic(fn() { cell.get(value) }),
      Some(fn(v) {
        cell.set(
          value,
          table.functional_update(
            t.UpdaterT(v),
            cell.get(value),
            runtime.codec_dynamic(),
          ),
        )
      }),
    )
    |> atom_of
  })
  reactivity.set_create_readonly_atom(bindings, fn(compute, _) {
    atom(runtime.dynamic(compute), None) |> readonly_atom_of
  })
  reactivity.set_untrack(bindings, fn(f) {
    table.call_void_to_t_fn(f, runtime.codec_dynamic())
  })
  let run_it = fn(f) { table.call_void_fn(f) }
  reactivity.set_batch(bindings, run_it)
  reactivity.set_schedule(bindings, run_it)
  reactivity.set_add_subscription(bindings, fn(_) { Nil })

  let person = fn(name: String, age: Float) {
    let o = table.make_object_map_result_create()
    table.make_object_map_result_set_at(
      o,
      "name",
      dynamic.string(name),
      runtime.codec_dynamic(),
    )
    table.make_object_map_result_set_at(
      o,
      "age",
      dynamic.float(age),
      runtime.codec_dynamic(),
    )
    runtime.dynamic(o)
  }
  let column = fn(key: String, header: String) {
    let o = table.make_object_map_result_create()
    table.make_object_map_result_set_at(
      o,
      "accessorKey",
      dynamic.string(key),
      runtime.codec_dynamic(),
    )
    table.make_object_map_result_set_at(
      o,
      "header",
      dynamic.string(header),
      runtime.codec_dynamic(),
    )
    runtime.dynamic(o)
  }
  let feature_map = table.make_object_map_result_create()
  table.make_object_map_result_set_at(
    feature_map,
    "coreReactivityFeature",
    runtime.dynamic(bindings),
    runtime.codec_dynamic(),
  )
  table.make_object_map_result_set_at(
    feature_map,
    "rowSortingFeature",
    runtime.dynamic(table.get_row_sorting_feature()),
    runtime.codec_dynamic(),
  )
  table.make_object_map_result_set_at(
    feature_map,
    "sortedRowModel",
    runtime.dynamic(table.create_sorted_row_model()),
    runtime.codec_dynamic(),
  )
  let options = table.make_object_map_result_create()
  table.make_object_map_result_set_at(
    options,
    "features",
    runtime.dynamic(feature_map),
    runtime.codec_dynamic(),
  )
  table.make_object_map_result_set_at(
    options,
    "data",
    dynamic.array([person("bo", 30.0), person("al", 20.0), person("cy", 25.0)]),
    runtime.codec_dynamic(),
  )
  table.make_object_map_result_set_at(
    options,
    "columns",
    dynamic.array([column("name", "Name"), column("age", "Age")]),
    runtime.codec_dynamic(),
  )
  let tbl = table.construct_table(coerce(options))
  let initial = table_core_properties.get_initial_state(tbl)
  let passed =
    passed
    + check(
      field(state_dyn(initial), ["sorting"], decode.list(decode.dynamic)) == [],
      "the table starts unsorted",
    )
    + check(
      field(
        runtime.dynamic(table_core_properties.get_options(tbl)),
        ["data"],
        decode.list(decode.dynamic),
      )
      |> list.length
        == 3,
      "the table's options hold the data",
    )
    + check(
      field(
        runtime.dynamic(table_core_properties.get_options(tbl)),
        ["columns"],
        decode.list(decode.dynamic),
      )
      |> list.length
        == 2,
      "and the columns",
    )

  // Core row model factory works directly and through getRowModel().
  let core_model_fn = table.create_core_row_model()
  let core_model = core_model_fn(tbl)()
  let name_of = fn(r) {
    field(
      row_core_properties.get_original(r, runtime.codec_dynamic()),
      ["name"],
      decode.string,
    )
  }
  let passed =
    passed
    + check(
      list.map(row_model.get_rows(core_model), name_of) == ["bo", "al", "cy"],
      "createCoreRowModel lists rows",
    )
  let model_view: t.TableRowModelsCore(dynamic.Dynamic, Nil) = coerce(tbl)
  let unsorted_model = table_row_models_core.get_row_model(model_view)
  let passed =
    passed
    + check(
      list.map(row_model.get_rows(unsorted_model), name_of)
        == ["bo", "al", "cy"],
      "getRowModel starts in data order",
    )

  // Sorting by age, through the table's setSorting (a function-valued property).
  let by_age = column_sort.create()
  column_sort.set_id(by_age, "age")
  column_sort.set_desc(by_age, False)
  let sortable: t.TableRowSorting(dynamic.Dynamic, Nil) = coerce(tbl)
  table_row_sorting.set_sorting(sortable, t.UpdaterT([by_age]))
  let sorted_model = table_row_models_core.get_row_model(model_view)
  let passed =
    passed
    + check(
      field(
        runtime.dynamic(table_core_properties.get_store(tbl)),
        ["state", "sorting"],
        decode.list(decode.at(["id"], decode.string)),
      )
        == ["age"],
      "setSorting reached the table's state",
    )
    + check(
      list.map(row_model.get_rows(sorted_model), name_of) == ["al", "cy", "bo"],
      "sorted row model orders by age",
    )

  io.println(int.to_string(passed) <> " checks passed")
}
