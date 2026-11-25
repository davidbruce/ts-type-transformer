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
import tanstack_table_core/column_pinning_state
import tanstack_table_core/column_visibility_state
import tanstack_table_core/row_model
import tanstack_table_core/row_row
import tanstack_table_core/row_row_selection
import tanstack_table_core/row_selection_state
import tanstack_table_core/runtime as rt
import tanstack_table_core/table_column_pinning
import tanstack_table_core/table_column_visibility
import tanstack_table_core/table_columns
import tanstack_table_core/table_core_properties
import tanstack_table_core/table_row_models_core
import tanstack_table_core/table_row_selection
import tanstack_table_core/table_toggle_all_page_rows_selected_opts
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

fn task_row(
  id: String,
  task: String,
  owner: String,
  zone: String,
  cpu: Float,
  status: String,
) {
  rt.js_object([
    #("id", rt.dynamic(id)),
    #("task", rt.dynamic(task)),
    #("owner", rt.dynamic(owner)),
    #("zone", rt.dynamic(zone)),
    #("cpu", rt.dynamic(cpu)),
    #("status", rt.dynamic(status)),
  ])
}

fn wire_btn(doc, id: String, action: fn() -> Nil) {
  case document.get_element_by_id(doc, id) {
    Some(btn) ->
      html_element.add_event_listener(btn, ev.click, fn(_) { action() })
    None -> Nil
  }
}

fn is_col_visible(
  table: t.TableCore(Dynamic, Nil),
  col_id: String,
  default: Bool,
) -> Bool {
  let store = table_core_properties.get_store(coerce(table))
  case
    decode.run(
      rt.dynamic(store),
      decode.at(["state", "columnVisibility", col_id], decode.bool),
    )
  {
    Ok(v) -> v
    Error(_) -> default
  }
}

fn decode_str(v: Dynamic, fallback: String) -> String {
  case decode.run(v, decode.string) {
    Ok(s) -> s
    Error(_) -> fallback
  }
}

