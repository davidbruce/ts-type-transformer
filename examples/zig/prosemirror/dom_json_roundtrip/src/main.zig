const std = @import("std");
const dom = @import("dom");
const pm = @import("prosemirror");
const model = pm.prosemirror_model;
const state_mod = pm.prosemirror_state;
const view_mod = pm.prosemirror_view;

fn holeSpec(tag: []const u8) model.DOMOutputSpec {
    const a = pm.JsArray.new();
    a.push(pm.JsValue.string(tag));
    a.push(pm.JsValue.number(0));
    return .{ .array = a };
}

fn headingToDOM(_: ?*anyopaque, n: model.Node) model.DOMOutputSpec {
    const lvl: i32 = @intFromFloat(n.getAttrs().getAt("level").toNumber());
    return holeSpec(if (lvl == 1) "h1" else "h2");
}
fn pToDOM(_: ?*anyopaque, _: model.Node) model.DOMOutputSpec {
    return holeSpec("p");
}
fn bqToDOM(_: ?*anyopaque, _: model.Node) model.DOMOutputSpec {
    return holeSpec("blockquote");
}
fn strongToDOM(_: ?*anyopaque, _: model.Mark, _: bool) model.DOMOutputSpec {
    return holeSpec("strong");
}
fn emToDOM(_: ?*anyopaque, _: model.Mark, _: bool) model.DOMOutputSpec {
    return holeSpec("em");
}
fn codeToDOM(_: ?*anyopaque, _: model.Mark, _: bool) model.DOMOutputSpec {
    return holeSpec("code");
}

fn makeTagRule(tag: []const u8, level: f64) model.TagParseRule {
    const r = model.TagParseRule.create();
    r.setTag(tag);
    if (level > 0) {
        const a = model.Attrs.create();
        a.setAt("level", pm.JsValue.number(level));
        r.asGenericParseRule().setAttrs(a);
    }
    return r;
}

fn makeSchema() model.Schema {
    const doc = model.NodeSpec.create();
    doc.setContent("block+");

    const lvl = model.AttributeSpec.create();
    lvl.setDefault(pm.JsValue.number(2));
    const hattrs = pm.MarkSpec_attrs.create();
    hattrs.setAt("level", lvl);

    const heading = model.NodeSpec.create();
    heading.setAttrs(hattrs);
    heading.setContent("inline*");
    heading.setGroup("block");
    heading.setDefining(true);
    heading.setParseDOM(pm.JsArrayOf(model.TagParseRule).of(&.{ makeTagRule("h1", 1), makeTagRule("h2", 2) }));
    heading.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(headingToDOM, null));

    const para = model.NodeSpec.create();
    para.setContent("inline*");
    para.setGroup("block");
    para.setParseDOM(pm.JsArrayOf(model.TagParseRule).of(&.{makeTagRule("p", 0)}));
    para.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(pToDOM, null));

    const quote = model.NodeSpec.create();
    quote.setContent("block+");
    quote.setGroup("block");
    quote.setDefining(true);
    quote.setParseDOM(pm.JsArrayOf(model.TagParseRule).of(&.{makeTagRule("blockquote", 0)}));
    quote.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(bqToDOM, null));

    const text = model.NodeSpec.create();
    text.setGroup("inline");

    const nodes = model.NodeSpec.create();
    nodes.setAt("doc", .{ .h = doc.h });
    nodes.setAt("heading", .{ .h = heading.h });
    nodes.setAt("paragraph", .{ .h = para.h });
    nodes.setAt("blockquote", .{ .h = quote.h });
    nodes.setAt("text", .{ .h = text.h });

    const ms = model.MarkSpec.create();
    ms.setParseDOM(pm.JsArrayOf(model.ParseRule).of(&.{.{ .TagParseRule = makeTagRule("strong", 0) }}));
    ms.setToDOM(pm.Mark_boolean_to_DOMOutputSpec_fn.from(strongToDOM, null));
    const me = model.MarkSpec.create();
    me.setParseDOM(pm.JsArrayOf(model.ParseRule).of(&.{.{ .TagParseRule = makeTagRule("em", 0) }}));
    me.setToDOM(pm.Mark_boolean_to_DOMOutputSpec_fn.from(emToDOM, null));
    const mc = model.MarkSpec.create();
    mc.setParseDOM(pm.JsArrayOf(model.ParseRule).of(&.{.{ .TagParseRule = makeTagRule("code", 0) }}));
    mc.setToDOM(pm.Mark_boolean_to_DOMOutputSpec_fn.from(codeToDOM, null));

    const marks = model.MarkSpec.create();
    marks.setAt("strong", .{ .h = ms.h });
    marks.setAt("em", .{ .h = me.h });
    marks.setAt("code", .{ .h = mc.h });

    const spec = model.SchemaSpec.create();
    spec.setNodes(.{ .h = nodes.h });
    spec.setMarks(.{ .h = marks.h });
    return model.Schema.new(spec);
}

