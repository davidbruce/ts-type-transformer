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
import tanstack_table_core/column_filter
import tanstack_table_core/row_model
import tanstack_table_core/row_row
import tanstack_table_core/runtime as rt
import tanstack_table_core/table_column_filtering
import tanstack_table_core/table_global_filtering
import tanstack_table_core/table_row_models_core
import tanstack_table_core/types as t
import tanstack_table_core/updater

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

fn col_def(key: String, header: String, filter_fn: Option(String)) {
  case filter_fn {
    Some(fname) ->
      rt.js_object([
        #("accessorKey", rt.dynamic(key)),
        #("header", rt.dynamic(header)),
        #("filterFn", rt.dynamic(fname)),
      ])
    None ->
      rt.js_object([
        #("accessorKey", rt.dynamic(key)),
        #("header", rt.dynamic(header)),
      ])
  }
}

fn svc_row(
  id: String,
  service: String,
  tier: String,
  region: String,
  reqs: Float,
  errs: Float,
) {
  rt.js_object([
    #("id", rt.dynamic(id)),
    #("service", rt.dynamic(service)),
    #("tier", rt.dynamic(tier)),
    #("region", rt.dynamic(region)),
    #("requests", rt.dynamic(reqs)),
    #("errors", rt.dynamic(errs)),
  ])
}

fn filter_spec(id: String, value: Dynamic) -> t.ColumnFilter(Nil) {
  column_filter.create()
  |> column_filter.with_id(id)
  |> column_filter.with_value(value)
}

fn wire_btn(doc, id: String, action: fn() -> Nil) {
  case document.get_element_by_id(doc, id) {
    Some(btn) ->
      html_element.add_event_listener(btn, ev.click, fn(_) { action() })
    None -> Nil
  }
}

fn render(table: t.TableCore(Dynamic, Nil), doc, label: String) {
  let rows_view: t.TableRowModelsCore(Dynamic, Nil) = coerce(table)
  let rows = row_model.get_rows(table_row_models_core.get_row_model(rows_view))
  let total =
    list.length(
      row_model.get_rows(table_row_models_core.get_core_row_model(rows_view)),
    )

  let body =
    list.map(rows, fn(r) {
      let str_val = fn(k) -> String {
        case
          decode.run(
            row_row.get_value(coerce(r), k, rt.codec_dynamic()),
            decode.string,
          )
        {
          Ok(s) -> s
          Error(_) -> ""
        }
      }
      let int_val = fn(k) -> String {
        let num = case
          decode.run(
            row_row.get_value(coerce(r), k, rt.codec_dynamic()),
            decode.float,
          )
        {
          Ok(n) -> n
          Error(_) -> 0.0
        }
        int.to_string(float.round(num))
      }
      "<tr><td>"
      <> esc_html(str_val("id"))
      <> "</td><td>"
      <> esc_html(str_val("service"))
      <> "</td><td>"
      <> esc_html(str_val("tier"))
      <> "</td><td>"
      <> esc_html(str_val("region"))
      <> "</td><td class=\"num\">"
      <> int_val("requests")
      <> "</td><td class=\"num\">"
      <> int_val("errors")
      <> "</td></tr>"
    })
    |> string.concat

  let html =
    "<table><thead><tr><th>ID</th><th>Service</th><th>Tier</th><th>Region</th><th class=\"num\">Req/s</th><th class=\"num\">Errors</th></tr></thead><tbody>"
    <> body
    <> "</tbody></table>"
  case document.get_element_by_id(doc, "table-wrap") {
    Some(wrap) -> element.set_inner_html(wrap, html)
    None -> Nil
  }
  let msg =
    label
    <> " · showing "
    <> int.to_string(list.length(rows))
    <> " / "
    <> int.to_string(total)
    <> " services"
  case document.get_element_by_id(doc, "meta") {
    Some(meta) -> dom_node.set_text_content(meta, Some(msg))
    None -> Nil
  }
}

