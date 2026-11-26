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
fn paraToDOM(_: model.Node) model.DOMOutputSpec {
    return holeSpec("p");
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
    const text = model.NodeSpec.init(.{ .group = "inline" });

    const ms = model.MarkSpec.init(.{ .toDOM = strongToDOM });
    const mc = model.MarkSpec.init(.{ .toDOM = codeToDOM });

    return model.Schema.new(.{
        .nodes = .{ .doc = doc, .heading = heading, .paragraph = para, .text = text },
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
    locked: bool = false,
    blocked: usize = 0,
    rev: usize = 0,
    last_action: []const u8 = "Ready — open write access",
    undo: [64]pm.JsArrayOf(tr_mod.Step) = undefined,
    undo_len: usize = 0,
    redo: [64]pm.JsArrayOf(tr_mod.Step) = undefined,
    redo_len: usize = 0,
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};

fn clearRedo(app: *App) void {
    for (0..app.redo_len) |i| app.redo[i].deinit();
    app.redo_len = 0;
}

fn pNode(schema: model.Schema, t: []const u8) model.Node {
    return schema.nodeWith("paragraph", .{ .content = schema.text(t) });
}

fn makeInitialDoc(schema: model.Schema) model.Node {
    const h = schema.nodeWith("heading", .{ .attrs = .{ .level = 2 },
        .content = schema.text("Step-Inversion Undo & Transaction Filter"),
    });
    return schema.nodeWith("doc", .{
        .content = &.{
            h,
            pNode(schema, "Every document change produces atomic Transform steps that can be inverted against prior snapshots."),
            pNode(schema, "A ProseMirror Plugin inspects transactions in filterTransaction and can enforce a read-only lock."),
            pNode(schema, "Click Append Revision or Bold First Line, then step backward with Undo and forward with Redo."),
        },
    });
}

fn auditInit(_: state_mod.EditorStateConfig, _: state_mod.EditorState) pm.JsValue {
    return pm.JsValue.number(0);
}

fn auditApply(tr: state_mod.Transaction, prev: pm.JsValue, _: state_mod.EditorState, _: state_mod.EditorState) pm.JsValue {
    const n = prev.toNumber();
    return pm.JsValue.number(if (tr.getDocChanged()) n + 1.0 else n);
}

fn filterTr(app: *App, tr: state_mod.Transaction, _: state_mod.EditorState) bool {
    if (app.locked and tr.getDocChanged()) {
        app.blocked += 1;
        app.last_action = "Blocked document mutation (filterTransaction=false)";
        return false;
    }
    return true;
}

fn updateMeta(app: *App, st: state_mod.EditorState) void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    const edits: usize = @intFromFloat(app.key.getState(st).toNumber());
    const lock_txt: []const u8 = if (app.locked) "LOCKED" else "OPEN";
    var buf: [512]u8 = undefined;
    const msg = std.fmt.bufPrint(&buf, "{s} · edits={d} · undoDepth={d} · redoDepth={d} · lock={s} · blocked={d} · blocks={d}", .{
        app.last_action,
        edits,
        app.undo_len,
        app.redo_len,
        lock_txt,
        app.blocked,
        @as(usize, @intFromFloat(st.getDoc().getChildCount())),
    }) catch app.last_action;
    app.meta.setTextContent(msg);
    app.doc.getElementById("btn-4").?.setClassName(if (app.locked) "active" else "");
}

fn onDispatch(app: *App, tr: state_mod.Transaction) void {
    if (app.view.h == 0) return;
    const scope = pm.JsScope.open();
    defer scope.deinit();

    const res = app.view.getState().applyTransaction(tr);
    if (res.getTransactions().len() > 0) {
        if (tr.getDocChanged()) {
            const steps = tr.getSteps();
            const docs = tr.getDocs();
            const inv = pm.JsArrayOf(tr_mod.Step).new();
            var idx = steps.len();
            while (idx > 0) {
                idx -= 1;
                inv.push(steps.get(idx).invert(docs.get(idx)));
            }
            const hm = tr.getMeta("history");
            if (hm.h == 0) {
                if (app.undo_len < 64) {
                    app.undo[app.undo_len] = inv.retain();
                    app.undo_len += 1;
                }
                clearRedo(app);
            } else if (hm.as(pm.JsString).eql("undo")) {
                if (app.redo_len < 64) {
                    app.redo[app.redo_len] = inv.retain();
                    app.redo_len += 1;
                }
            } else {
                if (app.undo_len < 64) {
                    app.undo[app.undo_len] = inv.retain();
                    app.undo_len += 1;
                }
            }
        }
        app.view.updateState(res.getState());
    }
    updateMeta(app, app.view.getState());
}

fn onClick(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    const app = ctx.app;
    const st = app.view.getState();
    if (ctx.id == 0) {
        app.rev += 1;
        var rbuf: [96]u8 = undefined;
        const msg = std.fmt.bufPrint(&rbuf, "Revision #{d} recorded via Transform step.", .{app.rev}) catch "Revision recorded.";
        const tr = st.getTr().insert(st.getDoc().getContent().getSize(), pNode(app.schema, msg));
        if (!app.locked) app.last_action = "Appended revision paragraph";
        app.view.dispatch(tr);
    } else if (ctx.id == 1) {
        const doc = st.getDoc();
        if (doc.getChildCount() >= 2.0) {
            const fp = doc.child(0).getNodeSize() + 1.0;
            const tp = fp + doc.child(1).getContent().getSize();
            if (tp > fp) {
                const tr = st.getTr().addMark(fp, tp, app.schema.mark("strong"));
                if (!app.locked) app.last_action = "Bolded first body paragraph";
                app.view.dispatch(tr);
            }
        }
    } else if (ctx.id == 2) {
        if (app.undo_len == 0) {
            app.last_action = "Undo stack empty";
            updateMeta(app, st);
        } else {
            const batch = if (app.locked) app.undo[app.undo_len - 1] else blk: {
                app.undo_len -= 1;
                break :blk app.undo[app.undo_len];
            };
            const tr = st.getTr().setMeta("history", "undo");
            for (0..batch.len()) |k| _ = tr.step(batch.get(k));
            if (!app.locked) {
                batch.deinit();
                app.last_action = "Undid step(s) via Step.invert";
            }
            app.view.dispatch(tr);
        }
    } else if (ctx.id == 3) {
        if (app.redo_len == 0) {
            app.last_action = "Redo stack empty";
            updateMeta(app, st);
        } else {
            const batch = if (app.locked) app.redo[app.redo_len - 1] else blk: {
                app.redo_len -= 1;
                break :blk app.redo[app.redo_len];
            };
            const tr = st.getTr().setMeta("history", "redo");
            for (0..batch.len()) |k| _ = tr.step(batch.get(k));
            if (!app.locked) {
                batch.deinit();
                app.last_action = "Redid inverted step(s)";
            }
            app.view.dispatch(tr);
        }
    } else {
        app.locked = !app.locked;
        app.last_action = if (app.locked) "Enabled read-only lock" else "Disabled read-only lock";
        updateMeta(app, st);
    }
}

export fn start() void {
    const scope = pm.JsScope.open();
    defer scope.deinit();

    g_app.doc = dom.getDocument().retain();
    g_app.meta = g_app.doc.getElementById("meta").?.retain();
    g_app.schema = makeSchema().retain();
    g_app.key = state_mod.PluginKey.new("audit").retain();

    const pspec = state_mod.PluginSpec.init(.{
        .key = g_app.key,
        .state = state_mod.StateField.fromFields(.{
            .init = auditInit,
            .apply = auditApply,
        }),
        .filterTransaction = .{ filterTr, &g_app },
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
    updateMeta(&g_app, g_app.view.getState());

    const ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (ids, 0..) |id, i| {
        g_app.btns[i] = .{ .app = &g_app, .id = i };
        g_app.doc.getElementById(id).?.addEventListener(.click, .{ onClick, &g_app.btns[i] });
    }
}
