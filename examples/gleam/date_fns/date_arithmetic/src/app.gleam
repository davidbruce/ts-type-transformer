import date_fns/areas/main as df
import date_fns/areas/main/duration
import date_fns/types as dt
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}

fn fmt_signed(v: Float) -> String {
  let n = float.round(v)
  case n > 0 {
    True -> "+" <> int.to_string(n)
    False -> int.to_string(n)
  }
}

fn add_kv(doc, row, label: String, val: String) -> Nil {
  let box = document.create_element_div(doc, None)
  element.set_class_name(box, "kv")
  let lbl = document.create_element_span(doc, None)
  element.set_class_name(lbl, "label")
  node.set_text_content(lbl, Some(label))
  let s = document.create_element_div(doc, None)
  element.set_class_name(s, "val")
  node.set_text_content(s, Some(val))
  let _ = node.append_child(box, lbl)
  let _ = node.append_child(box, s)
  let _ = node.append_child(row, box)
  Nil
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let day = dt.DateOrStringOrNumberDate
      let arg = dt.DateArgDateType

      let head = document.create_element_div(doc, None)
      element.set_class_name(head, "head")
      let h1 = document.create_element_h1(doc, None)
      node.set_text_content(h1, Some("Date-fns Arithmetic & Durations (Gleam)"))
      let badge = document.create_element_span(doc, None)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("Base 2024-03-15"))
      let _ = node.append_child(head, h1)
      let _ = node.append_child(head, badge)
      let _ = node.append_child(app, head)

      let base_d = df.parse_iso("2024-03-15", None)
      let jan31 = df.parse_iso("2024-01-31", None)
      let dur =
        duration.create()
        |> duration.with_months(1.0)
        |> duration.with_days(2.0)

      let ops = [
        #("addDays(base, 3)", df.add_days(arg(base_d), 3.0, None), "yyyy-MM-dd"),
        #("subDays(base, 15)", df.sub_days(arg(base_d), 15.0, None), "yyyy-MM-dd"),
        #("addWeeks(base, 2)", df.add_weeks(arg(base_d), 2.0, None), "yyyy-MM-dd"),
        #("addMonths(jan31, 1)", df.add_months(arg(jan31), 1.0, None), "yyyy-MM-dd"),
        #("subMonths(base, 3)", df.sub_months(arg(base_d), 3.0, None), "yyyy-MM-dd"),
        #("addYears(base, 1)", df.add_years(arg(base_d), 1.0, None), "yyyy-MM-dd"),
        #("startOfWeek(base)", df.start_of_week(arg(base_d), None), "yyyy-MM-dd"),
        #("endOfMonth(base)", df.end_of_month(arg(base_d), None), "yyyy-MM-dd HH:mm:ss"),
        #("add(base, { months: 1, days: 2 })", df.add(arg(base_d), dur, None), "yyyy-MM-dd"),
        #("base (immutable check)", base_d, "yyyy-MM-dd"),
      ]

      let card = document.create_element_div(doc, None)
      element.set_class_name(card, "card")
      let lbl = document.create_element_div(doc, None)
      element.set_class_name(lbl, "label")
      node.set_text_content(lbl, Some("Immutable Date Stepping"))
      let _ = node.append_child(card, lbl)

      let tbl = document.create_element_table(doc, None)
      let thead = document.create_element_thead(doc, None)
      let hrow = document.create_element_tr(doc, None)
      list.each(["Operation", "Result", "Δ Days (vs base)", "Δ Hours"], fn(h) {
        let th = document.create_element_th(doc, None)
        node.set_text_content(th, Some(h))
        let _ = node.append_child(hrow, th)
        Nil
      })
      let _ = node.append_child(thead, hrow)
      let _ = node.append_child(tbl, thead)

      let tbody = document.create_element_tbody(doc, None)
      list.each(ops, fn(item) {
        let #(name, res, fmt) = item
        let tr = document.create_element_tr(doc, None)
        let td_op = document.create_element_td(doc, None)
        let code = document.create_element_code(doc, None)
        node.set_text_content(code, Some(name))
        let _ = node.append_child(td_op, code)
        let td_res = document.create_element_td(doc, None)
        node.set_text_content(td_res, Some(df.format_date(day(res), fmt, None)))
        let td_dd = document.create_element_td(doc, None)
        node.set_text_content(
          td_dd,
          Some(fmt_signed(df.difference_in_calendar_days(day(res), day(base_d), None))),
        )
        let td_dh = document.create_element_td(doc, None)
        node.set_text_content(
          td_dh,
          Some(fmt_signed(df.difference_in_hours(day(res), day(base_d), None))),
        )
        let _ = node.append_child(tr, td_op)
        let _ = node.append_child(tr, td_res)
        let _ = node.append_child(tr, td_dd)
        let _ = node.append_child(tr, td_dh)
        let _ = node.append_child(tbody, tr)
        Nil
      })
      let _ = node.append_child(tbl, tbody)
      let _ = node.append_child(card, tbl)
      let _ = node.append_child(app, card)

      let footer = document.create_element_div(doc, None)
      element.set_class_name(footer, "card")
      let f_lbl = document.create_element_div(doc, None)
      element.set_class_name(f_lbl, "label")
      node.set_text_content(f_lbl, Some("Duration & Unit Conversions"))
      let _ = node.append_child(footer, f_lbl)

      let srow = document.create_element_div(doc, None)
      element.set_class_name(srow, "summary-row")
      add_kv(doc, srow, "formatDuration(dur)", df.format_duration(dur, None))
      add_kv(doc, srow, "formatISODuration(dur)", df.format_iso_duration(dur))
      add_kv(
        doc,
        srow,
        "hoursToMinutes(2)",
        int.to_string(float.round(df.hours_to_minutes(2.0))) <> " min",
      )
      add_kv(
        doc,
        srow,
        "daysToWeeks(14)",
        int.to_string(float.round(df.days_to_weeks(14.0))) <> " wk",
      )
      let _ = node.append_child(footer, srow)
      let _ = node.append_child(app, footer)
      Nil
    }
  }
}
