#![no_main]

use date_fns::{Date, DateArg, FormatDistanceOptions, FormatDistanceOptionsMethods as _};
use dom::{Document, DocumentMethods as _, ElementMethods as _, HTMLElement, NodeMethods};

fn make_el(doc: &Document, tag: &str, cls: &str, text: Option<&str>) -> HTMLElement {
    let el = doc.create_element(tag, None);
    if !cls.is_empty() {
        el.set_class_name(cls);
    }
    if let Some(t) = text {
        el.set_text_content(Some(t));
    }
    el
}

fn append(parent: &HTMLElement, child: HTMLElement) {
    let _ = NodeMethods::append_child(parent, child);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let app = doc.get_element_by_id("app").unwrap();

    let head = make_el(&doc, "div", "head", None);
    append(&head, make_el(&doc, "h1", "", Some("Date-fns Relative Timeline (Rust)")));
    append(&head, make_el(&doc, "span", "badge", Some("Anchor 2024-03-15")));
    append(&app, head);

    let anchor: Date = date_fns::parse_iso::<Date, Date>("2024-03-15", None);
    let opts = FormatDistanceOptions::create();
    opts.set_add_suffix(Some(true));

    let raw = [
        ("2024-01-10", "Spec Draft"),
        ("2024-02-29", "Leap Day Freeze"),
        ("2024-03-12", "Beta Tag"),
        ("2024-03-15", "Release Anchor"),
        ("2024-03-18", "Patch Window"),
        ("2024-06-15", "Q2 Audit"),
    ];

    let tl_card = make_el(&doc, "div", "card", None);
    append(&tl_card, make_el(&doc, "div", "label", Some("Milestones Relative to 2024-03-15")));
    let timeline = make_el(&doc, "div", "timeline", None);

    let mut all_dates: Vec<DateArg<Date>> = Vec::new();
    for (iso, title) in raw {
        let d: Date = date_fns::parse_iso::<Date, Date>(iso, None);
        all_dates.push(DateArg::DateType(d.clone()));

        let same = date_fns::is_same_day(&d, &anchor, None);
        let row = make_el(&doc, "div", if same { "milestone anchor" } else { "milestone" }, None);
        append(&row, make_el(&doc, "span", "muted", Some(&date_fns::format_date(&d, "yyyy-MM-dd (EEE)", None))));
        append(&row, make_el(&doc, "span", "title", Some(title)));
        let rel = if same {
            "today (anchor)".to_string()
        } else {
            date_fns::format_distance(&d, &anchor, Some(&opts))
        };
        append(&row, make_el(&doc, "span", "", Some(&rel)));
        let strict = date_fns::format_distance_strict(&d, &anchor, None);
        let cmp = date_fns::compare_asc(&d, &anchor) as i32;
        append(&row, make_el(&doc, "span", "muted", Some(&format!("{strict} \u{00B7} cmp {cmp}"))));
        let status = if same {
            "ANCHOR"
        } else if date_fns::is_before(&d, &anchor) {
            "PAST"
        } else if date_fns::is_after(&d, &anchor) {
            "FUTURE"
        } else {
            "OTHER"
        };
        append(&row, make_el(&doc, "span", "pill", Some(status)));
        append(&timeline, row);
    }
    append(&tl_card, timeline);
    append(&app, tl_card);

    let earliest: Date = date_fns::min::<Date, Date>(all_dates.clone(), None);
    let latest: Date = date_fns::max::<Date, Date>(all_dates, None);
    let total_span = date_fns::difference_in_calendar_days(&latest, &earliest, None) as i32;

    let sum_card = make_el(&doc, "div", "card", None);
    append(&sum_card, make_el(&doc, "div", "label", Some("Extremes & Total Span")));
    let summary = make_el(&doc, "div", "summary", None);
    append(&summary, make_el(&doc, "span", "", Some(&format!("Earliest: {}", date_fns::format_date(&earliest, "yyyy-MM-dd", None)))));
    append(&summary, make_el(&doc, "span", "", Some(&format!("Latest: {}", date_fns::format_date(&latest, "yyyy-MM-dd", None)))));
    append(&summary, make_el(&doc, "span", "", Some(&format!("Total Span: {total_span} days"))));
    append(&sum_card, summary);
    append(&app, sum_card);
}
