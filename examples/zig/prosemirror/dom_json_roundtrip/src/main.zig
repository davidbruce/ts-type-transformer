const std = @import("std");
const dom = @import("dom");
const pm = @import("prosemirror");
const model = pm.prosemirror_model;
const state_mod = pm.prosemirror_state;
const view_mod = pm.prosemirror_view;

fn holeSpec(tag: []const u8) model.DOMOutputSpec {
    return .{ .array = pm.JsArray.of(.{ tag, 0 }) };
}

fn headingToDOM(n: model.Node) model.DOMOutputSpec {
    const lvl: i32 = @intFromFloat(n.getAttrs().getAt("level").toNumber());
    return holeSpec(if (lvl == 1) "h1" else "h2");
}
fn pToDOM(_: model.Node) model.DOMOutputSpec {
    return holeSpec("p");
}
fn bqToDOM(_: model.Node) model.DOMOutputSpec {
    return holeSpec("blockquote");
}
fn strongToDOM(_: model.Mark, _: bool) model.DOMOutputSpec {
    return holeSpec("strong");
}
fn emToDOM(_: model.Mark, _: bool) model.DOMOutputSpec {
    return holeSpec("em");
}
fn codeToDOM(_: model.Mark, _: bool) model.DOMOutputSpec {
    return holeSpec("code");
}

fn makeTagRule(tag: []const u8, level: f64) model.TagParseRule {
    if (level > 0) {
        return model.TagParseRule.init(.{ .tag = tag, .attrs = .{ .level = level } });
    }
    return model.TagParseRule.init(.{ .tag = tag });
}

fn makeSchema() model.Schema {
    const doc = model.NodeSpec.init(.{ .content = "block+" });
    const heading = model.NodeSpec.init(.{
        .attrs = .{ .level = model.AttributeSpec.init(.{ .default = 2 }) },
        .content = "inline*",
        .group = "block",
        .defining = true,
        .parseDOM = &.{ makeTagRule("h1", 1), makeTagRule("h2", 2) },
        .toDOM = headingToDOM,
    });
    const para = model.NodeSpec.init(.{
        .content = "inline*",
        .group = "block",
        .parseDOM = &.{makeTagRule("p", 0)},
        .toDOM = pToDOM,
    });
    const quote = model.NodeSpec.init(.{
        .content = "block+",
        .group = "block",
        .defining = true,
        .parseDOM = &.{makeTagRule("blockquote", 0)},
        .toDOM = bqToDOM,
    });
    const text = model.NodeSpec.init(.{ .group = "inline" });

    const ms = model.MarkSpec.init(.{
        .parseDOM = &.{makeTagRule("strong", 0)},
        .toDOM = strongToDOM,
    });
    const me = model.MarkSpec.init(.{
        .parseDOM = &.{makeTagRule("em", 0)},
        .toDOM = emToDOM,
    });
    const mc = model.MarkSpec.init(.{
        .parseDOM = &.{makeTagRule("code", 0)},
        .toDOM = codeToDOM,
    });

    return model.Schema.new(.{
        .nodes = .{ .doc = doc, .heading = heading, .paragraph = para, .blockquote = quote, .text = text },
        .marks = .{ .strong = ms, .em = me, .code = mc },
    });
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

const App = struct {
    schema: model.Schema = undefined,
    parser: model.DOMParser = undefined,
    serializer: model.DOMSerializer = undefined,
    view: view_mod.EditorView = std.mem.zeroes(view_mod.EditorView),
    doc: dom.Document = undefined,
    output: dom.HTMLElement = undefined,
    meta: dom.HTMLElement = undefined,
    btns: [5]BtnCtx = undefined,
    status: []const u8 = "JSON Node.eq() pending",
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};

fn parseHtml(app: *App, html: []const u8) model.Node {
    const div = app.doc.createElementWithString("div");
    div.setInnerHTML(html);
    return app.parser.parse(div);
}

fn refreshOutput(app: *App, doc: model.Node) void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    const div = app.doc.createElementWithString("div");
    _ = app.serializer.serializeFragmentWith(doc.getContent(), .{ .target = div });
    var hbuf: [4096]u8 = undefined;
    const html_out = div.getInnerHTML().copy(&hbuf);

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
        const b = doc.child(i);
        var kbuf: [64]u8 = undefined;
        const kname = b.getType().getName().copy(&kbuf);
        var tbuf: [512]u8 = undefined;
        const txt = b.getTextContent().copy(&tbuf);
        const bsz: usize = @intFromFloat(b.getNodeSize());
        if (std.mem.eql(u8, kname, "heading")) {
            const lvl: usize = @intFromFloat(b.getAttrs().getAt("level").toNumber());
            w.print("  [{d}] heading[level={d}] ({d} bytes): {s}\n", .{ i, lvl, bsz, txt }) catch {};
        } else {
            w.print("  [{d}] {s} ({d} bytes): {s}\n", .{ i, kname, bsz, txt }) catch {};
        }
    }
    w.print("\nROUND-TRIP STATUS: {s}", .{app.status}) catch {};
    app.output.setTextContent(w.buffered());

    var mbuf: [256]u8 = undefined;
    const m = std.fmt.bufPrint(&mbuf, "{s} · blocks={d} · nodeSize={d} · htmlBytes={d}", .{
        app.status,
        nb,
        nsz,
        html_out.len,
    }) catch app.status;
    app.meta.setTextContent(m);
}

