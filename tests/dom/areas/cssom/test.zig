// The CSS object model from Zig: inline styles, computed styles, a style
// sheet's rules and media queries. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

// The rule at an index, checked via CSSStyleRule.cast.
fn ruleAt(sheet: b.CSSStyleSheet, i: f64) b.CSSStyleRule {
    return b.CSSStyleRule.cast(sheet.getCssRules().item(i).?).?;
}
fn selectorAt(sheet: b.CSSStyleSheet, i: f64) b.JsString {
    return ruleAt(sheet, i).getSelectorText();
}

export fn run() void {
    const doc = b.getDocument();
    const body = doc.getBody();

    // The element the styles apply to.
    const box = doc.createElement(.div, .{}).asHTMLElement();
    box.setId("box");
    box.setTextContent("styled");
    _ = body.appendChild(box.asNode());

    // Inline style: set, get, priority, remove.
    const style = box.getStyle();
    expect(style.getLength() == 0, @src());
    style.setProperty("color", "red", .{});
    style.setProperty("margin-left", "12px", .{});
    expect(style.getPropertyValue("color").eql("red"), @src());
    expect(style.getColor().eql("red"), @src());
    expect(style.getLength() == 2, @src());
    expect(style.item(0).eql("color"), @src());
    expect(box.getAttribute("style").?.eql("color: red; margin-left: 12px;"), @src());
    expect(style.getCssText().eql("color: red; margin-left: 12px;"), @src());
    style.setProperty("width", "50px", .{ .priority = "important" });
    expect(style.getPropertyPriority("width").eql("important"), @src());
    expect(style.getPropertyPriority("color").len() == 0, @src());
    expect(style.removeProperty("margin-left").eql("12px"), @src());
    expect(style.getPropertyValue("margin-left").len() == 0 and style.getLength() == 2, @src());
    style.setDisplay("block");
    style.setProperty("color", null, .{});
    expect(style.getPropertyValue("color").len() == 0, @src());
    style.setCssText("background-color: blue; height: 20px");
    expect(style.getPropertyValue("background-color").eql("blue"), @src());
    expect(style.getPropertyValue("height").eql("20px") and style.getLength() == 2, @src());

    // Computed style resolves what was set: colors become rgb(), lengths pixels.
    style.setProperty("color", "rgb(0, 128, 0)", .{});
    style.setProperty("width", "100px", .{});
    const computed = b.getComputedStyle(box.asElement(), null);
    expect(computed.getPropertyValue("color").eql("rgb(0, 128, 0)"), @src());
    expect(computed.getPropertyValue("width").eql("100px"), @src());
    expect(computed.getPropertyValue("height").eql("20px"), @src());
    expect(computed.getPropertyValue("background-color").eql("rgb(0, 0, 255)"), @src());
    expect(computed.getDisplay().eql("block"), @src());
    expect(computed.getPropertyValue("margin-left").eql("0px"), @src());
    expect(box.getBoundingClientRect().getWidth() == 100, @src());

    // A style sheet made from a <style> element.
    const sheet_el = doc.createElement(.style, .{});
    sheet_el.asHTMLElement().setTextContent(".big { font-size: 30px; } #target { color: red; width: 5px; }");
    _ = doc.getHead().asNode().appendChild(sheet_el.asHTMLElement().asNode());
    const sheet = sheet_el.getSheet().?;
    const rules = sheet.getCssRules();
    expect(rules.getLength() == 2, @src());
    const first = rules.item(0).?;
    expect(first.getCssText().eql(".big { font-size: 30px; }"), @src());
    const big = b.CSSStyleRule{ .h = first.h };
    expect(big.getSelectorText().eql(".big"), @src());
    expect(big.getStyle().getPropertyValue("font-size").eql("30px"), @src());
    expect(b.jsSame(first.getParentStyleSheet().?, sheet), @src());
    const second = b.CSSStyleRule{ .h = rules.item(1).?.h };
    expect(second.getSelectorText().eql("#target"), @src());
    expect(second.getStyle().getLength() == 2, @src());

    // Rules inserted and deleted.
    const at = sheet.insertRule(".small { font-size: 8px; }", .{ .index = 0 });
    expect(at == 0 and sheet.getCssRules().getLength() == 3, @src());
    expect(selectorAt(sheet, 0).eql(".small"), @src());
    expect(sheet.insertRule(".last { color: blue; }", .{ .index = 3 }) == 3, @src());
    sheet.deleteRule(3);
    expect(sheet.getCssRules().getLength() == 3, @src());
    sheet.deleteRule(0);
    expect(sheet.getCssRules().getLength() == 2, @src());
    expect(selectorAt(sheet, 0).eql(".big"), @src());

    // A rule that does not parse throws.
    expect(sheet.insertRule("{{ nope", .{}) == 0, @src());
    const bad = b.jsTakeError();
    expect(bad != null, @src());
    if (bad) |e| expect(b.jsErrorName(e).eql("SyntaxError"), @src());
    expect(sheet.getCssRules().getLength() == 2, @src());

    // Rules written to change what a rule does show up in the computed style.
    second.getStyle().setProperty("color", "blue", .{});
    expect(second.getStyle().getPropertyValue("color").eql("blue"), @src());
    second.setSelectorText("#box");
    expect(second.getSelectorText().eql("#box"), @src());

    // A class drives the computed style through the sheet.
    const probe = doc.createElement(.p, .{}).asHTMLElement();
    probe.setTextContent("probe");
    _ = body.appendChild(probe.asNode());
    const before = b.getComputedStyle(probe.asElement(), null).getPropertyValue("font-size");
    expect(before.eql("16px"), @src());
    const classes = b.JsArrayOf(b.JsString).new();
    classes.pushString("big");
    probe.getClassList().add(classes);
    expect(b.getComputedStyle(probe.asElement(), null).getPropertyValue("font-size").eql("30px"), @src());
    probe.getClassList().remove(classes);
    expect(b.getComputedStyle(probe.asElement(), null).getPropertyValue("font-size").eql("16px"), @src());
    // The id rule, rewritten to #box, now applies to the box and its blue wins over the inline-free color.
    _ = style.removeProperty("color");
    expect(b.getComputedStyle(box.asElement(), null).getPropertyValue("color").eql("rgb(0, 0, 255)"), @src());

    // A sheet made in script and filled at once.
    const own = b.CSSStyleSheet.new(null);
    own.replaceSync("a { color: red } b { color: blue } @media print { i { color: green } }");
    expect(own.getCssRules().getLength() == 3, @src());
    expect(own.getCssRules().item(2).?.getCssText().len() > 0, @src());

    // Media queries.
    const wide = b.matchMedia("(min-width: 1px)");
    expect(wide.getMatches() and wide.getMedia().eql("(min-width: 1px)"), @src());
    expect(!b.matchMedia("(max-width: 1px)").getMatches(), @src());
    expect(b.matchMedia("screen").getMatches() and !b.matchMedia("print").getMatches(), @src());
    expect(b.matchMedia("(min-width: 1px) and (max-width: 100000px)").getMatches(), @src());
}
