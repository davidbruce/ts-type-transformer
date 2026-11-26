// The CSS object model from Rust: inline style, computed style, a style
// sheet's rules, media queries and class-driven style. Every check! must hold.
#![no_main]
use bindings::*;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check {
    ($c:expr) => {
        unsafe { check_at($c as i32, line!()) }
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    let doc = document();
    let body = doc.body();

    // Inline style: set, read back, remove.
    let el: HTMLDivElement = doc.create_element(HTMLElementTagNameMap::DIV, None);
    el.set_id("styled");
    body.append_child(el.clone());
    let style = el.style();
    check!(style.length() == 0.0 && style.css_text().is_empty());
    style.set_property("color", "red");
    style.set_property("margin-left", "4px");
    check!(style.get_property_value("color") == "red");
    check!(style.color() == "red");
    check!(style.margin_left() == "4px" && style.get_property_value("margin-left") == "4px");
    check!(style.length() == 2.0);
    check!(el.get_attribute("style").as_deref() == Some("color: red; margin-left: 4px;"));
    // A typed property setter, and an important priority.
    style.set_width("120px");
    check!(style.width() == "120px");
    style.set_property_with("height", "50px", "important");
    check!(style.get_property_priority("height") == "important");
    check!(style.get_property_priority("width").is_empty());
    check!(style.css_text().contains("height: 50px !important"));
    let names: Vec<String> = style.iter().collect();
    check!(names == ["color", "margin-left", "width", "height"]);
    check!(style.item(0.0) == "color");
    // removeProperty returns what was removed.
    check!(style.remove_property("color") == "red");
    check!(style.color().is_empty() && style.length() == 3.0);
    check!(style.remove_property("color").is_empty());
    // A value the engine rejects leaves the property as it was.
    style.set_property("width", "not a width");
    check!(style.width() == "120px");
    style.set_css_text("padding: 2px; display: block");
    check!(style.padding() == "2px" && style.display() == "block" && style.width().is_empty());
    style.set_css_text("");

    // Computed style: resolved values, not the written ones.
    style.set_property("width", "10em");
    style.set_property("font-size", "10px");
    style.set_property("color", "rgb(1, 2, 3)");
    style.set_property("background-color", "#00ff00");
    let computed = get_computed_style(&el);
    check!(computed.width() == "100px");
    check!(computed.get_property_value("font-size") == "10px");
    check!(computed.color() == "rgb(1, 2, 3)");
    check!(computed.background_color() == "rgb(0, 255, 0)");
    check!(computed.display() == "block");
    check!(style.background_color() == "rgb(0, 255, 0)");
    // Computed style is live and read-only.
    style.set_property("display", "none");
    check!(computed.display() == "none");
    check!(js::attempt(|| computed.set_property("color", "red")).is_err());
    style.remove_property("display");
    check!(computed.display() == "block");

    // A style element's sheet.
    let sheet_el: HTMLStyleElement = doc.create_element(HTMLElementTagNameMap::STYLE, None);
    sheet_el.set_text_content(Some(
        ".box { width: 50px; height: 20px; color: blue }\n\
         .wide { width: 200px }\n\
         #styled.boxed { background-color: rgb(9, 8, 7) }\n\
         @media (min-width: 1px) { .box { margin-top: 3px } }",
    ));
    doc.head().append_child(sheet_el.clone());
    let sheet = sheet_el.sheet().unwrap();
    let rules = sheet.css_rules();
    check!(rules.length() == 4.0);
    // FINDING 4: a union (Element | ProcessingInstruction) has no ==, and a subtype (an
    // HTMLStyleElement) does not convert into it: only the exact member does (awkward).
    check!(matches!(sheet.owner_node(), Some(OwnerNode::Element(e)) if e == *sheet_el.as_ref()));
    let first_rule = rules.item(0.0).unwrap();
    let first = CSSStyleRule::cast(&first_rule).unwrap();
    check!(CSSMediaRule::cast(&first_rule).is_none());
    check!(first.selector_text() == ".box");
    check!(first.style().width() == "50px" && first.style().get_property_value("color") == "blue");
    check!(first.css_text().starts_with(".box {"));
    check!(first.parent_style_sheet() == Some(sheet.clone()));
    check!(first.type_() == 1.0);
    let media_rule = rules.item(3.0).unwrap();
    check!(media_rule.type_() == 4.0 && CSSStyleRule::cast(&media_rule).is_none() && CSSMediaRule::cast(&media_rule).is_some());
    let selectors: Vec<String> = rules.iter().take(3).map(|r| CSSStyleRule::cast(&r).unwrap().selector_text()).collect();
    check!(selectors == [".box", ".wide", "#styled.boxed"]);

    // A class makes the sheet apply.
    check!(computed.width() == "100px");
    style.remove_property("width");
    check!(computed.width() != "50px");
    el.class_list().add(["box"]);
    check!(computed.width() == "50px" && computed.height() == "20px" && computed.color() == "rgb(1, 2, 3)");
    check!(computed.margin_top() == "3px");
    style.remove_property("color");
    check!(computed.color() == "rgb(0, 0, 255)");
    el.class_list().replace("box", "wide");
    check!(computed.width() == "200px" && computed.height() != "20px");
    el.class_list().toggle("boxed");
    // Inline style outranks the sheet.
    check!(computed.background_color() == "rgb(0, 255, 0)");
    style.remove_property("background-color");
    check!(computed.background_color() == "rgb(9, 8, 7)");
    el.class_list().remove(["boxed", "wide"]);
    check!(computed.background_color() == "rgba(0, 0, 0, 0)");

    // insertRule and deleteRule.
    let at = sheet.insert_rule_with(".extra { width: 7px }", 1.0);
    check!(at == 1.0 && rules.length() == 5.0);
    check!(CSSStyleRule(rules.item(1.0).unwrap().0.clone()).selector_text() == ".extra");
    el.class_list().add(["extra"]);
    check!(computed.width() == "7px");
    let rule1 = CSSStyleRule(rules.item(1.0).unwrap().0.clone());
    rule1.style().set_width("9px");
    check!(computed.width() == "9px");
    rule1.set_selector_text(".extra2");
    check!(rule1.selector_text() == ".extra2" && computed.width() != "9px");
    sheet.delete_rule(1.0);
    check!(rules.length() == 4.0);
    check!(CSSStyleRule(rules.item(1.0).unwrap().0.clone()).selector_text() == ".wide");
    // A bad rule throws; the sheet is unchanged.
    check!(js::attempt(|| sheet.insert_rule("{ nonsense")).is_err());
    check!(rules.length() == 4.0);
    check!(js::attempt(|| sheet.delete_rule(99.0)).is_err());
    // Disabling the sheet's element turns it off.
    el.class_list().remove(["extra"]);
    el.class_list().add(["box"]);
    check!(computed.width() == "50px");
    sheet_el.set_disabled(true);
    check!(computed.width() != "50px");
    sheet_el.set_disabled(false);
    check!(computed.width() == "50px");

    // Media queries.
    let wide = match_media("(min-width: 1px)");
    check!(wide.matches() && wide.media() == "(min-width: 1px)");
    check!(!match_media("(max-width: 0px)").matches());
    check!(match_media("all").matches() && !match_media("not all").matches());
    let portrait_or_landscape = match_media("(orientation: portrait), (orientation: landscape)");
    check!(portrait_or_landscape.matches());
    check!(match_media("(min-width: 1px) and (max-width: 100000px)").matches());
}
