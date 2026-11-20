#![no_main]

use date_fns::{Date, DateArg, Interval};
use dom::{Document, HTMLElement};

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
    parent.append_child(child);
}

fn day(d: &Date) -> DateArg<Date> {
    DateArg::DateType(d.clone())
}

fn add_metric(doc: &Document, metrics: &HTMLElement, label: &str, val: &str) {
    let m = make_el(doc, "div", "metric", None);
    append(&m, make_el(doc, "div", "label", Some(label)));
    append(&m, make_el(doc, "div", "val", Some(val)));
    append(metrics, m);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let app = doc.get_element_by_id("app").expect("missing #app");

    let head = make_el(&doc, "div", "head", None);
    append(&head, make_el(&doc, "h1", "", Some("Date-fns Calendar Grid (Rust)")));
    append(&head, make_el(&doc, "span", "badge", Some("March 2024")));
    append(&app, head);

    let ref_day: Date = date_fns::parse_iso("2024-03-15", None);
    let month_start: Date = date_fns::start_of_month::<Date, Date>(day(&ref_day), None);
    let month_end: Date = date_fns::end_of_month::<Date, Date>(day(&ref_day), None);
    let offset = date_fns::get_day(&month_start, None) as usize;

    let metrics = make_el(&doc, "div", "metrics", None);
    add_metric(&doc, &metrics, "Days in Month", &format!("{}", date_fns::get_days_in_month(&ref_day, None) as i32));
    add_metric(&doc, &metrics, "Quarter", &format!("Q{}", date_fns::get_quarter(&ref_day, None) as i32));
    add_metric(&doc, &metrics, "ISO Week", &format!("W{}", date_fns::get_iso_week(&ref_day, None) as i32));
    let leap_str = if date_fns::is_leap_year(&ref_day, None) { "Yes" } else { "No" };
    add_metric(&doc, &metrics, "Leap Year / DOY", &format!("{leap_str} \u{00B7} #{}", date_fns::get_day_of_year(&ref_day, None) as i32));
    append(&app, metrics);

    let card = make_el(&doc, "div", "card", None);
    append(&card, make_el(&doc, "div", "label", Some("March 2024 \u{2014} Reference 2024-03-15")));

    let grid = make_el(&doc, "div", "cal-grid", None);
    for dow in ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] {
        append(&grid, make_el(&doc, "div", "dow", Some(dow)));
    }
    for _ in 0..offset {
        append(&grid, make_el(&doc, "div", "day-cell empty", None));
    }

    let span: Interval<DateArg<Date>, DateArg<Date>> = Interval::default()
        .with_start(day(&month_start))
        .with_end(day(&month_end));
    let days = date_fns::each_day_of_interval::<_, date_fns::EachDayOfIntervalOptions<Date>>(span, None);
    for i in 0..days.len() {
        let item = days.get(i);
        let is_ref = date_fns::is_same_day(item.clone(), &ref_day, None);
        let is_wknd = date_fns::is_weekend(item.clone(), None);
        let cls = if is_ref {
            "day-cell today"
        } else if is_wknd {
            "day-cell weekend"
        } else {
            "day-cell"
        };
        let cell = make_el(&doc, "div", cls, None);
        let day_num = date_fns::format_date(item.clone(), "d", None);
        append(&cell, make_el(&doc, "span", "day-num", Some(&day_num)));
        let sub = if is_ref {
            "REF".to_string()
        } else {
            date_fns::format_date(item, "EEE", None)
        };
        append(&cell, make_el(&doc, "span", "sub", Some(&sub)));
        append(&grid, cell);
    }

    append(&card, grid);
    append(&app, card);
}
