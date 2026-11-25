// Drives a real page through the generated DOM bindings; every check must hold.
import bindings/areas/core as dom
import bindings/areas/core/add_event_listener_options
import bindings/areas/core/document
import bindings/areas/core/dom_token_list
import bindings/areas/core/element
import bindings/areas/core/event_target
import bindings/areas/core/node
import bindings/areas/css/css_style_declaration_base as style
import bindings/areas/events/event
import bindings/areas/html/html_collection_base as collection
import bindings/areas/html/html_element
import bindings/areas/html/html_element_event_map as ev
import bindings/areas/html/html_element_tag_name_map as tag
import bindings/types as t
import cell
import gleam/int
import gleam/list
import gleam/option.{None, Some}

@external(javascript, "./page_ffi.mjs", "same")
fn same(a: a, b: b) -> Bool

@external(javascript, "./page_ffi.mjs", "report")
fn report(passed: Int, failed: List(String)) -> Nil

pub fn main() {
  let checks = cell.new([])
  let check = fn(ok: Bool, what: String) {
    cell.set(checks, [#(ok, what), ..cell.get(checks)])
  }
  let doc = dom.get_document()
  let body = document.get_body(doc)

  // Create an element and put it in the page. A div is an HTMLElement, an
  // Element and a Node: each one's functions take it as it is.
  let box = document.create_element(doc, tag.div)
  element.set_id(box, "made-by-gleam")
  element.set_text_content(box, Some("hello from gleam"))
  dom_token_list.add(element.get_class_list(box), ["box"])
  node.append_child(body, box)
  check(
    option.map(document.get_element_by_id(doc, "made-by-gleam"), same(_, box))
      == Some(True),
    "getElementById",
  )
  check(
    dom_token_list.contains(element.get_class_list(box), "box"),
    "classList",
  )

  // Read an element that was already in the page, and copy its text.
  let assert Some(seed) = document.get_element_by_id(doc, "seed")
  check(element.get_text_content(seed) == "seed text", "textContent")
  check(element.get_attribute(seed, "data-x") == Some("42"), "getAttribute")
  check(
    element.get_attribute(seed, "data-missing") == None,
    "missing attribute",
  )
  let copy = document.create_element(doc, tag.p)
  element.set_text_content(copy, Some(element.get_text_content(seed)))
  node.append_child(box, copy)

  // A listener called by real clicks, then removed: the same function value
  // is the same JavaScript function.
  let clicks = cell.new(0)
  let button = document.create_element(doc, tag.button)
  element.set_text_content(button, Some("press"))
  node.append_child(box, button)
  let on_click = fn(_) {
    cell.set(clicks, cell.get(clicks) + 1)
    element.set_attribute(box, "data-count", int.to_string(cell.get(clicks)))
  }
  // A typed key selects the event type: the listener of "click" takes a PointerEvent.
  html_element.add_event_listener(button, ev.click, on_click)
  html_element.click(button)
  html_element.click(button)
  check(cell.get(clicks) == 2, "listener called")
  html_element.remove_event_listener(button, ev.click, on_click)
  html_element.click(button)
  check(cell.get(clicks) == 2, "listener removed")

  // An options bag, { once: true }, and events made and dispatched here.
  // Two unions, the listener's and the options', go in by `_of`.
  let pings = cell.new(0)
  let on_ping = fn(_) {
    cell.set(pings, cell.get(pings) + 1)
    element.set_attribute(box, "data-pings", int.to_string(cell.get(pings)))
  }
  let once = add_event_listener_options.create()
  add_event_listener_options.set_once(once, Some(True))
  html_element.add_event_listener_with_string_of_with(
    box,
    "ping",
    t.EventListenerOrEventListenerObjectEventListener(on_ping),
    Some(t.BooleanOrAddEventListenerOptionsAddEventListenerOptions(once)),
  )
  event_target.dispatch_event(box, event.new("ping"))
  event_target.dispatch_event(box, event.new("ping"))
  check(cell.get(pings) == 1, "once listener")

  // Style, and the children.
  style.set_property(html_element.get_style(box), "color", Some("red"))
  check(collection.get_length(element.get_children(box)) == 2.0, "children")
  // A union of a Node and a string: a span and a text.
  let holder = document.create_element(doc, tag.section)
  element.append(holder, [
    t.NodeOrStringNode(node.upcast(document.create_element(doc, tag.span))),
    t.NodeOrStringString("tail"),
  ])
  check(
    element.get_text_content(holder) == "tail",
    "append a node and a string",
  )
  document.set_title(doc, "driven by gleam")

  let all = list.reverse(cell.get(checks))
  let failed =
    list.filter_map(all, fn(c) {
      case c.0 {
        True -> Error(Nil)
        False -> Ok(c.1)
      }
    })
  report(list.length(all) - list.length(failed), failed)
}
