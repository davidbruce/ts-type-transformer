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
import gleam/option.{None, Some}
import gleam/string
import tanstack_table_core as tt
import tanstack_table_core/column_core_properties
import tanstack_table_core/column_row_sorting
import tanstack_table_core/column_sort
import tanstack_table_core/row_model
import tanstack_table_core/row_row
import tanstack_table_core/table_columns
import tanstack_table_core/table_row_models_core
import tanstack_table_core/table_row_sorting
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

fn col_def(key: String, header: String) {
  rt.js_object([#("accessorKey", rt.dynamic(key)), #("header", rt.dynamic(header))])
}

fn node_row(id: String, host: String, region: String, lat: Float, thr: Float, up: Float) {
  rt.js_object([
    #("id", rt.dynamic(id)),
    #("host", rt.dynamic(host)),
    #("region", rt.dynamic(region)),
    #("latency", rt.dynamic(lat)),
    #("throughput", rt.dynamic(thr)),
    #("uptime", rt.dynamic(up)),
  ])
}

fn sort_spec(id: String, desc: Bool) -> t.ColumnSort(Nil) {
  column_sort.create()
  |> column_sort.with_id(id)
  |> column_sort.with_desc(desc)
}

fn wire_btn(doc, id: String, action: fn() -> Nil) {
  case document.get_element_by_id(doc, id) {
    Some(btn) -> html_element.add_event_listener(btn, ev.click, fn(_) { action() }, None)
    None -> Nil
  }
}

fn fmt1(val: Float) -> String {
  let scaled = float.round(val *. 10.0)
  int.to_string(scaled / 10) <> "." <> int.to_string(scaled % 10)
}

fn fmt2(val: Float) -> String {
  let scaled = float.round(val *. 100.0)
  let frac = scaled % 100
  let pad = case frac < 10 {
    True -> "0"
    False -> ""
  }
  int.to_string(scaled / 100) <> "." <> pad <> int.to_string(frac)
}

fn decode_str(v: Dynamic, fallback: String) -> String {
  case decode.run(v, decode.string) {
    Ok(s) -> s
    Error(_) -> fallback
  }
}

fn decode_num(v: Dynamic) -> Float {
  case decode.run(v, decode.float) {
    Ok(n) -> n
    Error(_) -> 0.0
  }
}

fn render(table: t.TableCore(t.TableFeatures(Nil), Dynamic, Nil), doc, label: String) {
  let rows_view: t.TableRowModelsCore(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)
  let rows = row_model.get_rows(table_row_models_core.get_row_model(rows_view))

  let cols_view: t.TableColumns(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)
  let cols = table_columns.get_all_leaf_columns(cols_view)

  let header_cells =
    list.map(cols, fn(col) {
      let cid = column_core_properties.get_id(col)
      let hdr = case rt.js_get(rt.dynamic(column_core_properties.get_column_def(col)), "header") {
        Some(h) -> decode_str(h, cid)
        None -> cid
      }
      let csort: t.ColumnRowSorting(t.TableFeatures(Nil), Dynamic, Nil) = coerce(col)
      let mark = case column_row_sorting.get_is_sorted(csort) {
        t.BooleanOrSortDirectionSortDirection(t.SortDirectionAsc) -> " &#9650;"
        t.BooleanOrSortDirectionSortDirection(t.SortDirectionDesc) -> " &#9660;"
        _ -> ""
      }
      let cls = case cid == "latency" || cid == "throughput" || cid == "uptime" {
        True -> " class=\"num\""
        False -> ""
      }
      "<th" <> cls <> ">" <> esc_html(hdr) <> mark <> "</th>"
    })
    |> string.concat

  let body_rows =
    list.map(rows, fn(r) {
      let str_val = fn(k) { decode_str(row_row.get_value(coerce(r), k, rt.codec_dynamic()), "") }
      let num_val = fn(k) { decode_num(row_row.get_value(coerce(r), k, rt.codec_dynamic())) }
      let lat = fmt1(num_val("latency"))
      let thr = int.to_string(float.round(num_val("throughput")))
      let up = fmt2(num_val("uptime"))
      "<tr><td>"
      <> esc_html(str_val("id"))
      <> "</td><td>"
      <> esc_html(str_val("host"))
      <> "</td><td>"
      <> esc_html(str_val("region"))
      <> "</td><td class=\"num\">"
      <> lat
      <> "</td><td class=\"num\">"
      <> thr
      <> "</td><td class=\"num\">"
      <> up
      <> "%</td></tr>"
    })
    |> string.concat

  let html = "<table><thead><tr>" <> header_cells <> "</tr></thead><tbody>" <> body_rows <> "</tbody></table>"
  case document.get_element_by_id(doc, "table-wrap") {
    Some(wrap) -> element.set_inner_html(wrap, html)
    None -> Nil
  }
  let top_host = case rows {
    [first, ..] -> decode_str(row_row.get_value(coerce(first), "host", rt.codec_dynamic()), "-")
    [] -> "-"
  }
  let msg = label <> " · " <> int.to_string(list.length(rows)) <> " rows · top node: " <> top_host
  case document.get_element_by_id(doc, "meta") {
    Some(meta) -> dom_node.set_text_content(meta, Some(msg))
    None -> Nil
  }
}

pub fn main() {
  let sort_fns =
    rt.js_object([
      #("alphanumeric", rt.dynamic(tt.get_sort_fn_alphanumeric())),
      #("basic", rt.dynamic(tt.get_sort_fn_basic())),
      #("text", rt.dynamic(tt.get_sort_fn_text())),
    ])
  let features =
    rt.js_object([
      #("coreReactivityFeature", rt.dynamic(store_reactivity_bindings())),
      #("rowSortingFeature", rt.dynamic(tt.get_row_sorting_feature())),
      #("coreRowModel", rt.dynamic(tt.create_core_row_model())),
      #("sortedRowModel", rt.dynamic(tt.create_sorted_row_model())),
      #("sortFns", sort_fns),
    ])
  let data =
    rt.js_array([
      node_row("N-01", "eu-west-1a", "CH", 12.4, 4200.0, 99.98),
      node_row("N-02", "eu-west-1b", "CH", 8.7, 5100.0, 99.99),
      node_row("N-03", "eu-central-1a", "DE", 15.2, 3900.0, 99.91),
      node_row("N-04", "eu-central-1b", "DE", 6.3, 6400.0, 99.97),
      node_row("N-05", "eu-north-1a", "SE", 19.8, 2800.0, 99.85),
      node_row("N-06", "eu-north-1b", "SE", 11.1, 4750.0, 99.95),
      node_row("N-07", "us-east-1a", "US", 28.5, 7100.0, 99.94),
      node_row("N-08", "us-east-1b", "US", 21.0, 5900.0, 99.96),
    ])
  let cols =
    rt.js_array([
      col_def("id", "Node"),
      col_def("host", "Host"),
      col_def("region", "Region"),
      col_def("latency", "Latency (ms)"),
      col_def("throughput", "Req/s"),
      col_def("uptime", "Uptime %"),
    ])
  let init_state =
    rt.js_object([
      #("sorting", rt.dynamic(rt.js_array([rt.dynamic(sort_spec("region", False)), rt.dynamic(sort_spec("latency", False))]))),
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
  let tsort: t.TableRowSorting(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)

  wire_btn(doc, "btn-0", fn() {
    table_row_sorting.set_sorting(tsort, t.UpdaterT([sort_spec("region", False), sort_spec("latency", False)]))
    render(table, doc, "Sort: region asc, latency asc")
  })
  wire_btn(doc, "btn-1", fn() {
    table_row_sorting.set_sorting(tsort, t.UpdaterT([sort_spec("latency", False)]))
    render(table, doc, "Sort: latency asc")
  })
  wire_btn(doc, "btn-2", fn() {
    table_row_sorting.set_sorting(tsort, t.UpdaterT([sort_spec("throughput", True)]))
    render(table, doc, "Sort: throughput desc")
  })
  wire_btn(doc, "btn-3", fn() {
    table_row_sorting.set_sorting(tsort, t.UpdaterT([sort_spec("host", False)]))
    render(table, doc, "Sort: host asc")
  })
  wire_btn(doc, "btn-4", fn() {
    table_row_sorting.set_sorting(tsort, t.UpdaterT([]))
    render(table, doc, "Sort: unsorted (data order)")
  })
  render(table, doc, "Sort: region asc, latency asc")
}