pub fn main() {
  let filter_fns =
    rt.js_object([
      #("includesString", rt.dynamic(tt.get_filter_fn_includes_string())),
      #("equalsString", rt.dynamic(tt.get_filter_fn_equals_string())),
      #(
        "greaterThanOrEqualTo",
        rt.dynamic(tt.get_filter_fn_greater_than_or_equal_to()),
      ),
    ])
  let features =
    rt.js_object([
      #("coreReactivityFeature", rt.dynamic(store_reactivity_bindings())),
      #("columnFilteringFeature", rt.dynamic(tt.get_column_filtering_feature())),
      #("globalFilteringFeature", rt.dynamic(tt.get_global_filtering_feature())),
      #("coreRowModel", rt.dynamic(tt.create_core_row_model())),
      #("filteredRowModel", rt.dynamic(tt.create_filtered_row_model())),
      #("filterFns", filter_fns),
    ])
  let data =
    rt.js_array([
      svc_row("S-01", "auth-gateway", "core", "CH", 620.0, 2.0),
      svc_row("S-02", "session-cache", "core", "CH", 480.0, 1.0),
      svc_row("S-03", "billing-ledger", "core", "DE", 510.0, 6.0),
      svc_row("S-04", "oauth-signer", "core", "FR", 430.0, 3.0),
      svc_row("S-05", "trace-ingestor", "edge", "DE", 290.0, 8.0),
      svc_row("S-06", "cdn-router", "edge", "FR", 840.0, 5.0),
      svc_row("S-07", "static-assets", "edge", "CH", 710.0, 0.0),
      svc_row("S-08", "mail-relay", "batch", "DE", 140.0, 7.0),
      svc_row("S-09", "report-export", "batch", "FR", 210.0, 4.0),
      svc_row("S-10", "audit-auth-log", "core", "US", 360.0, 9.0),
    ])
  let cols =
    rt.js_array([
      col_def("id", "ID", None),
      col_def("service", "Service", None),
      col_def("tier", "Tier", Some("equalsString")),
      col_def("region", "Region", None),
      col_def("requests", "Req/s", Some("greaterThanOrEqualTo")),
      col_def("errors", "Errors", Some("greaterThanOrEqualTo")),
    ])
  let init_state =
    rt.js_object([
      #(
        "columnFilters",
        rt.js_array([
          rt.dynamic(filter_spec("tier", rt.dynamic("core"))),
          rt.dynamic(filter_spec("requests", rt.dynamic(400.0))),
        ]),
      ),
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
  let cf: t.TableColumnFiltering(Nil) = coerce(table)
  let gf: t.TableGlobalFiltering(Dynamic, Nil) = coerce(table)

  wire_btn(doc, "btn-0", fn() {
    table_global_filtering.set_global_filter(gf, updater.T(rt.dynamic("")))
    table_column_filtering.set_column_filters(
      cf,
      updater.T([
        filter_spec("tier", rt.dynamic("core")),
        filter_spec("requests", rt.dynamic(400.0)),
      ]),
    )
    render(table, doc, "Filter: tier = core & requests >= 400")
  })
  wire_btn(doc, "btn-1", fn() {
    table_global_filtering.set_global_filter(gf, updater.T(rt.dynamic("")))
    table_column_filtering.set_column_filters(cf, updater.T([]))
    render(table, doc, "Filter: all services")
  })
  wire_btn(doc, "btn-2", fn() {
    table_global_filtering.set_global_filter(gf, updater.T(rt.dynamic("")))
    table_column_filtering.set_column_filters(
      cf,
      updater.T([filter_spec("tier", rt.dynamic("edge"))]),
    )
    render(table, doc, "Filter: tier = edge")
  })
  wire_btn(doc, "btn-3", fn() {
    table_column_filtering.set_column_filters(cf, updater.T([]))
    table_global_filtering.set_global_filter(gf, updater.T(rt.dynamic("auth")))
    render(table, doc, "Global search: \"auth\"")
  })
  wire_btn(doc, "btn-4", fn() {
    table_global_filtering.set_global_filter(gf, updater.T(rt.dynamic("")))
    table_column_filtering.set_column_filters(
      cf,
      updater.T([filter_spec("errors", rt.dynamic(5.0))]),
    )
    render(table, doc, "Filter: errors >= 5")
  })
  render(table, doc, "Filter: tier = core & requests >= 400")
}
