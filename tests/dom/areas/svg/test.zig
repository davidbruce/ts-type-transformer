// SVG from Zig: elements made in the SVG namespace, attributes, boxes, lengths,
// the view box and path geometry. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn near(a: f64, want: f64) bool {
    return @abs(a - want) < 0.01;
}

export fn run() void {
    const doc = b.getDocument();

    // An <svg> in the page, sized in pixels with a view box twice its size.
    const svg = doc.createElementNSSvg(.svg);
    svg.asElement().setAttribute("width", "200");
    svg.asElement().setAttribute("height", "100");
    svg.asElement().setAttribute("viewBox", "0 0 400 200");
    _ = doc.getBody().asNode().appendChild(svg.asNode());
    expect(svg.asElement().getNamespaceURI().?.eql("http://www.w3.org/2000/svg"), @src());
    expect(svg.asElement().getTagName().eql("svg"), @src());
    expect(svg.asNode().getIsConnected(), @src());
    expect(svg.asElement().getBoundingClientRect().getWidth() == 200, @src());

    // The view box and the lengths are read as typed values.
    const view = svg.getViewBox().getBaseVal();
    expect(view.getX() == 0 and view.getY() == 0 and view.getWidth() == 400 and view.getHeight() == 200, @src());
    const width = svg.getWidth().getBaseVal();
    expect(width.getValue() == 200 and width.getValueInSpecifiedUnits() == 200, @src());
    expect(width.getUnitType() == 1, @src());
    expect(width.getValueAsString().eql("200"), @src());
    width.setValue(300);
    expect(svg.asElement().getAttribute("width").?.eql("300"), @src());
    expect(svg.asElement().getBoundingClientRect().getWidth() == 300, @src());
    width.setValueAsString("50%");
    expect(width.getUnitType() == 2 and width.getValueInSpecifiedUnits() == 50, @src());
    svg.asElement().setAttribute("width", "200");
    expect(svg.getWidth().getBaseVal().getValue() == 200 and svg.getWidth().getAnimVal().getValue() == 200, @src());
    svg.getHeight().getBaseVal().setValueInSpecifiedUnits(100);
    expect(svg.getHeight().getBaseVal().getValue() == 100, @src());

    // A rectangle, with its attributes read back as lengths.
    const rect = doc.createElementNSSvg(.rect);
    rect.asElement().setAttribute("x", "10");
    rect.asElement().setAttribute("y", "20");
    rect.asElement().setAttribute("width", "80");
    rect.asElement().setAttribute("height", "30");
    rect.asElement().setAttribute("fill", "red");
    _ = svg.asNode().appendChild(rect.asNode());
    expect(rect.getX().getBaseVal().getValue() == 10 and rect.getY().getBaseVal().getValue() == 20, @src());
    expect(rect.getWidth().getBaseVal().getValue() == 80 and rect.getHeight().getBaseVal().getValue() == 30, @src());
    const box = rect.getBBox(null);
    expect(box.getX() == 10 and box.getY() == 20 and box.getWidth() == 80 and box.getHeight() == 30, @src());
    rect.getWidth().getBaseVal().setValue(100);
    expect(rect.asElement().getAttribute("width").?.eql("100"), @src());
    expect(rect.getBBox(null).getWidth() == 100, @src());
    // Its screen box is scaled by the view box: 400 user units over 200 pixels.
    const screen = rect.asElement().getBoundingClientRect();
    expect(near(screen.getWidth(), 50) and near(screen.getHeight(), 15), @src());

    // Style reaches SVG elements: the fill attribute and the CSS property agree.
    expect(b.getComputedStyle(rect.asElement(), null).getPropertyValue("fill").eql("rgb(255, 0, 0)"), @src());
    rect.asSVGElement().getStyle().setProperty("fill", "blue", .{});
    expect(b.getComputedStyle(rect.asElement(), null).getPropertyValue("fill").eql("rgb(0, 0, 255)"), @src());

    // A point is in or out of the fill.
    const inside = b.DOMPointInit.create();
    inside.setX(20);
    inside.setY(30);
    const outside = b.DOMPointInit.create();
    outside.setX(5);
    outside.setY(5);
    expect(rect.isPointInFill(inside) and !rect.isPointInFill(outside), @src());

    // A circle: its box, and its length around.
    const circle = doc.createElementNSSvg(.circle);
    circle.asElement().setAttribute("cx", "100");
    circle.asElement().setAttribute("cy", "50");
    circle.asElement().setAttribute("r", "25");
    _ = svg.asNode().appendChild(circle.asNode());
    expect(circle.getCx().getBaseVal().getValue() == 100 and circle.getR().getBaseVal().getValue() == 25, @src());
    const cbox = circle.getBBox(null);
    expect(cbox.getX() == 75 and cbox.getY() == 25 and cbox.getWidth() == 50 and cbox.getHeight() == 50, @src());
    // Chrome measures a circle as a sum of curves, a little under 2 pi r; it is within 2 units.
    expect(@abs(circle.getTotalLength() - 2 * std.math.pi * 25) < 2.0, @src());

    // A path: lengths along it, and points on it.
    const path = doc.createElementNSSvg(.path);
    path.asElement().setAttribute("d", "M 0 0 H 100 V 100");
    path.asElement().setAttribute("fill", "none");
    _ = svg.asNode().appendChild(path.asNode());
    expect(path.getTotalLength() == 200, @src());
    const mid = path.getPointAtLength(50);
    expect(mid.getX() == 50 and mid.getY() == 0, @src());
    const turn = path.getPointAtLength(150);
    expect(turn.getX() == 100 and turn.getY() == 50, @src());
    path.asElement().setAttribute("d", "M 0 0 L 30 40");
    expect(near(path.getTotalLength(), 50), @src());
    const pbox = path.getBBox(null);
    expect(pbox.getWidth() == 30 and pbox.getHeight() == 40, @src());

    // A line, made in the namespace by name instead of by key.
    const line = doc.createElementNSSvgWithString("line");
    line.asElement().setAttribute("x1", "0");
    line.asElement().setAttribute("y1", "0");
    line.asElement().setAttribute("x2", "6");
    line.asElement().setAttribute("y2", "8");
    _ = svg.asNode().appendChild(line.asNode());
    const as_line = b.SVGLineElement.cast(line).?;
    expect(b.SVGCircleElement.cast(line) == null, @src());
    expect(near(as_line.getTotalLength(), 10), @src());
    expect(line.asElement().getTagName().eql("line"), @src());

    // A group transforms what it holds; the matrix includes the view box scale.
    const group = doc.createElementNSSvg(.g);
    group.asElement().setAttribute("transform", "translate(10 20)");
    _ = svg.asNode().appendChild(group.asNode());
    const inner = doc.createElementNSSvg(.rect);
    inner.asElement().setAttribute("width", "10");
    inner.asElement().setAttribute("height", "10");
    _ = group.asNode().appendChild(inner.asNode());
    const ctm = group.getCTM().?;
    expect(near(ctm.getA(), 0.5) and near(ctm.getD(), 0.5), @src());
    // The group's own transform is part of its matrix: translate(10 20) scaled by half.
    expect(near(ctm.getE(), 5) and near(ctm.getF(), 10), @src());
    const inner_ctm = inner.getCTM().?;
    expect(near(inner_ctm.getE(), 5) and near(inner_ctm.getF(), 10), @src());
    expect(group.getBBox(null).getWidth() == 10, @src());
    expect(near(inner.asElement().getBoundingClientRect().getWidth(), 5), @src());

    // Text: counted and measured.
    const label = doc.createElementNSSvg(.text);
    label.asElement().setAttribute("x", "5");
    label.asElement().setAttribute("y", "90");
    label.asNode().setTextContent("Hello");
    _ = svg.asNode().appendChild(label.asNode());
    expect(label.getNumberOfChars() == 5, @src());
    expect(label.getComputedTextLength() > 10, @src());
    expect(label.getBBox(null).getWidth() > 10, @src());

    // The tree: what was appended is there, in order, and removing takes it out.
    expect(svg.asElement().getChildElementCount() == 6, @src());
    expect(b.jsSame(svg.asElement().getFirstElementChild().?, rect), @src());
    expect(b.jsSame(inner.asNode().getParentNode().?, group), @src());
    _ = svg.asNode().removeChild(circle.asNode());
    expect(svg.asElement().getChildElementCount() == 5 and !circle.asNode().getIsConnected(), @src());
    expect(svg.asElement().querySelectorElement("path[fill=none]") != null, @src());

    // The same element made by the general call, with the namespace as text.
    const generic = doc.createElementNS("http://www.w3.org/2000/svg", "ellipse", .{});
    expect(generic.getNamespaceURI().?.eql("http://www.w3.org/2000/svg") and generic.getLocalName().eql("ellipse"), @src());
    // An element made without the namespace is not an SVG element.
    const wrong = doc.createElementNSXhtml("rect");
    expect(wrong.asElement().getNamespaceURI().?.eql("http://www.w3.org/1999/xhtml"), @src());
}
