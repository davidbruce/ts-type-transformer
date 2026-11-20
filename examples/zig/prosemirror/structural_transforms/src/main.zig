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
fn pToDOM(_: ?*anyopaque, _: model.Node) model.DOMOutputSpec {
    return holeSpec("p");
}
fn bqToDOM(_: ?*anyopaque, _: model.Node) model.DOMOutputSpec {
    return holeSpec("blockquote");
}
fn ulToDOM(_: ?*anyopaque, _: model.Node) model.DOMOutputSpec {
    return holeSpec("ul");
}
fn liToDOM(_: ?*anyopaque, _: model.Node) model.DOMOutputSpec {
    return holeSpec("li");
}
fn hrToDOM(_: ?*anyopaque, _: model.Node) model.DOMOutputSpec {
    const a = pm.JsArray.new();
    a.push(pm.JsValue.string("hr"));
    return .{ .array = a };
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
    heading.setDefining(true);
    heading.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(headingToDOM, null));

    const para = model.NodeSpec.create();
    para.setContent("inline*");
    para.setGroup("block");
    para.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(pToDOM, null));

    const quote = model.NodeSpec.create();
    quote.setContent("block+");
    quote.setGroup("block");
    quote.setDefining(true);
    quote.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(bqToDOM, null));

    const ulist = model.NodeSpec.create();
    ulist.setContent("list_item+");
    ulist.setGroup("block");
    ulist.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(ulToDOM, null));

    const li = model.NodeSpec.create();
    li.setContent("paragraph block*");
    li.setDefining(true);
    li.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(liToDOM, null));

    const hr = model.NodeSpec.create();
    hr.setGroup("block");
    hr.setAtom(true);
    hr.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(hrToDOM, null));

    const text = model.NodeSpec.create();
    text.setGroup("inline");

    const nodes = model.NodeSpec.create();
    nodes.setAt("doc", .{ .h = doc.h });
    nodes.setAt("heading", .{ .h = heading.h });
    nodes.setAt("paragraph", .{ .h = para.h });
    nodes.setAt("blockquote", .{ .h = quote.h });
    nodes.setAt("bullet_list", .{ .h = ulist.h });
    nodes.setAt("list_item", .{ .h = li.h });
    nodes.setAt("horizontal_rule", .{ .h = hr.h });
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
var g_view: view_mod.EditorView = undefined;
var g_doc: dom.Document = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_total_steps: usize = 0;
var g_note_counter: usize = 0;
var g_last_op: []const u8 = "Initialized document structure";

fn pNode(t: []const u8) model.Node {
    return g_schema.node(.{ .string = pm.JsString.from("paragraph") }, null, .{ .content = .{ .Node = g_schema.text(t, null) } });
}
fn liNode(t: []const u8) model.Node {
    return g_schema.node(.{ .string = pm.JsString.from("list_item") }, null, .{ .content = .{ .Node = pNode(t) } });
}

fn makeInitialDoc() model.Node {
    const ha = model.Attrs.create();
    ha.setAt("level", pm.JsValue.number(2));
    const h = g_schema.node(.{ .string = pm.JsString.from("heading") }, ha, .{
        .content = .{ .Node = g_schema.text("Structural Transform Pipeline", null) },
    });
    const quote = g_schema.node(.{ .string = pm.JsString.from("blockquote") }, null, .{
        .content = .{ .Node = pNode("Blockquote content can be lifted back to top-level document depth.") },
    });
    const ulist = g_schema.node(.{ .string = pm.JsString.from("bullet_list") }, null, .{
        .content = .{ .array = pm.JsArrayOf(model.Node).of(&.{
            liNode("Split at cursor position"),
            liNode("Wrap range with blockquote"),
            liNode("Lift nested block to root"),
        }) },
    });
    return g_schema.node(.{ .string = pm.JsString.from("doc") }, null, .{
        .content = .{ .array = pm.JsArrayOf(model.Node).of(&.{
            h,
            pNode("ProseMirror transforms express structural edits as invertible steps."),
            pNode("Wrap blocks in quotes, lift nested content, or split paragraphs cleanly."),
            quote,
            ulist,
        }) },
    });
}

fn makeState() state_mod.EditorState {
    const cfg = state_mod.EditorStateConfig.create();
    cfg.setSchema(g_schema);
    cfg.setDoc(makeInitialDoc());
    return state_mod.EditorState.create(cfg);
}

fn updateMeta(doc: model.Node) void {
    var obuf: [512]u8 = undefined;
    var w = std.Io.Writer.fixed(&obuf);
    const n: usize = @intFromFloat(doc.getChildCount());
    for (0..n) |i| {
        if (i > 0) w.writeAll(" · ") catch {};
        var nbuf: [48]u8 = undefined;
        w.writeAll(doc.child(@floatFromInt(i)).getType().getName().read(&nbuf)) catch {};
    }
    var mbuf: [768]u8 = undefined;
    const msg = std.fmt.bufPrint(&mbuf, "{s} · blocks={d} · outline=[{s}] · totalSteps={d} · nodeSize={d}", .{
        g_last_op,
        n,
        w.buffered(),
        g_total_steps,
        @as(usize, @intFromFloat(doc.getNodeSize())),
    }) catch g_last_op;
    g_meta.asNode().setTextContent(msg);
}

