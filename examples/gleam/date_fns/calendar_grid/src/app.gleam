import date_fns as df
import date_fns/interval
import date_fns/types as t
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import dom/areas/html/html_element_tag_name_map as tag
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}

fn add_metric(doc, metrics, label: String, val: String) -> Nil {
  let m = document.create_element(doc, tag.div)
  element.set_class_name(m, "metric")
  let lbl = document.create_element(doc, tag.div)
  element.set_class_name(lbl, "label")
  node.set_text_content(lbl, Some(label))
  let v = document.create_element(doc, tag.div)
  element.set_class_name(v, "val")
  node.set_text_content(v, Some(val))
  node.append_child(m, lbl)
  node.append_child(m, v)
  node.append_child(metrics, m)
  Nil
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let head = document.create_element(doc, tag.div)
      element.set_class_name(head, "head")
      let h1 = document.create_element(doc, tag.h1)
      node.set_text_content(h1, Some("Date-fns Calendar Grid (Gleam)"))
      let badge = document.create_element(doc, tag.span)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("March 2024"))
      node.append_child(head, h1)
      node.append_child(head, badge)
      node.append_child(app, head)

      let ref_day = df.parse_iso("2024-03-15")
      let month_start = df.start_of_month(ref_day)
      let month_end = df.end_of_month(ref_day)
      let offset = float.round(df.get_day(month_start))

      let metrics = document.create_element(doc, tag.div)
      element.set_class_name(metrics, "metrics")
      add_metric(
        doc,
        metrics,
        "Days in Month",
        int.to_string(float.round(df.get_days_in_month(ref_day))),
      )
      add_metric(
        doc,
        metrics,
        "Quarter",
        "Q" <> int.to_string(float.round(df.get_quarter(ref_day))),
      )
      add_metric(
        doc,
        metrics,
        "ISO Week",
        "W" <> int.to_string(float.round(df.get_iso_week(ref_day))),
      )
      let leap = case df.is_leap_year(ref_day) {
        True -> "Yes · #"
        False -> "No · #"
      }
      add_metric(
        doc,
        metrics,
        "Leap Year / DOY",
        leap <> int.to_string(float.round(df.get_day_of_year(ref_day))),
      )
      node.append_child(app, metrics)

      let card = document.create_element(doc, tag.div)
      element.set_class_name(card, "card")
      let lbl = document.create_element(doc, tag.div)
      element.set_class_name(lbl, "label")
      node.set_text_content(lbl, Some("March 2024 — Reference 2024-03-15"))
      node.append_child(card, lbl)

      let grid = document.create_element(doc, tag.div)
      element.set_class_name(grid, "cal-grid")
      list.each(["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"], fn(dow) {
        let el = document.create_element(doc, tag.div)
        element.set_class_name(el, "dow")
        node.set_text_content(el, Some(dow))
        node.append_child(grid, el)
        Nil
      })

      list.each(list.repeat(Nil, offset), fn(_) {
        let empty = document.create_element(doc, tag.div)
        element.set_class_name(empty, "day-cell empty")
        node.append_child(grid, empty)
        Nil
      })

      let span =
        interval.create()
        |> interval.with_start(month_start)
        |> interval.with_end(month_end)
      let days = df.each_day_of_interval(span) |> list.map(as_date)

      list.each(days, fn(d) {
        let is_ref = df.is_same_day(d, ref_day)
        let is_wknd = df.is_weekend(d)
        let cls = case is_ref, is_wknd {
          True, _ -> "day-cell today"
          False, True -> "day-cell weekend"
          False, False -> "day-cell"
        }
        let cell = document.create_element(doc, tag.div)
        element.set_class_name(cell, cls)
        let day_num = document.create_element(doc, tag.span)
        element.set_class_name(day_num, "day-num")
        node.set_text_content(day_num, Some(df.format_date(d, "d")))
        let sub = document.create_element(doc, tag.span)
        element.set_class_name(sub, "sub")
        let sub_txt = case is_ref {
          True -> "REF"
          False -> df.format_date(d, "EEE")
        }
        node.set_text_content(sub, Some(sub_txt))
        node.append_child(cell, day_num)
        node.append_child(cell, sub)
        node.append_child(grid, cell)
        Nil
      })

      node.append_child(card, grid)
      node.append_child(app, card)
      Nil
    }
  }
}

// `each_day_of_interval` and `clamp` give back a `DateArg`; JavaScript gives a Date.
fn as_date(x: t.DateArg) -> t.Date {
  let assert t.DateArgDate(d) = x
  d
}
