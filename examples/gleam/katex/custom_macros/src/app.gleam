import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import dom/areas/html/html_element_tag_name_map as tag
import gleam/list
import gleam/option.{None, Some}
import katex as katex
import katex/katex_options
import katex/katex_options_macros
import katex/runtime as kr

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let head = document.create_element(doc, tag.div, None)
      element.set_class_name(head, "head")
      let title = document.create_element(doc, tag.h1, None)
      node.set_text_content(title, Some("Custom Macros (Gleam)"))
      let badge = document.create_element(doc, tag.span, None)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("\\RR, \\NN, \\dd, \\norm"))
      node.append_child(head, title)
      node.append_child(head, badge)
      node.append_child(app, head)

      let macros =
        katex_options_macros.create()
        |> katex_options_macros.with_at("\\RR", kr.dynamic("\\mathbb{R}"))
        |> katex_options_macros.with_at("\\NN", kr.dynamic("\\mathbb{N}"))
        |> katex_options_macros.with_at("\\dd", kr.dynamic("\\mathrm{d}"))
        |> katex_options_macros.with_at(
          "\\norm",
          kr.dynamic("\\left\\| #1 \\right\\|"),
        )

      let opts =
        katex_options.create()
        |> katex_options.with_display_mode(True)
        |> katex_options.with_macros(macros)

      let legend = document.create_element(doc, tag.div, None)
      element.set_class_name(legend, "card")
      let leg_lbl = document.create_element(doc, tag.div, None)
      element.set_class_name(leg_lbl, "label")
      node.set_text_content(leg_lbl, Some("Defined Macros"))
      let leg_pre = document.create_element(doc, tag.pre, None)
      element.set_class_name(leg_pre, "tex")
      node.set_text_content(
        leg_pre,
        Some(
          "\\RR -> \\mathbb{R}\n\\NN -> \\mathbb{N}\n\\dd -> \\mathrm{d}\n\\norm -> \\left\\| #1 \\right\\|",
        ),
      )
      node.append_child(legend, leg_lbl)
      node.append_child(legend, leg_pre)
      node.append_child(app, legend)

      let grid = document.create_element(doc, tag.div, None)
      element.set_class_name(grid, "grid")
      node.append_child(app, grid)

      let exprs = [
        #("Euclidean Space", "v \\in \\RR^n,\\quad k \\in \\NN"),
        #("Vector Norm", "\\norm{x + y} \\le \\norm{x} + \\norm{y}"),
        #("Differential Form", "\\int_a^b f(x)\\,\\dd x = F(b) - F(a)"),
        #(
          "Inner Product Norm",
          "\\norm{\\phi}_2 = \\sqrt{\\int_{\\RR} |\\phi(t)|^2\\,\\dd t}",
        ),
      ]

      list.each(exprs, fn(item) {
        let #(name, tex) = item
        let card = document.create_element(doc, tag.div, None)
        element.set_class_name(card, "card")
        let lbl = document.create_element(doc, tag.div, None)
        element.set_class_name(lbl, "label")
        node.set_text_content(lbl, Some(name))
        let src = document.create_element(doc, tag.pre, None)
        element.set_class_name(src, "tex")
        node.set_text_content(src, Some(tex))
        let slot = document.create_element(doc, tag.div, None)
        element.set_class_name(slot, "slot")
        element.set_inner_html(slot, katex.render_to_string(tex, Some(opts)))
        node.append_child(card, lbl)
        node.append_child(card, src)
        node.append_child(card, slot)
        node.append_child(grid, card)
        Nil
      })
    }
  }
}
