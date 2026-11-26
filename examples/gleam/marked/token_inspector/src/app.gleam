import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import dom/areas/html/html_element_tag_name_map as tag
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/string
import marked/lexer
import marked/token.{type Token}
import marked/tokens_list

fn token_kind_raw(tok: Token) -> #(String, String) {
  #(token.get_type(tok), token.get_raw(tok))
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let sample =
        "# Heading\n\nParagraph with **bold** and `code`.\n\n- alpha\n- beta\n\n> quote\n\n---\n"
      let toks = lexer.static_lex(sample)
      let items = tokens_list.to_list(toks)

      let head = document.create_element(doc, tag.div)
      element.set_class_name(head, "head")
      let title = document.create_element(doc, tag.h1)
      node.set_text_content(title, Some("Marked Token Inspector (Gleam)"))
      let badge = document.create_element(doc, tag.span)
      element.set_class_name(badge, "badge")
      node.set_text_content(
        badge,
        Some(int.to_string(list.length(items)) <> " tokens"),
      )
      node.append_child(head, title)
      node.append_child(head, badge)
      node.append_child(app, head)

      let src_card = document.create_element(doc, tag.div)
      element.set_class_name(src_card, "card")
      let src_lbl = document.create_element(doc, tag.div)
      element.set_class_name(src_lbl, "label")
      node.set_text_content(src_lbl, Some("Markdown Input"))
      let src_pre = document.create_element(doc, tag.pre)
      element.set_class_name(src_pre, "src")
      node.set_text_content(src_pre, Some(sample))
      node.append_child(src_card, src_lbl)
      node.append_child(src_card, src_pre)
      node.append_child(app, src_card)

      let tbl_card = document.create_element(doc, tag.div)
      element.set_class_name(tbl_card, "card")
      let tbl_lbl = document.create_element(doc, tag.div)
      element.set_class_name(tbl_lbl, "label")
      node.set_text_content(tbl_lbl, Some("Lexer Token Stream"))
      node.append_child(tbl_card, tbl_lbl)

      let table = document.create_element(doc, tag.table)
      element.set_class_name(table, "tok-table")
      let thead = document.create_element(doc, tag.thead)
      let hrow = document.create_element(doc, tag.tr)
      list.each(["#", "Type", "Bytes", "Raw Snippet"], fn(col) {
        let th = document.create_element(doc, tag.th)
        node.set_text_content(th, Some(col))
        node.append_child(hrow, th)
        Nil
      })
      node.append_child(thead, hrow)
      node.append_child(table, thead)

      let tbody = document.create_element(doc, tag.tbody)
      list.index_fold(items, Nil, fn(_, tok, i) {
        let #(kind, raw) = token_kind_raw(tok)
        let snip = string.replace(raw, "\n", "↵")
        let tr = document.create_element(doc, tag.tr)

        list.each(
          [
            int.to_string(i),
            kind,
            int.to_string(string.byte_size(raw)),
            snip,
          ],
          fn(cell_txt) {
            let td = document.create_element(doc, tag.td)
            node.set_text_content(td, Some(cell_txt))
            node.append_child(tr, td)
            Nil
          },
        )
        node.append_child(tbody, tr)
        Nil
      })

      node.append_child(table, tbody)
      node.append_child(tbl_card, table)
      node.append_child(app, tbl_card)
      Nil
    }
  }
}
