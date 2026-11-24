import date_fns as df
import date_fns/runtime
import date_fns/types as dt
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import dom/areas/html/html_element_tag_name_map as tag
import gleam/list
import gleam/option.{None, Some}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let day = dt.DateOrStringOrNumberDate
      let arg = dt.DateArgDateType

      let head = document.create_element(doc, tag.div)
      element.set_class_name(head, "head")
      let h1 = document.create_element(doc, tag.h1)
      node.set_text_content(
        h1,
        Some("Date-fns Format & Parse Cookbook (Gleam)"),
      )
      let badge = document.create_element(doc, tag.span)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("2024-03-15T14:30:45"))
      node.append_child(head, h1)
      node.append_child(head, badge)
      node.append_child(app, head)

      let sample = df.parse_iso("2024-03-15T14:30:45")
      let parsed = df.parse("15/03/2024", "dd/MM/yyyy", arg(sample))

      let fmt_card = document.create_element(doc, tag.div)
      element.set_class_name(fmt_card, "card")
      let fmt_lbl = document.create_element(doc, tag.div)
      element.set_class_name(fmt_lbl, "label")
      node.set_text_content(fmt_lbl, Some("Format Tokens & ISO Output"))
      node.append_child(fmt_card, fmt_lbl)

      let tbl = document.create_element(doc, tag.table)
      let thead = document.create_element(doc, tag.thead)
      let hrow = document.create_element(doc, tag.tr)
      list.each(["API / Pattern", "Output", "Notes"], fn(h) {
        let th = document.create_element(doc, tag.th)
        node.set_text_content(th, Some(h))
        node.append_child(hrow, th)
        Nil
      })
      node.append_child(thead, hrow)
      node.append_child(tbl, thead)

      let rows = [
        #(
          "formatDate(\"yyyy-MM-dd\")",
          df.format_date(day(sample), "yyyy-MM-dd"),
          "ISO date",
        ),
        #(
          "formatDate(\"EEEE, do MMMM yyyy\")",
          df.format_date(day(sample), "EEEE, do MMMM yyyy"),
          "Ordinal full date",
        ),
        #(
          "formatDate(\"EEE, d MMM yy\")",
          df.format_date(day(sample), "EEE, d MMM yy"),
          "Compact short date",
        ),
        #(
          "formatDate(\"HH:mm:ss\")",
          df.format_date(day(sample), "HH:mm:ss"),
          "24-hour clock",
        ),
        #(
          "formatDate(\"h:mm a\")",
          df.format_date(day(sample), "h:mm a"),
          "12-hour clock",
        ),
        #(
          "formatDate(\"'Q'Q · 'W'I\")",
          df.format_date(day(sample), "'Q'Q · 'W'I"),
          "Quarter & ISO week",
        ),
        #(
          "formatDate(\"DDD 'of' yyyy\")",
          df.format_date(day(sample), "DDD 'of' yyyy"),
          "Day of year",
        ),
        #(
          "lightFormat(\"yyyy/MM/dd HH:mm\")",
          df.light_format(day(sample), "yyyy/MM/dd HH:mm"),
          "Zero-locale light formatter",
        ),
        #("formatISO(sample)", df.format_iso(day(sample)), "ISO 8601 timestamp"),
        #(
          "parse(\"15/03/2024\", \"dd/MM/yyyy\")",
          df.format_date(day(parsed), "yyyy-MM-dd"),
          "Custom parse roundtrip",
        ),
      ]

      let tbody = document.create_element(doc, tag.tbody)
      list.each(rows, fn(row) {
        let #(api, output, note) = row
        let tr = document.create_element(doc, tag.tr)
        let td_api = document.create_element(doc, tag.td)
        let code = document.create_element(doc, tag.code)
        node.set_text_content(code, Some(api))
        node.append_child(td_api, code)
        let td_out = document.create_element(doc, tag.td)
        node.set_text_content(td_out, Some(output))
        let td_note = document.create_element(doc, tag.td)
        node.set_text_content(td_note, Some(note))
        node.append_child(tr, td_api)
        node.append_child(tr, td_out)
        node.append_child(tr, td_note)
        node.append_child(tbody, tr)
        Nil
      })
      node.append_child(tbl, tbody)
      node.append_child(fmt_card, tbl)
      node.append_child(app, fmt_card)

      let val_card = document.create_element(doc, tag.div)
      element.set_class_name(val_card, "card")
      let val_lbl = document.create_element(doc, tag.div)
      element.set_class_name(val_lbl, "label")
      node.set_text_content(val_lbl, Some("Validation & Calendar Existence"))
      node.append_child(val_card, val_lbl)

      let vtbl = document.create_element(doc, tag.table)
      let vhead = document.create_element(doc, tag.thead)
      let vhrow = document.create_element(doc, tag.tr)
      list.each(["Input", "Check", "Result"], fn(h) {
        let th = document.create_element(doc, tag.th)
        node.set_text_content(th, Some(h))
        node.append_child(vhrow, th)
        Nil
      })
      node.append_child(vhead, vhrow)
      node.append_child(vtbl, vhead)

      let good_iso = df.parse_iso("2024-03-15")
      let bad_iso = df.parse_iso("not-a-date")
      let ok_good =
        df.is_valid(runtime.dynamic(good_iso))
        && df.is_match("2024-03-15", "yyyy-MM-dd")
      let ok_bad =
        df.is_valid(runtime.dynamic(bad_iso))
        && df.is_match("not-a-date", "yyyy-MM-dd")
      let ok_2024 = df.is_exists(2024.0, 1.0, 29.0)
      let ok_2023 = df.is_exists(2023.0, 1.0, 29.0)

      let checks = [
        #(
          "\"2024-03-15\"",
          "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")",
          ok_good,
          case ok_good {
            True -> "VALID"
            False -> "INVALID"
          },
        ),
        #(
          "\"not-a-date\"",
          "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")",
          ok_bad,
          case ok_bad {
            True -> "VALID"
            False -> "INVALID"
          },
        ),
        #(
          "(2024, 1, 29)",
          "isExists(2024, 1, 29) leap Feb 29",
          ok_2024,
          case ok_2024 {
            True -> "EXISTS"
            False -> "INVALID"
          },
        ),
        #(
          "(2023, 1, 29)",
          "isExists(2023, 1, 29) non-leap Feb 29",
          ok_2023,
          case ok_2023 {
            True -> "EXISTS"
            False -> "INVALID"
          },
        ),
      ]

      let vbody = document.create_element(doc, tag.tbody)
      list.each(checks, fn(item) {
        let #(input, chk, ok, label_txt) = item
        let tr = document.create_element(doc, tag.tr)
        let td_in = document.create_element(doc, tag.td)
        let code = document.create_element(doc, tag.code)
        node.set_text_content(code, Some(input))
        node.append_child(td_in, code)
        let td_chk = document.create_element(doc, tag.td)
        node.set_text_content(td_chk, Some(chk))
        let td_res = document.create_element(doc, tag.td)
        let pill = document.create_element(doc, tag.span)
        element.set_class_name(pill, case ok {
          True -> "pill"
          False -> "pill-muted"
        })
        node.set_text_content(pill, Some(label_txt))
        node.append_child(td_res, pill)
        node.append_child(tr, td_in)
        node.append_child(tr, td_chk)
        node.append_child(tr, td_res)
        node.append_child(vbody, tr)
        Nil
      })
      node.append_child(vtbl, vbody)
      node.append_child(val_card, vtbl)
      node.append_child(app, val_card)
      Nil
    }
  }
}
