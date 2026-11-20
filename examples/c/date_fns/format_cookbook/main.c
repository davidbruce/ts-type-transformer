#include "dom/dom.h"
#include "date_fns/date_fns.h"

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag);
    if (cls && cls[0]) HTMLElement_set_className(el, cls);
    if (text) HTMLElement_set_textContent(el, text);
    return el;
}

static void add_fmt_row(Document doc, HTMLElement tbody, const char* api, JsString out, const char* note) {
    HTMLElement tr = make_el(doc, "tr", 0, 0);
    HTMLElement td_api = make_el(doc, "td", 0, 0);
    HTMLElement_appendChild(td_api, make_el(doc, "code", 0, api));
    HTMLElement_appendChild(tr, td_api);

    HTMLElement td_out = make_el(doc, "td", 0, 0);
    HTMLElement_set_textContent(td_out, out);
    HTMLElement_appendChild(tr, td_out);
    HTMLElement_appendChild(tr, make_el(doc, "td", 0, note));
    HTMLElement_appendChild(tbody, tr);
}

static void add_val_row(Document doc, HTMLElement tbody, const char* input, const char* check, bool ok, const char* pill_text) {
    HTMLElement tr = make_el(doc, "tr", 0, 0);
    HTMLElement td_in = make_el(doc, "td", 0, 0);
    HTMLElement_appendChild(td_in, make_el(doc, "code", 0, input));
    HTMLElement_appendChild(tr, td_in);
    HTMLElement_appendChild(tr, make_el(doc, "td", 0, check));
    HTMLElement td_res = make_el(doc, "td", 0, 0);
    HTMLElement_appendChild(td_res, make_el(doc, "span", ok ? "pill" : "pill-muted", pill_text));
    HTMLElement_appendChild(tr, td_res);
    HTMLElement_appendChild(tbody, tr);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "Date-fns Format & Parse Cookbook (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", "2024-03-15T14:30:45"));
    HTMLElement_appendChild(app, head);

    Date sample = parseISO("2024-03-15T14:30:45");
    Date parsed = parse("15/03/2024", "dd/MM/yyyy", DateArg_of(sample));

    HTMLElement fmt_card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(fmt_card, make_el(doc, "div", "label", "Format Tokens & ISO Output"));

    HTMLElement tbl = make_el(doc, "table", 0, 0);
    HTMLElement thead = make_el(doc, "thead", 0, 0);
    HTMLElement hrow = make_el(doc, "tr", 0, 0);
    HTMLElement_appendChild(hrow, make_el(doc, "th", 0, "API / Pattern"));
    HTMLElement_appendChild(hrow, make_el(doc, "th", 0, "Output"));
    HTMLElement_appendChild(hrow, make_el(doc, "th", 0, "Notes"));
    HTMLElement_appendChild(thead, hrow);
    HTMLElement_appendChild(tbl, thead);

    HTMLElement tbody = make_el(doc, "tbody", 0, 0);
    add_fmt_row(doc, tbody, "formatDate(\"yyyy-MM-dd\")", formatDate_with_Date(sample, "yyyy-MM-dd"), "ISO date");
    add_fmt_row(doc, tbody, "formatDate(\"EEEE, do MMMM yyyy\")", formatDate_with_Date(sample, "EEEE, do MMMM yyyy"), "Ordinal full date");
    add_fmt_row(doc, tbody, "formatDate(\"EEE, d MMM yy\")", formatDate_with_Date(sample, "EEE, d MMM yy"), "Compact short date");
    add_fmt_row(doc, tbody, "formatDate(\"HH:mm:ss\")", formatDate_with_Date(sample, "HH:mm:ss"), "24-hour clock");
    add_fmt_row(doc, tbody, "formatDate(\"h:mm a\")", formatDate_with_Date(sample, "h:mm a"), "12-hour clock");
    add_fmt_row(doc, tbody, "formatDate(\"'Q'Q \xC2\xB7 'W'I\")", formatDate_with_Date(sample, "'Q'Q \xC2\xB7 'W'I"), "Quarter & ISO week");
    add_fmt_row(doc, tbody, "formatDate(\"DDD 'of' yyyy\")", formatDate_with_Date(sample, "DDD 'of' yyyy"), "Day of year");
    add_fmt_row(doc, tbody, "lightFormat(\"yyyy/MM/dd HH:mm\")", lightFormat_with_Date(sample, "yyyy/MM/dd HH:mm"), "Zero-locale light formatter");
    add_fmt_row(doc, tbody, "formatISO(sample)", formatISO_with_Date(sample), "ISO 8601 timestamp");
    add_fmt_row(doc, tbody, "parse(\"15/03/2024\", \"dd/MM/yyyy\")", formatDate_with_Date(parsed, "yyyy-MM-dd"), "Custom parse roundtrip");
    HTMLElement_appendChild(tbl, tbody);
    HTMLElement_appendChild(fmt_card, tbl);
    HTMLElement_appendChild(app, fmt_card);

    HTMLElement val_card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(val_card, make_el(doc, "div", "label", "Validation & Calendar Existence"));

    HTMLElement vtbl = make_el(doc, "table", 0, 0);
    HTMLElement vhead = make_el(doc, "thead", 0, 0);
    HTMLElement vhrow = make_el(doc, "tr", 0, 0);
    HTMLElement_appendChild(vhrow, make_el(doc, "th", 0, "Input"));
    HTMLElement_appendChild(vhrow, make_el(doc, "th", 0, "Check"));
    HTMLElement_appendChild(vhrow, make_el(doc, "th", 0, "Result"));
    HTMLElement_appendChild(vhead, vhrow);
    HTMLElement_appendChild(vtbl, vhead);

    HTMLElement vbody = make_el(doc, "tbody", 0, 0);
    Date good_iso = parseISO("2024-03-15");
    Date bad_iso = parseISO("not-a-date");
    bool ok_good = isValid(js_value(good_iso)) && isMatch("2024-03-15", "yyyy-MM-dd");
    bool ok_bad = isValid(js_value(bad_iso)) && isMatch("not-a-date", "yyyy-MM-dd");
    add_val_row(doc, vbody, "\"2024-03-15\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", ok_good, ok_good ? "VALID" : "INVALID");
    add_val_row(doc, vbody, "\"not-a-date\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", ok_bad, ok_bad ? "VALID" : "INVALID");
    bool ok_2024 = isExists(2024.0, 1.0, 29.0);
    bool ok_2023 = isExists(2023.0, 1.0, 29.0);
    add_val_row(doc, vbody, "(2024, 1, 29)", "isExists(2024, 1, 29) leap Feb 29", ok_2024, ok_2024 ? "EXISTS" : "INVALID");
    add_val_row(doc, vbody, "(2023, 1, 29)", "isExists(2023, 1, 29) non-leap Feb 29", ok_2023, ok_2023 ? "EXISTS" : "INVALID");
    HTMLElement_appendChild(vtbl, vbody);
    HTMLElement_appendChild(val_card, vtbl);
    HTMLElement_appendChild(app, val_card);

    js_scope_close();
}
