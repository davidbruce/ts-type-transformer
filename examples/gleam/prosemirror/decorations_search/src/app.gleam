import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/string
import prosemirror/areas/main/mark_spec_attrs
import prosemirror/areas/prosemirror_model/attribute_spec
import prosemirror/areas/prosemirror_model/attrs
import prosemirror/areas/prosemirror_model/mark_spec
import prosemirror/areas/prosemirror_model/node
import prosemirror/areas/prosemirror_model/node_spec
import prosemirror/areas/prosemirror_model/schema
import prosemirror/areas/prosemirror_model/schema_spec
import prosemirror/areas/prosemirror_state/editor_state
import prosemirror/areas/prosemirror_state/editor_state_config
import prosemirror/areas/prosemirror_state/plugin
import prosemirror/areas/prosemirror_state/plugin_key
import prosemirror/areas/prosemirror_state/plugin_spec
import prosemirror/areas/prosemirror_state/state_field
import prosemirror/areas/prosemirror_state/transaction
import prosemirror/areas/prosemirror_view/decoration
import prosemirror/areas/prosemirror_view/decoration_attrs
import prosemirror/areas/prosemirror_view/decoration_set
import prosemirror/areas/prosemirror_view/decoration_source
import prosemirror/areas/prosemirror_view/direct_editor_props
import prosemirror/areas/prosemirror_view/editor_props
import prosemirror/areas/prosemirror_view/editor_view
import prosemirror/runtime as pm_rt
import prosemirror/types as t

fn hole_spec(tag: String) -> t.DOMOutputSpec {
  t.DOMOutputSpecArray([dynamic.string(tag), dynamic.int(0)])
}

fn build_schema() -> t.Schema(Dynamic, Dynamic, Nil) {
  let doc_spec =
    node_spec.create()
    |> node_spec.with_content("block+")

  let lvl_spec =
    attribute_spec.create()
    |> attribute_spec.with_default(dynamic.int(2))
  let h_attrs =
    mark_spec_attrs.create()
    |> mark_spec_attrs.with_at("level", lvl_spec)

  let heading_spec =
    node_spec.create()
    |> node_spec.with_attrs(h_attrs)
    |> node_spec.with_content("inline*")
    |> node_spec.with_group("block")
    |> node_spec.with_to_dom(fn(n) {
      let lvl = case decode.run(attrs.get_at(node.get_attrs(n), "level"), decode.int) {
        Ok(1) -> "h1"
        _ -> "h2"
      }
      hole_spec(lvl)
    })

  let para_spec =
    node_spec.create()
    |> node_spec.with_content("inline*")
    |> node_spec.with_group("block")
    |> node_spec.with_to_dom(fn(_) { hole_spec("p") })

  let cb_spec =
    node_spec.create()
    |> node_spec.with_content("text*")
    |> node_spec.with_marks("")
    |> node_spec.with_group("block")
    |> node_spec.with_code(True)
    |> node_spec.with_to_dom(fn(_) {
      t.DOMOutputSpecArray([
        dynamic.string("pre"),
        dynamic.array([dynamic.string("code"), dynamic.int(0)]),
      ])
    })

  let text_spec =
    node_spec.create()
    |> node_spec.with_group("inline")

  let nodes =
    node_spec.create()
    |> node_spec.with_at("doc", pm_rt.dynamic(doc_spec))
    |> node_spec.with_at("heading", pm_rt.dynamic(heading_spec))
    |> node_spec.with_at("paragraph", pm_rt.dynamic(para_spec))
    |> node_spec.with_at("code_block", pm_rt.dynamic(cb_spec))
    |> node_spec.with_at("text", pm_rt.dynamic(text_spec))

  let make_mark = fn(tag: String) {
    mark_spec.create()
    |> mark_spec.with_to_dom(fn(_, _) { hole_spec(tag) })
  }
  let marks =
    mark_spec.create()
    |> mark_spec.with_at("strong", pm_rt.dynamic(make_mark("strong")))
    |> mark_spec.with_at("code", pm_rt.dynamic(make_mark("code")))

  let spec =
    schema_spec.create()
    |> schema_spec.with_nodes(pm_rt.dynamic(nodes))
    |> schema_spec.with_marks(pm_rt.dynamic(marks))
  schema.new(spec)
}

fn p_node(sch, txt: String) -> t.Node(Nil) {
  schema.node(sch, t.StringOrNodeTypeString("paragraph"), None, Some(t.FragmentOrNodeOrNodeArrayNode(schema.text(sch, txt, None))), None)
}

