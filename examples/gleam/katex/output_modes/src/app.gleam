import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import gleam/int
import gleam/option.{None, Some}
import gleam/string
import katex/areas/main as katex
import katex/areas/main/katex_options
import katex/types as kt

fn add_card(doc, grid, label: String, cls: String, html: String) -> Nil {
  let card = document.create_element_div(doc, None)
  element.set_class_name(card, cls)
  let row = document.create_element_div(doc, None)
  element.set_class_name(row, "head")
  let lbl = document.create_element_div(doc, None)
  element.set_class_name(lbl, "label")
  node.set_text_content(lbl, Some(label))
  let badge = document.create_element_span(doc, None)
  element.set_class_name(badge, "badge")
  node.set_text_content(
    badge,
    Some(int.to_string(string.length(html)) <> " chars"),
  )
  let _ = node.append_child(row, lbl)
  let _ = node.append_child(row, badge)
  let slot = document.create_element_div(doc, None)
  element.set_class_name(slot, "slot")
  element.set_inner_html(slot, html)
  let _ = node.append_child(card, row)
  let _ = node.append_child(card, slot)
  let _ = node.append_child(grid, card)
  Nil
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let head = document.create_element_div(doc, None)
      element.set_class_name(head, "head")
      let title = document.create_element_h1(doc, None)
      node.set_text_content(title, Some("Output & Layout Options (Gleam)"))
      let badge = document.create_element_span(doc, None)
      element.set_class_name(badge, "badge")
      node.set_text_content(
        badge,
        Some("html | mathml | leqno | fleqn | maxSize | strict"),
      )
      let _ = node.append_child(head, title)
      let _ = node.append_child(head, badge)
      let _ = node.append_child(app, head)

      let grid = document.create_element_div(doc, None)
      element.set_class_name(grid, "grid")
      let _ = node.append_child(app, grid)

      let o_html = katex_options.create()
      katex_options.set_output(o_html, Some(kt.KatexOptionsOutputHtml))
      add_card(
        doc,
        grid,
        "output: html",
        "card mode-html",
        katex.render_to_string("a^2 + b^2 = c^2", Some(o_html)),
      )

      let o_math = katex_options.create()
      katex_options.set_output(o_math, Some(kt.KatexOptionsOutputMathml))
      add_card(
        doc,
        grid,
        "output: mathml",
        "card",
        katex.render_to_string("\\frac{1}{1 + x^2}", Some(o_math)),
      )

      let o_both = katex_options.create()
      katex_options.set_output(o_both, Some(kt.KatexOptionsOutputHtmlAndMathml))
      add_card(
        doc,
        grid,
        "output: htmlAndMathml",
        "card",
        katex.render_to_string("\\sqrt{\\alpha + \\beta}", Some(o_both)),
      )

      let o_leq = katex_options.create()
      katex_options.set_display_mode(o_leq, Some(True))
      katex_options.set_leqno(o_leq, Some(True))
      add_card(
        doc,
        grid,
        "displayMode + leqno",
        "card",
        katex.render_to_string(
          "\\int_0^1 x^2\\,dx = \\frac{1}{3} \\tag{1}",
          Some(o_leq),
        ),
      )

      let o_fl = katex_options.create()
      katex_options.set_display_mode(o_fl, Some(True))
      katex_options.set_fleqn(o_fl, Some(True))
      katex_options.set_max_size(o_fl, Some(2.0))
      katex_options.set_strict(
        o_fl,
        Some(kt.BooleanOrStringOrStrictFunctionBoolean(False)),
      )
      add_card(
        doc,
        grid,
        "fleqn + maxSize(2) + strict(false)",
        "card",
        katex.render_to_string(
          "\\text{café} + \\rule{10em}{0.4em} + é",
          Some(o_fl),
        ),
      )
    }
  }
}
