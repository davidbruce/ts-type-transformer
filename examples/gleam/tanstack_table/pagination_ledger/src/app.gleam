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
import tanstack_table_core/pagination_state
import tanstack_table_core/row_model
import tanstack_table_core/row_row
import tanstack_table_core/runtime as rt
import tanstack_table_core/table_core_properties
import tanstack_table_core/table_row_models_core
import tanstack_table_core/table_row_pagination
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
  rt.js_object([
    #("accessorKey", rt.dynamic(key)),
    #("header", rt.dynamic(header)),
  ])
}

fn pad3(n: Int) -> String {
  case n < 10 {
    True -> "00" <> int.to_string(n)
    False ->
      case n < 100 {
        True -> "0" <> int.to_string(n)
        False -> int.to_string(n)
      }
  }
}

fn at(xs: List(String), i: Int) -> String {
  case xs, i {
    [x, ..], 0 -> x
    [_, ..rest], k -> at(rest, k - 1)
    [], _ -> ""
  }
}

fn store_field(
  table: t.TableCore(Dynamic, Nil),
  path: List(String),
  decoder: decode.Decoder(a),
) -> a {
  let store = table_core_properties.get_store(coerce(table))
  let assert Ok(v) =
    decode.run(rt.dynamic(store), decode.at(["state", ..path], decoder))
  v
}

fn wire_btn(doc, id: String, action: fn() -> Nil) {
  case document.get_element_by_id(doc, id) {
    Some(btn) ->
      html_element.add_event_listener(btn, ev.click, fn(_) { action() })
    None -> Nil
  }
}

fn render(table: t.TableCore(Dynamic, Nil), doc) {
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
      <> esc_html(str_val("account"))
      <> "</td><td>"
      <> esc_html(str_val("category"))
      <> "</td><td class=\"num\">"
      <> int_val("units")
      <> "</td><td class=\"num\">"
      <> int_val("amount")
      <> "</td></tr>"
    })
    |> string.concat

  let html =
    "<table><thead><tr><th>Entry</th><th>Account</th><th>Category</th><th class=\"num\">Units</th><th class=\"num\">Amount (CHF)</th></tr></thead><tbody>"
    <> body
    <> "</tbody></table>"
  case document.get_element_by_id(doc, "table-wrap") {
    Some(wrap) -> element.set_inner_html(wrap, html)
    None -> Nil
  }

  let page_idx = store_field(table, ["pagination", "pageIndex"], decode.int)
  let page_size = store_field(table, ["pagination", "pageSize"], decode.int)
  let pag: t.TableRowPagination(Dynamic, Nil) = coerce(table)
  let page_cnt = float.round(table_row_pagination.get_page_count(pag))
  let msg =
    "Page "
    <> int.to_string(page_idx + 1)
    <> " / "
    <> int.to_string(page_cnt)
    <> " · pageSize "
    <> int.to_string(page_size)
    <> " · showing "
    <> int.to_string(list.length(rows))
    <> " of "
    <> int.to_string(total)
    <> " rows"
  case document.get_element_by_id(doc, "meta") {
    Some(meta) -> dom_node.set_text_content(meta, Some(msg))
    None -> Nil
  }
}

pub fn main() {
  let features =
    rt.js_object([
      #("coreReactivityFeature", rt.dynamic(store_reactivity_bindings())),
      #("rowPaginationFeature", rt.dynamic(tt.get_row_pagination_feature())),
      #("coreRowModel", rt.dynamic(tt.create_core_row_model())),
      #("paginatedRowModel", rt.dynamic(tt.create_paginated_row_model())),
    ])
  let accounts = ["ops-pool", "clearing-ch", "reserve-de"]
  let cats = ["compute", "storage", "egress", "audit"]
  let rows =
    int.range(from: 0, to: 18, with: [], run: fn(acc, i) {
      let item =
        rt.js_object([
          #("id", rt.dynamic("TX-" <> pad3(i + 1))),
          #("account", rt.dynamic(at(accounts, i % 3))),
          #("category", rt.dynamic(at(cats, i % 4))),
          #("units", rt.dynamic(int.to_float({ i * 7 } % 19 + 4))),
          #("amount", rt.dynamic(int.to_float({ { i * 37 } % 90 + 15 } * 12))),
        ])
      [item, ..acc]
    })
    |> list.reverse
  let cols =
    rt.js_array([
      col_def("id", "Entry"),
      col_def("account", "Account"),
      col_def("category", "Category"),
      col_def("units", "Units"),
      col_def("amount", "Amount (CHF)"),
    ])
  let init_pg =
    pagination_state.create()
    |> pagination_state.with_page_index(0.0)
    |> pagination_state.with_page_size(5.0)
  let init_state = rt.js_object([#("pagination", rt.dynamic(init_pg))])
  let options =
    rt.js_object([
      #("features", features),
      #("data", rt.js_array(rows)),
      #("columns", cols),
      #("initialState", init_state),
    ])
  let table = tt.construct_table(coerce(options))
  let doc = core.get_document()
  let pag: t.TableRowPagination(Dynamic, Nil) = coerce(table)

  wire_btn(doc, "btn-0", fn() {
    let idx = store_field(table, ["pagination", "pageIndex"], decode.int)
    case idx > 0 {
      True ->
        table_row_pagination.set_page_index(
          pag,
          t.UpdaterT(int.to_float(idx - 1)),
        )
      False -> Nil
    }
    render(table, doc)
  })
  wire_btn(doc, "btn-1", fn() {
    let idx = store_field(table, ["pagination", "pageIndex"], decode.int)
    let cnt = float.round(table_row_pagination.get_page_count(pag))
    case idx + 1 < cnt {
      True ->
        table_row_pagination.set_page_index(
          pag,
          t.UpdaterT(int.to_float(idx + 1)),
        )
      False -> Nil
    }
    render(table, doc)
  })
  wire_btn(doc, "btn-2", fn() {
    table_row_pagination.set_page_index(pag, t.UpdaterT(0.0))
    table_row_pagination.set_page_size(pag, t.UpdaterT(5.0))
    render(table, doc)
  })
  wire_btn(doc, "btn-3", fn() {
    table_row_pagination.set_page_index(pag, t.UpdaterT(0.0))
    table_row_pagination.set_page_size(pag, t.UpdaterT(6.0))
    render(table, doc)
  })
  wire_btn(doc, "btn-4", fn() {
    table_row_pagination.set_page_index(pag, t.UpdaterT(0.0))
    table_row_pagination.set_page_size(pag, t.UpdaterT(9.0))
    render(table, doc)
  })
  render(table, doc)
}
