import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import dom/areas/html/html_element_tag_name_map as tag
import gleam/list
import gleam/option.{None, Some}
import marked/marked as marked_class

fn make_result_card(doc, title: String, html: String) {
  let card = document.create_element(doc, tag.div)
  element.set_class_name(card, "card")
  let lbl = document.create_element(doc, tag.div)
  element.set_class_name(lbl, "label")
  node.set_text_content(lbl, Some(title))
  let out = document.create_element(doc, tag.div)
  element.set_class_name(out, "md-out")
  element.set_inner_html(out, html)
  let raw = document.create_element(doc, tag.pre)
  element.set_class_name(raw, "html-raw")
  node.set_text_content(raw, Some(html))
  node.append_child(card, lbl)
  node.append_child(card, out)
  node.append_child(card, raw)
  card
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let head = document.create_element(doc, tag.div)
      element.set_class_name(head, "head")
      let title = document.create_element(doc, tag.h1)
      node.set_text_content(title, Some("Marked Inline vs Block (Gleam)"))
      let badge = document.create_element(doc, tag.span)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("parse vs parseInline"))
      node.append_child(head, title)
      node.append_child(head, badge)
      node.append_child(app, head)

      let inst = marked_class.new([])

      let snippets = [
        "Hello **world** and *italics*",
        "# Heading stays literal inline",
        "Inline `code` and [link](https://example.com)",
        "~~strike~~ and **nested *em***",
      ]

      list.each(snippets, fn(s) {
        let row = document.create_element(doc, tag.div)
        element.set_class_name(row, "row-card")
        let lbl = document.create_element(doc, tag.div)
        element.set_class_name(lbl, "label")
        node.set_text_content(lbl, Some("Markdown Snippet"))
        let pre = document.create_element(doc, tag.pre)
        element.set_class_name(pre, "src")
        node.set_text_content(pre, Some(s))

        let grid = document.create_element(doc, tag.div)
        element.set_class_name(grid, "grid")
        let block_html = marked_class.parse(inst, s)
        let inl_html = marked_class.parse_inline(inst, s)
        node.append_child(
          grid,
          make_result_card(doc, "Block (parse)", block_html),
        )
        node.append_child(
          grid,
          make_result_card(doc, "Inline (parseInline)", inl_html),
        )

        node.append_child(row, lbl)
        node.append_child(row, pre)
        node.append_child(row, grid)
        node.append_child(app, row)
        Nil
      })
    }
  }
}
