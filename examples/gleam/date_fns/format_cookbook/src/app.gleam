import date_fns/areas/main as df
import date_fns/runtime
import date_fns/types as dt
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import gleam/list
import gleam/option.{None, Some}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let dc = dt.date_codec()
      let day = dt.DateOrStringOrNumberDate
      let arg = dt.DateArgDateType

      let head = document.create_element_div(doc, None)
      element.set_class_name(head, "head")
      let h1 = document.create_element_h1(doc, None)
      node.set_text_content(h1, Some("Date-fns Format & Parse Cookbook (Gleam)"))
      let badge = document.create_element_span(doc, None)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("2024-03-15T14:30:45"))
      let _ = node.append_child(head, h1)
      let _ = node.append_child(head, badge)
      let _ = node.append_child(app, head)

      let sample = df.parse_iso("2024-03-15T14:30:45", None, dc)
      let parsed = df.parse("15/03/2024", "dd/MM/yyyy", arg(sample), None, dc, dc)

      let fmt_card = document.create_element_div(doc, None)
      element.set_class_name(fmt_card, "card")
      let fmt_lbl = document.create_element_div(doc, None)
      element.set_class_name(fmt_lbl, "label")
      node.set_text_content(fmt_lbl, Some("Format Tokens & ISO Output"))
      let _ = node.append_child(fmt_card, fmt_lbl)

      let tbl = document.create_element_table(doc, None)
      let thead = document.create_element_thead(doc, None)
      let hrow = document.create_element_tr(doc, None)
      list.each(["API / Pattern", "Output", "Notes"], fn(h) {
        let th = document.create_element_th(doc, None)
        node.set_text_content(th, Some(h))
        let _ = node.append_child(hrow, th)
        Nil
      })
      let _ = node.append_child(thead, hrow)
      let _ = node.append_child(tbl, thead)

      let rows = [
        #("formatDate(\"yyyy-MM-dd\")", df.format_date(day(sample), "yyyy-MM-dd", None), "ISO date"),
        #("formatDate(\"EEEE, do MMMM yyyy\")", df.format_date(day(sample), "EEEE, do MMMM yyyy", None), "Ordinal full date"),
        #("formatDate(\"EEE, d MMM yy\")", df.format_date(day(sample), "EEE, d MMM yy", None), "Compact short date"),
        #("formatDate(\"HH:mm:ss\")", df.format_date(day(sample), "HH:mm:ss", None), "24-hour clock"),
        #("formatDate(\"h:mm a\")", df.format_date(day(sample), "h:mm a", None), "12-hour clock"),
        #("formatDate(\"'Q'Q · 'W'I\")", df.format_date(day(sample), "'Q'Q · 'W'I", None), "Quarter & ISO week"),
        #("formatDate(\"DDD 'of' yyyy\")", df.format_date(day(sample), "DDD 'of' yyyy", None), "Day of year"),
        #("lightFormat(\"yyyy/MM/dd HH:mm\")", df.light_format(day(sample), "yyyy/MM/dd HH:mm"), "Zero-locale light formatter"),
        #("formatISO(sample)", df.format_iso(day(sample), None), "ISO 8601 timestamp"),
        #("parse(\"15/03/2024\", \"dd/MM/yyyy\")", df.format_date(day(parsed), "yyyy-MM-dd", None), "Custom parse roundtrip"),
      ]

      let tbody = document.create_element_tbody(doc, None)
      list.each(rows, fn(row) {
        let #(api, output, note) = row
        let tr = document.create_element_tr(doc, None)
        let td_api = document.create_element_td(doc, None)
        let code = document.create_element_code(doc, None)
        node.set_text_content(code, Some(api))
        let _ = node.append_child(td_api, code)
        let td_out = document.create_element_td(doc, None)
        node.set_text_content(td_out, Some(output))
        let td_note = document.create_element_td(doc, None)
        node.set_text_content(td_note, Some(note))
        let _ = node.append_child(tr, td_api)
        let _ = node.append_child(tr, td_out)
        let _ = node.append_child(tr, td_note)
        let _ = node.append_child(tbody, tr)
        Nil
      })
      let _ = node.append_child(tbl, tbody)
      let _ = node.append_child(fmt_card, tbl)
      let _ = node.append_child(app, fmt_card)

      let val_card = document.create_element_div(doc, None)
      element.set_class_name(val_card, "card")
      let val_lbl = document.create_element_div(doc, None)
      element.set_class_name(val_lbl, "label")
      node.set_text_content(val_lbl, Some("Validation & Calendar Existence"))
      let _ = node.append_child(val_card, val_lbl)

      let vtbl = document.create_element_table(doc, None)
      let vhead = document.create_element_thead(doc, None)
      let vhrow = document.create_element_tr(doc, None)
      list.each(["Input", "Check", "Result"], fn(h) {
        let th = document.create_element_th(doc, None)
        node.set_text_content(th, Some(h))
        let _ = node.append_child(vhrow, th)
        Nil
      })
      let _ = node.append_child(vhead, vhrow)
      let _ = node.append_child(vtbl, vhead)

      let good_iso = df.parse_iso("2024-03-15", None, dc)
      let bad_iso = df.parse_iso("not-a-date", None, dc)
      let ok_good = df.is_valid(runtime.dynamic(good_iso)) && df.is_match("2024-03-15", "yyyy-MM-dd", None)
      let ok_bad = df.is_valid(runtime.dynamic(bad_iso)) && df.is_match("not-a-date", "yyyy-MM-dd", None)
      let ok_2024 = df.is_exists(2024.0, 1.0, 29.0)
      let ok_2023 = df.is_exists(2023.0, 1.0, 29.0)

      let checks = [
        #("\"2024-03-15\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", ok_good, case ok_good { True -> "VALID" False -> "INVALID" }),
        #("\"not-a-date\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", ok_bad, case ok_bad { True -> "VALID" False -> "INVALID" }),
        #("(2024, 1, 29)", "isExists(2024, 1, 29) leap Feb 29", ok_2024, case ok_2024 { True -> "EXISTS" False -> "INVALID" }),
        #("(2023, 1, 29)", "isExists(2023, 1, 29) non-leap Feb 29", ok_2023, case ok_2023 { True -> "EXISTS" False -> "INVALID" }),
      ]

      let vbody = document.create_element_tbody(doc, None)
      list.each(checks, fn(item) {
        let #(input, chk, ok, label_txt) = item
        let tr = document.create_element_tr(doc, None)
        let td_in = document.create_element_td(doc, None)
        let code = document.create_element_code(doc, None)
        node.set_text_content(code, Some(input))
        let _ = node.append_child(td_in, code)
        let td_chk = document.create_element_td(doc, None)
        node.set_text_content(td_chk, Some(chk))
        let td_res = document.create_element_td(doc, None)
        let pill = document.create_element_span(doc, None)
        element.set_class_name(pill, case ok { True -> "pill" False -> "pill-muted" })
        node.set_text_content(pill, Some(label_txt))
        let _ = node.append_child(td_res, pill)
        let _ = node.append_child(tr, td_in)
        let _ = node.append_child(tr, td_chk)
        let _ = node.append_child(tr, td_res)
        let _ = node.append_child(vbody, tr)
        Nil
      })
      let _ = node.append_child(vtbl, vbody)
      let _ = node.append_child(val_card, vtbl)
      let _ = node.append_child(app, val_card)
      Nil
    }
  }
}
