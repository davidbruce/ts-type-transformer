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
fn quoteToDOM(_: ?*anyopaque, _: model.Node) model.DOMOutputSpec {
    return holeSpec("blockquote");
}
fn codeBlockToDOM(_: ?*anyopaque, _: model.Node) model.DOMOutputSpec {
    const inner = pm.JsArray.new();
    inner.push(pm.JsValue.string("code"));
    inner.push(pm.JsValue.number(0));
    const outer = pm.JsArray.new();
    outer.push(pm.JsValue.string("pre"));
    outer.push(.{ .h = inner.h });
    return .{ .array = outer };
}
fn hrToDOM(_: ?*anyopaque, _: model.Node) model.DOMOutputSpec {
    const a = pm.JsArray.new();
    a.push(pm.JsValue.string("hr"));
    return .{ .array = a };
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
    para.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(paraToDOM, null));

    const quote = model.NodeSpec.create();
    quote.setContent("block+");
    quote.setGroup("block");
    quote.setDefining(true);
    quote.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(quoteToDOM, null));

    const cblock = model.NodeSpec.create();
    cblock.setContent("text*");
    cblock.setMarks("");
    cblock.setGroup("block");
    cblock.setCode(true);
    cblock.setDefining(true);
    cblock.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(codeBlockToDOM, null));

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
    nodes.setAt("code_block", .{ .h = cblock.h });
    nodes.setAt("horizontal_rule", .{ .h = hr.h });
    nodes.setAt("text", .{ .h = text.h });

    const ms = model.MarkSpec.create();
    ms.setToDOM(pm.Mark_boolean_to_DOMOutputSpec_fn.from(strongToDOM, null));
    const me = model.MarkSpec.create();
    me.setToDOM(pm.Mark_boolean_to_DOMOutputSpec_fn.from(emToDOM, null));
    const mc = model.MarkSpec.create();
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

var g_schema: model.Schema = undefined;
var g_view: view_mod.EditorView = undefined;
var g_doc: dom.Document = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_dispatches: usize = 0;
var g_last_action: []const u8 = "Initialized schema with 7 node types & 3 marks";

fn makeInitialDoc() model.Node {
    const ha = model.Attrs.create();
    ha.setAt("level", pm.JsValue.number(2));
    const h = g_schema.node(.{ .string = pm.JsString.from("heading") }, ha, .{ .content = .{ .Node = g_schema.text("ProseMirror Document Schema", null) } });

    const m_strong = g_schema.mark(.{ .string = pm.JsString.from("strong") }, null);
    const m_em = g_schema.mark(.{ .string = pm.JsString.from("em") }, null);
    const m_code = g_schema.mark(.{ .string = pm.JsString.from("code") }, null);

    const p1 = g_schema.node(.{ .string = pm.JsString.from("paragraph") }, null, .{ .content = .{ .array = pm.JsArrayOf(model.Node).of(&.{
        g_schema.text("Inline marks compose cleanly: ", null),
        g_schema.text("bold", pm.JsArrayOf(model.Mark).of(&.{m_strong})),
        g_schema.text(", ", null),
        g_schema.text("italic", pm.JsArrayOf(model.Mark).of(&.{m_em})),
        g_schema.text(", and ", null),
        g_schema.text("inline_code", pm.JsArrayOf(model.Mark).of(&.{m_code})),
        g_schema.text(" spans.", null),
    }) } });

    const q_inner = g_schema.node(.{ .string = pm.JsString.from("paragraph") }, null, .{
        .content = .{ .Node = g_schema.text("Every document conforms to a strict schema enforced on every transaction.", null) },
    });
    const quote = g_schema.node(.{ .string = pm.JsString.from("blockquote") }, null, .{ .content = .{ .Node = q_inner } });
    const cb = g_schema.node(.{ .string = pm.JsString.from("code_block") }, null, .{
        .content = .{ .Node = g_schema.text("schema.node(\"paragraph\", null, [schema.text(\"typed\")])", null) },
    });
    const outro = g_schema.node(.{ .string = pm.JsString.from("paragraph") }, null, .{
        .content = .{ .Node = g_schema.text("Select any text in this editor and click a toolbar action to toggle marks or switch block types.", null) },
    });

    return g_schema.node(.{ .string = pm.JsString.from("doc") }, null, .{
        .content = .{ .array = pm.JsArrayOf(model.Node).of(&.{ h, p1, quote, cb, outro }) },
    });
}

