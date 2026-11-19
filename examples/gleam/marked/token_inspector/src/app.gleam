import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/string
import marked/areas/main/lexer
import marked/areas/main/tokens_list

fn field_string(tok: Dynamic, name: String) -> String {
  let assert Ok(s) = decode.run(tok, decode.at([name], decode.string))
  s
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let sample =
        "# Heading\n\nParagraph with **bold** and `code`.\n\n- alpha\n- beta\n\n> quote\n\n---\n"
      let toks = lexer.lex(sample, None)
      let items = tokens_list.to_list(toks)

      let head = document.create_element_div(doc, None)
      element.set_class_name(head, "head")
      let title = document.create_element_h1(doc, None)
      node.set_text_content(title, Some("Marked Token Inspector (Gleam)"))
      let badge = document.create_element_span(doc, None)
      element.set_class_name(badge, "badge")
      node.set_text_content(
        badge,
        Some(int.to_string(list.length(items)) <> " tokens"),
      )
      let _ = node.append_child(head, title)
      let _ = node.append_child(head, badge)
      let _ = node.append_child(app, head)

      let src_card = document.create_element_div(doc, None)
      element.set_class_name(src_card, "card")
      let src_lbl = document.create_element_div(doc, None)
      element.set_class_name(src_lbl, "label")
      node.set_text_content(src_lbl, Some("Markdown Input"))
      let src_pre = document.create_element_pre(doc, None)
      element.set_class_name(src_pre, "src")
      node.set_text_content(src_pre, Some(sample))
      let _ = node.append_child(src_card, src_lbl)
      let _ = node.append_child(src_card, src_pre)
      let _ = node.append_child(app, src_card)

      let tbl_card = document.create_element_div(doc, None)
      element.set_class_name(tbl_card, "card")
      let tbl_lbl = document.create_element_div(doc, None)
      element.set_class_name(tbl_lbl, "label")
      node.set_text_content(tbl_lbl, Some("Lexer Token Stream"))
      let _ = node.append_child(tbl_card, tbl_lbl)

      let table = document.create_element_table(doc, None)
      element.set_class_name(table, "tok-table")
      let thead = document.create_element_thead(doc, None)
      let hrow = document.create_element_tr(doc, None)
      list.each(["#", "Type", "Bytes", "Raw Snippet"], fn(col) {
        let th = document.create_element_th(doc, None)
        node.set_text_content(th, Some(col))
        let _ = node.append_child(hrow, th)
        Nil
      })
      let _ = node.append_child(thead, hrow)
      let _ = node.append_child(table, thead)

      let tbody = document.create_element_tbody(doc, None)
      list.index_map(items, fn(tok, i) {
        let kind = field_string(tok, "type")
        let raw = field_string(tok, "raw")
        let snip = string.replace(raw, "\n", "↵")
        let tr = document.create_element_tr(doc, None)

        list.each(
          [
            int.to_string(i),
            kind,
            int.to_string(string.byte_size(raw)),
            snip,
          ],
          fn(cell_txt) {
            let td = document.create_element_td(doc, None)
            node.set_text_content(td, Some(cell_txt))
            let _ = node.append_child(tr, td)
            Nil
          },
        )
        let _ = node.append_child(tbody, tr)
        Nil
      })

      let _ = node.append_child(table, tbody)
      let _ = node.append_child(tbl_card, table)
      let _ = node.append_child(app, tbl_card)
      Nil
    }
  }
}
