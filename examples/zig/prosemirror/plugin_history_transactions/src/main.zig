const std = @import("std");
const dom = @import("dom");
const pm = @import("prosemirror");
const model = pm.prosemirror_model;
const tr_mod = pm.prosemirror_transform;
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
fn paraToDOM(_: ?*anyopaque, _: model.Node) model.DOMOutputSpec {
    return holeSpec("p");
}
fn strongToDOM(_: ?*anyopaque, _: model.Mark, _: bool) model.DOMOutputSpec {
    return holeSpec("strong");
}
fn codeToDOM(_: ?*anyopaque, _: model.Mark, _: bool) model.DOMOutputSpec {
    return holeSpec("code");
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
    heading.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(headingToDOM, null));

    const para = model.NodeSpec.create();
    para.setContent("inline*");
    para.setGroup("block");
    para.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(paraToDOM, null));

    const text = model.NodeSpec.create();
    text.setGroup("inline");

    const nodes = model.NodeSpec.create();
    nodes.setAt("doc", .{ .h = doc.h });
    nodes.setAt("heading", .{ .h = heading.h });
    nodes.setAt("paragraph", .{ .h = para.h });
    nodes.setAt("text", .{ .h = text.h });

    const ms = model.MarkSpec.create();
    ms.setToDOM(pm.Mark_boolean_to_DOMOutputSpec_fn.from(strongToDOM, null));
    const mc = model.MarkSpec.create();
    mc.setToDOM(pm.Mark_boolean_to_DOMOutputSpec_fn.from(codeToDOM, null));
    const marks = model.MarkSpec.create();
    marks.setAt("strong", .{ .h = ms.h });
    marks.setAt("code", .{ .h = mc.h });

    const spec = model.SchemaSpec.create();
    spec.setNodes(.{ .h = nodes.h });
    spec.setMarks(.{ .h = marks.h });
    return model.Schema.new(spec);
}

var g_schema: model.Schema = undefined;
var g_key: state_mod.PluginKey = undefined;
var g_view: view_mod.EditorView = undefined;
var g_doc: dom.Document = undefined;
var g_meta: dom.HTMLElement = undefined;

var g_locked: bool = false;
var g_blocked: usize = 0;
var g_rev: usize = 0;
var g_last_action: []const u8 = "Ready — open write access";

var g_undo: [64]pm.JsArrayOf(tr_mod.Step) = undefined;
var g_undo_len: usize = 0;
var g_redo: [64]pm.JsArrayOf(tr_mod.Step) = undefined;
var g_redo_len: usize = 0;

fn clearRedo() void {
    for (0..g_redo_len) |i| pm.jsRelease(g_redo[i]);
    g_redo_len = 0;
}

fn pNode(t: []const u8) model.Node {
    return g_schema.node(.{ .string = pm.JsString.from("paragraph") }, null, .{ .content = .{ .Node = g_schema.text(t, null) } });
}

fn makeInitialDoc() model.Node {
    const ha = model.Attrs.create();
    ha.setAt("level", pm.JsValue.number(2));
    const h = g_schema.node(.{ .string = pm.JsString.from("heading") }, ha, .{
        .content = .{ .Node = g_schema.text("Step-Inversion Undo & Transaction Filter", null) },
    });
    return g_schema.node(.{ .string = pm.JsString.from("doc") }, null, .{
        .content = .{ .array = pm.JsArrayOf(model.Node).of(&.{
            h,
            pNode("Every document change produces atomic Transform steps that can be inverted against prior snapshots."),
            pNode("A ProseMirror Plugin inspects transactions in filterTransaction and can enforce a read-only lock."),
            pNode("Click Append Revision or Bold First Line, then step backward with Undo and forward with Redo."),
        }) },
    });
}

fn auditInit(_: ?*anyopaque, _: state_mod.EditorStateConfig, _: state_mod.EditorState) pm.JsValue {
    return pm.JsValue.number(0);
}
fn auditApply(_: ?*anyopaque, tr: state_mod.Transaction, prev: pm.JsValue, _: state_mod.EditorState, _: state_mod.EditorState) pm.JsValue {
    const n = prev.toNumber();
    return pm.JsValue.number(if (tr.asTransform().getDocChanged()) n + 1.0 else n);
}
fn filterTr(_: ?*anyopaque, tr: state_mod.Transaction, _: state_mod.EditorState) bool {
    if (g_locked and tr.asTransform().getDocChanged()) {
        g_blocked += 1;
        g_last_action = "Blocked document mutation (filterTransaction=false)";
        return false;
    }
    return true;
}

fn updateMeta(st: state_mod.EditorState) void {
    const edits: usize = @intFromFloat(g_key.getState(st).toNumber());
    const lock_txt: []const u8 = if (g_locked) "LOCKED" else "OPEN";
    var buf: [512]u8 = undefined;
    const msg = std.fmt.bufPrint(&buf, "{s} · edits={d} · undoDepth={d} · redoDepth={d} · lock={s} · blocked={d} · blocks={d}", .{
        g_last_action,
        edits,
        g_undo_len,
        g_redo_len,
        lock_txt,
        g_blocked,
        @as(usize, @intFromFloat(st.getDoc().getChildCount())),
    }) catch g_last_action;
    g_meta.asNode().setTextContent(msg);
    g_doc.getElementById("btn-4").?.asElement().setClassName(if (g_locked) "active" else "");
}