fn onDispatch(app: *App, tr: state_mod.Transaction) void {
    if (app.view.h == 0) return;
    const scope = pm.JsScope.open();
    defer scope.deinit();
    const next = app.view.getState().apply(tr);
    app.view.updateState(next);
    refreshOutput(app, next.getDoc());
}

fn onClick(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    const app = ctx.app;
    const st = app.view.getState();
    const doc = st.getDoc();
    if (ctx.id == 0 or ctx.id == 1) {
        app.status = if (ctx.id == 0) "Parsed Spec HTML via DOMParser" else "Parsed Changelog HTML via DOMParser";
        const next = state_mod.EditorState.create(.{
            .schema = app.schema,
            .doc = parseHtml(app, if (ctx.id == 0) SPEC_HTML else CHANGELOG_HTML),
        });
        app.view.updateState(next);
        refreshOutput(app, next.getDoc());
    } else if (ctx.id == 2) {
        const rebuilt = model.Node.fromJSON(app.schema, doc.toJSON());
        app.status = if (doc.eq(rebuilt)) "JSON toJSON->fromJSON eq() = true" else "JSON toJSON->fromJSON eq() = false";
        refreshOutput(app, doc);
    } else if (ctx.id == 3) {
        if (doc.getChildCount() >= 2.0) {
            const insert_pos = doc.child(0).getNodeSize() + doc.child(1).getNodeSize() - 1.0;
            const mc = app.schema.mark("code");
            const span = app.schema.textWith(" roundtrip()", &.{mc});
            const tr = st.getTr().insert(insert_pos, span);
            app.status = "Inserted code span into paragraph";
            app.view.dispatch(tr);
        }
    } else {
        const div = app.doc.createElementWithString("div");
        _ = app.serializer.serializeFragmentWith(doc.getContent(), .{ .target = div });
        const reparsed = app.parser.parse(div);
        app.status = if (doc.eq(reparsed)) "DOM serializeFragment->parse eq() = true" else "DOM serializeFragment->parse eq() = false";
        const next = state_mod.EditorState.create(.{ .schema = app.schema, .doc = reparsed });
        app.view.updateState(next);
        refreshOutput(app, next.getDoc());
    }
}

export fn start() void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    g_app.doc = dom.getDocument().retain();
    g_app.output = g_app.doc.getElementById("output").?.retain();
    g_app.meta = g_app.doc.getElementById("meta").?.retain();

    g_app.schema = makeSchema().retain();
    g_app.parser = model.DOMParser.fromSchema(g_app.schema).retain();
    g_app.serializer = model.DOMSerializer.fromSchema(g_app.schema).retain();

    const st = state_mod.EditorState.create(.{
        .schema = g_app.schema,
        .doc = parseHtml(&g_app, SPEC_HTML),
    });

    const mount = g_app.doc.getElementById("editor").?;
    g_app.view = view_mod.EditorView.new(mount, .{
        .state = st,
        .dispatchTransaction = .{ onDispatch, &g_app },
    }).retain();
    g_app.status = "Parsed Spec HTML via DOMParser";
    refreshOutput(&g_app, g_app.view.getState().getDoc());

    const ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (ids, 0..) |id, i| {
        g_app.btns[i] = .{ .app = &g_app, .id = i };
        g_app.doc.getElementById(id).?.addEventListener(.click, .{ onClick, &g_app.btns[i] });
    }
}
