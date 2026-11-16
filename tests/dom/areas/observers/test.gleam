// Observers from Gleam: MutationObserver (every kind of record, take_records,
// disconnect), ResizeObserver, IntersectionObserver, and AbortSignal.timeout.
// Observers call back later, so the program waits for them, then reports.
import bindings/areas/core as dom
import bindings/areas/core/abort_signal
import bindings/areas/core/character_data
import bindings/areas/core/document
import bindings/areas/core/dom_rect_read_only
import bindings/areas/core/element
import bindings/areas/core/intersection_observer
import bindings/areas/core/intersection_observer_entry as io_entry
import bindings/areas/core/mutation_observer
import bindings/areas/core/mutation_observer_init
import bindings/areas/core/mutation_record
import bindings/areas/core/node
import bindings/areas/core/node_list
import bindings/areas/core/performance
import bindings/areas/core/resize_observer
import bindings/areas/core/resize_observer_entry
import bindings/areas/core/resize_observer_size
import bindings/areas/html/html_element
import bindings/areas/css/css_style_declaration_base as decl
import bindings/types as t
import cell
import gleam/dynamic/decode
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}

fn when(ready: fn() -> Bool, what: String, then: fn() -> Nil) -> Nil {
  wait(ready, what, performance.now(dom.get_performance()), then)
}

fn wait(ready: fn() -> Bool, what: String, started: Float, then: fn() -> Nil) -> Nil {
  dom.request_animation_frame(fn(_) {
    case ready() {
      True -> then()
      False ->
        case performance.now(dom.get_performance()) -. started >. 8000.0 {
          True -> panic as { "timed out waiting for " <> what }
          False -> wait(ready, what, started, then)
        }
    }
  })
  Nil
}

fn kind(r) -> String {
  case mutation_record.get_type(r) {
    t.MutationRecordTypeAttributes -> "attributes"
    t.MutationRecordTypeCharacterData -> "characterData"
    t.MutationRecordTypeChildList -> "childList"
  }
}

