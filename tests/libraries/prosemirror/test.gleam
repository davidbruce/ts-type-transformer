// ProseMirror (model, transform, state, view) from Gleam, in a page: a schema
// defined here, documents from JSON, an editor state and its transactions, a
// plugin with state, and an editor view on the page's element. Every check must
// hold.
import bindings/areas/prosemirror_model/attrs
import bindings/areas/prosemirror_model/fragment
import bindings/areas/prosemirror_model/mark
import bindings/areas/prosemirror_model/mark_spec
import bindings/areas/main/mark_spec_attrs
import bindings/areas/prosemirror_model/attribute_spec
import bindings/areas/prosemirror_model/mark_type
import bindings/areas/prosemirror_model/node
import bindings/areas/prosemirror_model/node_spec
import bindings/areas/prosemirror_model/node_type
import bindings/areas/prosemirror_model/resolved_pos
import bindings/areas/prosemirror_model/schema
import bindings/areas/prosemirror_model/schema_spec
import bindings/areas/main/schema_marks
import bindings/areas/main/schema_nodes
import bindings/areas/main/editor_state_apply_transaction_result as applied
import bindings/areas/main/editor_view_coords_at_pos_result as coords
import bindings/areas/prosemirror_state/editor_state
import bindings/areas/prosemirror_state/editor_state_config
import bindings/areas/prosemirror_state/plugin
import bindings/areas/prosemirror_state/plugin_key
import bindings/areas/prosemirror_state/plugin_spec
import bindings/areas/prosemirror_state/selection
import bindings/areas/prosemirror_state/state_field
import bindings/areas/prosemirror_state/text_selection
import bindings/areas/prosemirror_state/transaction
import bindings/areas/prosemirror_transform/transform
import bindings/areas/prosemirror_view/direct_editor_props
import bindings/areas/prosemirror_view/editor_view
import bindings/areas/test_page
import bindings/runtime
import bindings/types as t
import cell
import gleam/dynamic
import gleam/dynamic/decode
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

