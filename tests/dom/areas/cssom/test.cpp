// CSS from C++: inline styles, computed styles, a style sheet's rules and
// media queries. Every CHECK must hold.
#include <string>
#include <utility>

#include "areas/core/Document.hpp"
#include "areas/css/CSSMediaRule.hpp"
#include "areas/css/CSSStyleRule.hpp"
#include "areas/css/CSSStyleSheet.hpp"
#include "areas/core/functions.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// FINDING 4: a CSSRuleList holds CSSRule, and there is no cast to CSSStyleRule (or
// from Node to a derived element, Event to CustomEvent...). The handle is
// copied (a retain) and the copy's reference handed to the derived type.
template <class D, class B>
static D downcast(const B& b) {
    B copy = b;
    return D(ts::handle, std::exchange(copy.h, 0));
}

extern "C" __attribute__((export_name("run"))) void run() {
    Document doc = get_document();
    HTMLElement body = doc.body();

    // Inline style: set, get, remove.
    HTMLDivElement box = doc.createElement(HTMLElementTagNameMapKeys::div);
    box.id("css-box");
    box.textContent("styled");
    body.appendChild(box);
    CSSStyleDeclaration style = box.style();
    CHECK(style.length() == 0 && style.cssText() == "");
    style.setProperty("color", "red");
    style.setProperty("width", "120px");
    style.setProperty("height", "40px");
    CHECK(style.getPropertyValue("color") == "red");
    CHECK(style.color() == "red" && style.width() == "120px");
    CHECK(style.length() == 3 && style.item(0) == "color");
    CHECK(box.getAttribute("style").value_or("").find("color: red") != std::string::npos);
    style.height("50px");
    CHECK(style.getPropertyValue("height") == "50px");
    CHECK(style.removeProperty("height") == "50px");
    CHECK(style.getPropertyValue("height") == "" && style.length() == 2);
    style.setProperty("margin-left", "3px", "important");
    CHECK(style.getPropertyPriority("margin-left") == "important");
    style.cssText("color: blue; display: block");
    CHECK(style.color() == "blue" && style.display() == "block" && style.getPropertyValue("width") == "");
    style.setProperty("width", "120px");

    // Computed style: resolved values.
    CSSStyleDeclaration computed = getComputedStyle(box);
    CHECK(computed.color() == "rgb(0, 0, 255)");
    CHECK(computed.width() == "120px");
    CHECK(computed.getPropertyValue("display") == "block");
    style.removeProperty("width");
    CHECK(computed.width() != "120px");

    // A style element's sheet: rules read and written.
    HTMLStyleElement sheetEl = doc.createElement(HTMLElementTagNameMapKeys::style);
    sheetEl.textContent(
        ".card { color: green; padding: 4px }\n"
        "#css-box.wide { width: 300px }\n"
        "@media (min-width: 1px) { .card { margin: 2px } }");
    doc.head().appendChild(sheetEl);
    CSSStyleSheet sheet = *sheetEl.sheet();
    CHECK(sheet.cssRules().length() == 3);
    CSSRule rule0 = *sheet.cssRules().item(0);
    CHECK(rule0.type() == CSSRule::STYLE_RULE);
    CSSStyleRule card = downcast<CSSStyleRule>(rule0);
    CHECK(card.selectorText() == ".card");
    CHECK(card.style().color() == "green" && card.style().getPropertyValue("padding") == "4px");
    CHECK(card.cssText().starts_with(".card {"));
    CHECK(card.parentStyleSheet() && *card.parentStyleSheet() == sheet);
    CSSRule rule2 = *sheet.cssRules().item(2);
    CHECK(rule2.type() == CSSRule::MEDIA_RULE);
    CHECK(downcast<CSSMediaRule>(rule2).cssRules().length() == 1);

    // Rules change the page's styles.
    card.style().setProperty("color", "rgb(1, 2, 3)");
    HTMLDivElement item = doc.createElement(HTMLElementTagNameMapKeys::div);
    item.className("card");
    body.appendChild(item);
    CHECK(getComputedStyle(item).color() == "rgb(1, 2, 3)");
    CHECK(getComputedStyle(item).paddingTop() == "4px");

    // insertRule and deleteRule.
    double at = sheet.insertRule(".extra { display: none }", 1);
    CHECK(at == 1 && sheet.cssRules().length() == 4);
    CHECK(downcast<CSSStyleRule>(*sheet.cssRules().item(1)).selectorText() == ".extra");
    HTMLDivElement hidden = doc.createElement(HTMLElementTagNameMapKeys::div);
    hidden.className("extra");
    body.appendChild(hidden);
    CHECK(getComputedStyle(hidden).display() == "none");
    sheet.deleteRule(1);
    CHECK(sheet.cssRules().length() == 3);
    CHECK(getComputedStyle(hidden).display() == "block");
    CHECK(sheet.insertRule("p { margin: 0 }") == 0);
    CHECK(downcast<CSSStyleRule>(*sheet.cssRules().item(0)).selectorText() == "p");
    sheet.deleteRule(0);

    // Changing a rule's selector.
    card.selectorText(".panel");
    CHECK(card.selectorText() == ".panel");
    CHECK(getComputedStyle(item).color() != "rgb(1, 2, 3)");

    // A class toggled changes the computed style.
    HTMLDivElement chip = doc.createElement(HTMLElementTagNameMapKeys::div);
    chip.id("css-chip");
    body.appendChild(chip);
    HTMLStyleElement more = doc.createElement(HTMLElementTagNameMapKeys::style);
    more.textContent("#css-chip { opacity: 1 } #css-chip.dim { opacity: 0.5 }");
    doc.head().appendChild(more);
    CHECK(getComputedStyle(chip).opacity() == "1");
    chip.classList().add({"dim"});
    CHECK(getComputedStyle(chip).opacity() == "0.5");
    chip.classList().toggle("dim");
    CHECK(getComputedStyle(chip).opacity() == "1");

    // A custom property inherits.
    box.style().setProperty("--accent", "purple");
    HTMLSpanElement child = doc.createElement(HTMLElementTagNameMapKeys::span);
    box.appendChild(child);
    child.style().setProperty("color", "var(--accent)");
    CHECK(getComputedStyle(child).getPropertyValue("--accent") == "purple");
    CHECK(getComputedStyle(child).color() == "rgb(128, 0, 128)");

    // Media queries.
    MediaQueryList wide = matchMedia("(min-width: 1px)");
    CHECK(wide.matches() && wide.media() == "(min-width: 1px)");
    CHECK(!matchMedia("(max-width: 0px)").matches());
    CHECK(matchMedia("screen").matches());
    CHECK(!matchMedia("print").matches());
    CHECK(matchMedia("(min-width: 1px)").matches() == wide.matches());
}