fn make_initial_doc(sch: t.Schema(Dynamic, Dynamic, Nil)) -> t.Node(Nil) {
  let ha =
    attrs.create()
    |> attrs.with_at("level", dynamic.int(2))
  let h =
    schema.node(
      sch,
      t.StringOrNodeTypeString("heading"),
      Some(ha),
      Some(t.FragmentOrNodeOrNodeArrayNode(schema.text(sch, "Immutable State & View Decorations", None))),
      None,
    )
  let cb =
    schema.node(
      sch,
      t.StringOrNodeTypeString("code_block"),
      None,
      Some(t.FragmentOrNodeOrNodeArrayNode(schema.text(sch, "const nextState = state.apply(transaction);\nview.updateState(nextState); // sync view", None))),
      None,
    )
  schema.node(
    sch,
    t.StringOrNodeTypeString("doc"),
    None,
    Some(
      t.FragmentOrNodeOrNodeArrayArray([
        h,
        p_node(sch, "A ProseMirror schema validates every document tree before state updates commit to the view."),
        p_node(sch, "Each transaction maps positions forward so state plugins can recompute decorations without mutating the schema."),
        cb,
        p_node(sch, "Inline decorations highlight matching tokens while node decorations outline matching blocks in the view."),
      ]),
    ),
    None,
  )
}

fn find_matches_loop(parts: List(String), qlen: Int, inline_pos: Float, inl_attr, acc: List(t.Decoration(Nil)), hits: Int) -> #(List(t.Decoration(Nil)), Int) {
  case parts {
    [] | [_] -> #(acc, hits)
    [before, ..rest] -> {
      let from_p = inline_pos +. int.to_float(string.length(before))
      let to_p = from_p +. int.to_float(qlen)
      let dec = decoration.inline(from_p, to_p, inl_attr, None)
      find_matches_loop(rest, qlen, to_p, inl_attr, [dec, ..acc], hits + 1)
    }
  }
}

fn scan_block_children(
  children: List(t.Node(Nil)),
  q: String,
  qlen: Int,
  inline_pos: Float,
  inl_attr,
  acc: List(t.Decoration(Nil)),
  hits: Int,
  matched: Bool,
) -> #(List(t.Decoration(Nil)), Int, Bool) {
  case children {
    [] -> #(acc, hits, matched)
    [ch, ..rest] -> {
      let #(acc2, hits2, matched2) = case node.get_is_text(ch) {
        True -> {
          let parts = string.split(string.lowercase(node.get_text_content(ch)), on: q)
          let #(decs, added) = find_matches_loop(parts, qlen, inline_pos, inl_attr, acc, hits)
          #(decs, added, matched || added > hits)
        }
        False -> #(acc, hits, matched)
      }
      scan_block_children(rest, q, qlen, inline_pos +. node.get_node_size(ch), inl_attr, acc2, hits2, matched2)
    }
  }
}

fn scan_doc_blocks(
  blocks: List(t.Node(Nil)),
  q: String,
  qlen: Int,
  block_pos: Float,
  inl_attr,
  blk_attr,
  acc: List(t.Decoration(Nil)),
  inl_hits: Int,
  blk_hits: Int,
) -> #(List(t.Decoration(Nil)), Int, Int) {
  case blocks {
    [] -> #(list.reverse(acc), inl_hits, blk_hits)
    [blk, ..rest] -> {
      let #(acc2, hits2, matched) =
        scan_block_children(node.get_children(blk), q, qlen, block_pos +. 1.0, inl_attr, acc, inl_hits, False)
      let bsz = node.get_node_size(blk)
      let #(acc3, bh2) = case matched {
        True -> #([decoration.node(block_pos, block_pos +. bsz, blk_attr, None), ..acc2], blk_hits + 1)
        False -> #(acc2, blk_hits)
      }
      scan_doc_blocks(rest, q, qlen, block_pos +. bsz, inl_attr, blk_attr, acc3, hits2, bh2)
    }
  }
}

fn compute_decorations(d: t.Node(Nil), raw_q: String) -> #(t.DecorationSet(Nil), Int, Int) {
  let q = string.lowercase(string.trim(raw_q))
  case string.length(q) {
    0 -> #(decoration_set.get_empty(), 0, 0)
    qlen -> {
      let inl_attr =
        decoration_attrs.create()
        |> decoration_attrs.with_class("pm-match")
      let blk_attr =
        decoration_attrs.create()
        |> decoration_attrs.with_class("pm-hit-block")
      let #(decos, ih, bh) = scan_doc_blocks(node.get_children(d), q, qlen, 0.0, inl_attr, blk_attr, [], 0, 0)
      #(decoration_set.create(d, decos), ih, bh)
    }
  }
}

