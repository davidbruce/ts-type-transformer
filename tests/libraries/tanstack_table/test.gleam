// TanStack Table (table-core v9) from Gleam, in Node: what can be reached
// without a constructed table. A table itself cannot be made (FINDING 9).
import bindings/areas/main as table
import bindings/areas/main/column_helper
import bindings/areas/main/column_helper_accessor_column_header_string as header_options
import bindings/areas/main/core_features
import bindings/areas/main/get_function_name_info_result as name_info
import bindings/areas/main/table_features
import bindings/areas/main/table_options_core
import bindings/runtime
import bindings/types as t
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

fn field(d: dynamic.Dynamic, path: List(String), decoder: decode.Decoder(a)) -> a {
  let assert Ok(v) = decode.run(d, decode.at(path, decoder))
  v
}

pub fn main() {
  // ---- features: a registry of the ones a table uses ----
  let features = table_features.create()
  table_features.set_row_sorting_feature(features, Some(table.get_row_sorting_feature()))
  table_features.set_row_pagination_feature(features, Some(table.get_row_pagination_feature()))
  let registry = table.table_features(runtime.dynamic(features))
  let state = table.get_initial_table_state(registry, None)
  let passed =
    check(option.is_some(table_features.get_row_sorting_feature(registry)), "the sorting feature is registered")
    + check(option.is_none(table_features.get_column_filtering_feature(registry)), "filtering is not")
    + check(field(state, ["sorting"], decode.list(decode.dynamic)) == [], "initial sorting state is empty")
    + check(field(state, ["pagination", "pageIndex"], decode.int) == 0, "initial page index")
    + check(field(state, ["pagination", "pageSize"], decode.int) == 10, "initial page size")
  let core = core_features.create()
  let passed =
    passed
    + check(
      option.is_none(core_features.get_core_reactivity_feature(core)),
      "the core features have no reactivity of their own",
    )

  // ---- column definitions from a column helper ----
  let helper = table.create_column_helper()
  let accessor = column_helper.get_accessor(helper, t.table_features_codec(), runtime.codec_dynamic())
  let name_options = header_options.create()
  header_options.set_header(name_options, "Name")
  let name = accessor(runtime.dynamic("name"), runtime.dynamic(name_options))
  let age_options = header_options.create()
  header_options.set_header(age_options, "Age")
  let age = accessor(runtime.dynamic("age"), runtime.dynamic(age_options))
  let passed =
    passed
    + check(field(name, ["accessorKey"], decode.string) == "name", "accessorKey")
    + check(field(name, ["header"], decode.string) == "Name", "header")
    + check(field(age, ["accessorKey"], decode.string) == "age", "a second column")
  let columns = column_helper.get_columns(helper, t.table_features_codec(), runtime.codec_dynamic())
  let defs = columns([name, age])
  let passed = passed + check(list.length(defs) == 2, "columns() keeps the definitions")

  // ---- small helpers ----
  let info = table.get_function_name_info("row_getValue", None)
  let passed =
    passed
    + check(name_info.get_fn_key(info) == "getValue", "fn key")
    + check(name_info.get_fn_name(info) == "row.getValue", "fn name")
    + check(name_info.get_parent_name(info) == "row", "parent name")
    + check(
      table.functional_update(t.UpdaterT(9.0), 3.0, runtime.codec_float()) == 9.0,
      "functional_update with a value",
    )
    + check(
      table.functional_update(t.UpdaterUpdaterFn2(fn(x) { x +. 1.0 }), 3.0, runtime.codec_float()) == 4.0,
      "functional_update with a function",
    )
    + check(table.is_function(runtime.dynamic(fn() { 1 })), "is_function")
    + check(!table.is_function(runtime.dynamic(1)), "a number is no function")
    + check(!table.state_slices_equal(runtime.dynamic("a"), runtime.dynamic("b")), "different state slices")
    + check(table.state_slices_equal(runtime.dynamic("a"), runtime.dynamic("a")), "equal state slices")
    + check(table.clone_state([1.0, 2.0], runtime.codec_list(runtime.codec_float())) == [1.0, 2.0], "clone_state")
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
    + check(table.flatten_by([1.0], children, runtime.codec_float()) == [1.0, 2.0, 3.0, 4.0], "flatten_by walks the tree")
    + check(table.flatten_by([2.0, 4.0], children, runtime.codec_float()) == [2.0, 4.0], "flatten_by of leaves")

  // ---- a table ----
  // FINDING 9: constructTable needs features.coreReactivityFeature, a
  // TableReactivityBindings whose atoms (`Atom`, `ReadonlyAtom`) are opaque types
  // with no members or constructor here, and the module that makes one
  // (@tanstack/table-core/store-reactivity-bindings) is not a bound package. No
  // table, row model or sorted rows can be reached from Gleam; the call below
  // throws a TypeError.
  let attempt =
    runtime.attempt(fn() { table.construct_table(table_options_core.create()) })
  let passed =
    passed
    + case attempt {
      Ok(_) -> panic as "a table without features should not be made"
      Error(e) ->
        check(string.contains(runtime.js_error_message(e), "coreReactivityFeature"), "no table without reactivity")
    }

  io.println(int.to_string(passed) <> " checks passed")
}
