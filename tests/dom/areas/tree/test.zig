// The DOM tree from Zig: nodes made, attached, moved, cloned, searched and
// walked. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn text(n: b.Node, want: []const u8) bool {
    return if (n.getTextContent()) |t| t.eql(want) else false;
}

export fn run() void {
    const doc = b.getDocument();
    const body = doc.getBody();

    // A list built from elements and text.
    const list = doc.createElement(.ul, .{}).asHTMLElement();
    list.setId("list");
    for ([_][]const u8{ "one", "two", "three" }) |word| {
        const item = doc.createElement(.li, .{}).asHTMLElement();
        _ = item.appendChild(doc.createTextNode(word).asNode());
        item.setAttribute("data-word", word);
        _ = list.appendChild(item.asNode());
    }
    _ = body.appendChild(list.asNode());
    expect(list.getChildElementCount() == 3, @src());
    expect(list.getTextContent().eql("onetwothree"), @src());
    expect(list.getFirstElementChild().?.getTextContent().eql("one"), @src());
    expect(list.getLastElementChild().?.getAttribute("data-word").?.eql("three"), @src());
    expect(list.getTagName().eql("UL"), @src());
    expect(list.getIsConnected(), @src());

    // Parents, siblings and containment.
    const two = list.getChildren().item(1).?;
    expect(jsSameOpt(two.getParentElement(), list), @src());
    expect(two.getPreviousElementSibling().?.getTextContent().eql("one"), @src());
    expect(two.getNextElementSibling().?.getTextContent().eql("three"), @src());
    expect(body.contains(two.asNode()), @src());
    expect(two.matchesWithString("li[data-word=two]"), @src());
    expect(!two.matchesWithString("p"), @src());
    expect(two.closestElement("ul").?.getId().eql("list"), @src());
    expect(two.closest(.ul).?.asHTMLElement().getId().eql("list"), @src());
    expect(two.getNodeName().eql("LI"), @src());
    expect(two.getNodeType() == 1, @src());
    expect(two.asNode().getFirstChild().?.asNode().getNodeType() == b.Node.TEXT_NODE, @src());

    // Moved: appending a node that is in the tree moves it.
    _ = list.appendChild(two.asNode());
    expect(list.getLastElementChild().?.getTextContent().eql("two"), @src());
    expect(list.getTextContent().eql("onethreetwo"), @src());

    // insertBefore puts a node ahead of another.
    const first = list.getFirstElementChild().?;
    const pre = doc.createElement(.li, .{}).asHTMLElement();
    pre.setTextContent("pre");
    _ = list.insertBefore(pre.asNode(), first.asNode());
    expect(list.getFirstElementChild().?.getTextContent().eql("pre"), @src());
    expect(first.getPreviousElementSibling().?.getTextContent().eql("pre"), @src());
    _ = list.removeChild(pre.asNode());
    expect(pre.getParentNode() == null, @src());

    // Removed and replaced.
    const zero = doc.createElement(.li, .{}).asHTMLElement();
    zero.setTextContent("zero");
    _ = list.replaceChild(zero.asNode(), first.asNode());
    expect(first.getParentNode() == null, @src());
    expect(list.getFirstElementChild().?.getTextContent().eql("zero"), @src());
    two.remove();
    expect(list.getChildElementCount() == 2, @src());
    expect(!two.getIsConnected(), @src());

    // A fragment's children move in at once; it is left empty.
    const fragment = doc.createDocumentFragment();
    for ([_][]const u8{ "a", "b" }) |word| {
        const item = doc.createElement(.li, .{}).asHTMLElement();
        item.setTextContent(word);
        _ = fragment.asNode().appendChild(item.asNode());
    }
    expect(fragment.getChildElementCount() == 2, @src());
    _ = list.appendChild(fragment.asNode());
    expect(list.getChildElementCount() == 4, @src());
    expect(fragment.getChildNodes().getLength() == 0, @src());

    // A deep clone copies the subtree; a shallow one does not.
    const deep = list.cloneNode(true);
    const shallow = list.cloneNode(null);
    expect(text(deep, "zerothreeab"), @src());
    expect(shallow.getChildNodes().getLength() == 0, @src());
    expect(deep.isEqualNode(list.asNode()) and !deep.isSameNode(list.asNode()), @src());
    expect(!deep.getIsConnected(), @src());

    // Markup parsed and written.
    const host = doc.createElement(.div, .{}).asHTMLElement();
    host.setInnerHTML("<p class=\"x\">hi <b>there</b></p>");
    expect(host.querySelectorElement("b").?.getTextContent().eql("there"), @src());
    expect(host.getInnerHTML().eql("<p class=\"x\">hi <b>there</b></p>"), @src());
    const p = host.getFirstElementChild().?;
    var buf: [64]u8 = undefined;
    expect(std.mem.startsWith(u8, p.getOuterHTML().copy(&buf), "<p class=\"x\">"), @src());
    expect(p.getClassList().contains("x"), @src());
    expect(p.getClassList().toggle("y", .{}), @src());
    expect(p.getAttribute("class").?.eql("x y"), @src());
    const gone = b.JsArrayOf(b.JsString).new();
    gone.pushString("x");
    p.getClassList().remove(gone);
    expect(p.getClassName().eql("y"), @src());
    expect(p.getClassList().getLength() == 1, @src());
    p.insertAdjacentHTML(.beforeend, "<i>!</i>");
    expect(host.getTextContent().eql("hi there!"), @src());

    // Positions compared: `zero` precedes `a`.
    const a = list.getChildren().item(2).?;
    const zero_el = list.getChildren().item(0).?;
    const position = zero_el.compareDocumentPosition(a.asNode());
    expect(@as(u32, @intFromFloat(position)) & b.Node.DOCUMENT_POSITION_FOLLOWING != 0, @src());
    expect(@as(u32, @intFromFloat(a.compareDocumentPosition(zero_el.asNode()))) & b.Node.DOCUMENT_POSITION_PRECEDING != 0, @src());
    expect(@as(u32, @intFromFloat(body.compareDocumentPosition(a.asNode()))) & b.Node.DOCUMENT_POSITION_CONTAINED_BY != 0, @src());

    // A walk over the list's elements.
    const walker = doc.createTreeWalker(list.asNode(), 1, null);
    var seen: [8][8]u8 = undefined;
    var lens: [8]usize = undefined;
    var n: usize = 0;
    while (walker.nextNode()) |node| {
        const t = node.getTextContent().?.copy(&seen[n]);
        lens[n] = t.len;
        n += 1;
    }
    expect(n == 4, @src());
    expect(std.mem.eql(u8, seen[0][0..lens[0]], "zero"), @src());
    expect(std.mem.eql(u8, seen[1][0..lens[1]], "three"), @src());
    expect(std.mem.eql(u8, seen[2][0..lens[2]], "a"), @src());
    expect(std.mem.eql(u8, seen[3][0..lens[3]], "b"), @src());
    expect(walker.previousNode() != null, @src());
}

fn jsSameOpt(x: anytype, y: anytype) bool {
    return if (x) |v| b.jsSame(v, y) else false;
}
