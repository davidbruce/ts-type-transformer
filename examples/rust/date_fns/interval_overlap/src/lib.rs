#![no_main]

use date_fns::{Date, DateArg, Interval};
use dom::{Document, HTMLElement};

fn make_el(doc: &Document, tag: &str, cls: &str, text: Option<&str>) -> HTMLElement {
    let el = doc.create_element(tag, None);
    if !cls.is_empty() {
        el.set_class_name(cls);
    }
    if let Some(t) = text {
        el.set_text_content(t);
    }
    el
}

fn append(parent: &HTMLElement, child: HTMLElement) {
    parent.append_child(child);
}

fn make_span(s_iso: &str, e_iso: &str) -> Interval<DateArg<Date>, DateArg<Date>> {
    let s: Date = date_fns::parse_iso(s_iso, None);
    let e: Date = date_fns::parse_iso(e_iso, None);
    Interval::default()
        .with_start(DateArg::DateType(s))
        .with_end(DateArg::DateType(e))
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let app = doc.get_element_by_id("app").expect("missing #app");

    let head = make_el(&doc, "div", "head", None);
    append(&head, make_el(&doc, "h1", "", Some("Date-fns Interval Overlap & Clamp (Rust)")));
    append(&head, make_el(&doc, "span", "badge", Some("March 2024 \u{00B7} 31 days")));
    append(&app, head);

    let win_span = make_span("2024-03-01", "2024-03-31");
    let month_days = date_fns::each_day_of_interval::<_, date_fns::EachDayOfIntervalOptions<Date>>(win_span, None);

    let sprint_a = make_span("2024-03-08", "2024-03-18");
    let tracks = [
        ("Sprint A (Ref)", sprint_a.clone(), true),
        ("Design Review", make_span("2024-03-03", "2024-03-10"), false),
        ("QA Hardening", make_span("2024-03-15", "2024-03-25"), false),
        ("Post-Release", make_span("2024-03-22", "2024-03-29"), false),
    ];

    let gantt_card = make_el(&doc, "div", "card", None);
    append(&gantt_card, make_el(&doc, "div", "label", Some("31-Day Gantt & Overlap Matrix (dark = active, grey = overlaps Sprint A)")));
    let gantt = make_el(&doc, "div", "gantt", None);

    let ghead = make_el(&doc, "div", "gantt-head", None);
    append(&ghead, make_el(&doc, "div", "", Some("Track")));
    let hslots = make_el(&doc, "div", "slots", None);
    for i in 0..month_days.len() {
        let lbl = date_fns::format_date(month_days.get(i), "d", None);
        append(&hslots, make_el(&doc, "div", "slot-lbl", Some(&lbl)));
    }
    append(&ghead, hslots);
    append(&ghead, make_el(&doc, "div", "", Some("vs Ref")));
    append(&gantt, ghead);

    for (name, span, is_ref) in &tracks {
        let row = make_el(&doc, "div", "gantt-row", None);
        let meta = make_el(&doc, "div", "track-meta", None);
        append(&meta, make_el(&doc, "span", "track-name", Some(name)));
        let dur = date_fns::interval_to_duration(span, None);
        let dur_txt = date_fns::format_duration(&dur, None);
        append(&meta, make_el(&doc, "span", "track-dur", Some(&dur_txt)));
        append(&row, meta);

        let slots = make_el(&doc, "div", "slots", None);
        for i in 0..month_days.len() {
            let d = month_days.get(i);
            let in_track = date_fns::is_within_interval(d.clone(), span, None);
            let in_ref = date_fns::is_within_interval(d, &sprint_a, None);
            let cls = if in_track && !is_ref && in_ref {
                "slot overlap"
            } else if in_track {
                "slot active"
            } else {
                "slot"
            };
            append(&slots, make_el(&doc, "div", cls, None));
        }
        append(&row, slots);

        let overlaps = date_fns::are_intervals_overlapping(span, &sprint_a, None);
        let b_text = if *is_ref {
            "REF"
        } else if overlaps {
            "OVERLAP"
        } else {
            "DISJOINT"
        };
        let b_cls = if *is_ref || overlaps { "pill" } else { "pill-muted" };
        append(&row, make_el(&doc, "span", b_cls, Some(b_text)));
        append(&gantt, row);
    }

    append(&gantt_card, gantt);
    append(&app, gantt_card);

    let clamp_card = make_el(&doc, "div", "card", None);
    append(&clamp_card, make_el(&doc, "div", "label", Some("Clamp Probes against Sprint A [2024-03-08 .. 2024-03-18]")));

    let tbl = make_el(&doc, "table", "", None);
    let thead = make_el(&doc, "thead", "", None);
    let hrow = make_el(&doc, "tr", "", None);
    for h in ["Probe Date", "Clamped Output", "Rule"] {
        append(&hrow, make_el(&doc, "th", "", Some(h)));
    }
    append(&thead, hrow);
    append(&tbl, thead);

    let tbody = make_el(&doc, "tbody", "", None);
    for (iso, rule) in [
        ("2024-03-02", "Clamped to start"),
        ("2024-03-14", "Inside interval"),
        ("2024-03-28", "Clamped to end"),
    ] {
        let probe: Date = date_fns::parse_iso(iso, None);
        let clamped = date_fns::clamp::<DateArg<Date>, Interval<DateArg<Date>, DateArg<Date>>, date_fns::ClampOptions<Date>>(
            DateArg::DateType(probe),
            sprint_a.clone(),
            None,
        );
        let out_txt = date_fns::format_date(clamped, "yyyy-MM-dd", None);
        let tr = make_el(&doc, "tr", "", None);
        append(&tr, make_el(&doc, "td", "", Some(iso)));
        append(&tr, make_el(&doc, "td", "", Some(&out_txt)));
        append(&tr, make_el(&doc, "td", "", Some(rule)));
        append(&tbody, tr);
    }
    append(&tbl, tbody);
    append(&clamp_card, tbl);
    append(&app, clamp_card);
}