fn flag(opts, set: fn(a, option.Option(Bool)) -> Nil) {
  set(opts, Some(True))
}

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

  // ---- take_records: what happened, with no callback ----
  let target = document.create_element_div(doc, None)
  node.append_child(body, target)
  let text = document.create_text_node(doc, "abc")
  node.append_child(target, text)
  let calls = cell.new(0)
  let watcher = mutation_observer.new(fn(_, _) { cell.set(calls, cell.get(calls) + 1) })
  let all = mutation_observer_init.create()
  flag(all, mutation_observer_init.set_child_list)
  flag(all, mutation_observer_init.set_attributes)
  flag(all, mutation_observer_init.set_character_data)
  flag(all, mutation_observer_init.set_subtree)
  flag(all, mutation_observer_init.set_attribute_old_value)
  flag(all, mutation_observer_init.set_character_data_old_value)
  mutation_observer.observe(watcher, target, Some(all))
  check(mutation_observer.take_records(watcher) == [], "nothing happened yet")

  let child = document.create_element_span(doc, None)
  node.append_child(target, child)
  element.set_attribute(target, "data-a", "1")
  element.set_attribute(target, "data-a", "2")
  character_data.set_data(text, "abcd")
  node.remove_child(target, child)
  let records = mutation_observer.take_records(watcher)
  check(list.map(records, kind) == ["childList", "attributes", "attributes", "characterData", "childList"], "record kinds in order")
  let assert [added, set1, set2, chars, removed] = records
  check(node_list.get_length(mutation_record.get_added_nodes(added)) == 1.0, "one node added")
  check(node_list.get_length(mutation_record.get_removed_nodes(added)) == 0.0, "none removed in the add")
  check(node.is_same_node(target, Some(mutation_record.get_target(added))), "target of the add")
  check(mutation_record.get_attribute_name(set1) == Some("data-a"), "attribute name")
  check(mutation_record.get_old_value(set1) == None, "no old value for a new attribute")
  check(mutation_record.get_old_value(set2) == Some("1"), "old attribute value")
  check(mutation_record.get_old_value(chars) == Some("abc"), "old character data")
  check(node.is_same_node(text, Some(mutation_record.get_target(chars))), "character data target")
  check(node_list.get_length(mutation_record.get_removed_nodes(removed)) == 1.0, "one node removed")
  check(option.is_some(mutation_record.get_previous_sibling(removed)), "the removed node had a previous sibling")
  check(mutation_observer.take_records(watcher) == [], "take_records empties the queue")
  check(cell.get(calls) == 0, "taken records are not delivered")

  // ---- delivery: the callback gets what is left, once ----
  let delivered = cell.new([])
  let observer = mutation_observer.new(fn(records, _) {
    cell.set(delivered, list.append(cell.get(delivered), list.map(records, kind)))
  })
  let scope = mutation_observer_init.create()
  flag(scope, mutation_observer_init.set_child_list)
  mutation_observer.observe(observer, target, Some(scope))
  let grandchild_host = document.create_element_div(doc, None)
  node.append_child(target, grandchild_host)
  node.append_child(grandchild_host, document.create_element_i(doc, None))
  element.set_attribute(target, "data-a", "3")
  mutation_observer.disconnect(watcher)

  // ---- an attribute filter and the subtree flag ----
  let filtered = cell.new([])
  let filter = mutation_observer.new(fn(records, _) {
    cell.set(filtered, list.append(cell.get(filtered), list.map(records, fn(r) { option.unwrap(mutation_record.get_attribute_name(r), "?") })))
  })
  let only_a = mutation_observer_init.create()
  mutation_observer_init.set_attribute_filter(only_a, Some(["a"]))
  mutation_observer.observe(filter, target, Some(only_a))
  element.set_attribute(target, "b", "ignored")
  element.set_attribute(target, "a", "seen")

  when(fn() { cell.get(delivered) != [] && cell.get(filtered) != [] }, "mutation callbacks", fn() {
    check(cell.get(delivered) == ["childList"], "only direct child changes without subtree")
    check(cell.get(filtered) == ["a"], "attribute filter")
    check(cell.get(calls) == 0, "a disconnected observer stays quiet")

    // ---- disconnect drops what is queued ----
    let late = cell.new(0)
    let dropped = mutation_observer.new(fn(_, _) { cell.set(late, 1) })
    mutation_observer.observe(dropped, target, Some(all))
    element.set_attribute(target, "data-z", "1")
    mutation_observer.disconnect(dropped)
    check(mutation_observer.take_records(dropped) == [], "disconnect drops the queue")
    resize(check, count, body, late)
  })
}

