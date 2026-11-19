import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import gleam/list
import gleam/option.{None, Some}
import katex/areas/main as katex
import katex/areas/main/katex_options
import katex/runtime as kr

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let head = document.create_element_div(doc, None)
      element.set_class_name(head, "head")
      let title = document.create_element_h1(doc, None)
      node.set_text_content(title, Some("ParseError Inspector (Gleam)"))
      let badge = document.create_element_span(doc, None)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("throwOnError: false vs true"))
      let _ = node.append_child(head, title)
      let _ = node.append_child(head, badge)
      let _ = node.append_child(app, head)

      let lenient = katex_options.create()
      katex_options.set_throw_on_error(lenient, Some(False))
      katex_options.set_error_color(lenient, Some("#555555"))

      let strict = katex_options.create()
      katex_options.set_throw_on_error(strict, Some(True))

      let bad_inputs = [
        "\\frac{1}{",
        "\\unknowncmd{x}",
        "x^{2 + ",
        "\\sqrt{a + b",
      ]

      list.each(bad_inputs, fn(tex) {
        let row = document.create_element_div(doc, None)
        element.set_class_name(row, "row")

        let left = document.create_element_div(doc, None)
        element.set_class_name(left, "cell mode-html")
        let l_lbl = document.create_element_div(doc, None)
        element.set_class_name(l_lbl, "label")
        node.set_text_content(l_lbl, Some("throwOnError: false (#555555)"))
        let l_slot = document.create_element_div(doc, None)
        element.set_class_name(l_slot, "slot")
        element.set_inner_html(
          l_slot,
          katex.render_to_string(tex, Some(lenient)),
        )
        let _ = node.append_child(left, l_lbl)
        let _ = node.append_child(left, l_slot)

        let right = document.create_element_div(doc, None)
        element.set_class_name(right, "cell")
        let r_lbl = document.create_element_div(doc, None)
        element.set_class_name(r_lbl, "label")
        node.set_text_content(
          r_lbl,
          Some("throwOnError: true (caught ParseError)"),
        )
        let r_err = document.create_element_div(doc, None)
        element.set_class_name(r_err, "err")
        let msg = case
          kr.attempt(fn() { katex.render_to_string(tex, Some(strict)) })
        {
          Ok(_) -> "(did not throw)"
          Error(e) -> kr.js_error_message(e)
        }
        node.set_text_content(r_err, Some(msg))
        let _ = node.append_child(right, r_lbl)
        let _ = node.append_child(right, r_err)

        let _ = node.append_child(row, left)
        let _ = node.append_child(row, right)
        let _ = node.append_child(app, row)
        Nil
      })
    }
  }
}
