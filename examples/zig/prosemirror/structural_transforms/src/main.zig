const std = @import("std");
const dom = @import("dom");
const pm = @import("prosemirror");
const model = pm.prosemirror_model;
const tr_mod = pm.prosemirror_transform;
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
fn ulToDOM(_: model.Node) model.DOMOutputSpec {
    return holeSpec("ul");
}
fn liToDOM(_: model.Node) model.DOMOutputSpec {
    return holeSpec("li");
}
fn hrToDOM(_: model.Node) model.DOMOutputSpec {
    return .{ .array = pm.JsArray.of(.{"hr"}) };
}
fn strongToDOM(_: model.Mark, _: bool) model.DOMOutputSpec {
    return holeSpec("strong");
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
    const para = model.NodeSpec.init(.{ .content = "inline*", .group = "block", .toDOM = pToDOM });
    const quote = model.NodeSpec.init(.{ .content = "block+", .group = "block", .defining = true, .toDOM = bqToDOM });
    const ulist = model.NodeSpec.init(.{ .content = "list_item+", .group = "block", .toDOM = ulToDOM });
    const li = model.NodeSpec.init(.{ .content = "paragraph block*", .defining = true, .toDOM = liToDOM });
    const hr = model.NodeSpec.init(.{ .group = "block", .atom = true, .toDOM = hrToDOM });
    const text = model.NodeSpec.init(.{ .group = "inline" });

    const ms = model.MarkSpec.init(.{ .toDOM = strongToDOM });
    const mc = model.MarkSpec.init(.{ .toDOM = codeToDOM });

    return model.Schema.new(.{
        .nodes = .{ .doc = doc, .heading = heading, .paragraph = para, .blockquote = quote, .bullet_list = ulist, .list_item = li, .horizontal_rule = hr, .text = text },
        .marks = .{ .strong = ms, .code = mc },
    });
}

const App = struct {
    schema: model.Schema = undefined,
    view: view_mod.EditorView = std.mem.zeroes(view_mod.EditorView),
    doc: dom.Document = undefined,
    meta: dom.HTMLElement = undefined,
    btns: [5]BtnCtx = undefined,
    total_steps: usize = 0,
    note_counter: usize = 0,
    last_op: []const u8 = "Initialized document structure",
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};

fn pNode(schema: model.Schema, t: []const u8) model.Node {
    return schema.node("paragraph", null, .{ .content = schema.text(t, null) });
}
fn liNode(schema: model.Schema, t: []const u8) model.Node {
    return schema.node("list_item", null, .{ .content = pNode(schema, t) });
}

fn makeInitialDoc(schema: model.Schema) model.Node {
    const h = schema.node("heading", .{ .level = 2 }, .{
        .content = schema.text("Structural Transform Pipeline", null),
    });
    const quote = schema.node("blockquote", null, .{
        .content = pNode(schema, "Blockquote content can be lifted back to top-level document depth."),
    });
    const ulist = schema.node("bullet_list", null, .{
        .content = &.{
            liNode(schema, "Split at cursor position"),
            liNode(schema, "Wrap range with blockquote"),
            liNode(schema, "Lift nested block to root"),
        },
    });
    return schema.node("doc", null, .{
        .content = &.{
            h,
            pNode(schema, "ProseMirror transforms express structural edits as invertible steps."),
            pNode(schema, "Wrap blocks in quotes, lift nested content, or split paragraphs cleanly."),
            quote,
            ulist,
        },
    });
}

fn makeState(schema: model.Schema) state_mod.EditorState {
    return state_mod.EditorState.create(.{
        .schema = schema,
        .doc = makeInitialDoc(schema),
    });
}

fn updateMeta(app: *App, doc: model.Node) void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    var obuf: [512]u8 = undefined;
    var w = std.Io.Writer.fixed(&obuf);
    const n: usize = @intFromFloat(doc.getChildCount());
    for (0..n) |i| {
        if (i > 0) w.writeAll(" · ") catch {};
        var nbuf: [48]u8 = undefined;
        w.writeAll(doc.child(@floatFromInt(i)).getType().getName().copy(&nbuf)) catch {};
    }
    var mbuf: [768]u8 = undefined;
    const msg = std.fmt.bufPrint(&mbuf, "{s} · blocks={d} · outline=[{s}] · totalSteps={d} · nodeSize={d}", .{
        app.last_op,
        n,
        w.buffered(),
        app.total_steps,
        @as(usize, @intFromFloat(doc.getNodeSize())),
    }) catch app.last_op;
    app.meta.setTextContent(msg);
}

fn onDispatch(app: *App, tr: state_mod.Transaction) void {
    if (app.view.h == 0) return;
    const scope = pm.JsScope.open();
    defer scope.deinit();
    app.total_steps += tr.getSteps().len();
    const next = app.view.getState().apply(tr);
    app.view.updateState(next);
    updateMeta(app, next.getDoc());
}

