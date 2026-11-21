import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/string
import marked/areas/main/lexer
import marked/areas/main/tokens_list
import marked/namespaces/tokens/blockquote as token_blockquote
import marked/namespaces/tokens/br as token_br
import marked/namespaces/tokens/checkbox as token_checkbox
import marked/namespaces/tokens/code as token_code
import marked/namespaces/tokens/codespan as token_codespan
import marked/namespaces/tokens/def as token_def
import marked/namespaces/tokens/del as token_del
import marked/namespaces/tokens/em as token_em
import marked/namespaces/tokens/escape as token_escape
import marked/namespaces/tokens/generic as token_generic
import marked/namespaces/tokens/heading as token_heading
import marked/namespaces/tokens/hr as token_hr
import marked/namespaces/tokens/html as token_html
import marked/namespaces/tokens/image as token_image
import marked/namespaces/tokens/link as token_link
import marked/namespaces/tokens/list as token_list
import marked/namespaces/tokens/list_item as token_list_item
import marked/namespaces/tokens/paragraph as token_paragraph
import marked/namespaces/tokens/space as token_space
import marked/namespaces/tokens/strong as token_strong
import marked/namespaces/tokens/table as token_table
import marked/namespaces/tokens/tag as token_tag
import marked/namespaces/tokens/text as token_text
import marked/types as mt

fn token_kind_raw(tok: mt.Token) -> #(String, String) {
  case tok {
    mt.TokenTokensBlockquote(x) -> #("blockquote", token_blockquote.get_raw(x))
    mt.TokenTokensBr(x) -> #("br", token_br.get_raw(x))
    mt.TokenTokensCheckbox(x) -> #("checkbox", token_checkbox.get_raw(x))
    mt.TokenTokensCode(x) -> #("code", token_code.get_raw(x))
    mt.TokenTokensCodespan(x) -> #("codespan", token_codespan.get_raw(x))
    mt.TokenTokensDef(x) -> #("def", token_def.get_raw(x))
    mt.TokenTokensDel(x) -> #("del", token_del.get_raw(x))
    mt.TokenTokensEm(x) -> #("em", token_em.get_raw(x))
    mt.TokenTokensEscape(x) -> #("escape", token_escape.get_raw(x))
    mt.TokenTokensHeading(x) -> #("heading", token_heading.get_raw(x))
    mt.TokenTokensHr(x) -> #("hr", token_hr.get_raw(x))
    mt.TokenTokensHTML(x) -> #("html", token_html.get_raw(x))
    mt.TokenTokensImage(x) -> #("image", token_image.get_raw(x))
    mt.TokenTokensLink(x) -> #("link", token_link.get_raw(x))
    mt.TokenTokensList2(x) -> #("list", token_list.get_raw(x))
    mt.TokenTokensListItem(x) -> #("list_item", token_list_item.get_raw(x))
    mt.TokenTokensParagraph(x) -> #("paragraph", token_paragraph.get_raw(x))
    mt.TokenTokensSpace(x) -> #("space", token_space.get_raw(x))
    mt.TokenTokensStrong(x) -> #("strong", token_strong.get_raw(x))
    mt.TokenTokensTable(x) -> #("table", token_table.get_raw(x))
    mt.TokenTokensTag(x) -> #("html", token_tag.get_raw(x))
    mt.TokenTokensText(x) -> #("text", token_text.get_raw(x))
    mt.TokenTokensGeneric(x) -> #(token_generic.get_type(x), token_generic.get_raw(x))
  }
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
      list.index_fold(items, Nil, fn(_, tok, i) {
        let #(kind, raw) = token_kind_raw(tok)
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
