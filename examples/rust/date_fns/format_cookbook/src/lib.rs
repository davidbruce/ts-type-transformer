#![no_main]

use date_fns::Date;
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

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let app = doc.get_element_by_id("app").expect("missing #app");

    let head = make_el(&doc, "div", "head", None);
    append(
        &head,
        make_el(
            &doc,
            "h1",
            "",
            Some("Date-fns Format & Parse Cookbook (Rust)"),
        ),
    );
    append(
        &head,
        make_el(&doc, "span", "badge", Some("2024-03-15T14:30:45")),
    );
    append(&app, head);

    let sample: Date = date_fns::parse_iso("2024-03-15T14:30:45");
    let parsed: Date = date_fns::parse("15/03/2024", "dd/MM/yyyy", &sample);

    let fmt_card = make_el(&doc, "div", "card", None);
    append(
        &fmt_card,
        make_el(&doc, "div", "label", Some("Format Tokens & ISO Output")),
    );

    let tbl = make_el(&doc, "table", "", None);
    let thead = make_el(&doc, "thead", "", None);
    let hrow = make_el(&doc, "tr", "", None);
    for h in ["API / Pattern", "Output", "Notes"] {
        append(&hrow, make_el(&doc, "th", "", Some(h)));
    }
    append(&thead, hrow);
    append(&tbl, thead);

    let rows = [
        (
            "formatDate(\"yyyy-MM-dd\")",
            date_fns::format_date(&sample, "yyyy-MM-dd"),
            "ISO date",
        ),
        (
            "formatDate(\"EEEE, do MMMM yyyy\")",
            date_fns::format_date(&sample, "EEEE, do MMMM yyyy"),
            "Ordinal full date",
        ),
        (
            "formatDate(\"EEE, d MMM yy\")",
            date_fns::format_date(&sample, "EEE, d MMM yy"),
            "Compact short date",
        ),
        (
            "formatDate(\"HH:mm:ss\")",
            date_fns::format_date(&sample, "HH:mm:ss"),
            "24-hour clock",
        ),
        (
            "formatDate(\"h:mm a\")",
            date_fns::format_date(&sample, "h:mm a"),
            "12-hour clock",
        ),
        (
            "formatDate(\"'Q'Q \u{00B7} 'W'I\")",
            date_fns::format_date(&sample, "'Q'Q \u{00B7} 'W'I"),
            "Quarter & ISO week",
        ),
        (
            "formatDate(\"DDD 'of' yyyy\")",
            date_fns::format_date(&sample, "DDD 'of' yyyy"),
            "Day of year",
        ),
        (
            "lightFormat(\"yyyy/MM/dd HH:mm\")",
            date_fns::light_format(&sample, "yyyy/MM/dd HH:mm"),
            "Zero-locale light formatter",
        ),
        (
            "formatISO(sample)",
            date_fns::format_iso(&sample),
            "ISO 8601 timestamp",
        ),
        (
            "parse(\"15/03/2024\", \"dd/MM/yyyy\")",
            date_fns::format_date(&parsed, "yyyy-MM-dd"),
            "Custom parse roundtrip",
        ),
    ];

    let tbody = make_el(&doc, "tbody", "", None);
    for (api, output, note) in rows {
        let tr = make_el(&doc, "tr", "", None);
        let td_api = make_el(&doc, "td", "", None);
        append(&td_api, make_el(&doc, "code", "", Some(api)));
        append(&tr, td_api);
        append(&tr, make_el(&doc, "td", "", Some(&output)));
        append(&tr, make_el(&doc, "td", "", Some(note)));
        append(&tbody, tr);
    }
    append(&tbl, tbody);
    append(&fmt_card, tbl);
    append(&app, fmt_card);

    let val_card = make_el(&doc, "div", "card", None);
    append(
        &val_card,
        make_el(
            &doc,
            "div",
            "label",
            Some("Validation & Calendar Existence"),
        ),
    );

    let vtbl = make_el(&doc, "table", "", None);
    let vhead = make_el(&doc, "thead", "", None);
    let vhrow = make_el(&doc, "tr", "", None);
    for h in ["Input", "Check", "Result"] {
        append(&vhrow, make_el(&doc, "th", "", Some(h)));
    }
    append(&vhead, vhrow);
    append(&vtbl, vhead);

    let good_iso: Date = date_fns::parse_iso("2024-03-15");
    let bad_iso: Date = date_fns::parse_iso("not-a-date");
    let ok_good = date_fns::is_valid(&good_iso) && date_fns::is_match("2024-03-15", "yyyy-MM-dd");
    let ok_bad = date_fns::is_valid(&bad_iso) && date_fns::is_match("not-a-date", "yyyy-MM-dd");
    let ok_2024 = date_fns::is_exists(2024.0, 1.0, 29.0);
    let ok_2023 = date_fns::is_exists(2023.0, 1.0, 29.0);

    let checks = [
        (
            "\"2024-03-15\"",
            "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")",
            ok_good,
            if ok_good { "VALID" } else { "INVALID" },
        ),
        (
            "\"not-a-date\"",
            "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")",
            ok_bad,
            if ok_bad { "VALID" } else { "INVALID" },
        ),
        (
            "(2024, 1, 29)",
            "isExists(2024, 1, 29) leap Feb 29",
            ok_2024,
            if ok_2024 { "EXISTS" } else { "INVALID" },
        ),
        (
            "(2023, 1, 29)",
            "isExists(2023, 1, 29) non-leap Feb 29",
            ok_2023,
            if ok_2023 { "EXISTS" } else { "INVALID" },
        ),
    ];

    let vbody = make_el(&doc, "tbody", "", None);
    for (input, chk, ok, label_txt) in checks {
        let tr = make_el(&doc, "tr", "", None);
        let td_in = make_el(&doc, "td", "", None);
        append(&td_in, make_el(&doc, "code", "", Some(input)));
        append(&tr, td_in);
        append(&tr, make_el(&doc, "td", "", Some(chk)));
        let td_res = make_el(&doc, "td", "", None);
        append(
            &td_res,
            make_el(
                &doc,
                "span",
                if ok { "pill" } else { "pill-muted" },
                Some(label_txt),
            ),
        );
        append(&tr, td_res);
        append(&vbody, tr);
    }
    append(&vtbl, vbody);
    append(&val_card, vtbl);
    append(&app, val_card);
}