fn onClick(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    const app = ctx.app;
    const st = app.view.getState();
    const doc = st.getDoc();

    if (ctx.id == 0) {
        const sel_from = st.getSelection().getFrom();
        var split_pos: ?f64 = null;
        if (tr_mod.canSplit(doc, sel_from, null)) {
            split_pos = sel_from;
        } else {
            var pos: f64 = 0;
            const n: usize = @intFromFloat(doc.getChildCount());
            for (0..n) |i| {
                const ch = doc.child(@floatFromInt(i));
                if (split_pos == null and ch.getType().getName().eql("paragraph")) {
                    const len = ch.getContent().getSize();
                    if (len >= 8.0) {
                        const mid = pos + 1.0 + @as(f64, @floatFromInt(@divTrunc(@as(usize, @intFromFloat(len)), 2)));
                        if (tr_mod.canSplit(doc, mid, null)) split_pos = mid;
                    }
                }
                pos += ch.getNodeSize();
            }
        }
        if (split_pos) |sp| {
            const tr = st.getTr().split(sp, null);
            app.last_op = "Split paragraph node";
            app.view.getDispatch().call(tr);
        } else {
            app.last_op = "No splittable paragraph found";
            updateMeta(app, doc);
        }
    } else if (ctx.id == 1) {
        const qtype = app.schema.getNodes().getAt("blockquote").?;
        const sel = st.getSelection();
        var found: ?model.NodeRange = null;
        if (sel.get_From().getDepth() == 1.0 and sel.get_From().getParent().getType().getName().eql("paragraph")) {
            if (sel.get_From().blockRange(.{ .other = sel.get_To() })) |r| {
                if (tr_mod.findWrapping(r, qtype, null, null) != null) found = r;
            }
        }
        if (found == null) {
            var pos: f64 = 0;
            const n: usize = @intFromFloat(doc.getChildCount());
            for (0..n) |i| {
                const ch = doc.child(@floatFromInt(i));
                if (found == null and ch.getType().getName().eql("paragraph")) {
                    const rp = doc.resolve(pos + 1.0);
                    if (rp.blockRange(null)) |r| {
                        if (tr_mod.findWrapping(r, qtype, null, null) != null) found = r;
                    }
                }
                pos += ch.getNodeSize();
            }
        }
        if (found) |r| {
            if (tr_mod.findWrapping(r, qtype, null, null)) |wrapped| {
                const tr = st.getTr().wrap(r, wrapped);
                app.last_op = "Wrapped paragraph block in blockquote";
                app.view.getDispatch().call(tr);
                return;
            }
        }
        app.last_op = "No top-level paragraph left to wrap";
        updateMeta(app, doc);
    } else if (ctx.id == 2) {
        var pos: f64 = 0;
        const n: usize = @intFromFloat(doc.getChildCount());
        for (0..n) |i| {
            const ch = doc.child(@floatFromInt(i));
            if (ch.getType().getName().eql("blockquote") and ch.getChildCount() > 0.0) {
                const rp = doc.resolve(pos + 2.0);
                if (rp.blockRange(null)) |r| {
                    if (tr_mod.liftTarget(r)) |target| {
                        const tr = st.getTr().lift(r, target);
                        app.last_op = "Lifted blockquote child to root depth";
                        app.view.getDispatch().call(tr);
                        return;
                    }
                }
            }
            pos += ch.getNodeSize();
        }
        app.last_op = "No blockquote left to lift";
        updateMeta(app, doc);
    } else if (ctx.id == 3) {
        app.note_counter += 1;
        var nbuf: [128]u8 = undefined;
        const msg = std.fmt.bufPrint(&nbuf, "Appended structural section #{d} at doc offset {d}.", .{
            app.note_counter,
            @as(usize, @intFromFloat(doc.getContent().getSize())),
        }) catch "Appended section.";
        const hr = app.schema.node("horizontal_rule", null, null);
        const tr = st.getTr().insert(doc.getContent().getSize(), &.{ hr, pNode(app.schema, msg) });
        app.last_op = "Inserted horizontal_rule + note";
        app.view.getDispatch().call(tr);
    } else {
        app.total_steps = 0;
        app.note_counter = 0;
        app.last_op = "Reset structural sample";
        const fresh = makeState(app.schema);
        app.view.updateState(fresh);
        updateMeta(app, fresh.getDoc());
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
    updateMeta(&g_app, g_app.view.getState().getDoc());

    const ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (ids, 0..) |id, i| {
        g_app.btns[i] = .{ .app = &g_app, .id = i };
        g_app.doc.getElementById(id).?.addEventListener(.click, .{ onClick, &g_app.btns[i] }, null);
    }
}