fn render(table: t.TableCore(Dynamic, Nil), doc) {
  let pin: t.TableColumnPinning(Dynamic, Nil) = coerce(table)
  let vis: t.TableColumnVisibility(Dynamic, Nil) = coerce(table)
  let sel: t.TableRowSelection(Dynamic, Nil) = coerce(table)
  let cols_view: t.TableColumns(Dynamic, Nil) = coerce(table)
  let rm_view: t.TableRowModelsCore(Dynamic, Nil) = coerce(table)

  let start_cols = table_column_pinning.get_start_visible_leaf_columns(pin)
  let center_cols = table_column_pinning.get_center_visible_leaf_columns(pin)
  let end_cols = table_column_pinning.get_end_visible_leaf_columns(pin)
  let ordered =
    list.flatten([
      list.map(start_cols, fn(c) { #(True, c) }),
      list.map(center_cols, fn(c) { #(False, c) }),
      list.map(end_cols, fn(c) { #(True, c) }),
    ])

  let header_cells =
    list.map(ordered, fn(pair) {
      let #(pinned, col) = pair
      let cid = column_core_properties.get_id(col)
      let hdr = case
        rt.js_get(
          rt.dynamic(column_core_properties.get_column_def(col)),
          "header",
        )
      {
        Some(h) -> decode_str(h, cid)
        None -> cid
      }
      let cls = case cid == "cpu" {
        True -> " class=\"num\""
        False -> ""
      }
      let tag = case pinned {
        True -> "<span class=\"pin-tag\">PIN</span>"
        False -> ""
      }
      "<th" <> cls <> ">" <> esc_html(hdr) <> tag <> "</th>"
    })
    |> string.concat

  let rows = row_model.get_rows(table_row_models_core.get_row_model(rm_view))
  let col_ids =
    list.map(ordered, fn(pair) { column_core_properties.get_id(pair.1) })
  let body_rows =
    list.map(rows, fn(r) {
      let is_sel = row_row_selection.get_is_selected(coerce(r))
      let row_open = case is_sel {
        True -> "<tr class=\"selected-row\"><td>&#9632;</td>"
        False -> "<tr><td>&#9633;</td>"
      }
      let tds =
        list.map(col_ids, fn(cid) {
          case cid == "cpu" {
            True -> {
              let cpu = case
                decode.run(
                  row_row.get_value(coerce(r), "cpu", rt.codec_dynamic()),
                  decode.float,
                )
              {
                Ok(n) -> n
                Error(_) -> 0.0
              }
              "<td class=\"num\">"
              <> int.to_string(float.round(cpu))
              <> "%</td>"
            }
            False -> {
              let v =
                decode_str(
                  row_row.get_value(coerce(r), cid, rt.codec_dynamic()),
                  "",
                )
              "<td>" <> esc_html(v) <> "</td>"
            }
          }
        })
        |> string.concat
      row_open <> tds <> "</tr>"
    })
    |> string.concat

  let html =
    "<table><thead><tr><th>Sel</th>"
    <> header_cells
    <> "</tr></thead><tbody>"
    <> body_rows
    <> "</tbody></table>"
  case document.get_element_by_id(doc, "table-wrap") {
    Some(wrap) -> element.set_inner_html(wrap, html)
    None -> Nil
  }

  let vis_n = list.length(table_column_visibility.get_visible_leaf_columns(vis))
  let all_n = list.length(table_columns.get_all_leaf_columns(cols_view))
  let sel_n =
    list.length(
      row_model.get_rows(table_row_selection.get_selected_row_model(sel)),
    )
  let msg =
    int.to_string(vis_n)
    <> " / "
    <> int.to_string(all_n)
    <> " visible columns · start:id end:status pinned · "
    <> int.to_string(sel_n)
    <> " / "
    <> int.to_string(list.length(rows))
    <> " rows selected"
  case document.get_element_by_id(doc, "meta") {
    Some(meta) -> dom_node.set_text_content(meta, Some(msg))
    None -> Nil
  }
}

pub fn main() {
  let features =
    rt.js_object([
      #("coreReactivityFeature", rt.dynamic(store_reactivity_bindings())),
      #(
        "columnVisibilityFeature",
        rt.dynamic(tt.get_column_visibility_feature()),
      ),
      #("columnPinningFeature", rt.dynamic(tt.get_column_pinning_feature())),
      #("rowSelectionFeature", rt.dynamic(tt.get_row_selection_feature())),
      #("coreRowModel", rt.dynamic(tt.create_core_row_model())),
    ])
  let data =
    rt.js_array([
      task_row(
        "T-101",
        "ingest-wal-sync",
        "a.meyer",
        "eu-ch-1",
        42.0,
        "running",
      ),
      task_row("T-102", "index-compact", "b.rossi", "eu-ch-2", 68.0, "running"),
      task_row(
        "T-103",
        "snapshot-verify",
        "c.keller",
        "eu-de-1",
        15.0,
        "queued",
      ),
      task_row(
        "T-104",
        "ledger-reconcile",
        "d.weber",
        "eu-de-2",
        79.0,
        "running",
      ),
      task_row("T-105", "cert-rotation", "e.morel", "eu-fr-1", 24.0, "ready"),
      task_row(
        "T-106",
        "shard-rebalance",
        "f.novak",
        "eu-fr-2",
        55.0,
        "running",
      ),
      task_row("T-107", "backup-archive", "g.steiner", "eu-ch-1", 31.0, "ready"),
    ])
  let cols =
    rt.js_array([
      col_def("id", "Task ID"),
      col_def("task", "Workload"),
      col_def("owner", "Owner"),
      col_def("zone", "Zone"),
      col_def("cpu", "CPU %"),
      col_def("status", "Status"),
    ])
  let cv0 =
    column_visibility_state.create()
    |> column_visibility_state.with_at("zone", False)
  let cp0 =
    column_pinning_state.create()
    |> column_pinning_state.with_start(["id"])
    |> column_pinning_state.with_end(["status"])
  let rs0 =
    row_selection_state.create()
    |> row_selection_state.with_at("0", True)
    |> row_selection_state.with_at("2", True)
    |> row_selection_state.with_at("4", True)
  let init_state =
    rt.js_object([
      #("columnVisibility", rt.dynamic(cv0)),
      #("columnPinning", rt.dynamic(cp0)),
      #("rowSelection", rt.dynamic(rs0)),
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
  let pin: t.TableColumnPinning(Dynamic, Nil) = coerce(table)
  let vis: t.TableColumnVisibility(Dynamic, Nil) = coerce(table)
  let sel: t.TableRowSelection(Dynamic, Nil) = coerce(table)

  wire_btn(doc, "btn-0", fn() {
    let next_zone = !is_col_visible(table, "zone", False)
    let keep_owner = is_col_visible(table, "owner", True)
    let cv =
      column_visibility_state.create()
      |> column_visibility_state.with_at("zone", next_zone)
      |> column_visibility_state.with_at("owner", keep_owner)
    table_column_visibility.set_column_visibility(vis, t.UpdaterT(cv))
    render(table, doc)
  })
  wire_btn(doc, "btn-1", fn() {
    let keep_zone = is_col_visible(table, "zone", False)
    let next_owner = !is_col_visible(table, "owner", True)
    let cv =
      column_visibility_state.create()
      |> column_visibility_state.with_at("zone", keep_zone)
      |> column_visibility_state.with_at("owner", next_owner)
    table_column_visibility.set_column_visibility(vis, t.UpdaterT(cv))
    render(table, doc)
  })
  wire_btn(doc, "btn-2", fn() {
    table_row_selection.toggle_all_rows_selected_with(
      sel,
      True,
      table_toggle_all_page_rows_selected_opts.create(),
    )
    render(table, doc)
  })
  wire_btn(doc, "btn-3", fn() {
    table_row_selection.toggle_all_rows_selected_with(
      sel,
      False,
      table_toggle_all_page_rows_selected_opts.create(),
    )
    render(table, doc)
  })
  wire_btn(doc, "btn-4", fn() {
    let cv =
      column_visibility_state.create()
      |> column_visibility_state.with_at("zone", False)
      |> column_visibility_state.with_at("owner", True)
    table_column_visibility.set_column_visibility(vis, t.UpdaterT(cv))
    let cp =
      column_pinning_state.create()
      |> column_pinning_state.with_start(["id"])
      |> column_pinning_state.with_end(["status"])
    table_column_pinning.set_column_pinning(pin, t.UpdaterT(cp))
    let rs =
      row_selection_state.create()
      |> row_selection_state.with_at("0", True)
      |> row_selection_state.with_at("2", True)
      |> row_selection_state.with_at("4", True)
    table_row_selection.set_row_selection(sel, t.UpdaterT(rs))
    render(table, doc)
  })
  render(table, doc)
}
