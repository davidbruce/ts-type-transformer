#![no_main]

use date_fns::{Date, DateArg, Duration};
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

fn fmt_signed(v: i32) -> String {
    if v > 0 {
        format!("+{v}")
    } else {
        format!("{v}")
    }
}

fn add_kv(doc: &Document, row: &HTMLElement, label: &str, val: &str) {
    let box_el = make_el(doc, "div", "kv", None);
    append(&box_el, make_el(doc, "span", "label", Some(label)));
    append(&box_el, make_el(doc, "strong", "", Some(val)));
    append(row, box_el);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let app = doc.get_element_by_id("app").expect("missing #app");

    let head = make_el(&doc, "div", "head", None);
    append(&head, make_el(&doc, "h1", "", Some("Date-fns Arithmetic & Durations (Rust)")));
    append(&head, make_el(&doc, "span", "badge", Some("Base 2024-03-15")));
    append(&app, head);

    let base_d: Date = date_fns::parse_iso("2024-03-15", None);
    let jan31: Date = date_fns::parse_iso("2024-01-31", None);
    let dur = Duration::default().with_months(1.0).with_days(2.0);

    let ops = [
        ("addDays(base, 3)", date_fns::add_days::<Date, Date>(day(&base_d), 3.0, None), "yyyy-MM-dd"),
        ("subDays(base, 15)", date_fns::sub_days::<Date, Date>(day(&base_d), 15.0, None), "yyyy-MM-dd"),
        ("addWeeks(base, 2)", date_fns::add_weeks::<Date, Date>(day(&base_d), 2.0, None), "yyyy-MM-dd"),
        ("addMonths(jan31, 1)", date_fns::add_months::<Date, Date>(day(&jan31), 1.0, None), "yyyy-MM-dd"),
        ("subMonths(base, 3)", date_fns::sub_months::<Date, Date>(day(&base_d), 3.0, None), "yyyy-MM-dd"),
        ("addYears(base, 1)", date_fns::add_years::<Date, Date>(day(&base_d), 1.0, None), "yyyy-MM-dd"),
        ("startOfWeek(base)", date_fns::start_of_week::<Date, Date>(day(&base_d), None), "yyyy-MM-dd"),
        ("endOfMonth(base)", date_fns::end_of_month::<Date, Date>(day(&base_d), None), "yyyy-MM-dd HH:mm:ss"),
        ("add(base, { months: 1, days: 2 })", date_fns::add::<Date, Date>(day(&base_d), &dur, None), "yyyy-MM-dd"),
        ("base (immutable check)", base_d.clone(), "yyyy-MM-dd"),
    ];

    let card = make_el(&doc, "div", "card", None);
    append(&card, make_el(&doc, "div", "label", Some("Immutable Date Stepping")));

    let tbl = make_el(&doc, "table", "", None);
    let thead = make_el(&doc, "thead", "", None);
    let hrow = make_el(&doc, "tr", "", None);
    for h in ["Operation", "Result", "\u{0394} Days (vs base)", "\u{0394} Hours"] {
        append(&hrow, make_el(&doc, "th", "", Some(h)));
    }
    append(&thead, hrow);
    append(&tbl, thead);

    let tbody = make_el(&doc, "tbody", "", None);
    for (op_name, res, fmt) in &ops {
        let tr = make_el(&doc, "tr", "", None);
        let td_op = make_el(&doc, "td", "", None);
        append(&td_op, make_el(&doc, "code", "", Some(op_name)));
        append(&tr, td_op);
        append(&tr, make_el(&doc, "td", "", Some(&date_fns::format_date(res, fmt, None))));
        let dd = date_fns::difference_in_calendar_days(res, &base_d, None) as i32;
        let dh = date_fns::difference_in_hours(res, &base_d, None) as i32;
        append(&tr, make_el(&doc, "td", "", Some(&fmt_signed(dd))));
        append(&tr, make_el(&doc, "td", "", Some(&fmt_signed(dh))));
        append(&tbody, tr);
    }
    append(&tbl, tbody);
    append(&card, tbl);
    append(&app, card);

    let footer = make_el(&doc, "div", "card", None);
    append(&footer, make_el(&doc, "div", "label", Some("Duration & Unit Conversions")));
    let srow = make_el(&doc, "div", "summary-row", None);
    add_kv(&doc, &srow, "formatDuration(dur)", &date_fns::format_duration(&dur, None));
    add_kv(&doc, &srow, "formatISODuration(dur)", &date_fns::format_iso_duration(&dur));
    add_kv(&doc, &srow, "hoursToMinutes(2)", &format!("{} min", date_fns::hours_to_minutes(2.0) as i32));
    add_kv(&doc, &srow, "daysToWeeks(14)", &format!("{} wk", date_fns::days_to_weeks(14.0) as i32));
    append(&footer, srow);
    append(&app, footer);
}
