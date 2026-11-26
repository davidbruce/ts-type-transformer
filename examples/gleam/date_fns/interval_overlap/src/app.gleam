import date_fns as df
import date_fns/date_arg.{type DateArg}
import date_fns/interval
import date_fns/types as t
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import dom/areas/html/html_element_tag_name_map as tag
import gleam/list
import gleam/option.{None, Some}

fn make_span(s_iso: String, e_iso: String) {
  let s = df.parse_iso(s_iso)
  let e = df.parse_iso(e_iso)
  interval.create()
  |> interval.with_start(s)
  |> interval.with_end(e)
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let head = document.create_element(doc, tag.div)
      element.set_class_name(head, "head")
      let h1 = document.create_element(doc, tag.h1)
      node.set_text_content(
        h1,
        Some("Date-fns Interval Overlap & Clamp (Gleam)"),
      )
      let badge = document.create_element(doc, tag.span)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("March 2024 · 31 days"))
      node.append_child(head, h1)
      node.append_child(head, badge)
      node.append_child(app, head)

      let win_span = make_span("2024-03-01", "2024-03-31")
      let month_days = df.each_day_of_interval(win_span) |> list.map(as_date)

      let sprint_a = make_span("2024-03-08", "2024-03-18")
      let tracks = [
        #("Sprint A (Ref)", sprint_a, True),
        #("Design Review", make_span("2024-03-03", "2024-03-10"), False),
        #("QA Hardening", make_span("2024-03-15", "2024-03-25"), False),
        #("Post-Release", make_span("2024-03-22", "2024-03-29"), False),
      ]

      let gantt_card = document.create_element(doc, tag.div)
      element.set_class_name(gantt_card, "card")
      let g_lbl = document.create_element(doc, tag.div)
      element.set_class_name(g_lbl, "label")
      node.set_text_content(
        g_lbl,
        Some(
          "31-Day Gantt & Overlap Matrix (dark = active, grey = overlaps Sprint A)",
        ),
      )
      node.append_child(gantt_card, g_lbl)

      let gantt = document.create_element(doc, tag.div)
      element.set_class_name(gantt, "gantt")

      let ghead = document.create_element(doc, tag.div)
      element.set_class_name(ghead, "gantt-head")
      let h_track = document.create_element(doc, tag.div)
      node.set_text_content(h_track, Some("Track"))
      let hslots = document.create_element(doc, tag.div)
      element.set_class_name(hslots, "slots")
      list.each(month_days, fn(d) {
        let lbl = document.create_element(doc, tag.div)
        element.set_class_name(lbl, "slot-lbl")
        node.set_text_content(lbl, Some(df.format_date(d, "d")))
        node.append_child(hslots, lbl)
        Nil
      })
      let h_ref = document.create_element(doc, tag.div)
      node.set_text_content(h_ref, Some("vs Ref"))
      node.append_child(ghead, h_track)
      node.append_child(ghead, hslots)
      node.append_child(ghead, h_ref)
      node.append_child(gantt, ghead)

      list.each(tracks, fn(t) {
        let #(name, span, is_ref) = t
        let row = document.create_element(doc, tag.div)
        element.set_class_name(row, "gantt-row")

        let meta = document.create_element(doc, tag.div)
        element.set_class_name(meta, "track-meta")
        let s_name = document.create_element(doc, tag.span)
        element.set_class_name(s_name, "track-name")
        node.set_text_content(s_name, Some(name))
        let dur = df.interval_to_duration(span)
        let s_dur = document.create_element(doc, tag.span)
        element.set_class_name(s_dur, "track-dur")
        node.set_text_content(s_dur, Some(df.format_duration(dur)))
        node.append_child(meta, s_name)
        node.append_child(meta, s_dur)
        node.append_child(row, meta)

        let slots = document.create_element(doc, tag.div)
        element.set_class_name(slots, "slots")
        list.each(month_days, fn(d) {
          let in_track = df.is_within_interval(d, span)
          let in_ref = df.is_within_interval(d, sprint_a)
          let cls = case in_track, is_ref, in_ref {
            True, False, True -> "slot overlap"
            True, _, _ -> "slot active"
            False, _, _ -> "slot"
          }
          let slot = document.create_element(doc, tag.div)
          element.set_class_name(slot, cls)
          node.append_child(slots, slot)
          Nil
        })
        node.append_child(row, slots)

        let overlaps = df.are_intervals_overlapping(span, sprint_a)
        let b_text = case is_ref, overlaps {
          True, _ -> "REF"
          False, True -> "OVERLAP"
          False, False -> "DISJOINT"
        }
        let b_cls = case is_ref || overlaps {
          True -> "pill"
          False -> "pill-muted"
        }
        let pill = document.create_element(doc, tag.span)
        element.set_class_name(pill, b_cls)
        node.set_text_content(pill, Some(b_text))
        node.append_child(row, pill)
        node.append_child(gantt, row)
        Nil
      })

      node.append_child(gantt_card, gantt)
      node.append_child(app, gantt_card)

      let clamp_card = document.create_element(doc, tag.div)
      element.set_class_name(clamp_card, "card")
      let c_lbl = document.create_element(doc, tag.div)
      element.set_class_name(c_lbl, "label")
      node.set_text_content(
        c_lbl,
        Some("Clamp Probes against Sprint A [2024-03-08 .. 2024-03-18]"),
      )
      node.append_child(clamp_card, c_lbl)

      let tbl = document.create_element(doc, tag.table)
      let thead = document.create_element(doc, tag.thead)
      let hrow = document.create_element(doc, tag.tr)
      list.each(["Probe Date", "Clamped Output", "Rule"], fn(h) {
        let th = document.create_element(doc, tag.th)
        node.set_text_content(th, Some(h))
        node.append_child(hrow, th)
        Nil
      })
      node.append_child(thead, hrow)
      node.append_child(tbl, thead)

      let tbody = document.create_element(doc, tag.tbody)
      list.each(
        [
          #("2024-03-02", "Clamped to start"),
          #("2024-03-14", "Inside interval"),
          #("2024-03-28", "Clamped to end"),
        ],
        fn(p) {
          let #(iso, rule) = p
          let probe = df.parse_iso(iso)
          let clamped = as_date(df.clamp(probe, sprint_a))
          let out_txt = df.format_date(clamped, "yyyy-MM-dd")
          let tr = document.create_element(doc, tag.tr)
          list.each([iso, out_txt, rule], fn(txt) {
            let td = document.create_element(doc, tag.td)
            node.set_text_content(td, Some(txt))
            node.append_child(tr, td)
            Nil
          })
          node.append_child(tbody, tr)
          Nil
        },
      )
      node.append_child(tbl, tbody)
      node.append_child(clamp_card, tbl)
      node.append_child(app, clamp_card)
      Nil
    }
  }
}

// `each_day_of_interval` and `clamp` give back a `DateArg`; JavaScript gives a Date.
fn as_date(x: DateArg) -> t.Date {
  let assert date_arg.Date(d) = x
  d
}
