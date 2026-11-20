#include "dom/dom.hpp"
#include "date_fns/date_fns.hpp"

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

TS_EXPORT(start) void start() {
    auto doc = dom::get_document();
    auto app = *doc.getElementById("app");

    auto head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "Date-fns Format & Parse Cookbook (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "2024-03-15T14:30:45"));
    app.appendChild(head);

    auto sample = date_fns::parseISO("2024-03-15T14:30:45");
    auto parsed = date_fns::parse("15/03/2024", "dd/MM/yyyy", sample);

    dom::HTMLElement fmt_card = make_el(doc, "div", "card", "");
    fmt_card.appendChild(make_el(doc, "div", "label", "Format Tokens & ISO Output"));

    dom::HTMLElement tbl = make_el(doc, "table", "", "");
    dom::HTMLElement thead = make_el(doc, "thead", "", "");
    dom::HTMLElement hrow = make_el(doc, "tr", "", "");
    for (const char* h : { "API / Pattern", "Output", "Notes" }) {
        hrow.appendChild(make_el(doc, "th", "", h));
    }
    thead.appendChild(hrow);
    tbl.appendChild(thead);

    struct FmtRow { const char* api; std::string out; const char* note; };
    FmtRow rows[] = {
        { "formatDate(\"yyyy-MM-dd\")", date_fns::formatDate(sample, "yyyy-MM-dd"), "ISO date" },
        { "formatDate(\"EEEE, do MMMM yyyy\")", date_fns::formatDate(sample, "EEEE, do MMMM yyyy"), "Ordinal full date" },
        { "formatDate(\"EEE, d MMM yy\")", date_fns::formatDate(sample, "EEE, d MMM yy"), "Compact short date" },
        { "formatDate(\"HH:mm:ss\")", date_fns::formatDate(sample, "HH:mm:ss"), "24-hour clock" },
        { "formatDate(\"h:mm a\")", date_fns::formatDate(sample, "h:mm a"), "12-hour clock" },
        { "formatDate(\"'Q'Q \xC2\xB7 'W'I\")", date_fns::formatDate(sample, "'Q'Q \xC2\xB7 'W'I"), "Quarter & ISO week" },
        { "formatDate(\"DDD 'of' yyyy\")", date_fns::formatDate(sample, "DDD 'of' yyyy"), "Day of year" },
        { "lightFormat(\"yyyy/MM/dd HH:mm\")", date_fns::lightFormat(sample, "yyyy/MM/dd HH:mm"), "Zero-locale light formatter" },
        { "formatISO(sample)", date_fns::formatISO(sample), "ISO 8601 timestamp" },
        { "parse(\"15/03/2024\", \"dd/MM/yyyy\")", date_fns::formatDate(parsed, "yyyy-MM-dd"), "Custom parse roundtrip" },
    };

    dom::HTMLElement tbody = make_el(doc, "tbody", "", "");
    for (const auto& r : rows) {
        dom::HTMLElement tr = make_el(doc, "tr", "", "");
        dom::HTMLElement td_api = make_el(doc, "td", "", "");
        td_api.appendChild(make_el(doc, "code", "", r.api));
        tr.appendChild(td_api);
        tr.appendChild(make_el(doc, "td", "", r.out));
        tr.appendChild(make_el(doc, "td", "", r.note));
        tbody.appendChild(tr);
    }
    tbl.appendChild(tbody);
    fmt_card.appendChild(tbl);
    app.appendChild(fmt_card);

    dom::HTMLElement val_card = make_el(doc, "div", "card", "");
    val_card.appendChild(make_el(doc, "div", "label", "Validation & Calendar Existence"));

    dom::HTMLElement vtbl = make_el(doc, "table", "", "");
    dom::HTMLElement vhead = make_el(doc, "thead", "", "");
    dom::HTMLElement vhrow = make_el(doc, "tr", "", "");
    for (const char* h : { "Input", "Check", "Result" }) {
        vhrow.appendChild(make_el(doc, "th", "", h));
    }
    vhead.appendChild(vhrow);
    vtbl.appendChild(vhead);

    struct ValRow { const char* input; const char* check; bool ok; const char* pill; };
    bool ok_good = date_fns::isValid(date_fns::parseISO("2024-03-15")) && date_fns::isMatch("2024-03-15", "yyyy-MM-dd");
    bool ok_bad = date_fns::isValid(date_fns::parseISO("not-a-date")) && date_fns::isMatch("not-a-date", "yyyy-MM-dd");
    bool ok_2024 = date_fns::isExists(2024, 1, 29);
    bool ok_2023 = date_fns::isExists(2023, 1, 29);
    ValRow vrows[] = {
        { "\"2024-03-15\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", ok_good, ok_good ? "VALID" : "INVALID" },
        { "\"not-a-date\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", ok_bad, ok_bad ? "VALID" : "INVALID" },
        { "(2024, 1, 29)", "isExists(2024, 1, 29) leap Feb 29", ok_2024, ok_2024 ? "EXISTS" : "INVALID" },
        { "(2023, 1, 29)", "isExists(2023, 1, 29) non-leap Feb 29", ok_2023, ok_2023 ? "EXISTS" : "INVALID" },
    };

    dom::HTMLElement vbody = make_el(doc, "tbody", "", "");
    for (const auto& r : vrows) {
        dom::HTMLElement tr = make_el(doc, "tr", "", "");
        dom::HTMLElement td_in = make_el(doc, "td", "", "");
        td_in.appendChild(make_el(doc, "code", "", r.input));
        tr.appendChild(td_in);
        tr.appendChild(make_el(doc, "td", "", r.check));
        dom::HTMLElement td_res = make_el(doc, "td", "", "");
        td_res.appendChild(make_el(doc, "span", r.ok ? "pill" : "pill-muted", r.pill));
        tr.appendChild(td_res);
        vbody.appendChild(tr);
    }
    vtbl.appendChild(vbody);
    val_card.appendChild(vtbl);
    app.appendChild(val_card);
}