// ---- ResizeObserver ----
fn resize(check, count, body, late) -> Nil {
  let doc = dom.get_document()
  check(cell.get(late) == 0, "a disconnected observer never delivers")
  let box = document.create_element_div(doc, None)
  element.set_attribute(box, "style", "width: 50px; height: 30px; background: #ccc")
  node.append_child(body, box)
  let sizes = cell.new([])
  let resizer = resize_observer.new(fn(entries, _) {
    list.each(entries, fn(entry) {
      let rect = resize_observer_entry.get_content_rect(entry)
      let assert [content] = resize_observer_entry.get_content_box_size(entry)
      cell.set(sizes, [
        #(
          dom_rect_read_only.get_width(rect),
          dom_rect_read_only.get_height(rect),
          resize_observer_size.get_inline_size(content),
          resize_observer_size.get_block_size(content),
          node.is_same_node(box, Some(resize_observer_entry.get_target(entry))),
        ),
        ..cell.get(sizes)
      ])
    })
  })
  resize_observer.observe(resizer, box, None)
  when(fn() { cell.get(sizes) != [] }, "the first resize report", fn() {
    check(cell.get(sizes) == [#(50.0, 30.0, 50.0, 30.0, True)], "first report: size and target")
    html_element.get_style(box) |> decl.set_property("width", Some("80px"), None)
    when(fn() { list.length(cell.get(sizes)) == 2 }, "the second resize report", fn() {
      let assert [latest, ..] = cell.get(sizes)
      check(latest == #(80.0, 30.0, 80.0, 30.0, True), "second report after a change")
      resize_observer.unobserve(resizer, box)
      html_element.get_style(box) |> decl.set_property("width", Some("90px"), None)
      // A frame or two for a report that must not come.
      when(fn() { True }, "a frame", fn() {
        when(fn() { True }, "another frame", fn() {
          check(list.length(cell.get(sizes)) == 2, "unobserved: no more reports")
          resize_observer.disconnect(resizer)
          intersect(check, count, body)
        })
      })
    })
  })
}

// ---- IntersectionObserver ----
fn intersect(check, count, body) -> Nil {
  let doc = dom.get_document()
  let seen = cell.new([])
  let in_view = document.create_element_div(doc, None)
  element.set_attribute(in_view, "style", "width: 40px; height: 40px")
  let far = document.create_element_div(doc, None)
  element.set_attribute(far, "style", "position: absolute; top: 20000px; left: 0; width: 40px; height: 40px")
  node.append_child(body, in_view)
  node.append_child(body, far)
  let observer = intersection_observer.new(fn(entries, _) {
    list.each(entries, fn(entry) {
      let which = case node.is_same_node(in_view, Some(io_entry.get_target(entry))) {
        True -> "in"
        False -> "far"
      }
      cell.set(seen, [#(which, io_entry.get_is_intersecting(entry), io_entry.get_intersection_ratio(entry), dom_rect_read_only.get_width(io_entry.get_bounding_client_rect(entry))), ..cell.get(seen)])
    })
  }, None)
  check(intersection_observer.get_thresholds(observer) == [0.0], "default threshold")
  check(intersection_observer.get_root(observer) == None, "default root")
  intersection_observer.observe(observer, in_view)
  intersection_observer.observe(observer, far)
  when(fn() { list.length(cell.get(seen)) >= 2 }, "intersection reports", fn() {
    let reports = cell.get(seen)
    let assert Ok(#(_, in_is, in_ratio, in_width)) = list.find(reports, fn(r) { r.0 == "in" })
    let assert Ok(#(_, far_is, far_ratio, _)) = list.find(reports, fn(r) { r.0 == "far" })
    check(in_is && in_ratio == 1.0, "the visible element intersects fully")
    check(!far_is && far_ratio == 0.0, "the far element does not intersect")
    check(in_width == 40.0, "bounding rect width")
    intersection_observer.disconnect(observer)
    check(intersection_observer.take_records(observer) == [], "no records after disconnect")
    timeout(check, count)
  })
}

// ---- AbortSignal.timeout ----
fn timeout(check, count) -> Nil {
  let signal = abort_signal.timeout(40.0)
  check(!abort_signal.get_aborted(signal), "not aborted at once")
  let heard = cell.new(0)
  abort_signal.add_event_listener(
    signal,
    "abort",
    t.EventListenerOrEventListenerObjectEventListener(fn(_) { cell.set(heard, cell.get(heard) + 1) }),
    None,
  )
  let other = abort_signal.timeout(100000.0)
  when(fn() { cell.get(heard) == 1 }, "the timeout signal", fn() {
    check(abort_signal.get_aborted(signal), "aborted after the delay")
    check(decode.run(abort_signal.get_reason(signal), decode.at(["name"], decode.string)) == Ok("TimeoutError"), "the reason is a TimeoutError")
    check(!abort_signal.get_aborted(other), "a long timeout has not fired")
    io.println(int.to_string(cell.get(count)) <> " checks passed")
  })
}
