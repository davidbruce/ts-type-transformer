// The CSS object model from C: inline styles, computed styles, a style sheet
// edited rule by rule, media queries and classes that restyle. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)
#define STR(s, text) JsString_equals((s), (text))

// FINDING 2: getComputedStyle(elt, pseudoElt) has no default for its optional second argument
// (setProperty, getContext and others do), so every call passes 0.
#define COMPUTED(el, prop) CSSStyleDeclarationBase_getPropertyValue(getComputedStyle((el), 0), (prop))

__attribute__((export_name("run"))) void run(void) {
    Document doc = get_document();
    HTMLElement body = Document_get_body(doc);

    HTMLDivElement box = Document_createElement_div(doc);
    Element_set_id(box, "box");
    Node_set_textContent(box, "styled");
    Node_appendChild(body, box);

    // Inline style: set, get and remove properties.
    CSSStyleDeclaration style = ElementCSSInlineStyle_get_style(box);
    CHECK(CSSStyleDeclarationBase_get_length(style) == 0);
    CSSStyleDeclarationBase_setProperty(style, "color", "red");
    CSSStyleDeclarationBase_setProperty(style, "margin-top", "4px");
    CHECK(STR(CSSStyleDeclarationBase_getPropertyValue(style, "color"), "red"));
    CHECK(STR(CSSStyleProperties_get_color(style), "red"));
    CHECK(STR(CSSStyleProperties_get_marginTop(style), "4px"));
    CHECK(CSSStyleDeclarationBase_get_length(style) == 2);
    CHECK(STR(CSSStyleDeclarationBase_item(style, 0), "color"));
    CSSStyleProperties_set_backgroundColor(style, "rgb(0, 128, 0)");
    CHECK(STR(CSSStyleProperties_get_backgroundColor(style), "rgb(0, 128, 0)"));
    CHECK(STR(Element_getAttribute(box, "style"), "color: red; margin-top: 4px; background-color: rgb(0, 128, 0);"));
    CHECK(STR(CSSStyleDeclarationBase_removeProperty(style, "margin-top"), "4px"));
    CHECK(JsString_len(CSSStyleDeclarationBase_getPropertyValue(style, "margin-top")) == 0);
    CHECK(CSSStyleDeclarationBase_get_length(style) == 2);
    CSSStyleDeclarationBase_setProperty(style, "width", "50px", "important");
    CHECK(STR(CSSStyleDeclarationBase_getPropertyPriority(style, "width"), "important"));
    CHECK(JsString_len(CSSStyleDeclarationBase_getPropertyPriority(style, "color")) == 0);
    CSSStyleDeclarationBase_setProperty(style, "--accent", "teal");
    CHECK(STR(CSSStyleDeclarationBase_getPropertyValue(style, "--accent"), "teal"));
    CSSStyleDeclarationBase_set_cssText(style, "color: blue");
    CHECK(CSSStyleDeclarationBase_get_length(style) == 1 && STR(CSSStyleProperties_get_color(style), "blue"));
    CHECK(STR(CSSStyleDeclarationBase_get_cssText(style), "color: blue;"));
    // A value the engine rejects leaves the property as it was.
    CSSStyleDeclarationBase_setProperty(style, "color", "not-a-color");
    CHECK(STR(CSSStyleProperties_get_color(style), "blue"));

    // Computed style resolves what is set: colours come back as rgb().
    CHECK(STR(COMPUTED(box, "color"), "rgb(0, 0, 255)"));
    CSSStyleProperties_set_display(style, "block");
    CSSStyleProperties_set_width(style, "120px");
    CHECK(STR(COMPUTED(box, "width"), "120px"));
    CHECK(STR(COMPUTED(box, "display"), "block"));
    CHECK(STR(CSSStyleProperties_get_width(getComputedStyle(box, 0)), "120px"));
    CSSStyleDeclarationBase_removeProperty(style, "color");
    CHECK(STR(COMPUTED(box, "color"), "rgb(0, 0, 0)"));
    CHECK(DOMRect_get_width(Element_getBoundingClientRect(box)) == 120);

    // A <style> element's sheet: rules read, inserted and deleted.
    HTMLStyleElement sheet_el = Document_createElement_style(doc);
    Node_set_textContent(sheet_el, ".themed { color: rgb(10, 20, 30); }\n.big { width: 200px; }");
    Node_appendChild(Document_get_head(doc), sheet_el);
    CSSStyleSheet sheet = LinkStyle_get_sheet(sheet_el);
    CHECK(sheet.h != 0);
    CSSRuleList rules = CSSStyleSheet_get_cssRules(sheet);
    CHECK(CSSRuleList_get_length(rules) == 2);
    CSSRule first = CSSRuleList_item(rules, 0);
    CHECK(CSSRule_get_type(first) == 1);
    CSSStyleRule themed = { first.h };
    CHECK(STR(CSSStyleRule_get_selectorText(themed), ".themed"));
    CHECK(STR(CSSStyleDeclarationBase_getPropertyValue(CSSStyleRule_get_style(themed), "color"), "rgb(10, 20, 30)"));
    CHECK(js_same(CSSRule_get_parentStyleSheet(first), sheet));
    CHECK(STR(CSSRule_get_cssText(CSSRuleList_item(rules, 1)), ".big { width: 200px; }"));
    CHECK(CSSStyleSheet_insertRule(sheet, ".extra { font-weight: bold; }", js_some_double(0)) == 0);
    CHECK(CSSRuleList_get_length(CSSStyleSheet_get_cssRules(sheet)) == 3);
    CHECK(STR(CSSStyleRule_get_selectorText((CSSStyleRule){ CSSRuleList_item(CSSStyleSheet_get_cssRules(sheet), 0).h }), ".extra"));
    CHECK(CSSStyleSheet_insertRule(sheet, ".tail { top: 0; }", js_some_double(3)) == 3);
    CSSStyleSheet_deleteRule(sheet, 0);
    CSSStyleSheet_deleteRule(sheet, 2);
    CHECK(CSSRuleList_get_length(CSSStyleSheet_get_cssRules(sheet)) == 2);
    CHECK(STR(CSSStyleRule_get_selectorText((CSSStyleRule){ CSSRuleList_item(CSSStyleSheet_get_cssRules(sheet), 0).h }), ".themed"));
    // Walk the rules by iteration.
    JsIterator it = CSSRuleList_iterate(CSSStyleSheet_get_cssRules(sheet));
    CSSRule rule;
    int seen = 0;
    while (CSSRuleList_iterate_next(it, &rule)) seen++;
    CHECK(seen == 2);
    // A rule that does not parse throws and leaves the sheet as it was.
    CHECK(CSSStyleSheet_insertRule(sheet, "{{ nope") == 0 && js_failed());
    JsValue bad = js_take_error();
    CHECK(bad.h != 0 && STR(js_error_name(bad), "SyntaxError"));
    CHECK(CSSRuleList_get_length(CSSStyleSheet_get_cssRules(sheet)) == 2);

    // Classes drive computed style through the sheet's rules.
    HTMLDivElement target = Document_createElement_div(doc);
    Node_set_textContent(target, "target");
    Node_appendChild(body, target);
    CHECK(STR(COMPUTED(target, "color"), "rgb(0, 0, 0)"));
    DOMTokenList classes = Element_get_classList(target);
    DOMTokenList_toggle(classes, "themed", js_none_bool());
    CHECK(STR(COMPUTED(target, "color"), "rgb(10, 20, 30)"));
    DOMTokenList_toggle(classes, "big", js_none_bool());
    CHECK(STR(COMPUTED(target, "width"), "200px"));
    DOMTokenList_toggle(classes, "themed", js_none_bool());
    CHECK(STR(COMPUTED(target, "color"), "rgb(0, 0, 0)"));
    // Editing the rule restyles the page; deleting it restores the default.
    CSSStyleDeclarationBase_setProperty(CSSStyleRule_get_style((CSSStyleRule){ CSSRuleList_item(CSSStyleSheet_get_cssRules(sheet), 1).h }), "width", "80px");
    CHECK(STR(COMPUTED(target, "width"), "80px"));
    CSSStyleRule_set_selectorText((CSSStyleRule){ CSSRuleList_item(CSSStyleSheet_get_cssRules(sheet), 1).h }, ".large");
    CHECK(!STR(COMPUTED(target, "width"), "80px"));
    DOMTokenList_replace(classes, "big", "large");
    CHECK(STR(COMPUTED(target, "width"), "80px"));
    CSSStyleSheet_deleteRule(sheet, 1);
    CHECK(!STR(COMPUTED(target, "width"), "80px"));
    // Custom properties inherit through computed style.
    CSSStyleDeclarationBase_setProperty(ElementCSSInlineStyle_get_style(body), "--gap", "7px");
    CHECK(STR(COMPUTED(target, "--gap"), "7px"));
    // Hidden elements compute to display none and have no box.
    CSSStyleProperties_set_display(ElementCSSInlineStyle_get_style(target), "none");
    CHECK(STR(COMPUTED(target, "display"), "none"));
    CHECK(DOMRect_get_width(Element_getBoundingClientRect(target)) == 0);

    // Media queries.
    MediaQueryList wide = matchMedia("(min-width: 1px)");
    CHECK(MediaQueryList_get_matches(wide));
    CHECK(STR(MediaQueryList_get_media(wide), "(min-width: 1px)"));
    CHECK(!MediaQueryList_get_matches(matchMedia("(max-width: 0px)")));
    CHECK(MediaQueryList_get_matches(matchMedia("not print")));
}