fn makeState() state_mod.EditorState {
    const cfg = state_mod.EditorStateConfig.create();
    cfg.setSchema(g_schema);
    cfg.setDoc(makeInitialDoc());
    return state_mod.EditorState.create(cfg);
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

fn updateMeta(st: state_mod.EditorState) void {
    const r = targetRange(st);
    const doc = st.getDoc();
    const marks = g_schema.getMarks();
    const hb = doc.rangeHasMark(r.f, r.t, .{ .MarkType = marks.getAt("strong").? });
    const he = doc.rangeHasMark(r.f, r.t, .{ .MarkType = marks.getAt("em").? });
    const hc = doc.rangeHasMark(r.f, r.t, .{ .MarkType = marks.getAt("code").? });

    g_doc.getElementById("btn-0").?.asElement().setClassName(if (hb) "active" else "");
    g_doc.getElementById("btn-1").?.asElement().setClassName(if (he) "active" else "");
    g_doc.getElementById("btn-2").?.asElement().setClassName(if (hc) "active" else "");

    var bname_buf: [64]u8 = undefined;
    const bname = st.getSelection().get_From().getParent().getType().getName().read(&bname_buf);

    var buf: [512]u8 = undefined;
    const msg = std.fmt.bufPrint(&buf, "{s} · blocks={d} · nodeSize={d} · range=[{d}..{d}] · block={s} · dispatches={d}", .{
        g_last_action,
        @as(usize, @intFromFloat(doc.getChildCount())),
        @as(usize, @intFromFloat(doc.getNodeSize())),
        @as(usize, @intFromFloat(r.f)),
        @as(usize, @intFromFloat(r.t)),
        bname,
        g_dispatches,
    }) catch g_last_action;
    g_meta.asNode().setTextContent(msg);
}

fn onDispatch(_: ?*anyopaque, tr: state_mod.Transaction) void {
    if (g_view.h == 0) return;
    g_dispatches += 1;
    const next = g_view.getState().apply(tr);
    g_view.updateState(next);
    updateMeta(next);
}

fn toggleMark(name: []const u8, added: []const u8, removed: []const u8) void {
    const st = g_view.getState();
    const r = targetRange(st);
    if (r.t <= r.f) return;
    const mt = g_schema.getMarks().getAt(name).?;
    const tr = st.getTr();
    if (st.getDoc().rangeHasMark(r.f, r.t, .{ .MarkType = mt })) {
        _ = tr.asTransform().removeMark(r.f, r.t, .{ .MarkType = mt });
        g_last_action = removed;
    } else {
        _ = tr.asTransform().addMark(r.f, r.t, g_schema.mark(.{ .MarkType = mt }, null));
        g_last_action = added;
    }
    pm.callTransactionFn(g_view.getDispatch(), tr);
}

const BTN_IDS = [5]usize{ 0, 1, 2, 3, 4 };
fn onClick(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const idx = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    if (idx == 0) {
        toggleMark("strong", "Added bold mark", "Removed bold mark");
    } else if (idx == 1) {
        toggleMark("em", "Added italic mark", "Removed italic mark");
    } else if (idx == 2) {
        toggleMark("code", "Added code mark", "Removed code mark");
    } else if (idx == 3) {
        const st = g_view.getState();
        const sel = st.getSelection();
        const tr = st.getTr();
        const nodes = g_schema.getNodes();
        if (sel.get_From().getParent().getType().getName().eql("heading")) {
            _ = tr.asTransform().setBlockType(sel.getFrom(), sel.getTo(), nodes.getAt("paragraph").?, null);
            g_last_action = "Converted heading -> paragraph";
        } else {
            const ha = model.Attrs.create();
            ha.setAt("level", pm.JsValue.number(2));
            _ = tr.asTransform().setBlockType(sel.getFrom(), sel.getTo(), nodes.getAt("heading").?, .{ .Attrs = ha });
            g_last_action = "Converted block -> heading[level=2]";
        }
        pm.callTransactionFn(g_view.getDispatch(), tr);
    } else {
        g_dispatches = 0;
        g_last_action = "Reset initial schema document";
        const fresh = makeState();
        g_view.updateState(fresh);
        updateMeta(fresh);
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
    g_view = view_mod.EditorView.new(.{ .dist_DOMNode = .{ .h = mount.h } }, props);
    _ = pm.jsRetain(g_view);
    updateMeta(g_view.getState());

    const ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (ids, 0..) |id, i| {
        const btn = g_doc.getElementById(id).?;
        btn.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onClick, @constCast(&BTN_IDS[i])), .{});
    }
}
