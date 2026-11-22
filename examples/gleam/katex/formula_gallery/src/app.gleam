import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import dom/areas/html/html_element_tag_name_map as tag
import gleam/list
import gleam/option.{None, Some}
import katex as katex
import katex/katex_options

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let head = document.create_element(doc, tag.div, None)
      element.set_class_name(head, "head")
      let title = document.create_element(doc, tag.h1, None)
      node.set_text_content(title, Some("KaTeX Formula Gallery (Gleam)"))
      let badge = document.create_element(doc, tag.span, None)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("katex v" <> katex.get_version()))
      node.append_child(head, title)
      node.append_child(head, badge)
      node.append_child(app, head)

      let grid = document.create_element(doc, tag.div, None)
      element.set_class_name(grid, "grid")
      node.append_child(app, grid)

      let opts =
        katex_options.create()
        |> katex_options.with_display_mode(True)

      let formulas = [
        #("Euler's Identity", "e^{i\\pi} + 1 = 0"),
        #("Quadratic Formula", "x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}"),
        #(
          "Gaussian Integral",
          "\\int_{-\\infty}^{\\infty} e^{-x^2}\\,dx = \\sqrt{\\pi}",
        ),
        #(
          "Maxwell-Faraday",
          "\\nabla \\times \\mathbf{E} = -\\frac{\\partial \\mathbf{B}}{\\partial t}",
        ),
        #(
          "Fourier Transform",
          "\\hat{f}(\\xi) = \\int_{-\\infty}^{\\infty} f(x)e^{-2\\pi i x \\xi}\\,dx",
        ),
      ]

      list.each(formulas, fn(item) {
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
        katex.render(tex, slot, Some(opts))
        node.append_child(card, lbl)
        node.append_child(card, src)
        node.append_child(card, slot)
        node.append_child(grid, card)
        Nil
      })
    }
  }
}