// A plain JavaScript object from keys and values: a NodeSpec has an index signature.
fn object(entries: List(#(String, dynamic.Dynamic))) -> dynamic.Dynamic {
  let o = node_spec.create()
  list.each(entries, fn(e) { node_spec.set_at(o, e.0, e.1) })
  runtime.dynamic(o)
}

fn array(items: List(dynamic.Dynamic)) -> dynamic.Dynamic {
  dynamic.array(items)
}

fn str(s: String) -> dynamic.Dynamic {
  dynamic.string(s)
}

fn tag_spec(tag: String) -> t.DOMOutputSpec {
  t.DOMOutputSpecArray([str(tag), dynamic.int(0)])
}

pub fn main() {
  // ---- A schema: doc, paragraph, text; strong, em and link marks ----
  let doc_spec = node_spec.create()
  node_spec.set_content(doc_spec, Some("block+"))
  let para_spec = node_spec.create()
  node_spec.set_content(para_spec, Some("inline*"))
  node_spec.set_group(para_spec, Some("block"))
  node_spec.set_to_dom(para_spec, Some(fn(_) { tag_spec("p") }))
  let text_spec = node_spec.create()
  node_spec.set_group(text_spec, Some("inline"))
  let nodes = node_spec.create()
  node_spec.set_at(nodes, "doc", runtime.dynamic(doc_spec))
  node_spec.set_at(nodes, "paragraph", runtime.dynamic(para_spec))
  node_spec.set_at(nodes, "text", runtime.dynamic(text_spec))

  let strong_spec = mark_spec.create()
  mark_spec.set_to_dom(strong_spec, Some(fn(_, _) { tag_spec("strong") }))
  let em_spec = mark_spec.create()
  mark_spec.set_to_dom(em_spec, Some(fn(_, _) { tag_spec("em") }))
  let link_spec = mark_spec.create()
  let link_attrs = mark_spec_attrs.create()
  let href = attribute_spec.create()
  attribute_spec.set_default(href, dynamic.nil())
  mark_spec_attrs.set_at(link_attrs, "href", href)
  mark_spec.set_attrs(link_spec, Some(link_attrs))
  mark_spec.set_to_dom(link_spec, Some(fn(_, _) { tag_spec("a") }))
  let marks = mark_spec.create()
  mark_spec.set_at(marks, "strong", runtime.dynamic(strong_spec))
  mark_spec.set_at(marks, "em", runtime.dynamic(em_spec))
  mark_spec.set_at(marks, "link", runtime.dynamic(link_spec))

  let spec = schema_spec.create()
  schema_spec.set_nodes(spec, runtime.dynamic(nodes))
  schema_spec.set_marks(spec, runtime.dynamic(marks))
  let sch = schema.new(spec)
  let assert Some(paragraph_type) = schema_nodes.get_at(schema.get_nodes(sch), "paragraph")
  let assert Some(strong) = schema_marks.get_at(schema.get_marks(sch), "strong")
  let passed =
    check(node_type.get_name(schema.get_top_node_type(sch)) == "doc", "the top node is doc")
    + check(node_type.get_name(paragraph_type) == "paragraph", "a node type by name")
    + check(mark_type.get_name(strong) == "strong", "a mark type by name")
    + check(option.is_none(schema_nodes.get_at(schema.get_nodes(sch), "nothing")), "an unknown node type")

  // ---- Nodes from JSON ----
  let text_json = fn(text, marks) {
    object([#("type", str("text")), #("text", str(text)), #("marks", array(marks))])
  }
  let para_json = fn(children) {
    object([#("type", str("paragraph")), #("content", array(children))])
  }
  let bold = object([#("type", str("strong"))])
  let doc_json =
    object([
      #("type", str("doc")),
      #(
        "content",
        array([
          para_json([text_json("Hello", [])]),
          para_json([text_json("World", [bold])]),
        ]),
      ),
    ])
  let doc = node.from_json(sch, doc_json)
  let first = node.child(doc, 0.0)
  let second = node.child(doc, 1.0)
  let world = node.child(second, 0.0)
  let passed =
    passed
    + check(node.get_child_count(doc) == 2.0, "two paragraphs")
    + check(node.get_text_content(doc) == "HelloWorld", "textContent")
    + check(node.get_node_size(first) == 7.0, "a paragraph's nodeSize is its text plus two")
    + check(fragment.get_size(node.get_content(doc)) == 14.0, "content size")
    + check(node.get_node_size(doc) == 16.0, "nodeSize of the doc")
    + check(node_type.get_name(node.get_type(first)) == "paragraph", "child type")
    + check(node.get_text(world) == Some("World"), "text of a text node")
    + check(node.get_is_text(world) && node.get_is_inline(world), "a text node is inline text")
    + check(node.get_is_block(first) && node.get_is_textblock(first), "a paragraph is a textblock")
    + check(list.length(node.get_marks(world)) == 1, "the text has one mark")
    + check(mark_type.get_name(mark.get_type(list.first(node.get_marks(world)) |> result_unwrap)) == "strong", "it is strong")
    + check(option.is_some(mark_type.is_in_set(strong, node.get_marks(world))), "isInSet")
    + check(node.get_marks(node.child(first, 0.0)) == [], "the first text has no marks")
  node.check(doc)

  // JSON back out, equal to the JSON in.
  let assert Ok(content) = decode.run(node.to_json(doc), decode.at(["content"], decode.list(decode.dynamic)))
  let assert Ok(kinds) =
    decode.run(node.to_json(doc), decode.at(["content"], decode.list(decode.at(["type"], decode.string))))
  let passed =
    passed
    + check(list.length(content) == 2, "toJSON keeps the paragraphs")
    + check(kinds == ["paragraph", "paragraph"], "their types")
    + check(node.eq(node.from_json(sch, node.to_json(doc)), doc), "fromJSON(toJSON(doc)) equals doc")

  // Positions and cuts.
  let at = node.resolve(doc, 3.0)
  let passed =
    passed
    + check(resolved_pos.get_depth(at) == 1.0, "depth of a position inside a paragraph")
    + check(resolved_pos.get_parent_offset(at) == 2.0, "offset in the paragraph")
    + check(node_type.get_name(node.get_type(resolved_pos.get_parent(at))) == "paragraph", "the parent")
    + check(resolved_pos.start(at, None) == 1.0 && resolved_pos.end(at, None) == 6.0, "the paragraph's start and end")
    + check(node.text_between(doc, 1.0, 13.0, Some(" "), None) == "Hello World", "textBetween with a separator")
    + check(node.get_text_content(node.cut(doc, 2.0, Some(5.0))) == "ell", "cut")

  // Nodes made from the schema.
  let abc = schema.text(sch, "abc", None)
  let made = schema.node(sch, t.StringOrNodeTypeString("paragraph"), None, Some(t.FragmentOrNodeOrNodeArrayNode(abc)), None)
  let linked = schema.mark(sch, t.StringOrMarkTypeString("link"), Some(href_attrs("https://example.com")))
  let linked_text = schema.text(sch, "site", Some([linked]))
  let passed =
    passed
    + check(node.get_text_content(made) == "abc" && node.get_node_size(made) == 5.0, "schema.node and schema.text")
    + check(attrs.get_at(mark.get_attrs(linked), "href") == dynamic.string("https://example.com"), "a mark's attrs")
    + check(mark.is_in_set(linked, node.get_marks(linked_text)), "the link is in the text's marks")
    + check(mark.same_set(node.get_marks(linked_text), [linked]), "sameSet")

  // ---- An editor state ----
  let config = editor_state_config.create()
  editor_state_config.set_doc(config, Some(doc))
  let st = editor_state.create(config)
  let start = editor_state.get_selection(st)
  let passed =
    passed
    + check(node.eq(editor_state.get_doc(st), doc), "the state's doc")
    + check(selection.get_from(start) == 1.0 && selection.get_empty(start), "a cursor at the start")

  // A transaction: select, type, mark.
  let tr = editor_state.get_tr(st)
  let _ = transaction.set_selection(tr, text_selection.create(doc, 1.0, Some(6.0)))
  let st2 = editor_state.apply(st, tr)
  let sel = editor_state.get_selection(st2)
  let passed =
    passed
    + check(selection.get_from(sel) == 1.0 && selection.get_to(sel) == 6.0, "a selection of Hello")
    + check(!selection.get_empty(sel), "it is not empty")
    + check(node.get_text_content(node.child(editor_state.get_doc(st2), 0.0)) == "Hello", "selecting changes no text")
  let tr2 = editor_state.get_tr(st2)
  let _ = transaction.insert_text(tr2, "Hi", None, None)
  let st3 = editor_state.apply(st2, tr2)
  let passed =
    passed
    + check(node.get_text_content(editor_state.get_doc(st3)) == "HiWorld", "insertText replaces the selection")
    + check(selection.get_empty(editor_state.get_selection(st3)), "the cursor is after the text")
    + check(selection.get_from(editor_state.get_selection(st3)) == 3.0, "at 3")
    + check(node.get_text_content(editor_state.get_doc(st)) == "HelloWorld", "the old state is unchanged")
  let tr3 = editor_state.get_tr(st3)
  let _ = transform.add_mark(tr3, 1.0, 3.0, mark_type.create(strong, None))
  let st4 = editor_state.apply(st3, tr3)
  let passed =
    passed
    + check(node.range_has_mark(editor_state.get_doc(st4), 1.0, 3.0, t.MarkOrMarkTypeMarkType(strong)), "addMark")
    + check(!node.range_has_mark(editor_state.get_doc(st3), 1.0, 3.0, t.MarkOrMarkTypeMarkType(strong)), "the earlier doc has no mark")
    + check(transform.get_doc_changed(tr3), "the transform changed the doc")
    + check(list.length(transform.get_steps(tr3)) == 1, "one step")
  let tr4 = editor_state.get_tr(st4)
  let _ = transform.delete(tr4, 1.0, 3.0)
  let st5 = editor_state.apply(st4, tr4)
  let passed = passed + check(node.get_text_content(editor_state.get_doc(st5)) == "World", "delete")

  // ---- A plugin with state: counts the transactions that changed the doc ----
  let key = plugin_key.new(Some("counter"))
  let field = state_field.create()
  // FINDING 19 (awkward): set_key takes a PluginKey(Dynamic, _) and set_plugins a
  // List(Plugin(Dynamic, _)), with no way to widen a Plugin(Float, _): a plugin's
  // state is a Dynamic, boxed and unboxed here.
  state_field.set_init(field, fn(_, _) { dynamic.float(0.0) }, runtime.codec_dynamic())
  state_field.set_apply(
    field,
    fn(tr, count, _, _) {
      let assert Ok(n) = decode.run(count, decode.float)
      case transform.get_doc_changed(tr) {
        True -> dynamic.float(n +. 1.0)
        False -> count
      }
    },
    runtime.codec_dynamic(),
  )
  let pspec = plugin_spec.create()
  plugin_spec.set_state(pspec, Some(field))
  plugin_spec.set_key(pspec, Some(key))
  let seen = cell.new(0)
  plugin_spec.set_filter_transaction(pspec, Some(fn(tr, _) {
    cell.set(seen, cell.get(seen) + 1)
    // transactions marked "block" are refused
    case decode.run(transaction.get_meta(tr, t.StringOrPluginOrPluginKeyString("block")), decode.bool) {
      Ok(True) -> False
      _ -> True
    }
  }))
  let counter = plugin.new(pspec)
  let with_plugin = editor_state_config.create()
  editor_state_config.set_doc(with_plugin, Some(doc))
  editor_state_config.set_plugins(with_plugin, Some([counter]))
  let p0 = editor_state.create(with_plugin)
  let count_of = fn(s) {
    case plugin_key.get_state(key, s, runtime.codec_dynamic()) {
      Some(d) -> {
        let assert Ok(n) = decode.run(d, decode.float)
        Some(n)
      }
      None -> None
    }
  }
  let typed = editor_state.get_tr(p0)
  let _ = transaction.insert_text(typed, "!", Some(1.0), Some(1.0))
  let p1 = editor_state.apply(p0, typed)
  let selected = editor_state.get_tr(p1)
  let _ = transaction.set_selection(selected, selection.at_end(editor_state.get_doc(p1)))
  let p2 = editor_state.apply(p1, selected)
  let typed_again = editor_state.get_tr(p2)
  let _ = transaction.insert_text(typed_again, "?", Some(1.0), Some(1.0))
  let p3 = editor_state.apply(p2, typed_again)
  let passed =
    passed
    + check(count_of(p0) == Some(0.0), "the plugin's initial state")
    + check(count_of(p1) == Some(1.0), "one doc change")
    + check(count_of(p2) == Some(1.0), "a selection change is not counted")
    + check(count_of(p3) == Some(2.0), "two doc changes")
    + check(list.length(editor_state.get_plugins(p3)) == 1, "the state keeps its plugin")
    + check(option.is_some(plugin_key.get(key, p3)), "the plugin by its key")

  // filterTransaction: applyTransaction refuses a blocked transaction.
  let blocked = editor_state.get_tr(p3)
  let _ = transaction.insert_text(blocked, "#", Some(1.0), Some(1.0))
  let _ = transaction.set_meta(blocked, t.StringOrPluginOrPluginKeyString("block"), runtime.dynamic(True))
  let refused = editor_state.apply_transaction(p3, blocked)
  let allowed = editor_state.get_tr(p3)
  let _ = transaction.insert_text(allowed, "#", Some(1.0), Some(1.0))
  let accepted = editor_state.apply_transaction(p3, allowed)
  let passed =
    passed
    + check(node.get_text_content(editor_state.get_doc(applied.get_state(refused))) == node.get_text_content(editor_state.get_doc(p3)), "a blocked transaction changes nothing")
    + check(applied.get_transactions(refused) == [], "and is not applied")
    + check(node.get_text_content(editor_state.get_doc(applied.get_state(accepted))) == "#?!HelloWorld", "an allowed one is")
    + check(cell.get(seen) >= 2, "the filter was asked")

  // ---- An editor view on the page's #stage ----
  let dispatched = cell.new(0)
  let view_config = editor_state_config.create()
  editor_state_config.set_doc(view_config, Some(doc))
  let view_state = editor_state.create(view_config)
  let props = direct_editor_props.create()
  direct_editor_props.set_state(props, view_state)
  let view_cell = cell.new(None)
  direct_editor_props.set_dispatch_transaction(props, Some(fn(tr) {
    cell.set(dispatched, cell.get(dispatched) + 1)
    case cell.get(view_cell) {
      Some(v) -> editor_view.update_state(v, editor_state.apply(editor_view.get_state(v), tr))
      None -> Nil
    }
  }))
  let view = editor_view.new(Some(runtime.dynamic(test_page.element("stage"))), props)
  cell.set(view_cell, Some(view))
  let at_one = editor_view.coords_at_pos(view, 1.0, None)
  let passed =
    passed
    + check(node.eq(editor_state.get_doc(editor_view.get_state(view)), doc), "the view shows the doc")
    + check(editor_view.get_editable(view), "editable")
    + check(!editor_view.get_is_destroyed(view), "not destroyed")
    + check(coords.get_bottom(at_one) >. coords.get_top(at_one), "coordinates of a position have a height")
    + check(coords.get_left(at_one) >=. 0.0, "and a place")
  let typing = editor_state.get_tr(editor_view.get_state(view))
  let _ = transaction.insert_text(typing, "Oh ", Some(1.0), Some(1.0))
  editor_view.update_state(view, editor_state.apply(editor_view.get_state(view), typing))
  let passed =
    passed
    + check(node.get_text_content(editor_state.get_doc(editor_view.get_state(view))) == "Oh HelloWorld", "update_state with a new state")
  // The view's dispatch goes to the props' dispatchTransaction.
  let sent = editor_state.get_tr(editor_view.get_state(view))
  let _ = transaction.insert_text(sent, "A ", Some(1.0), Some(1.0))
  editor_view.get_dispatch(view)(sent)
  let passed =
    passed
    + check(cell.get(dispatched) == 1, "dispatch reaches dispatchTransaction")
    + check(node.get_text_content(editor_state.get_doc(editor_view.get_state(view))) == "A Oh HelloWorld", "and the handler updated the view")
  editor_view.destroy(view)
  let passed = passed + check(editor_view.get_is_destroyed(view), "destroyed")

  io.println(int.to_string(passed) <> " checks passed")
}

fn href_attrs(url: String) {
  let a = attrs.create()
  attrs.set_at(a, "href", dynamic.string(url))
  a
}

fn result_unwrap(r: Result(a, Nil)) -> a {
  let assert Ok(v) = r
  v
}
