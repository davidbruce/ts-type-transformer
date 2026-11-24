import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import dom/areas/html/html_element_tag_name_map as tag
import gleam/option.{None, Some}
import marked
import marked/marked_options_no_async

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let head = document.create_element(doc, tag.div)
      element.set_class_name(head, "head")
      let title = document.create_element(doc, tag.h1)
      node.set_text_content(title, Some("Marked Document Preview (Gleam)"))
      let badge = document.create_element(doc, tag.span)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("marked"))
      node.append_child(head, title)
      node.append_child(head, badge)
      node.append_child(app, head)

      let markdown_src =
        "# Marked Wireframe Spec\n\n"
        <> "## Features\n\n"
        <> "Typed bindings with *emphasis*, **strong**, `inline code`, and [reference](https://example.com \"Spec\").\n\n"
        <> "- Lexer AST\n"
        <> "- Custom renderer\n"
        <> "- GFM tables\n\n"
        <> "1. Parse\n"
        <> "2. Walk\n"
        <> "3. Render\n\n"
        <> "> Typed bindings from TypeScript definitions.\n\n"
        <> "```ts\n"
        <> "const ok: boolean = true;\n"
        <> "```\n\n"
        <> "---\n"

      let grid = document.create_element(doc, tag.div)
      element.set_class_name(grid, "grid")

      let src_card = document.create_element(doc, tag.div)
      element.set_class_name(src_card, "card")
      let src_lbl = document.create_element(doc, tag.div)
      element.set_class_name(src_lbl, "label")
      node.set_text_content(src_lbl, Some("Markdown Source"))
      let src_pre = document.create_element(doc, tag.pre)
      element.set_class_name(src_pre, "src")
      node.set_text_content(src_pre, Some(markdown_src))
      node.append_child(src_card, src_lbl)
      node.append_child(src_card, src_pre)

      let out_card = document.create_element(doc, tag.div)
      element.set_class_name(out_card, "card")
      let out_lbl = document.create_element(doc, tag.div)
      element.set_class_name(out_lbl, "label")
      node.set_text_content(out_lbl, Some("Rendered HTML"))
      let md_out = document.create_element(doc, tag.div)
      element.set_class_name(md_out, "md-out")
      let html = marked.marked(markdown_src, marked_options_no_async.create())
      element.set_inner_html(md_out, html)
      node.append_child(out_card, out_lbl)
      node.append_child(out_card, md_out)

      node.append_child(grid, src_card)
      node.append_child(grid, out_card)
      node.append_child(app, grid)
      Nil
    }
  }
}
