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
fn codeBlockToDOM(_: model.Node) model.DOMOutputSpec {
    return .{ .array = pm.JsArray.of(.{ "pre", .{ "code", 0 } }) };
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
        .toDOM = headingToDOM,
    });
    const para = model.NodeSpec.init(.{
        .content = "inline*",
        .group = "block",
        .toDOM = paraToDOM,
    });
    const cblock = model.NodeSpec.init(.{
        .content = "text*",
        .marks = "",
        .group = "block",
        .code = true,
        .toDOM = codeBlockToDOM,
    });
    const text = model.NodeSpec.init(.{ .group = "inline" });

    const ms = model.MarkSpec.init(.{ .toDOM = strongToDOM });
    const mc = model.MarkSpec.init(.{ .toDOM = codeToDOM });

    return model.Schema.new(.{
        .nodes = .{ .doc = doc, .heading = heading, .paragraph = para, .code_block = cblock, .text = text },
        .marks = .{ .strong = ms, .code = mc },
    });
}

const App = struct {
    schema: model.Schema = undefined,
    key: state_mod.PluginKey = undefined,
    view: view_mod.EditorView = std.mem.zeroes(view_mod.EditorView),
    doc: dom.Document = undefined,
    meta: dom.HTMLElement = undefined,
    btns: [5]BtnCtx = undefined,
    active_btn: usize = 0,
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};

fn pNode(schema: model.Schema, t: []const u8) model.Node {
    return schema.node("paragraph", null, .{ .content = schema.text(t, null) });
}

fn makeInitialDoc(schema: model.Schema) model.Node {
    const h = schema.node("heading", .{ .level = 2 }, .{
        .content = schema.text("Immutable State & View Decorations", null),
    });
    const cb = schema.node("code_block", null, .{
        .content = schema.text("const nextState = state.apply(transaction);\nview.updateState(nextState); // sync view", null),
    });
    return schema.node("doc", null, .{
        .content = &.{
            h,
            pNode(schema, "A ProseMirror schema validates every document tree before state updates commit to the view."),
            pNode(schema, "Each transaction maps positions forward so state plugins can recompute decorations without mutating the schema."),
            cb,
            pNode(schema, "Inline decorations highlight matching tokens while node decorations outline matching blocks in the view."),
        },
    });
}

fn findCi(hay: []const u8, needle: []const u8, from: usize) ?usize {
    if (needle.len == 0 or hay.len < needle.len or from > hay.len - needle.len) return null;
    for (from..(hay.len - needle.len + 1)) |i| {
        if (std.ascii.eqlIgnoreCase(hay[i .. i + needle.len], needle)) return i;
    }
    return null;
}

const DecoStats = struct { set: view_mod.DecorationSet, inline_hits: usize, block_hits: usize };

fn computeDecorations(doc: model.Node, query: []const u8) DecoStats {
    if (query.len == 0) {
        return .{ .set = view_mod.DecorationSet.getEmpty(), .inline_hits = 0, .block_hits = 0 };
    }
    const decos = pm.JsArrayOf(view_mod.Decoration).new();
    const inl_attr = view_mod.DecorationAttrs.init(.{ .class = "pm-match" });
    const blk_attr = view_mod.DecorationAttrs.init(.{ .class = "pm-hit-block" });

    var inl_hits: usize = 0;
    var blk_hits: usize = 0;
    var block_pos: f64 = 0;
    const nb: usize = @intFromFloat(doc.getChildCount());
    for (0..nb) |i| {
        const blk = doc.child(i);
        var inline_pos: f64 = block_pos + 1.0;
        var matched = false;
        const nc: usize = @intFromFloat(blk.getChildCount());
        for (0..nc) |j| {
            const ch = blk.child(j);
            if (ch.getIsText()) {
                var tbuf: [1024]u8 = undefined;
                const txt = ch.getTextContent().copy(&tbuf);
                var at: usize = 0;
                while (findCi(txt, query, at)) |pos_idx| {
                    const fp = inline_pos + @as(f64, @floatFromInt(pos_idx));
                    const tp = fp + @as(f64, @floatFromInt(query.len));
                    decos.push(view_mod.Decoration.@"inline"(fp, tp, inl_attr, null));
                    inl_hits += 1;
                    matched = true;
                    at = pos_idx + query.len;
                }
            }
            inline_pos += ch.getNodeSize();
        }
        if (matched) {
            decos.push(view_mod.Decoration.node(block_pos, block_pos + blk.getNodeSize(), blk_attr, null));
            blk_hits += 1;
        }
        block_pos += blk.getNodeSize();
    }
    return .{ .set = view_mod.DecorationSet.create(doc, decos), .inline_hits = inl_hits, .block_hits = blk_hits };
}

