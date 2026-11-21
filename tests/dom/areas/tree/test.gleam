// The DOM tree from Gleam: nodes made, attached, moved, cloned, searched and
// walked. Every check must hold; a failed one panics with its name.
import bindings/areas/core as dom
import bindings/areas/core/child_node
import bindings/areas/core/document
import bindings/areas/core/document_fragment
import bindings/areas/core/dom_token_list
import bindings/areas/core/element
import bindings/areas/core/node
import bindings/areas/core/node_list
import bindings/areas/core/node_list_of
import bindings/areas/core/tree_walker
import bindings/areas/html/html_collection_base as collection
import bindings/areas/html/html_element_tag_name_map as tag
import bindings/types as t
import cell
import gleam/int
import gleam/io
import gleam/float
import gleam/list
import gleam/option.{None, Some}
import gleam/string

pub fn main() {
  let count = cell.new(0)
  let check = fn(ok: Bool, what: String) {
    case ok {
      True -> cell.set(count, cell.get(count) + 1)
      False -> panic as what
    }
  }
  let doc = dom.get_document()
  let body = document.get_body(doc)

  // A list built from elements and text.
  let list_el = document.create_element(doc, tag.ul, None)
  element.set_id(list_el, "list")
  list.each(["one", "two", "three"], fn(word) {
    let item = document.create_element(doc, tag.li, None)
    node.append_child(item, document.create_text_node(doc, word))
    element.set_attribute(item, "data-word", word)
    node.append_child(list_el, item)
  })
  node.append_child(body, list_el)
  check(element.get_child_element_count(list_el) == 3.0, "child count")
  check(element.get_text_content(list_el) == "onetwothree", "text content")
  let assert Some(first_el) = element.get_first_element_child(list_el)
  check(element.get_text_content(first_el) == "one", "first element child")
  let assert Some(last_el) = element.get_last_element_child(list_el)
  check(
    element.get_attribute(last_el, "data-word") == Some("three"),
    "last element child attribute",
  )
  check(node.get_is_connected(list_el), "connected")

  // Parents, siblings and containment.
  let assert [_, two, three] = collection.to_list(element.get_children(list_el))
  check(
    node.is_same_node(
      list_el,
      option.map(node.get_parent_element(two), node.upcast),
    ),
    "parent element",
  )
  let assert Some(prev) = element.get_previous_element_sibling(two)
  check(element.get_text_content(prev) == "one", "previous element sibling")
  let assert Some(next) = element.get_next_element_sibling(two)
  check(node.is_same_node(three, Some(next)), "next element sibling")
  check(node.contains(body, Some(two)), "body contains")
  check(element.matches_with_string(two, "li[data-word=two]"), "matches")
  check(!element.matches_with_string(two, "li.nope"), "does not match")
  let assert Some(up) = element.closest(two, tag.ul)
  check(element.get_id(up) == "list", "closest")
  check(option.is_none(element.closest(two, tag.table)), "closest none")

  // Moved: appending a node that is in the tree moves it.
  node.append_child(list_el, two)
  let assert Some(now_last) = element.get_last_element_child(list_el)
  check(element.get_text_content(now_last) == "two", "moved to the end")
  check(element.get_text_content(list_el) == "onethreetwo", "order after move")

  // insertBefore puts a node ahead of another.
  let zero0 = document.create_element(doc, tag.li, None)
  element.set_text_content(zero0, Some("start"))
  node.insert_before(list_el, zero0, Some(first_el))
  let assert Some(head) = element.get_first_element_child(list_el)
  check(element.get_text_content(head) == "start", "insertBefore")
  node.remove_child(list_el, zero0)
  check(element.get_child_element_count(list_el) == 3.0, "removeChild")

  // Removed and replaced.
  let zero = document.create_element(doc, tag.li, None)
  element.set_text_content(zero, Some("zero"))
  node.replace_child(list_el, zero, first_el)
  check(option.is_none(node.get_parent_node(first_el)), "replaced node has no parent")
  let assert Some(head) = element.get_first_element_child(list_el)
  check(element.get_text_content(head) == "zero", "replaceChild")
  element.remove(two)
  check(element.get_child_element_count(list_el) == 2.0, "remove")

  // A fragment's children move in at once; it is left empty.
  let fragment = document.create_document_fragment(doc)
  list.each(["a", "b"], fn(word) {
    let item = document.create_element(doc, tag.li, None)
    element.set_text_content(item, Some(word))
    node.append_child(fragment, item)
  })
  check(document_fragment.get_child_element_count(fragment) == 2.0, "fragment filled")
  node.append_child(list_el, fragment)
  check(element.get_child_element_count(list_el) == 4.0, "fragment appended")
  check(node_list.get_length(node.get_child_nodes(fragment)) == 0.0, "fragment emptied")

  // A deep clone copies the subtree; a shallow one does not.
  let deep = node.clone_node(list_el, Some(True))
  let shallow = node.clone_node(list_el, None)
  check(node.get_text_content(deep) == Some(element.get_text_content(list_el)), "deep clone text")
  check(node_list.get_length(node.get_child_nodes(shallow)) == 0.0, "shallow clone")
  check(
    node.is_equal_node(deep, Some(list_el))
      && !node.is_same_node(deep, Some(list_el)),
    "equal but not same",
  )

  // Markup parsed and written.
  let host = document.create_element(doc, tag.div, None)
  element.set_inner_html(host, "<p class=\"x\">hi <b>there</b></p>")
  let assert Some(b) = element.query_selector_element(host, "b")
  check(element.get_text_content(b) == "there", "querySelector")
  check(
    element.get_inner_html(host) == "<p class=\"x\">hi <b>there</b></p>",
    "innerHTML",
  )
  let assert Some(p) = element.get_first_element_child(host)
  check(string.starts_with(element.get_outer_html(p), "<p class=\"x\">"), "outerHTML")
  check(dom_token_list.contains(element.get_class_list(p), "x"), "classList contains")
  dom_token_list.toggle(element.get_class_list(p), "y", None)
  check(element.get_attribute(p, "class") == Some("x y"), "classList toggle")
  dom_token_list.remove(element.get_class_list(p), ["x"])
  check(element.get_class_name(p) == "y", "classList remove")
  check(
    node_list_of.to_list(element.query_selector_all_element(host, "p, b"), t.element_codec()) |> list.length == 2,
    "querySelectorAll",
  )

  // Positions compared: `zero` precedes `a`.
  let assert [_, a, ..] = list.drop(collection.to_list(element.get_children(list_el)), 1)
  let position = float.truncate(node.compare_document_position(zero, a))
  check(int.bitwise_and(position, 4) != 0, "zero precedes a")
  let back = float.truncate(node.compare_document_position(a, zero))
  check(int.bitwise_and(back, 2) != 0, "a follows zero")
  let contained = float.truncate(node.compare_document_position(list_el, a))
  check(int.bitwise_and(contained, 16) != 0, "list contains a")

  // Text and siblings among all child nodes.
  let assert Some(zero_text) = node.get_first_child(zero)
  check(node.get_node_type(zero_text) == node.text_node, "text node type")
  check(node.get_node_name(zero) == "LI", "node name")
  check(option.is_none(node.get_next_sibling(zero_text)), "no next sibling in zero")
  check(child_node.upcast(zero_text) |> node.get_text_content == Some("zero"), "text node content")

  // A walk over the list's elements.
  let walker = document.create_tree_walker(doc, list_el, Some(1.0), None)
  let seen = walk(walker, [])
  check(seen == ["zero", "three", "a", "b"], "walker visits elements in order")
  check(node.is_same_node(tree_walker.get_root(walker), Some(list_el)), "walker root")

  io.println(int.to_string(cell.get(count)) <> " checks passed")
}

fn walk(walker, seen) {
  case tree_walker.next_node(walker) {
    Some(n) -> walk(walker, [option.unwrap(node.get_text_content(n), ""), ..seen])
    None -> list.reverse(seen)
  }
}
