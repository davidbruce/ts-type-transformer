import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import gleam/list
import gleam/option.{None, Some}
import katex/areas/main as katex
import katex/areas/main/katex_options

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let head = document.create_element_div(doc, None)
      element.set_class_name(head, "head")
      let title = document.create_element_h1(doc, None)
      node.set_text_content(title, Some("Inline vs Display Mode (Gleam)"))
      let badge = document.create_element_span(doc, None)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("displayMode: false vs true"))
      let _ = node.append_child(head, title)
      let _ = node.append_child(head, badge)
      let _ = node.append_child(app, head)

      let inline_opts =
        katex_options.create()
        |> katex_options.with_display_mode(False)
      let display_opts =
        katex_options.create()
        |> katex_options.with_display_mode(True)

      let items = [
        #("Basel Series", "\\sum_{k=1}^{n} \\frac{1}{k^2} = \\frac{\\pi^2}{6}"),
        #("Dilogarithm", "\\int_0^1 \\frac{\\ln(1+x)}{x}\\,dx"),
        #("Sinc Limit", "\\lim_{x \\to 0} \\frac{\\sin x}{x} = 1"),
        #("Binomial", "\\binom{n}{k} = \\frac{n!}{k!(n-k)!}"),
      ]

      list.each(items, fn(item) {
        let #(name, tex) = item
        let row = document.create_element_div(doc, None)
        element.set_class_name(row, "row")
        list.each(
          [
            #(name <> " — inline", inline_opts),
            #(name <> " — display", display_opts),
          ],
          fn(pair) {
            let #(label, opts) = pair
            let cell = document.create_element_div(doc, None)
            element.set_class_name(cell, "cell")
            let lbl = document.create_element_div(doc, None)
            element.set_class_name(lbl, "label")
            node.set_text_content(lbl, Some(label))
            let slot = document.create_element_div(doc, None)
            element.set_class_name(slot, "slot")
            element.set_inner_html(slot, katex.render_to_string(tex, Some(opts)))
            let _ = node.append_child(cell, lbl)
            let _ = node.append_child(cell, slot)
            let _ = node.append_child(row, cell)
            Nil
          },
        )
        let _ = node.append_child(app, row)
        Nil
      })
    }
  }
}
