import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import gleam/list
import gleam/option.{None, Some}
import marked/areas/main/marked as marked_class
import marked/areas/main/marked_parse
import marked/areas/main/marked_parse_call_options_no_async
import marked/runtime as mr

fn make_result_card(doc, title: String, html: String) {
  let card = document.create_element_div(doc, None)
  element.set_class_name(card, "card")
  let lbl = document.create_element_div(doc, None)
  element.set_class_name(lbl, "label")
  node.set_text_content(lbl, Some(title))
  let out = document.create_element_div(doc, None)
  element.set_class_name(out, "md-out")
  element.set_inner_html(out, html)
  let raw = document.create_element_pre(doc, None)
  element.set_class_name(raw, "html-raw")
  node.set_text_content(raw, Some(html))
  let _ = node.append_child(card, lbl)
  let _ = node.append_child(card, out)
  let _ = node.append_child(card, raw)
  card
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let head = document.create_element_div(doc, None)
      element.set_class_name(head, "head")
      let title = document.create_element_h1(doc, None)
      node.set_text_content(title, Some("Marked Inline vs Block (Gleam)"))
      let badge = document.create_element_span(doc, None)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("parse vs parseInline"))
      let _ = node.append_child(head, title)
      let _ = node.append_child(head, badge)
      let _ = node.append_child(app, head)

      let inst = marked_class.new([])
      let parse_fn = marked_class.get_parse(inst)
      let inline_fn = marked_class.get_parse_inline(inst)
      let call = marked_parse_call_options_no_async.create()

      let snippets = [
        "Hello **world** and *italics*",
        "# Heading stays literal inline",
        "Inline `code` and [link](https://example.com)",
        "~~strike~~ and **nested *em***",
      ]

      list.each(snippets, fn(s) {
        let row = document.create_element_div(doc, None)
        element.set_class_name(row, "row-card")
        let lbl = document.create_element_div(doc, None)
        element.set_class_name(lbl, "label")
        node.set_text_content(lbl, Some("Markdown Snippet"))
        let pre = document.create_element_pre(doc, None)
        element.set_class_name(pre, "src")
        node.set_text_content(pre, Some(s))

        let grid = document.create_element_div(doc, None)
        element.set_class_name(grid, "grid")
        let block_html = marked_parse.invoke(parse_fn, s, call, mr.codec_string())
        let inl_html = marked_parse.invoke(inline_fn, s, call, mr.codec_string())
        let _ = node.append_child(grid, make_result_card(doc, "Block (parse)", block_html))
        let _ = node.append_child(grid, make_result_card(doc, "Inline (parseInline)", inl_html))

        let _ = node.append_child(row, lbl)
        let _ = node.append_child(row, pre)
        let _ = node.append_child(row, grid)
        let _ = node.append_child(app, row)
        Nil
      })
    }
  }
}