fn onDispatch(_: ?*anyopaque, tr: state_mod.Transaction) void {
    if (g_view.h == 0) return;
    g_total_steps += tr.asTransform().getSteps().len();
    const next = g_view.getState().apply(tr);
    g_view.updateState(next);
    updateMeta(next.getDoc());
}

const BTN_IDS = [5]usize{ 0, 1, 2, 3, 4 };
fn onClick(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const idx = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    const st = g_view.getState();
    const doc = st.getDoc();

    if (idx == 0) {
        const sel_from = st.getSelection().getFrom();
        var split_pos: ?f64 = null;
        if (tr_mod.canSplit(doc, sel_from, .{})) {
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
                        if (tr_mod.canSplit(doc, mid, .{})) split_pos = mid;
                    }
                }
                pos += ch.getNodeSize();
            }
        }
        if (split_pos) |sp| {
            const tr = st.getTr();
            _ = tr.asTransform().split(sp, .{});
            g_last_op = "Split paragraph node";
            pm.callTransactionFn(g_view.getDispatch(), tr);
        } else {
            g_last_op = "No splittable paragraph found";
            updateMeta(doc);
        }
    } else if (idx == 1) {
        const qtype = g_schema.getNodes().getAt("blockquote").?;
        const sel = st.getSelection();
        var found: ?model.NodeRange = null;
        if (sel.get_From().getDepth() == 1.0 and sel.get_From().getParent().getType().getName().eql("paragraph")) {
            if (sel.get_From().blockRange(.{ .other = sel.get_To() })) |r| {
                if (tr_mod.findWrapping(r, qtype, null, .{}) != null) found = r;
            }
        }
        if (found == null) {
            var pos: f64 = 0;
            const n: usize = @intFromFloat(doc.getChildCount());
            for (0..n) |i| {
                const ch = doc.child(@floatFromInt(i));
                if (found == null and ch.getType().getName().eql("paragraph")) {
                    const rp = doc.resolve(pos + 1.0);
                    if (rp.blockRange(.{}) ) |r| {
                        if (tr_mod.findWrapping(r, qtype, null, .{}) != null) found = r;
                    }
                }
                pos += ch.getNodeSize();
            }
        }
        if (found) |r| {
            if (tr_mod.findWrapping(r, qtype, null, .{})) |wrapped| {
                const tr = st.getTr();
                _ = tr.asTransform().wrap(r, wrapped);
                g_last_op = "Wrapped paragraph block in blockquote";
                pm.callTransactionFn(g_view.getDispatch(), tr);
                return;
            }
        }
        g_last_op = "No top-level paragraph left to wrap";
        updateMeta(doc);
    } else if (idx == 2) {
        var pos: f64 = 0;
        const n: usize = @intFromFloat(doc.getChildCount());
        for (0..n) |i| {
            const ch = doc.child(@floatFromInt(i));
            if (ch.getType().getName().eql("blockquote") and ch.getChildCount() > 0.0) {
                const rp = doc.resolve(pos + 2.0);
                if (rp.blockRange(.{})) |r| {
                    if (tr_mod.liftTarget(r)) |target| {
                        const tr = st.getTr();
                        _ = tr.asTransform().lift(r, target);
                        g_last_op = "Lifted blockquote child to root depth";
                        pm.callTransactionFn(g_view.getDispatch(), tr);
                        return;
                    }
                }
            }
            pos += ch.getNodeSize();
        }
        g_last_op = "No blockquote left to lift";
        updateMeta(doc);
    } else if (idx == 3) {
        g_note_counter += 1;
        var nbuf: [128]u8 = undefined;
        const msg = std.fmt.bufPrint(&nbuf, "Appended structural section #{d} at doc offset {d}.", .{
            g_note_counter,
            @as(usize, @intFromFloat(doc.getContent().getSize())),
        }) catch "Appended section.";
        const hr = g_schema.node(.{ .string = pm.JsString.from("horizontal_rule") }, null, .{});
        const tr = st.getTr();
        _ = tr.asTransform().insert(doc.getContent().getSize(), .{ .array = pm.JsArrayOf(model.Node).of(&.{ hr, pNode(msg) }) });
        g_last_op = "Inserted horizontal_rule + note";
        pm.callTransactionFn(g_view.getDispatch(), tr);
    } else {
        g_total_steps = 0;
        g_note_counter = 0;
        g_last_op = "Reset structural sample";
        const fresh = makeState();
        g_view.updateState(fresh);
        updateMeta(fresh.getDoc());
    }
}

export fn start() void {
    g_doc = dom.getDocument();
    _ = dom.jsRetain(g_doc);
    g_meta = g_doc.getElementById("meta").?;
    _ = dom.jsRetain(g_meta);
    g_schema = makeSchema();
    _ = pm.jsRetain(g_schema);

    const props = view_mod.DirectEditorProps.create();
    props.setState(makeState());
    props.setDispatchTransaction(pm.Transaction_fn.from(onDispatch, null));

    const mount = g_doc.getElementById("editor").?;
    g_view = view_mod.EditorView.new(.{ .Node_2 = mount.asNode() }, props);
    _ = pm.jsRetain(g_view);
    updateMeta(g_view.getState().getDoc());

    const ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (ids, 0..) |id, i| {
        const btn = g_doc.getElementById(id).?;
        btn.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onClick, @constCast(&BTN_IDS[i])), .{});
    }
}
