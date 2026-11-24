import date_fns as df
import date_fns/format_distance_options
import date_fns/types as dt
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import dom/areas/html/html_element_tag_name_map as tag
import gleam/float
import gleam/int
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
      node.set_text_content(h1, Some("Date-fns Relative Timeline (Gleam)"))
      let badge = document.create_element(doc, tag.span)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("Anchor 2024-03-15"))
      node.append_child(head, h1)
      node.append_child(head, badge)
      node.append_child(app, head)

      let anchor = df.parse_iso("2024-03-15")
      let opts =
        format_distance_options.create()
        |> format_distance_options.with_add_suffix(True)

      let entries = [
        #("2024-01-10", "Spec Draft"),
        #("2024-02-29", "Leap Day Freeze"),
        #("2024-03-12", "Beta Tag"),
        #("2024-03-15", "Release Anchor"),
        #("2024-03-18", "Patch Window"),
        #("2024-06-15", "Q2 Audit"),
      ]
      let milestones = list.map(entries, fn(e) { #(df.parse_iso(e.0), e.1) })

      let tl_card = document.create_element(doc, tag.div)
      element.set_class_name(tl_card, "card")
      let tl_lbl = document.create_element(doc, tag.div)
      element.set_class_name(tl_lbl, "label")
      node.set_text_content(tl_lbl, Some("Milestones Relative to 2024-03-15"))
      node.append_child(tl_card, tl_lbl)

      let timeline = document.create_element(doc, tag.div)
      element.set_class_name(timeline, "timeline")
      list.each(milestones, fn(item) {
        let #(d, title) = item
        let same = df.is_same_day(day(d), day(anchor))
        let row = document.create_element(doc, tag.div)
        element.set_class_name(row, case same {
          True -> "milestone anchor"
          False -> "milestone"
        })

        let s_date = document.create_element(doc, tag.span)
        element.set_class_name(s_date, "muted")
        node.set_text_content(
          s_date,
          Some(df.format_date(day(d), "yyyy-MM-dd (EEE)")),
        )

        let s_title = document.create_element(doc, tag.span)
        element.set_class_name(s_title, "title")
        node.set_text_content(s_title, Some(title))

        let s_rel = document.create_element(doc, tag.span)
        node.set_text_content(
          s_rel,
          Some(case same {
            True -> "today (anchor)"
            False -> df.format_distance_with(day(d), day(anchor), Some(opts))
          }),
        )

        let strict = df.format_distance_strict(day(d), day(anchor))
        let cmp =
          int.to_string(float.round(df.compare_asc(day(d), day(anchor))))
        let s_strict = document.create_element(doc, tag.span)
        element.set_class_name(s_strict, "muted")
        node.set_text_content(s_strict, Some(strict <> " · cmp " <> cmp))

        let status = case same {
          True -> "ANCHOR"
          False ->
            case df.is_before(day(d), day(anchor)) {
              True -> "PAST"
              False ->
                case df.is_after(day(d), day(anchor)) {
                  True -> "FUTURE"
                  False -> "OTHER"
                }
            }
        }
        let s_pill = document.create_element(doc, tag.span)
        element.set_class_name(s_pill, "pill")
        node.set_text_content(s_pill, Some(status))

        node.append_child(row, s_date)
        node.append_child(row, s_title)
        node.append_child(row, s_rel)
        node.append_child(row, s_strict)
        node.append_child(row, s_pill)
        node.append_child(timeline, row)
        Nil
      })
      node.append_child(tl_card, timeline)
      node.append_child(app, tl_card)

      let date_args = list.map(milestones, fn(item) { arg(item.0) })
      let earliest = df.min(date_args)
      let latest = df.max(date_args)
      let span_days =
        int.to_string(
          float.round(df.difference_in_calendar_days(day(latest), day(earliest))),
        )

      let sum_card = document.create_element(doc, tag.div)
      element.set_class_name(sum_card, "card")
      let s_lbl = document.create_element(doc, tag.div)
      element.set_class_name(s_lbl, "label")
      node.set_text_content(s_lbl, Some("Extremes & Total Span"))
      node.append_child(sum_card, s_lbl)

      let summary = document.create_element(doc, tag.div)
      element.set_class_name(summary, "summary")
      list.each(
        [
          "Earliest: " <> df.format_date(day(earliest), "yyyy-MM-dd"),
          "Latest: " <> df.format_date(day(latest), "yyyy-MM-dd"),
          "Total Span: " <> span_days <> " days",
        ],
        fn(txt) {
          let s = document.create_element(doc, tag.span)
          node.set_text_content(s, Some(txt))
          node.append_child(summary, s)
          Nil
        },
      )
      node.append_child(sum_card, summary)
      node.append_child(app, sum_card)
      Nil
    }
  }
}