fn get_query(search_key, st: t.EditorState(Nil)) -> String {
  case plugin_key.get_state(search_key, st, pm_rt.codec_dynamic()) {
    Some(v) ->
      case decode.run(v, decode.string) {
        Ok(s) -> s
        Error(_) -> ""
      }
    None -> ""
  }
}

fn sync_meta(doc, search_key, st: t.EditorState(Nil), active_id: String) {
  let q = get_query(search_key, st)
  let #(dset, ih, bh) = compute_decorations(editor_state.get_doc(st), q)
  let total = list.length(decoration_set.find(dset, None, None, None))
  let qdisp = case q {
    "" -> "(none)"
    _ -> "\"" <> q <> "\""
  }
  let msg =
    "Query: "
    <> qdisp
    <> " · inline matches="
    <> int.to_string(ih)
    <> " · highlighted blocks="
    <> int.to_string(bh)
    <> " · decorations="
    <> int.to_string(total)
  case document.get_element_by_id(doc, "meta") {
    Some(meta) -> dom_node.set_text_content(meta, Some(msg))
    None -> Nil
  }
  list.each(["btn-0", "btn-1", "btn-2", "btn-3", "btn-4"], fn(id) {
    case document.get_element_by_id(doc, id) {
      Some(b) ->
        element.set_class_name(b, case id == active_id {
          True -> "active"
          False -> ""
        })
      None -> Nil
    }
  })
}

pub fn main() {
  let doc = core.get_document()
  let sch = build_schema()
  let search_key = plugin_key.new(Some("search"))

  let field =
    state_field.create()
    |> state_field.with_init(fn(_, _) { dynamic.string("state") }, pm_rt.codec_dynamic())
    |> state_field.with_apply(
      fn(tr, prev, _, _) {
        let mv = transaction.get_meta(tr, t.StringOrPluginOrPluginKeyString("search"))
        case decode.run(mv, decode.string) {
          Ok(s) -> dynamic.string(s)
          Error(_) -> prev
        }
      },
      pm_rt.codec_dynamic(),
    )

  let eprops =
    editor_props.create()
    |> editor_props.with_decorations(fn(st) {
      let #(dset, _, _) = compute_decorations(editor_state.get_doc(st), get_query(search_key, st))
      Some(decoration_source.upcast(dset))
    })

  let pspec =
    plugin_spec.create()
    |> plugin_spec.with_key(search_key)
    |> plugin_spec.with_state(field)
    |> plugin_spec.with_props(eprops)

  let scfg =
    editor_state_config.create()
    |> editor_state_config.with_schema(sch)
    |> editor_state_config.with_doc(make_initial_doc(sch))
    |> editor_state_config.with_plugins([plugin.new(pspec)])

  let view_ref = pm_rt.ref_new(None)
  let active_ref = pm_rt.ref_new("btn-0")

  let props =
    direct_editor_props.create()
    |> direct_editor_props.with_state(editor_state.create(scfg))
    |> direct_editor_props.with_dispatch_transaction(fn(tr) {
      case pm_rt.ref_get(view_ref) {
        Some(v) -> {
          let next = editor_state.apply(editor_view.get_state(v), tr)
          editor_view.update_state(v, next)
          sync_meta(doc, search_key, next, pm_rt.ref_get(active_ref))
        }
        None -> Nil
      }
    })

  let assert Some(mount) = document.get_element_by_id(doc, "editor")
  let view = editor_view.new(Some(pm_rt.dynamic(mount)), props)
  pm_rt.ref_set(view_ref, Some(view))
  sync_meta(doc, search_key, editor_view.get_state(view), pm_rt.ref_get(active_ref))

  list.each(
    [
      #("btn-0", "state"),
      #("btn-1", "schema"),
      #("btn-2", "transaction"),
      #("btn-3", "view"),
      #("btn-4", ""),
    ],
    fn(pair) {
      let #(btn_id, query) = pair
      case document.get_element_by_id(doc, btn_id) {
        Some(btn) ->
          html_element.add_event_listener_click(
            btn,
            fn(_) {
              pm_rt.ref_set(active_ref, btn_id)
              let tr = editor_state.get_tr(editor_view.get_state(view))
              let _ = transaction.set_meta(tr, t.StringOrPluginOrPluginKeyString("search"), dynamic.string(query))
              editor_view.get_dispatch(view)(tr)
            },
            None,
          )
        None -> Nil
      }
    },
  )
}
