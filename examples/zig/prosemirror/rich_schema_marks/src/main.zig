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
fn paraToDOM(_: model.Node) model.DOMOutputSpec {
    return holeSpec("p");
}
fn quoteToDOM(_: model.Node) model.DOMOutputSpec {
    return holeSpec("blockquote");
}
fn codeBlockToDOM(_: model.Node) model.DOMOutputSpec {
    return .{ .array = pm.JsArray.of(.{ "pre", .{ "code", 0 } }) };
}
fn hrToDOM(_: model.Node) model.DOMOutputSpec {
    return .{ .array = pm.JsArray.of(.{"hr"}) };
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

fn makeSchema() model.Schema {
    const doc = model.NodeSpec.init(.{ .content = "block+" });
    const heading = model.NodeSpec.init(.{
        .attrs = .{ .level = model.AttributeSpec.init(.{ .default = 2 }) },
        .content = "inline*",
        .group = "block",
        .defining = true,
        .toDOM = headingToDOM,
    });
    const para = model.NodeSpec.init(.{ .content = "inline*", .group = "block", .toDOM = paraToDOM });
    const quote = model.NodeSpec.init(.{ .content = "block+", .group = "block", .defining = true, .toDOM = quoteToDOM });
    const cblock = model.NodeSpec.init(.{ .content = "text*", .marks = "", .group = "block", .code = true, .defining = true, .toDOM = codeBlockToDOM });
    const hr = model.NodeSpec.init(.{ .group = "block", .atom = true, .toDOM = hrToDOM });
    const text = model.NodeSpec.init(.{ .group = "inline" });

    const ms = model.MarkSpec.init(.{ .toDOM = strongToDOM });
    const me = model.MarkSpec.init(.{ .toDOM = emToDOM });
    const mc = model.MarkSpec.init(.{ .toDOM = codeToDOM });

    return model.Schema.new(.{
        .nodes = .{ .doc = doc, .heading = heading, .paragraph = para, .blockquote = quote, .code_block = cblock, .horizontal_rule = hr, .text = text },
        .marks = .{ .strong = ms, .em = me, .code = mc },
    });
}

const App = struct {
    schema: model.Schema = undefined,
    view: view_mod.EditorView = std.mem.zeroes(view_mod.EditorView),
    doc: dom.Document = undefined,
    meta: dom.HTMLElement = undefined,
    btns: [5]BtnCtx = undefined,
    dispatches: usize = 0,
    last_action: []const u8 = "Initialized schema with 7 node types & 3 marks",
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};

fn makeInitialDoc(schema: model.Schema) model.Node {
    const h = schema.node("heading", .{ .level = 2 }, .{ .content = schema.text("ProseMirror Document Schema", null) });

    const m_strong = schema.mark("strong", null);
    const m_em = schema.mark("em", null);
    const m_code = schema.mark("code", null);

    const p1 = schema.node("paragraph", null, .{
        .content = &.{
            schema.text("Inline marks compose cleanly: ", null),
            schema.text("bold", &.{m_strong}),
            schema.text(", ", null),
            schema.text("italic", &.{m_em}),
            schema.text(", and ", null),
            schema.text("inline_code", &.{m_code}),
            schema.text(" spans.", null),
        },
    });

    const q_inner = schema.node("paragraph", null, .{
        .content = schema.text("Every document conforms to a strict schema enforced on every transaction.", null),
    });
    const quote = schema.node("blockquote", null, .{ .content = q_inner });
    const cb = schema.node("code_block", null, .{
        .content = schema.text("schema.node(\"paragraph\", null, [schema.text(\"typed\")])", null),
    });
    const outro = schema.node("paragraph", null, .{
        .content = schema.text("Select any text in this editor and click a toolbar action to toggle marks or switch block types.", null),
    });

    return schema.node("doc", null, .{ .content = &.{ h, p1, quote, cb, outro } });
}

fn makeState(schema: model.Schema) state_mod.EditorState {
    return state_mod.EditorState.create(.{
        .schema = schema,
        .doc = makeInitialDoc(schema),
    });
}

fn targetRange(st: state_mod.EditorState) struct { f: f64, t: f64 } {
    const sel = st.getSelection();
    if (!sel.getEmpty()) return .{ .f = sel.getFrom(), .t = sel.getTo() };
    const doc = st.getDoc();
    if (doc.getChildCount() >= 2.0) {
        const c0 = doc.child(0);
        const c1 = doc.child(1);
        const start_pos = c0.getNodeSize() + 1.0;
        const len = c1.getContent().getSize();
        if (len > 0.0) {
            return .{ .f = start_pos, .t = start_pos + (if (len > 12.0) 12.0 else len) };
        }
    }
    const sz = doc.getContent().getSize();
    return .{ .f = 1.0, .t = if (sz > 2.0) 2.0 else sz };
}

fn updateMeta(app: *App, st: state_mod.EditorState) void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    const r = targetRange(st);
    const doc = st.getDoc();
    const marks = app.schema.getMarks();
    const hb = doc.rangeHasMark(r.f, r.t, marks.getAt("strong").?);
    const he = doc.rangeHasMark(r.f, r.t, marks.getAt("em").?);
    const hc = doc.rangeHasMark(r.f, r.t, marks.getAt("code").?);

    app.doc.getElementById("btn-0").?.setClassName(if (hb) "active" else "");
    app.doc.getElementById("btn-1").?.setClassName(if (he) "active" else "");
    app.doc.getElementById("btn-2").?.setClassName(if (hc) "active" else "");

    var bname_buf: [64]u8 = undefined;
    const bname = st.getSelection().get_From().getParent().getType().getName().copy(&bname_buf);

    var buf: [512]u8 = undefined;
    const msg = std.fmt.bufPrint(&buf, "{s} · blocks={d} · nodeSize={d} · range=[{d}..{d}] · block={s} · dispatches={d}", .{
        app.last_action,
        @as(usize, @intFromFloat(doc.getChildCount())),
        @as(usize, @intFromFloat(doc.getNodeSize())),
        @as(usize, @intFromFloat(r.f)),
        @as(usize, @intFromFloat(r.t)),
        bname,
        app.dispatches,
    }) catch app.last_action;
    app.meta.setTextContent(msg);
}

