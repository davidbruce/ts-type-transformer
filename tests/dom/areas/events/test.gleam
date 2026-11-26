// DOM events from Gleam: listeners and their options, dispatch and
// propagation, cancelation, custom events, aborting, and init dictionaries.
import bindings/areas/core as dom
import bindings/areas/core/abort_controller
import bindings/areas/core/abort_signal
import bindings/areas/core/add_event_listener_options as listen_options
import bindings/areas/core/add_event_listener_arg
import bindings/areas/core/remove_event_listener_arg
import bindings/areas/core/document
import bindings/areas/core/element
import bindings/areas/core/event_listener_options as remove_options
import bindings/areas/core/event_listener_or_event_listener_object
import bindings/areas/core/event_modifier_init
import bindings/areas/core/event_target
import bindings/areas/core/node
import bindings/areas/events/custom_event
import bindings/areas/events/custom_event_init
import bindings/areas/events/event
import bindings/areas/events/event_init
import bindings/areas/events/keyboard_event
import bindings/areas/events/keyboard_event_init
import bindings/areas/events/mouse_event
import bindings/areas/events/mouse_event_init
import bindings/areas/html/html_element
import bindings/areas/html/html_element_event_map as ev
import bindings/areas/html/html_element_tag_name_map as tag
import bindings/runtime
import bindings/types as t
import cell
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{Some}

fn on(f) {
  Some(event_listener_or_event_listener_object.Fn(f))
}

