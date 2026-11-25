import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import dom/areas/html/html_element_tag_name_map as tag
import gleam/int
import gleam/option.{None, Some}
import gleam/string
import katex
import katex/katex_options
import katex/types as kt

fn add_card(doc, grid, label: String, cls: String, html: String) -> Nil {
  let card = document.create_element(doc, tag.div)
  element.set_class_name(card, cls)
  let row = document.create_element(doc, tag.div)
  element.set_class_name(row, "head")
  let lbl = document.create_element(doc, tag.div)
  element.set_class_name(lbl, "label")
  node.set_text_content(lbl, Some(label))
  let badge = document.create_element(doc, tag.span)
  element.set_class_name(badge, "badge")
  node.set_text_content(
    badge,
    Some(int.to_string(string.length(html)) <> " chars"),
  )
  node.append_child(row, lbl)
  node.append_child(row, badge)
  let slot = document.create_element(doc, tag.div)
  element.set_class_name(slot, "slot")
  element.set_inner_html(slot, html)
  node.append_child(card, row)
  node.append_child(card, slot)
  node.append_child(grid, card)
  Nil
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let head = document.create_element(doc, tag.div)
      element.set_class_name(head, "head")
      let title = document.create_element(doc, tag.h1)
      node.set_text_content(title, Some("Output & Layout Options (Gleam)"))
      let badge = document.create_element(doc, tag.span)
      element.set_class_name(badge, "badge")
      node.set_text_content(
        badge,
        Some("html | mathml | leqno | fleqn | maxSize | strict"),
      )
      node.append_child(head, title)
      node.append_child(head, badge)
      node.append_child(app, head)

      let grid = document.create_element(doc, tag.div)
      element.set_class_name(grid, "grid")
      node.append_child(app, grid)

      let o_html =
        katex_options.create()
        |> katex_options.with_output(kt.KatexOptionsOutputHtml)
      add_card(
        doc,
        grid,
        "output: html",
        "card mode-html",
        katex.render_to_string_with("a^2 + b^2 = c^2", o_html),
      )

      let o_math =
        katex_options.create()
        |> katex_options.with_output(kt.KatexOptionsOutputMathml)
      add_card(
        doc,
        grid,
        "output: mathml",
        "card",
        katex.render_to_string_with("\\frac{1}{1 + x^2}", o_math),
      )

      let o_both =
        katex_options.create()
        |> katex_options.with_output(kt.KatexOptionsOutputHtmlAndMathml)
      add_card(
        doc,
        grid,
        "output: htmlAndMathml",
        "card",
        katex.render_to_string_with("\\sqrt{\\alpha + \\beta}", o_both),
      )

      let o_leq =
        katex_options.create()
        |> katex_options.with_display_mode(True)
        |> katex_options.with_leqno(True)
      add_card(
        doc,
        grid,
        "displayMode + leqno",
        "card",
        katex.render_to_string_with(
          "\\int_0^1 x^2\\,dx = \\frac{1}{3} \\tag{1}",
          o_leq,
        ),
      )

      let o_fl =
        katex_options.create()
        |> katex_options.with_display_mode(True)
        |> katex_options.with_fleqn(True)
        |> katex_options.with_max_size(2.0)
        |> katex_options.with_strict(False)
      add_card(
        doc,
        grid,
        "fleqn + maxSize(2) + strict(false)",
        "card",
        katex.render_to_string_with(
          "\\text{café} + \\rule{10em}{0.4em} + é",
          o_fl,
        ),
      )
    }
  }
}
