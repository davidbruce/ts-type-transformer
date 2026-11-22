import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import dom/areas/html/html_element_event_map as ev
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/string
import tanstack_table_core as tt
import tanstack_table_core/row_column_grouping
import tanstack_table_core/row_core_properties
import tanstack_table_core/row_model
import tanstack_table_core/row_row
import tanstack_table_core/row_row_expanding
import tanstack_table_core/table_column_grouping
import tanstack_table_core/table_core_properties
import tanstack_table_core/table_row_expanding
import tanstack_table_core/table_row_models_core
import tanstack_table_core/runtime as rt
import tanstack_table_core/types as t

@external(javascript, "@tanstack/table-core/store-reactivity-bindings", "storeReactivityBindings")
fn store_reactivity_bindings() -> t.TableReactivityBindings(Nil)

// Used strictly for TanStack v9 structural feature-mixin intersection views and construct_table options.
@external(javascript, "./app_ffi.mjs", "coerce")
fn coerce(a: a) -> b

fn esc_html(s: String) -> String {
  s
  |> string.replace("&", "&amp;")
  |> string.replace("<", "&lt;")
  |> string.replace(">", "&gt;")
}

fn col_def(key: String, header: String, agg_fn: Option(String)) {
  case agg_fn {
    Some(aname) ->
      rt.js_object([
        #("accessorKey", rt.dynamic(key)),
        #("header", rt.dynamic(header)),
        #("aggregationFn", rt.dynamic(aname)),
      ])
    None ->
      rt.js_object([
        #("accessorKey", rt.dynamic(key)),
        #("header", rt.dynamic(header)),
      ])
  }
}

fn host_row(region: String, host: String, role: String, cpu: Float, mem: Float) {
  rt.js_object([
    #("region", rt.dynamic(region)),
    #("host", rt.dynamic(host)),
    #("role", rt.dynamic(role)),
    #("cpu", rt.dynamic(cpu)),
    #("mem", rt.dynamic(mem)),
    #("nodes", rt.dynamic(1.0)),
  ])
}

fn wire_btn(doc, id: String, action: fn() -> Nil) {
  case document.get_element_by_id(doc, id) {
    Some(btn) -> html_element.add_event_listener(btn, ev.click, fn(_) { action() }, None)
    None -> Nil
  }
}

fn current_grp_col(table: t.TableCore(t.TableFeatures(Nil), Dynamic, Nil)) -> String {
  let store = table_core_properties.get_store(coerce(table))
  case decode.run(rt.dynamic(store), decode.at(["state", "grouping"], decode.list(decode.string))) {
    Ok([first, ..]) -> first
    _ -> "region"
  }
}

fn render(table: t.TableCore(t.TableFeatures(Nil), Dynamic, Nil), doc) {
  let rows_view: t.TableRowModelsCore(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)
  let rows = row_model.get_rows(table_row_models_core.get_row_model(rows_view))
  let grp_col = current_grp_col(table)

  let #(group_cnt, row_htmls) =
    list.map_fold(rows, 0, fn(acc, r) {
      let is_grp = row_column_grouping.get_is_grouped(coerce(r))
      let num_val = fn(k) -> Float {
        case decode.run(row_row.get_value(coerce(r), k, rt.codec_dynamic()), decode.float) {
          Ok(n) -> n
          Error(_) -> 0.0
        }
      }
      let str_val = fn(k) -> String {
        case decode.run(row_row.get_value(coerce(r), k, rt.codec_dynamic()), decode.string) {
          Ok(s) -> s
          Error(_) -> ""
        }
      }
      let cpu = num_val("cpu")
      let mem = int.to_string(float.round(num_val("mem")))
      let nodes = int.to_string(float.round(num_val("nodes")))
      case is_grp {
        True -> {
          let is_exp = row_row_expanding.get_is_expanded(coerce(r))
          let mark = case is_exp {
            True -> "[-] "
            False -> "[+] "
          }
          let sub_n = list.length(row_core_properties.get_sub_rows(coerce(r)))
          let tr =
            "<tr class=\"group-row\"><td>"
            <> mark
            <> esc_html(grp_col)
            <> "="
            <> esc_html(str_val(grp_col))
            <> " ("
            <> int.to_string(sub_n)
            <> ")</td><td>"
            <> esc_html(str_val("region"))
            <> "</td><td>"
            <> esc_html(str_val("role"))
            <> "</td><td class=\"num\">"
            <> rt.to_fixed(cpu, 1)
            <> "</td><td class=\"num\">"
            <> mem
            <> "</td><td class=\"num\">"
            <> nodes
            <> "</td></tr>"
          #(acc + 1, tr)
        }
        False -> {
          let tr =
            "<tr><td class=\"child-indent\">"
            <> esc_html(str_val("host"))
            <> "</td><td>"
            <> esc_html(str_val("region"))
            <> "</td><td>"
            <> esc_html(str_val("role"))
            <> "</td><td class=\"num\">"
            <> int.to_string(float.round(cpu))
            <> "</td><td class=\"num\">"
            <> mem
            <> "</td><td class=\"num\">"
            <> nodes
            <> "</td></tr>"
          #(acc, tr)
        }
      }
    })

  let html =
    "<table><thead><tr><th>Group / Host</th><th>Region</th><th>Role</th><th class=\"num\">CPU %</th><th class=\"num\">Mem GB</th><th class=\"num\">Nodes</th></tr></thead><tbody>"
    <> string.concat(row_htmls)
    <> "</tbody></table>"
  case document.get_element_by_id(doc, "table-wrap") {
    Some(wrap) -> element.set_inner_html(wrap, html)
    None -> Nil
  }
  let msg =
    "Grouped by "
    <> grp_col
    <> " · "
    <> int.to_string(group_cnt)
    <> " groups · "
    <> int.to_string(list.length(rows))
    <> " visible rows"
  case document.get_element_by_id(doc, "meta") {
    Some(meta) -> dom_node.set_text_content(meta, Some(msg))
    None -> Nil
  }
}

