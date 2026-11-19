import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/string
import tanstack_table_core/areas/main as tt
import tanstack_table_core/areas/main/column_core_properties
import tanstack_table_core/areas/main/column_pinning_state
import tanstack_table_core/areas/main/column_visibility_state
import tanstack_table_core/areas/main/row_model
import tanstack_table_core/areas/main/row_row
import tanstack_table_core/areas/main/row_row_selection
import tanstack_table_core/areas/main/row_selection_state
import tanstack_table_core/areas/main/table_column_pinning
import tanstack_table_core/areas/main/table_column_visibility
import tanstack_table_core/areas/main/table_columns
import tanstack_table_core/areas/main/table_core_properties
import tanstack_table_core/areas/main/table_row_models_core
import tanstack_table_core/areas/main/table_row_selection
import tanstack_table_core/runtime as rt
import tanstack_table_core/types as t

@external(javascript, "@tanstack/table-core/store-reactivity-bindings", "storeReactivityBindings")
fn store_reactivity_bindings() -> t.TableReactivityBindings(Nil)

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

fn task_row(id: String, task: String, owner: String, zone: String, cpu: Float, status: String) {
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
    Some(btn) -> html_element.add_event_listener_click(btn, fn(_) { action() }, None)
    None -> Nil
  }
}

fn is_col_visible(table: t.TableCore(t.TableFeatures(Nil), Dynamic, Nil), col_id: String, default: Bool) -> Bool {
  let store = table_core_properties.get_store(coerce(table))
  case decode.run(rt.dynamic(store), decode.at(["state", "columnVisibility", col_id], decode.bool)) {
    Ok(v) -> v
    Error(_) -> default
  }
}