const SPEC_HTML =
    "<h2>DOMParser &amp; JSON Serialization</h2>" ++
    "<p>ProseMirror converts <strong>HTML</strong> via <code>DOMParser</code> and renders back through <em>DOMSerializer</em>.</p>" ++
    "<blockquote><p>Schema parseDOM rules keep DOM and JSON representations aligned.</p></blockquote>" ++
    "<p>Trigger any action above to inspect live DOMSerializer HTML and Node.eq() round-trips.</p>";

const CHANGELOG_HTML =
    "<h2>Release 1.4 Notes</h2>" ++
    "<p>Added <strong>typed bindings</strong> for <code>DOMParser</code> and <em>DOMSerializer</em>.</p>" ++
    "<blockquote><p>All round-trips verified with Node.eq().</p></blockquote>" ++
    "<p>Edit directly above to watch HTML output update live.</p>";

var g_schema: model.Schema = undefined;
var g_parser: model.DOMParser = undefined;
var g_serializer: model.DOMSerializer = undefined;
var g_view: view_mod.EditorView = undefined;
var g_doc: dom.Document = undefined;
var g_output: dom.HTMLElement = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_status: []const u8 = "JSON Node.eq() pending";

fn parseHtml(html: []const u8) model.Node {
    const div = g_doc.createElementWithString("div", .{});
    div.asElement().setInnerHTML(html);
    return g_parser.parse(.{ .h = div.h }, .{});
}

fn refreshOutput(doc: model.Node) void {
    const div = g_doc.createElementWithString("div", .{});
    _ = g_serializer.serializeFragment(doc.getContent(), .{ .target = .{ .HTMLElement = .{ .h = div.h } } });
    var hbuf: [4096]u8 = undefined;
    const html_out = div.asElement().getInnerHTML().read(&hbuf);

    var obuf: [8192]u8 = undefined;
    var w = std.Io.Writer.fixed(&obuf);
    const nb: usize = @intFromFloat(doc.getChildCount());
    const nsz: usize = @intFromFloat(doc.getNodeSize());
    w.print("SERIALIZED HTML (DOMSerializer):\n{s}\n\nDOCUMENT AST & JSON ROUND-TRIP:\ndoc(blocks={d}, nodeSize={d})\n", .{
        html_out,
        nb,
        nsz,
    }) catch {};
    for (0..nb) |i| {
        const b = doc.child(@floatFromInt(i));
        var kbuf: [64]u8 = undefined;
        const kname = b.getType().getName().read(&kbuf);
        var tbuf: [512]u8 = undefined;
        const txt = b.getTextContent().read(&tbuf);
        const bsz: usize = @intFromFloat(b.getNodeSize());
        if (std.mem.eql(u8, kname, "heading")) {
            const lvl: usize = @intFromFloat(b.getAttrs().getAt("level").toNumber());
            w.print("  [{d}] heading[level={d}] ({d} bytes): {s}\n", .{ i, lvl, bsz, txt }) catch {};
        } else {
            w.print("  [{d}] {s} ({d} bytes): {s}\n", .{ i, kname, bsz, txt }) catch {};
        }
    }
    w.print("\nROUND-TRIP STATUS: {s}", .{g_status}) catch {};
    g_output.asNode().setTextContent(w.buffered());

    var mbuf: [256]u8 = undefined;
    const m = std.fmt.bufPrint(&mbuf, "{s} · blocks={d} · nodeSize={d} · htmlBytes={d}", .{
        g_status,
        nb,
        nsz,
        html_out.len,
    }) catch g_status;
    g_meta.asNode().setTextContent(m);
}