pub fn main() {
  let agg_fns =
    rt.js_object([
      #("sum", rt.dynamic(tt.get_aggregation_fn_sum())),
      #("mean", rt.dynamic(tt.get_aggregation_fn_mean())),
      #("count", rt.dynamic(tt.get_aggregation_fn_count())),
    ])
  let features =
    rt.js_object([
      #("coreReactivityFeature", rt.dynamic(store_reactivity_bindings())),
      #("columnGroupingFeature", rt.dynamic(tt.get_column_grouping_feature())),
      #("rowAggregationFeature", rt.dynamic(tt.get_row_aggregation_feature())),
      #("rowExpandingFeature", rt.dynamic(tt.get_row_expanding_feature())),
      #("coreRowModel", rt.dynamic(tt.create_core_row_model())),
      #("groupedRowModel", rt.dynamic(tt.create_grouped_row_model())),
      #("expandedRowModel", rt.dynamic(tt.create_expanded_row_model())),
      #("aggregationFns", agg_fns),
    ])
  let data =
    rt.js_array([
      host_row("CH", "zurich-api-01", "api", 72.0, 64.0),
      host_row("CH", "basel-db-01", "db", 58.0, 128.0),
      host_row("CH", "geneva-edge-01", "edge", 35.0, 32.0),
      host_row("DE", "munich-api-02", "api", 84.0, 64.0),
      host_row("DE", "berlin-db-02", "db", 66.0, 128.0),
      host_row("DE", "frankfurt-edge-02", "edge", 42.0, 32.0),
      host_row("FR", "paris-api-03", "api", 60.0, 64.0),
      host_row("FR", "lyon-db-03", "db", 48.0, 128.0),
      host_row("FR", "marseille-edge-03", "edge", 30.0, 32.0),
    ])
  let cols =
    rt.js_array([
      col_def("region", "Region", None),
      col_def("host", "Host", None),
      col_def("role", "Role", None),
      col_def("cpu", "CPU % (mean)", Some("mean")),
      col_def("mem", "Mem GB (sum)", Some("sum")),
      col_def("nodes", "Nodes (count)", Some("count")),
    ])
  let init_state =
    rt.js_object([
      #("grouping", rt.js_array([rt.dynamic("region")])),
      #("expanded", rt.dynamic(True)),
    ])
  let options =
    rt.js_object([
      #("features", features),
      #("data", data),
      #("columns", cols),
      #("initialState", init_state),
    ])
  let table = tt.construct_table(coerce(options))
  let doc = core.get_document()
  let grp: t.TableColumnGrouping(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)
  let exp: t.TableRowExpanding(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)

  wire_btn(doc, "btn-0", fn() {
    table_row_expanding.toggle_all_rows_expanded(exp, Some(True))
    render(table, doc)
  })
  wire_btn(doc, "btn-1", fn() {
    table_row_expanding.toggle_all_rows_expanded(exp, Some(False))
    render(table, doc)
  })
  wire_btn(doc, "btn-2", fn() {
    table_column_grouping.set_grouping(grp, t.UpdaterT(["region"]))
    table_row_expanding.set_expanded(exp, t.UpdaterT(t.ExpandedStateBoolean(True)))
    render(table, doc)
  })
  wire_btn(doc, "btn-3", fn() {
    table_column_grouping.set_grouping(grp, t.UpdaterT(["role"]))
    table_row_expanding.set_expanded(exp, t.UpdaterT(t.ExpandedStateBoolean(True)))
    render(table, doc)
  })
  render(table, doc)
}