fn render(table: t.TableCore(t.TableFeatures(Nil), Dynamic, Nil), doc) {
  let fc = t.table_features_codec()
  let dc = rt.codec_dynamic()
  let pin: t.TableColumnPinning(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)
  let vis: t.TableColumnVisibility(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)
  let sel: t.TableRowSelection(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)
  let cols_view: t.TableColumns(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)
  let rm_view: t.TableRowModelsCore(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)

  let start_cols = table_column_pinning.get_get_start_visible_leaf_columns(pin, fc, dc)()
  let center_cols = table_column_pinning.get_get_center_visible_leaf_columns(pin, fc, dc)()
  let end_cols = table_column_pinning.get_get_end_visible_leaf_columns(pin, fc, dc)()
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
      let hdr: String = case rt.js_get(rt.dynamic(column_core_properties.get_column_def(col)), "header") {
        Some(h) -> coerce(h)
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

  let rows = row_model.get_rows(table_row_models_core.get_get_row_model(rm_view, fc, dc)())
  let col_ids = list.map(ordered, fn(pair) { column_core_properties.get_id(pair.1) })
  let body_rows =
    list.map(rows, fn(r) {
      let gv = row_row.get_get_value(coerce(r))
      let is_sel = tt.call_void_to_boolean_fn(row_row_selection.get_get_is_selected(coerce(r)))
      let row_open = case is_sel {
        True -> "<tr class=\"selected-row\"><td>&#9632;</td>"
        False -> "<tr><td>&#9633;</td>"
      }
      let tds =
        list.map(col_ids, fn(cid) {
          case cid == "cpu" {
            True -> {
              let cpu: Float = coerce(tt.call_string_to_t_value_fn(gv, "cpu"))
              "<td class=\"num\">" <> int.to_string(float.round(cpu)) <> "%</td>"
            }
            False -> {
              let v: String = coerce(tt.call_string_to_t_value_fn(gv, cid))
              "<td>" <> esc_html(v) <> "</td>"
            }
          }
        })
        |> string.concat
      row_open <> tds <> "</tr>"
    })
    |> string.concat

  let html = "<table><thead><tr><th>Sel</th>" <> header_cells <> "</tr></thead><tbody>" <> body_rows <> "</tbody></table>"
  case document.get_element_by_id(doc, "table-wrap") {
    Some(wrap) -> element.set_inner_html(wrap, html)
    None -> Nil
  }

  let vis_n = list.length(table_column_visibility.get_get_visible_leaf_columns(vis, fc, dc)())
  let all_n = list.length(table_columns.get_get_all_leaf_columns(cols_view, fc, dc)())
  let sel_n = list.length(row_model.get_rows(table_row_selection.get_get_selected_row_model(sel, fc, dc)()))
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
      #("columnVisibilityFeature", rt.dynamic(tt.get_column_visibility_feature())),
      #("columnPinningFeature", rt.dynamic(tt.get_column_pinning_feature())),
      #("rowSelectionFeature", rt.dynamic(tt.get_row_selection_feature())),
      #("coreRowModel", rt.dynamic(tt.create_core_row_model(rt.codec_dynamic()))),
    ])
  let data =
    rt.js_array([
      task_row("T-101", "ingest-wal-sync", "a.meyer", "eu-ch-1", 42.0, "running"),
      task_row("T-102", "index-compact", "b.rossi", "eu-ch-2", 68.0, "running"),
      task_row("T-103", "snapshot-verify", "c.keller", "eu-de-1", 15.0, "queued"),
      task_row("T-104", "ledger-reconcile", "d.weber", "eu-de-2", 79.0, "running"),
      task_row("T-105", "cert-rotation", "e.morel", "eu-fr-1", 24.0, "ready"),
      task_row("T-106", "shard-rebalance", "f.novak", "eu-fr-2", 55.0, "running"),
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
  let cv0 = column_visibility_state.create()
  column_visibility_state.set_at(cv0, "zone", False)
  let cp0 = column_pinning_state.create()
  column_pinning_state.set_start(cp0, ["id"])
  column_pinning_state.set_end(cp0, ["status"])
  let rs0 = row_selection_state.create()
  row_selection_state.set_at(rs0, "0", True)
  row_selection_state.set_at(rs0, "2", True)
  row_selection_state.set_at(rs0, "4", True)
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
  let pin: t.TableColumnPinning(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)
  let vis: t.TableColumnVisibility(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)
  let sel: t.TableRowSelection(t.TableFeatures(Nil), Dynamic, Nil) = coerce(table)
  let set_vis = table_column_visibility.get_set_column_visibility(vis)
  let set_pin = table_column_pinning.get_set_column_pinning(pin)
  let set_sel = table_row_selection.get_set_row_selection(sel)
  let toggle_sel = table_row_selection.get_toggle_all_rows_selected(sel)

  wire_btn(doc, "btn-0", fn() {
    let next_zone = !is_col_visible(table, "zone", False)
    let keep_owner = is_col_visible(table, "owner", True)
    let cv = column_visibility_state.create()
    column_visibility_state.set_at(cv, "zone", next_zone)
    column_visibility_state.set_at(cv, "owner", keep_owner)
    tt.call_table_column_visibility_set_column_visibility(set_vis, t.UpdaterT(cv))
    render(table, doc)
  })
  wire_btn(doc, "btn-1", fn() {
    let keep_zone = is_col_visible(table, "zone", False)
    let next_owner = !is_col_visible(table, "owner", True)
    let cv = column_visibility_state.create()
    column_visibility_state.set_at(cv, "zone", keep_zone)
    column_visibility_state.set_at(cv, "owner", next_owner)
    tt.call_table_column_visibility_set_column_visibility(set_vis, t.UpdaterT(cv))
    render(table, doc)
  })
  wire_btn(doc, "btn-2", fn() {
    tt.call_table_row_selection_toggle_all_page_rows_selected(toggle_sel, Some(True), None)
    render(table, doc)
  })
  wire_btn(doc, "btn-3", fn() {
    tt.call_table_row_selection_toggle_all_page_rows_selected(toggle_sel, Some(False), None)
    render(table, doc)
  })
  wire_btn(doc, "btn-4", fn() {
    let cv = column_visibility_state.create()
    column_visibility_state.set_at(cv, "zone", False)
    column_visibility_state.set_at(cv, "owner", True)
    tt.call_table_column_visibility_set_column_visibility(set_vis, t.UpdaterT(cv))
    let cp = column_pinning_state.create()
    column_pinning_state.set_start(cp, ["id"])
    column_pinning_state.set_end(cp, ["status"])
    tt.call_table_column_pinning_set_column_pinning(set_pin, t.UpdaterT(cp))
    let rs = row_selection_state.create()
    row_selection_state.set_at(rs, "0", True)
    row_selection_state.set_at(rs, "2", True)
    row_selection_state.set_at(rs, "4", True)
    tt.call_table_row_selection_set_row_selection(set_sel, t.UpdaterT(rs))
    render(table, doc)
  })
  render(table, doc)
}
