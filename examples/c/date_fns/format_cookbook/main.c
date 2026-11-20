#include "dom/dom.h"
#include "date_fns/date_fns.h"

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag, (ElementCreationOptions){ 0 });
    if (cls && cls[0]) Element_set_className(el, cls);
    if (text) Node_set_textContent(el, text);
    return el;
}

static void append_and_release(HTMLElement parent, HTMLElement child) {
    js_release(Node_appendChild(parent, child));
    js_release(child);
}

static void add_fmt_row(Document doc, HTMLElement tbody, const char* api, JsString out, const char* note) {
    static char buf[256];
    JsString_copy(out, buf, sizeof(buf));
    js_release(out);

    HTMLElement tr = make_el(doc, "tr", 0, 0);
    HTMLElement td_api = make_el(doc, "td", 0, 0);
    append_and_release(td_api, make_el(doc, "code", 0, api));
    append_and_release(tr, td_api);
    append_and_release(tr, make_el(doc, "td", 0, buf));
    append_and_release(tr, make_el(doc, "td", 0, note));
    append_and_release(tbody, tr);
}

static void add_val_row(Document doc, HTMLElement tbody, const char* input, const char* check, bool ok, const char* pill_text) {
    HTMLElement tr = make_el(doc, "tr", 0, 0);
    HTMLElement td_in = make_el(doc, "td", 0, 0);
    append_and_release(td_in, make_el(doc, "code", 0, input));
    append_and_release(tr, td_in);
    append_and_release(tr, make_el(doc, "td", 0, check));
    HTMLElement td_res = make_el(doc, "td", 0, 0);
    append_and_release(td_res, make_el(doc, "span", ok ? "pill" : "pill-muted", pill_text));
    append_and_release(tr, td_res);
    append_and_release(tbody, tr);
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "Date-fns Format & Parse Cookbook (C)"));
    append_and_release(head, make_el(doc, "span", "badge", "2024-03-15T14:30:45"));
    append_and_release(app, head);

    Date sample = parseISO("2024-03-15T14:30:45");
    Date parsed = parse("15/03/2024", "dd/MM/yyyy", DateArg_of_Date(sample), (ParseOptions){ 0 });

    HTMLElement fmt_card = make_el(doc, "div", "card", 0);
    append_and_release(fmt_card, make_el(doc, "div", "label", "Format Tokens & ISO Output"));

    HTMLElement tbl = make_el(doc, "table", 0, 0);
    HTMLElement thead = make_el(doc, "thead", 0, 0);
    HTMLElement hrow = make_el(doc, "tr", 0, 0);
    append_and_release(hrow, make_el(doc, "th", 0, "API / Pattern"));
    append_and_release(hrow, make_el(doc, "th", 0, "Output"));
    append_and_release(hrow, make_el(doc, "th", 0, "Notes"));
    append_and_release(thead, hrow);
    append_and_release(tbl, thead);

    HTMLElement tbody = make_el(doc, "tbody", 0, 0);
    FormatDateOptions fd0 = (FormatDateOptions){ 0 };
    add_fmt_row(doc, tbody, "formatDate(\"yyyy-MM-dd\")", formatDate_with_Date(sample, "yyyy-MM-dd", fd0), "ISO date");
    add_fmt_row(doc, tbody, "formatDate(\"EEEE, do MMMM yyyy\")", formatDate_with_Date(sample, "EEEE, do MMMM yyyy", fd0), "Ordinal full date");
    add_fmt_row(doc, tbody, "formatDate(\"EEE, d MMM yy\")", formatDate_with_Date(sample, "EEE, d MMM yy", fd0), "Compact short date");
    add_fmt_row(doc, tbody, "formatDate(\"HH:mm:ss\")", formatDate_with_Date(sample, "HH:mm:ss", fd0), "24-hour clock");
    add_fmt_row(doc, tbody, "formatDate(\"h:mm a\")", formatDate_with_Date(sample, "h:mm a", fd0), "12-hour clock");
    add_fmt_row(doc, tbody, "formatDate(\"'Q'Q \xC2\xB7 'W'I\")", formatDate_with_Date(sample, "'Q'Q \xC2\xB7 'W'I", fd0), "Quarter & ISO week");
    add_fmt_row(doc, tbody, "formatDate(\"DDD 'of' yyyy\")", formatDate_with_Date(sample, "DDD 'of' yyyy", fd0), "Day of year");
    add_fmt_row(doc, tbody, "lightFormat(\"yyyy/MM/dd HH:mm\")", lightFormat_with_Date(sample, "yyyy/MM/dd HH:mm"), "Zero-locale light formatter");
    add_fmt_row(doc, tbody, "formatISO(sample)", formatISO_with_Date(sample, (FormatISOOptions){ 0 }), "ISO 8601 timestamp");
    add_fmt_row(doc, tbody, "parse(\"15/03/2024\", \"dd/MM/yyyy\")", formatDate_with_Date(parsed, "yyyy-MM-dd", fd0), "Custom parse roundtrip");
    append_and_release(tbl, tbody);
    append_and_release(fmt_card, tbl);
    append_and_release(app, fmt_card);

    HTMLElement val_card = make_el(doc, "div", "card", 0);
    append_and_release(val_card, make_el(doc, "div", "label", "Validation & Calendar Existence"));

    HTMLElement vtbl = make_el(doc, "table", 0, 0);
    HTMLElement vhead = make_el(doc, "thead", 0, 0);
    HTMLElement vhrow = make_el(doc, "tr", 0, 0);
    append_and_release(vhrow, make_el(doc, "th", 0, "Input"));
    append_and_release(vhrow, make_el(doc, "th", 0, "Check"));
    append_and_release(vhrow, make_el(doc, "th", 0, "Result"));
    append_and_release(vhead, vhrow);
    append_and_release(vtbl, vhead);

    HTMLElement vbody = make_el(doc, "tbody", 0, 0);
    Date good_iso = parseISO("2024-03-15");
    Date bad_iso = parseISO("not-a-date");
    bool ok_good = isValid((JsValue){ good_iso.h }) && isMatch("2024-03-15", "yyyy-MM-dd");
    bool ok_bad = isValid((JsValue){ bad_iso.h }) && isMatch("not-a-date", "yyyy-MM-dd");
    add_val_row(doc, vbody, "\"2024-03-15\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", ok_good, ok_good ? "VALID" : "INVALID");
    add_val_row(doc, vbody, "\"not-a-date\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", ok_bad, ok_bad ? "VALID" : "INVALID");
    bool ok_2024 = isExists(2024.0, 1.0, 29.0);
    bool ok_2023 = isExists(2023.0, 1.0, 29.0);
    add_val_row(doc, vbody, "(2024, 1, 29)", "isExists(2024, 1, 29) leap Feb 29", ok_2024, ok_2024 ? "EXISTS" : "INVALID");
    add_val_row(doc, vbody, "(2023, 1, 29)", "isExists(2023, 1, 29) non-leap Feb 29", ok_2023, ok_2023 ? "EXISTS" : "INVALID");
    append_and_release(vtbl, vbody);
    append_and_release(val_card, vtbl);
    append_and_release(app, val_card);

    js_release(good_iso);
    js_release(bad_iso);
    js_release(parsed);
    js_release(sample);
    js_release(app);
    js_release(doc);
}