fn id_of(target: option.Option(t.EventTarget(Nil))) -> String {
  case option.map(target, element.from) {
    Some(Ok(el)) -> element.get_id(el)
    _ -> "?"
  }
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
  let log = cell.new([])
  let note = fn(s: String) { cell.set(log, [s, ..cell.get(log)]) }
  let take = fn() {
    let l = list.reverse(cell.get(log))
    cell.set(log, [])
    l
  }

  // outer > inner > leaf
  let outer = document.create_element(doc, tag.div)
  element.set_id(outer, "outer")
  let inner = document.create_element(doc, tag.div)
  element.set_id(inner, "inner")
  let leaf = document.create_element(doc, tag.button)
  element.set_id(leaf, "leaf")
  node.append_child(inner, leaf)
  node.append_child(outer, inner)
  node.append_child(body, outer)

  // A listener runs for each dispatch and not after removal.
  let hits = cell.new(0)
  let on_ping = fn(_) { cell.set(hits, cell.get(hits) + 1) }
  event_target.add_event_listener_fn(leaf, "ping", on_ping)
  check(event_target.dispatch_event(leaf, event.new("ping")), "dispatch returns true")
  event_target.dispatch_event(leaf, event.new("ping"))
  check(cell.get(hits) == 2, "listener called twice")
  event_target.remove_event_listener_fn(leaf, "ping", on_ping)
  event_target.dispatch_event(leaf, event.new("ping"))
  check(cell.get(hits) == 2, "listener removed")
  event_target.dispatch_event(leaf, event.new("other"))
  check(cell.get(hits) == 2, "other type ignored")

  // The event's own fields, read in a listener.
  let seen = cell.new("")
  let on_look = fn(ev) {
    cell.set(
      seen,
      event.get_type(ev)
        <> "/"
        <> id_of(event.get_target(ev))
        <> "/"
        <> id_of(event.get_current_target(ev))
        <> "/"
        <> int.to_string(float_int(event.get_event_phase(ev)))
        <> "/"
        <> bool(event.get_is_trusted(ev)),
    )
  }
  event_target.add_event_listener_fn(leaf, "look", on_look)
  event_target.dispatch_event(leaf, event.new("look"))
  check(cell.get(seen) == "look/leaf/leaf/2/false", "event fields: " <> cell.get(seen))

  // A once listener runs once.
  let once_hits = cell.new(0)
  let once = listen_options.create()
  listen_options.set_once(once, Some(True))
  event_target.add_event_listener_of_with(
    leaf,
    "once",
    on(fn(_) { cell.set(once_hits, cell.get(once_hits) + 1) }),
    add_event_listener_arg.AddEventListenerOptions(once),
  )
  event_target.dispatch_event(leaf, event.new("once"))
  event_target.dispatch_event(leaf, event.new("once"))
  check(cell.get(once_hits) == 1, "once listener")

  // Bubbling and capture: capture runs outer-first, bubble inner-first.
  let init = event_init.create()
  event_init.set_bubbles(init, Some(True))
  event_target.add_event_listener_fn_with(outer, "walk", fn(_) { note("outer-capture") }, True)
  event_target.add_event_listener_fn(outer, "walk", fn(_) { note("outer-bubble") })
  event_target.add_event_listener_fn(inner, "walk", fn(_) { note("inner-bubble") })
  event_target.add_event_listener_fn(leaf, "walk", fn(_) { note("leaf") })
  event_target.dispatch_event(leaf, event.new_with("walk", init))
  check(take() == ["outer-capture", "leaf", "inner-bubble", "outer-bubble"], "capture then bubble order")
  // An event that does not bubble reaches only the capture phase above.
  event_target.dispatch_event(leaf, event.new("walk"))
  check(take() == ["outer-capture", "leaf"], "non-bubbling event")

  // stopPropagation halts the bubble at that node; others on it still run.
  let stop = fn(ev) {
    note("stopper")
    event.stop_propagation(ev)
  }
  event_target.add_event_listener_fn(inner, "halt", stop)
  event_target.add_event_listener_fn(inner, "halt", fn(_) { note("same-node") })
  event_target.add_event_listener_fn(outer, "halt", fn(_) { note("outer-halt") })
  event_target.dispatch_event(leaf, event.new_with("halt", init))
  check(take() == ["stopper", "same-node"], "stopPropagation")
  // stopImmediatePropagation halts the other listeners on the same node too.
  let stop_now = fn(ev) {
    note("first")
    event.stop_immediate_propagation(ev)
  }
  event_target.add_event_listener_fn(leaf, "now", stop_now)
  event_target.add_event_listener_fn(leaf, "now", fn(_) { note("second") })
  event_target.dispatch_event(leaf, event.new("now"))
  check(take() == ["first"], "stopImmediatePropagation")

  // preventDefault works on a cancelable event only.
  let cancelable = event_init.create()
  event_init.set_cancelable(cancelable, Some(True))
  event_target.add_event_listener_fn(leaf, "veto", event.prevent_default)
  let vetoed = event.new_with("veto", cancelable)
  check(!event_target.dispatch_event(leaf, vetoed), "dispatch returns false when canceled")
  check(event.get_default_prevented(vetoed), "defaultPrevented")
  check(event.get_cancelable(vetoed), "cancelable")
  let plain = event.new("veto")
  event_target.dispatch_event(leaf, plain)
  check(!event.get_default_prevented(plain), "not cancelable, not prevented")

  // A passive listener cannot cancel.
  let passive = listen_options.create()
  listen_options.set_passive(passive, Some(True))
  event_target.add_event_listener_of_with(
    leaf,
    "soft",
    on(event.prevent_default),
    add_event_listener_arg.AddEventListenerOptions(passive),
  )
  let soft = event.new_with("soft", cancelable)
  event_target.dispatch_event(leaf, soft)
  check(!event.get_default_prevented(soft), "passive listener cannot cancel")

  // A custom event carries its detail.
  let details = custom_event_init.create()
  custom_event_init.set_detail(details, Some("payload"), runtime.codec_string())
  event_init.set_bubbles(details, Some(True))
  let got = cell.new("")
  event_target.add_event_listener_fn(outer, "custom", fn(ev) {
    case custom_event.from(ev) {
      Ok(c) -> {
        let d = custom_event.get_detail(c, runtime.codec_dynamic())
        case decode.run(d, decode.string) {
          Ok(s) -> cell.set(got, s)
          Error(_) -> cell.set(got, "not a string")
        }
      }
      Error(_) -> cell.set(got, "not custom")
    }
  })
  let custom = custom_event.new_with("custom", details)
  event_target.dispatch_event(leaf, custom)
  check(cell.get(got) == "payload", "custom event detail bubbled: " <> cell.get(got))
  check(custom_event.get_detail(custom, runtime.codec_string()) == "payload", "detail read directly")
  check(event.get_type(custom) == "custom", "custom event type")
  // A plain event is not a custom event.
  check(custom_event.from(event.new("custom")) |> result_is_error, "an Event is not a CustomEvent")

  // An AbortController's signal removes a listener.
  let controller = abort_controller.new()
  let signal = abort_controller.get_signal(controller)
  let sig_hits = cell.new(0)
  let with_signal = listen_options.create()
  listen_options.set_signal(with_signal, Some(signal))
  event_target.add_event_listener_of_with(
    leaf,
    "sig",
    on(fn(_) { cell.set(sig_hits, cell.get(sig_hits) + 1) }),
    add_event_listener_arg.AddEventListenerOptions(with_signal),
  )
  event_target.dispatch_event(leaf, event.new("sig"))
  check(cell.get(sig_hits) == 1 && !abort_signal.get_aborted(signal), "listener runs before abort")
  abort_controller.abort(controller)
  event_target.dispatch_event(leaf, event.new("sig"))
  check(cell.get(sig_hits) == 1, "abort removed the listener")
  check(abort_signal.get_aborted(signal), "signal aborted")

  // A signal is itself a target: its abort event is heard.
  let c2 = abort_controller.new()
  let aborts = cell.new(0)
  abort_signal.add_event_listener_with_string_fn(abort_controller.get_signal(c2), "abort", fn(_) {
    cell.set(aborts, cell.get(aborts) + 1)
  })
  abort_controller.abort(c2)
  check(cell.get(aborts) == 1, "signal abort event")

  // A removal with the capture flag only removes the capture listener.
  let cap_hits = cell.new(0)
  let cap = fn(_) { cell.set(cap_hits, cell.get(cap_hits) + 1) }
  event_target.add_event_listener_fn_with(leaf, "cap", cap, True)
  let not_capture = remove_options.create()
  remove_options.set_capture(not_capture, Some(False))
  event_target.remove_event_listener_of_with(
    leaf,
    "cap",
    on(cap),
    remove_event_listener_arg.EventListenerOptions(not_capture),
  )
  event_target.dispatch_event(leaf, event.new("cap"))
  check(cell.get(cap_hits) == 1, "capture listener survives a bubble removal")
  event_target.remove_event_listener_fn_with(leaf, "cap", cap, True)
  event_target.dispatch_event(leaf, event.new("cap"))
  check(cell.get(cap_hits) == 1, "capture listener removed")

  // A MouseEvent built from an init dictionary.
  let mouse_init = mouse_event_init.create()
  mouse_event_init.set_client_x(mouse_init, Some(12.0))
  mouse_event_init.set_client_y(mouse_init, Some(34.0))
  mouse_event_init.set_button(mouse_init, Some(2.0))
  event_modifier_init.set_ctrl_key(mouse_init, Some(True))
  event_init.set_bubbles(mouse_init, Some(True))
  let mouse = cell.new("")
  event_target.add_event_listener_fn(outer, "mousedown", fn(ev) {
    case mouse_event.from(ev) {
      Ok(m) ->
        cell.set(
          mouse,
          float_s(mouse_event.get_client_x(m))
            <> ","
            <> float_s(mouse_event.get_client_y(m))
            <> ","
            <> float_s(mouse_event.get_button(m))
            <> ","
            <> bool(mouse_event.get_ctrl_key(m))
            <> ","
            <> bool(mouse_event.get_shift_key(m)),
        )
      Error(_) -> cell.set(mouse, "not a mouse event")
    }
  })
  event_target.dispatch_event(leaf, mouse_event.new_with("mousedown", mouse_init))
  check(cell.get(mouse) == "12,34,2,true,false", "mouse event: " <> cell.get(mouse))
  let direct = mouse_event.new_with("mouseup", mouse_init)
  check(mouse_event.get_client_x(direct) == 12.0 && mouse_event.get_button(direct) == 2.0, "mouse init read back")
  check(mouse_event.get_modifier_state(direct, "Control"), "mouse modifier state")

  // A KeyboardEvent likewise.
  let key_init = keyboard_event_init.create()
  keyboard_event_init.set_key(key_init, Some("a"))
  keyboard_event_init.set_code(key_init, Some("KeyA"))
  keyboard_event_init.set_repeat(key_init, Some(True))
  event_modifier_init.set_shift_key(key_init, Some(True))
  let key = keyboard_event.new_with("keydown", key_init)
  check(keyboard_event.get_key(key) == "a", "key")
  check(keyboard_event.get_code(key) == "KeyA", "code")
  check(keyboard_event.get_repeat(key) && keyboard_event.get_shift_key(key), "repeat and shift")
  check(!keyboard_event.get_ctrl_key(key), "no ctrl")
  let typed = cell.new("")
  event_target.add_event_listener_fn(leaf, "keydown", fn(ev) {
    case keyboard_event.from(ev) {
      Ok(k) -> cell.set(typed, keyboard_event.get_key(k))
      Error(_) -> Nil
    }
  })
  event_target.dispatch_event(leaf, key)
  check(cell.get(typed) == "a", "keyboard listener")

  // A real click is heard by a typed listener, and removed.
  let clicks = cell.new(0)
  let on_click = fn(_) { cell.set(clicks, cell.get(clicks) + 1) }
  html_element.add_event_listener(leaf, ev.click, on_click)
  html_element.click(leaf)
  html_element.click(leaf)
  check(cell.get(clicks) == 2, "click listener")
  html_element.remove_event_listener(leaf, ev.click, on_click)
  html_element.click(leaf)
  check(cell.get(clicks) == 2, "click listener removed")

  io.println(int.to_string(cell.get(count)) <> " checks passed")
}

fn bool(b: Bool) -> String {
  case b {
    True -> "true"
    False -> "false"
  }
}

fn float_int(f: Float) -> Int {
  float.truncate(f)
}

fn float_s(f: Float) -> String {
  int.to_string(float.truncate(f))
}

fn result_is_error(r: Result(a, b)) -> Bool {
  case r {
    Ok(_) -> False
    Error(_) -> True
  }
}