fn searchInit(_: state_mod.EditorStateConfig, _: state_mod.EditorState) pm.JsValue {
    return pm.JsValue.string("state");
}

fn searchApply(tr: state_mod.Transaction, prev: pm.JsValue, _: state_mod.EditorState, _: state_mod.EditorState) pm.JsValue {
    const mv = tr.getMeta("search");
    if (mv.h != 0) return mv;
    return prev.retain();
}

fn searchDecos(app: *App, st: state_mod.EditorState) ?view_mod.DecorationSource {
    const scope = pm.JsScope.open();
    defer scope.deinit();
    const qv = app.key.getState(st);
    var qbuf: [64]u8 = undefined;
    const q = if (qv.h != 0) qv.as(pm.JsString).copy(&qbuf) else "";
    return computeDecorations(st.getDoc(), q).set.retain().as(view_mod.DecorationSource);
}

fn syncMeta(app: *App, st: state_mod.EditorState) void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    const qv = app.key.getState(st);
    var qbuf: [64]u8 = undefined;
    const q = if (qv.h != 0) qv.as(pm.JsString).copy(&qbuf) else "";
    const stats = computeDecorations(st.getDoc(), q);
    const total = stats.set.find(null).len();

    var qdisp_buf: [80]u8 = undefined;
    const qdisp = if (q.len == 0) "(none)" else (std.fmt.bufPrint(&qdisp_buf, "\"{s}\"", .{q}) catch q);

    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(&buf, "Query: {s} · inline matches={d} · highlighted blocks={d} · decorations={d}", .{
        qdisp,
        stats.inline_hits,
        stats.block_hits,
        total,
    }) catch "Updated search";
    app.meta.setTextContent(msg);

    const ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (ids, 0..) |id, i| {
        app.doc.getElementById(id).?.setClassName(if (i == app.active_btn) "active" else "");
    }
}

fn onDispatch(app: *App, tr: state_mod.Transaction) void {
    if (app.view.h == 0) return;
    const scope = pm.JsScope.open();
    defer scope.deinit();
    const next = app.view.getState().apply(tr);
    app.view.updateState(next);
    syncMeta(app, next);
}

const QUERIES = [5][]const u8{ "state", "schema", "transaction", "view", "" };

fn onClick(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    const app = ctx.app;
    app.active_btn = ctx.id;
    const tr = app.view.getState().getTr().setMeta("search", QUERIES[ctx.id]);
    app.view.dispatch(tr);
}

export fn start() void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    g_app.doc = dom.getDocument().retain();
    g_app.meta = g_app.doc.getElementById("meta").?.retain();
    g_app.schema = makeSchema().retain();
    g_app.key = state_mod.PluginKey.new("search").retain();

    const pspec = state_mod.PluginSpec.init(.{
        .key = g_app.key,
        .state = state_mod.StateField.fromFields(.{
            .init = searchInit,
            .apply = searchApply,
        }),
        .props = view_mod.EditorProps.init(.{
            .decorations = .{ searchDecos, &g_app },
        }),
    });

    const st = state_mod.EditorState.create(.{
        .schema = g_app.schema,
        .doc = makeInitialDoc(g_app.schema),
        .plugins = &.{state_mod.Plugin.new(pspec)},
    });

    const mount = g_app.doc.getElementById("editor").?;
    g_app.view = view_mod.EditorView.new(mount, .{
        .state = st,
        .dispatchTransaction = .{ onDispatch, &g_app },
    }).retain();
    syncMeta(&g_app, g_app.view.getState());

    const ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (ids, 0..) |id, i| {
        g_app.btns[i] = .{ .app = &g_app, .id = i };
        g_app.doc.getElementById(id).?.addEventListener(.click, .{ onClick, &g_app.btns[i] }, null);
    }
}
