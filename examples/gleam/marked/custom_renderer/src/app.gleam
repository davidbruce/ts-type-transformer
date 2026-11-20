import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/option.{None, Some}
import marked/areas/main as marked
import marked/areas/main/marked as marked_class
import marked/areas/main/marked_extension
import marked/areas/main/marked_extension_renderer
import marked/areas/main/marked_options_no_async
import marked/areas/main/marked_parse
import marked/areas/main/marked_parse_call_options_no_async
import marked/runtime as mr
import marked/types as mt

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

      let strip = document.create_element_div(doc, None)
      element.set_class_name(strip, "tok-strip")

      let renderer =
        marked_extension_renderer.create()
        |> marked_extension_renderer.with_heading(
          fn(args) {
            let #(heading) = args
            let tok = mr.dynamic(heading)
            let d = int.to_string(float.round(field_float(tok, "depth")))
            mt.BooleanOrRendererOutputRendererOutput(
              "<h"
              <> d
              <> " class=\"wire-h\">§ "
              <> field_string(tok, "text")
              <> "</h"
              <> d
              <> ">\n",
            )
          },
          mr.codec_string(),
        )

      let ext =
        marked_extension.create()
        |> marked_extension.with_renderer(renderer)
        |> marked_extension.with_walk_tokens(fn(token) {
          let pill = document.create_element_span(doc, None)
          element.set_class_name(pill, "tok-pill")
          node.set_text_content(pill, Some(field_string(token, "type")))
          let _ = node.append_child(strip, pill)
          None
        })

      let custom_m = marked_class.new([ext])
      let call = marked_parse_call_options_no_async.create()
      let def_html = marked.marked(md, marked_options_no_async.create())
      let cust_html =
        marked_parse.invoke(
          marked_class.get_parse(custom_m),
          md,
          call,
          mr.codec_string(),
        )

      let visited = float.round(element.get_child_element_count(strip))

      let head = document.create_element_div(doc, None)
      element.set_class_name(head, "head")
      let title = document.create_element_h1(doc, None)
      node.set_text_content(
        title,
        Some("Marked Custom Renderer & Walker (Gleam)"),
      )
      let badge = document.create_element_span(doc, None)
      element.set_class_name(badge, "badge")
      node.set_text_content(
        badge,
        Some(int.to_string(visited) <> " tokens visited"),
      )
      let _ = node.append_child(head, title)
      let _ = node.append_child(head, badge)
      let _ = node.append_child(app, head)

      let grid = document.create_element_div(doc, None)
      element.set_class_name(grid, "grid")

      let def_card = document.create_element_div(doc, None)
      element.set_class_name(def_card, "card")
      let def_lbl = document.create_element_div(doc, None)
      element.set_class_name(def_lbl, "label")
      node.set_text_content(def_lbl, Some("Default Renderer"))
      let def_out = document.create_element_div(doc, None)
      element.set_class_name(def_out, "md-out")
      element.set_inner_html(def_out, def_html)
      let _ = node.append_child(def_card, def_lbl)
      let _ = node.append_child(def_card, def_out)

      let cust_card = document.create_element_div(doc, None)
      element.set_class_name(cust_card, "card")
      let cust_lbl = document.create_element_div(doc, None)
      element.set_class_name(cust_lbl, "label")
      node.set_text_content(cust_lbl, Some("Custom Heading Renderer"))
      let cust_out = document.create_element_div(doc, None)
      element.set_class_name(cust_out, "md-out")
      element.set_inner_html(cust_out, cust_html)
      let _ = node.append_child(cust_card, cust_lbl)
      let _ = node.append_child(cust_card, cust_out)

      let _ = node.append_child(grid, def_card)
      let _ = node.append_child(grid, cust_card)
      let _ = node.append_child(app, grid)

      let walk_card = document.create_element_div(doc, None)
      element.set_class_name(walk_card, "card")
      let walk_lbl = document.create_element_div(doc, None)
      element.set_class_name(walk_lbl, "label")
      node.set_text_content(walk_lbl, Some("Visited Tokens (walkTokens)"))
      let _ = node.append_child(walk_card, walk_lbl)
      let _ = node.append_child(walk_card, strip)
      let _ = node.append_child(app, walk_card)
      Nil
    }
  }
}