fn onDispatch(_: ?*anyopaque, tr: state_mod.Transaction) void {
    if (g_view.h == 0) return;
    const next = g_view.getState().apply(tr);
    g_view.updateState(next);
    refreshOutput(next.getDoc());
}

const BTN_IDS = [5]usize{ 0, 1, 2, 3, 4 };
fn onClick(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const idx = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    const st = g_view.getState();
    const doc = st.getDoc();
    if (idx == 0 or idx == 1) {
        g_status = if (idx == 0) "Parsed Spec HTML via DOMParser" else "Parsed Changelog HTML via DOMParser";
        const cfg = state_mod.EditorStateConfig.create();
        cfg.setSchema(g_schema);
        cfg.setDoc(parseHtml(if (idx == 0) SPEC_HTML else CHANGELOG_HTML));
        const next = state_mod.EditorState.create(cfg);
        g_view.updateState(next);
        refreshOutput(next.getDoc());
    } else if (idx == 2) {
        const rebuilt = model.Node.fromJSON(g_schema, doc.toJSON());
        g_status = if (doc.eq(rebuilt)) "JSON toJSON->fromJSON eq() = true" else "JSON toJSON->fromJSON eq() = false";
        refreshOutput(doc);
    } else if (idx == 3) {
        if (doc.getChildCount() >= 2.0) {
            const insert_pos = doc.child(0).getNodeSize() + doc.child(1).getNodeSize() - 1.0;
            const mc = g_schema.mark(.{ .string = pm.JsString.from("code") }, null);
            const span = g_schema.text(" roundtrip()", pm.JsArrayOf(model.Mark).of(&.{mc}));
            const tr = st.getTr();
            _ = tr.asTransform().insert(insert_pos, .{ .Node = span });
            g_status = "Inserted code span into paragraph";
            pm.callTransactionFn(g_view.getDispatch(), tr);
        }
    } else {
        const div = g_doc.createElementWithString("div", .{});
        _ = g_serializer.serializeFragment(doc.getContent(), .{ .target = .{ .HTMLElement = .{ .h = div.h } } });
        const reparsed = g_parser.parse(.{ .h = div.h }, .{});
        g_status = if (doc.eq(reparsed)) "DOM serializeFragment->parse eq() = true" else "DOM serializeFragment->parse eq() = false";
        const cfg = state_mod.EditorStateConfig.create();
        cfg.setSchema(g_schema);
        cfg.setDoc(reparsed);
        const next = state_mod.EditorState.create(cfg);
        g_view.updateState(next);
        refreshOutput(next.getDoc());
    }
}

export fn start() void {
    g_doc = dom.getDocument();
    _ = dom.jsRetain(g_doc);
    g_output = g_doc.getElementById("output").?;
    _ = dom.jsRetain(g_output);
    g_meta = g_doc.getElementById("meta").?;
    _ = dom.jsRetain(g_meta);

    g_schema = makeSchema();
    _ = pm.jsRetain(g_schema);
    g_parser = model.DOMParser.fromSchema(g_schema);
    _ = pm.jsRetain(g_parser);
    g_serializer = model.DOMSerializer.fromSchema(g_schema);
    _ = pm.jsRetain(g_serializer);

    const scfg = state_mod.EditorStateConfig.create();
    scfg.setSchema(g_schema);
    scfg.setDoc(parseHtml(SPEC_HTML));

    const props = view_mod.DirectEditorProps.create();
    props.setState(state_mod.EditorState.create(scfg));
    props.setDispatchTransaction(pm.Transaction_fn.from(onDispatch, null));

    const mount = g_doc.getElementById("editor").?;
    g_view = view_mod.EditorView.new(.{ .Node_2 = mount.asNode() }, props);
    _ = pm.jsRetain(g_view);
    g_status = "Parsed Spec HTML via DOMParser";
    refreshOutput(g_view.getState().getDoc());

    const ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (ids, 0..) |id, i| {
        const btn = g_doc.getElementById(id).?;
        btn.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onClick, @constCast(&BTN_IDS[i])), .{});
    }
}