fn onDispatch(_: ?*anyopaque, tr: state_mod.Transaction) void {
    if (g_view.h == 0) return;
    const res = g_view.getState().applyTransaction(tr);
    if (res.getTransactions().len() > 0) {
        if (tr.asTransform().getDocChanged()) {
            const steps = tr.asTransform().getSteps();
            const docs = tr.asTransform().getDocs();
            const inv = pm.JsArrayOf(tr_mod.Step).new();
            var idx = steps.len();
            while (idx > 0) {
                idx -= 1;
                inv.push(steps.get(idx).invert(docs.get(idx)));
            }
            const hm = tr.getMeta2(.{ .string = pm.JsString.from("history") });
            if (hm.h == 0) {
                if (g_undo_len < 64) {
                    _ = pm.jsRetain(inv);
                    g_undo[g_undo_len] = inv;
                    g_undo_len += 1;
                }
                clearRedo();
            } else if ((pm.JsString{ .h = hm.h }).eql("undo")) {
                if (g_redo_len < 64) {
                    _ = pm.jsRetain(inv);
                    g_redo[g_redo_len] = inv;
                    g_redo_len += 1;
                }
            } else {
                if (g_undo_len < 64) {
                    _ = pm.jsRetain(inv);
                    g_undo[g_undo_len] = inv;
                    g_undo_len += 1;
                }
            }
        }
        g_view.updateState(res.getState());
    }
    updateMeta(g_view.getState());
}

const BTN_IDS = [5]usize{ 0, 1, 2, 3, 4 };
fn onClick(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const idx = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    const st = g_view.getState();
    if (idx == 0) {
        g_rev += 1;
        var rbuf: [96]u8 = undefined;
        const msg = std.fmt.bufPrint(&rbuf, "Revision #{d} recorded via Transform step.", .{g_rev}) catch "Revision recorded.";
        const tr = st.getTr();
        _ = tr.asTransform().insert(st.getDoc().getContent().getSize(), .{ .Node = pNode(msg) });
        if (!g_locked) g_last_action = "Appended revision paragraph";
        pm.callTransactionFn(g_view.getDispatch(), tr);
    } else if (idx == 1) {
        const doc = st.getDoc();
        if (doc.getChildCount() >= 2.0) {
            const fp = doc.child(0).getNodeSize() + 1.0;
            const tp = fp + doc.child(1).getContent().getSize();
            if (tp > fp) {
                const tr = st.getTr();
                _ = tr.asTransform().addMark(fp, tp, g_schema.mark(.{ .string = pm.JsString.from("strong") }, null));
                if (!g_locked) g_last_action = "Bolded first body paragraph";
                pm.callTransactionFn(g_view.getDispatch(), tr);
            }
        }
    } else if (idx == 2) {
        if (g_undo_len == 0) {
            g_last_action = "Undo stack empty";
            updateMeta(st);
        } else {
            const batch = if (g_locked) g_undo[g_undo_len - 1] else blk: {
                g_undo_len -= 1;
                break :blk g_undo[g_undo_len];
            };
            const tr = st.getTr();
            _ = tr.setMeta2(.{ .string = pm.JsString.from("history") }, pm.JsValue.string("undo"));
            for (0..batch.len()) |k| _ = tr.asTransform().step(batch.get(k));
            if (!g_locked) {
                pm.jsRelease(batch);
                g_last_action = "Undid step(s) via Step.invert";
            }
            pm.callTransactionFn(g_view.getDispatch(), tr);
        }
    } else if (idx == 3) {
        if (g_redo_len == 0) {
            g_last_action = "Redo stack empty";
            updateMeta(st);
        } else {
            const batch = if (g_locked) g_redo[g_redo_len - 1] else blk: {
                g_redo_len -= 1;
                break :blk g_redo[g_redo_len];
            };
            const tr = st.getTr();
            _ = tr.setMeta2(.{ .string = pm.JsString.from("history") }, pm.JsValue.string("redo"));
            for (0..batch.len()) |k| _ = tr.asTransform().step(batch.get(k));
            if (!g_locked) {
                pm.jsRelease(batch);
                g_last_action = "Redid inverted step(s)";
            }
            pm.callTransactionFn(g_view.getDispatch(), tr);
        }
    } else {
        g_locked = !g_locked;
        g_last_action = if (g_locked) "Enabled read-only lock" else "Disabled read-only lock";
        updateMeta(st);
    }
}

export fn start() void {
    g_doc = dom.getDocument();
    _ = dom.jsRetain(g_doc);
    g_meta = g_doc.getElementById("meta").?;
    _ = dom.jsRetain(g_meta);
    g_schema = makeSchema();
    _ = pm.jsRetain(g_schema);

    g_key = state_mod.PluginKey.new("audit");
    _ = pm.jsRetain(g_key);

    const field = state_mod.StateField.create();
    field.setInit(pm.StateField_init.from(auditInit, null));
    field.setApply(pm.StateField_apply.from(auditApply, null));

    const pspec = state_mod.PluginSpec.create();
    pspec.setKey(g_key);
    pspec.setState(field);
    pspec.setFilterTransaction(pm.PluginSpec_filterTransaction.from(filterTr, null));

    const scfg = state_mod.EditorStateConfig.create();
    scfg.setSchema(g_schema);
    scfg.setDoc(makeInitialDoc());
    scfg.setPlugins(pm.JsArrayOf(state_mod.Plugin).of(&.{state_mod.Plugin.new(pspec)}));

    const props = view_mod.DirectEditorProps.create();
    props.setState(state_mod.EditorState.create(scfg));
    props.setDispatchTransaction(pm.Transaction_fn.from(onDispatch, null));

    const mount = g_doc.getElementById("editor").?;
    g_view = view_mod.EditorView.new(.{ .Node_2 = mount.asNode() }, props);
    _ = pm.jsRetain(g_view);
    updateMeta(g_view.getState());

    const ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (ids, 0..) |id, i| {
        const btn = g_doc.getElementById(id).?;
        btn.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onClick, @constCast(&BTN_IDS[i])), .{});
    }
}
