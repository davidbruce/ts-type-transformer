import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import dom/areas/html/html_element_tag_name_map as tag
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/option.{None, Some}
import marked
import marked/marked as marked_class
import marked/marked_extension
import marked/renderer_object
import marked/runtime as mr
import marked/token

fn field_string(tok: Dynamic, name: String) -> String {
  let assert Ok(s) = decode.run(tok, decode.at([name], decode.string))
  s
}

fn field_float(tok: Dynamic, name: String) -> Float {
  let assert Ok(n) = decode.run(tok, decode.at([name], decode.float))
  n
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let md =
        "# Wireframe Spec\n\n## Custom Pipeline\n\nParagraph with *italic* and **bold** and `code`.\n"

      let strip = document.create_element(doc, tag.div)
      element.set_class_name(strip, "tok-strip")

      let renderer =
        renderer_object.create()
        |> renderer_object.with_heading(fn(heading) {
          let tok = mr.dynamic(heading)
          let d = int.to_string(float.round(field_float(tok, "depth")))
          Some(
            "<h"
            <> d
            <> " class=\"wire-h\">§ "
            <> field_string(tok, "text")
            <> "</h"
            <> d
            <> ">\n",
          )
        })

      let ext =
        marked_extension.create()
        |> marked_extension.with_renderer(renderer)
        |> marked_extension.with_walk_tokens(fn(tok_item) {
          let pill = document.create_element(doc, tag.span)
          element.set_class_name(pill, "tok-pill")
          node.set_text_content(pill, Some(token.get_type(tok_item)))
          node.append_child(strip, pill)
          None
        })

      let custom_m = marked_class.new([ext])
      let def_html = marked.marked(md)
      let cust_html = marked_class.parse(custom_m, md)

      let visited = float.round(element.get_child_element_count(strip))

      let head = document.create_element(doc, tag.div)
      element.set_class_name(head, "head")
      let title = document.create_element(doc, tag.h1)
      node.set_text_content(
        title,
        Some("Marked Custom Renderer & Walker (Gleam)"),
      )
      let badge = document.create_element(doc, tag.span)
      element.set_class_name(badge, "badge")
      node.set_text_content(
        badge,
        Some(int.to_string(visited) <> " tokens visited"),
      )
      node.append_child(head, title)
      node.append_child(head, badge)
      node.append_child(app, head)

      let grid = document.create_element(doc, tag.div)
      element.set_class_name(grid, "grid")

      let def_card = document.create_element(doc, tag.div)
      element.set_class_name(def_card, "card")
      let def_lbl = document.create_element(doc, tag.div)
      element.set_class_name(def_lbl, "label")
      node.set_text_content(def_lbl, Some("Default Renderer"))
      let def_out = document.create_element(doc, tag.div)
      element.set_class_name(def_out, "md-out")
      element.set_inner_html(def_out, def_html)
      node.append_child(def_card, def_lbl)
      node.append_child(def_card, def_out)

      let cust_card = document.create_element(doc, tag.div)
      element.set_class_name(cust_card, "card")
      let cust_lbl = document.create_element(doc, tag.div)
      element.set_class_name(cust_lbl, "label")
      node.set_text_content(cust_lbl, Some("Custom Heading Renderer"))
      let cust_out = document.create_element(doc, tag.div)
      element.set_class_name(cust_out, "md-out")
      element.set_inner_html(cust_out, cust_html)
      node.append_child(cust_card, cust_lbl)
      node.append_child(cust_card, cust_out)

      node.append_child(grid, def_card)
      node.append_child(grid, cust_card)
      node.append_child(app, grid)

      let walk_card = document.create_element(doc, tag.div)
      element.set_class_name(walk_card, "card")
      let walk_lbl = document.create_element(doc, tag.div)
      element.set_class_name(walk_lbl, "label")
      node.set_text_content(walk_lbl, Some("Visited Tokens (walkTokens)"))
      node.append_child(walk_card, walk_lbl)
      node.append_child(walk_card, strip)
      node.append_child(app, walk_card)
      Nil
    }
  }
}
