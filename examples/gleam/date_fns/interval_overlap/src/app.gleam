import date_fns/areas/main as df
import date_fns/areas/main/interval
import date_fns/runtime
import date_fns/types as dt
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node
import gleam/list
import gleam/option.{None, Some}

fn make_span(s_iso: String, e_iso: String) {
  let dc = dt.date_codec()
  let arg = dt.DateArgDateType
  let s = df.parse_iso(s_iso, None)
  let e = df.parse_iso(e_iso, None)
  interval.create()
  |> interval.with_start(arg(s), dt.date_arg_codec(dc))
  |> interval.with_end(arg(e), dt.date_arg_codec(dc))
}

pub fn main() -> Nil {
  let doc = core.get_document()
  case document.get_element_by_id(doc, "app") {
    None -> Nil
    Some(app) -> {
      let dc = dt.date_codec()
      let arg = dt.DateArgDateType

      let head = document.create_element_div(doc, None)
      element.set_class_name(head, "head")
      let h1 = document.create_element_h1(doc, None)
      node.set_text_content(h1, Some("Date-fns Interval Overlap & Clamp (Gleam)"))
      let badge = document.create_element_span(doc, None)
      element.set_class_name(badge, "badge")
      node.set_text_content(badge, Some("March 2024 · 31 days"))
      let _ = node.append_child(head, h1)
      let _ = node.append_child(head, badge)
      let _ = node.append_child(app, head)

      let win_span = make_span("2024-03-01", "2024-03-31")
      let month_days = df.each_day_of_interval(win_span, None, runtime.codec_dynamic())

      let sprint_a = make_span("2024-03-08", "2024-03-18")
      let tracks = [
        #("Sprint A (Ref)", sprint_a, True),
        #("Design Review", make_span("2024-03-03", "2024-03-10"), False),
        #("QA Hardening", make_span("2024-03-15", "2024-03-25"), False),
        #("Post-Release", make_span("2024-03-22", "2024-03-29"), False),
      ]

      let gantt_card = document.create_element_div(doc, None)
      element.set_class_name(gantt_card, "card")
      let g_lbl = document.create_element_div(doc, None)
      element.set_class_name(g_lbl, "label")
      node.set_text_content(
        g_lbl,
        Some("31-Day Gantt & Overlap Matrix (dark = active, grey = overlaps Sprint A)"),
      )
      let _ = node.append_child(gantt_card, g_lbl)

      let gantt = document.create_element_div(doc, None)
      element.set_class_name(gantt, "gantt")

      let ghead = document.create_element_div(doc, None)
      element.set_class_name(ghead, "gantt-head")
      let h_track = document.create_element_div(doc, None)
      node.set_text_content(h_track, Some("Track"))
      let hslots = document.create_element_div(doc, None)
      element.set_class_name(hslots, "slots")
      list.each(month_days, fn(d) {
        let lbl = document.create_element_div(doc, None)
        element.set_class_name(lbl, "slot-lbl")
        node.set_text_content(lbl, Some(df.format_date(d, "d", None)))
        let _ = node.append_child(hslots, lbl)
        Nil
      })
      let h_ref = document.create_element_div(doc, None)
      node.set_text_content(h_ref, Some("vs Ref"))
      let _ = node.append_child(ghead, h_track)
      let _ = node.append_child(ghead, hslots)
      let _ = node.append_child(ghead, h_ref)
      let _ = node.append_child(gantt, ghead)

      list.each(tracks, fn(t) {
        let #(name, span, is_ref) = t
        let row = document.create_element_div(doc, None)
        element.set_class_name(row, "gantt-row")

        let meta = document.create_element_div(doc, None)
        element.set_class_name(meta, "track-meta")
        let s_name = document.create_element_span(doc, None)
        element.set_class_name(s_name, "track-name")
        node.set_text_content(s_name, Some(name))
        let dur = df.interval_to_duration(span, None)
        let s_dur = document.create_element_span(doc, None)
        element.set_class_name(s_dur, "track-dur")
        node.set_text_content(s_dur, Some(df.format_duration(dur, None)))
        let _ = node.append_child(meta, s_name)
        let _ = node.append_child(meta, s_dur)
        let _ = node.append_child(row, meta)

        let slots = document.create_element_div(doc, None)
        element.set_class_name(slots, "slots")
        list.each(month_days, fn(d) {
          let in_track = df.is_within_interval(d, span, None)
          let in_ref = df.is_within_interval(d, sprint_a, None)
          let cls = case in_track, is_ref, in_ref {
            True, False, True -> "slot overlap"
            True, _, _ -> "slot active"
            False, _, _ -> "slot"
          }
          let slot = document.create_element_div(doc, None)
          element.set_class_name(slot, cls)
          let _ = node.append_child(slots, slot)
          Nil
        })
        let _ = node.append_child(row, slots)

        let overlaps = df.are_intervals_overlapping(span, sprint_a, None)
        let b_text = case is_ref, overlaps {
          True, _ -> "REF"
          False, True -> "OVERLAP"
          False, False -> "DISJOINT"
        }
        let b_cls = case is_ref || overlaps {
          True -> "pill"
          False -> "pill-muted"
        }
        let pill = document.create_element_span(doc, None)
        element.set_class_name(pill, b_cls)
        node.set_text_content(pill, Some(b_text))
        let _ = node.append_child(row, pill)
        let _ = node.append_child(gantt, row)
        Nil
      })

      let _ = node.append_child(gantt_card, gantt)
      let _ = node.append_child(app, gantt_card)

      let clamp_card = document.create_element_div(doc, None)
      element.set_class_name(clamp_card, "card")
      let c_lbl = document.create_element_div(doc, None)
      element.set_class_name(c_lbl, "label")
      node.set_text_content(
        c_lbl,
        Some("Clamp Probes against Sprint A [2024-03-08 .. 2024-03-18]"),
      )
      let _ = node.append_child(clamp_card, c_lbl)

      let tbl = document.create_element_table(doc, None)
      let thead = document.create_element_thead(doc, None)
      let hrow = document.create_element_tr(doc, None)
      list.each(["Probe Date", "Clamped Output", "Rule"], fn(h) {
        let th = document.create_element_th(doc, None)
        node.set_text_content(th, Some(h))
        let _ = node.append_child(hrow, th)
        Nil
      })
      let _ = node.append_child(thead, hrow)
      let _ = node.append_child(tbl, thead)

      let tbody = document.create_element_tbody(doc, None)
      list.each(
        [
          #("2024-03-02", "Clamped to start"),
          #("2024-03-14", "Inside interval"),
          #("2024-03-28", "Clamped to end"),
        ],
        fn(p) {
          let #(iso, rule) = p
          let probe = df.parse_iso(iso, None)
          let clamped =
            df.clamp(
              arg(probe),
              sprint_a,
              None,
              dt.date_arg_codec(dc),
              runtime.codec_dynamic(),
            )
          let out_txt = df.format_date(clamped, "yyyy-MM-dd", None)
          let tr = document.create_element_tr(doc, None)
          list.each([iso, out_txt, rule], fn(txt) {
            let td = document.create_element_td(doc, None)
            node.set_text_content(td, Some(txt))
            let _ = node.append_child(tr, td)
            Nil
          })
          let _ = node.append_child(tbody, tr)
          Nil
        },
      )
      let _ = node.append_child(tbl, tbody)
      let _ = node.append_child(clamp_card, tbl)
      let _ = node.append_child(app, clamp_card)
      Nil
    }
  }
}