fn onDispatch(app: *App, tr: state_mod.Transaction) void {
    if (app.view.h == 0) return;
    const scope = pm.JsScope.open();
    defer scope.deinit();
    app.dispatches += 1;
    const next = app.view.getState().apply(tr);
    app.view.updateState(next);
    updateMeta(app, next);
}

fn toggleMark(app: *App, name: []const u8, added: []const u8, removed: []const u8) void {
    const st = app.view.getState();
    const r = targetRange(st);
    if (r.t <= r.f) return;
    const mt = app.schema.getMarks().getAt(name).?;
    const tr = st.getTr();
    if (st.getDoc().rangeHasMark(r.f, r.t, mt)) {
        _ = tr.removeMark(r.f, r.t, mt);
        app.last_action = removed;
    } else {
        _ = tr.addMark(r.f, r.t, app.schema.mark(mt, null));
        app.last_action = added;
    }
    app.view.dispatch(tr);
}

fn onClick(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    const app = ctx.app;
    if (ctx.id == 0) {
        toggleMark(app, "strong", "Added bold mark", "Removed bold mark");
    } else if (ctx.id == 1) {
        toggleMark(app, "em", "Added italic mark", "Removed italic mark");
    } else if (ctx.id == 2) {
        toggleMark(app, "code", "Added code mark", "Removed code mark");
    } else if (ctx.id == 3) {
        const st = app.view.getState();
        const sel = st.getSelection();
        const tr = st.getTr();
        const nodes = app.schema.getNodes();
        if (sel.get_From().getParent().getType().getName().eql("heading")) {
            _ = tr.setBlockType(sel.getFrom(), sel.getTo(), nodes.getAt("paragraph").?, null);
            app.last_action = "Converted heading -> paragraph";
        } else {
            _ = tr.setBlockType(sel.getFrom(), sel.getTo(), nodes.getAt("heading").?, .{ .level = 2 });
            app.last_action = "Converted block -> heading[level=2]";
        }
        app.view.dispatch(tr);
    } else {
        app.dispatches = 0;
        app.last_action = "Reset initial schema document";
        const fresh = makeState(app.schema);
        app.view.updateState(fresh);
        updateMeta(app, fresh);
    }
}

export fn start() void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    g_app.doc = dom.getDocument().retain();
    g_app.meta = g_app.doc.getElementById("meta").?.retain();
    g_app.schema = makeSchema().retain();

    const mount = g_app.doc.getElementById("editor").?;
    g_app.view = view_mod.EditorView.new(mount, .{
        .state = makeState(g_app.schema),
        .dispatchTransaction = .{ onDispatch, &g_app },
    }).retain();
    updateMeta(&g_app, g_app.view.getState());

    const ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (ids, 0..) |id, i| {
        g_app.btns[i] = .{ .app = &g_app, .id = i };
        g_app.doc.getElementById(id).?.addEventListener(.click, .{ onClick, &g_app.btns[i] }, null);
    }
}
