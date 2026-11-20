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
fn paraToDOM(_: ?*anyopaque, _: model.Node) model.DOMOutputSpec {
    return holeSpec("p");
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

    const cblock = model.NodeSpec.create();
    cblock.setContent("text*");
    cblock.setMarks("");
    cblock.setGroup("block");
    cblock.setCode(true);
    cblock.setToDOM(pm.Node_to_DOMOutputSpec_fn.from(codeBlockToDOM, null));

    const text = model.NodeSpec.create();
    text.setGroup("inline");

    const nodes = model.NodeSpec.create();
    nodes.setAt("doc", .{ .h = doc.h });
    nodes.setAt("heading", .{ .h = heading.h });
    nodes.setAt("paragraph", .{ .h = para.h });
    nodes.setAt("code_block", .{ .h = cblock.h });
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
var g_active_btn: usize = 0;

fn pNode(t: []const u8) model.Node {
    return g_schema.node(.{ .string = pm.JsString.from("paragraph") }, null, .{ .content = .{ .Node = g_schema.text(t, null) } });
}

fn makeInitialDoc() model.Node {
    const ha = model.Attrs.create();
    ha.setAt("level", pm.JsValue.number(2));
    const h = g_schema.node(.{ .string = pm.JsString.from("heading") }, ha, .{
        .content = .{ .Node = g_schema.text("Immutable State & View Decorations", null) },
    });
    const cb = g_schema.node(.{ .string = pm.JsString.from("code_block") }, null, .{
        .content = .{ .Node = g_schema.text("const nextState = state.apply(transaction);\nview.updateState(nextState); // sync view", null) },
    });
    return g_schema.node(.{ .string = pm.JsString.from("doc") }, null, .{
        .content = .{ .array = pm.JsArrayOf(model.Node).of(&.{
            h,
            pNode("A ProseMirror schema validates every document tree before state updates commit to the view."),
            pNode("Each transaction maps positions forward so state plugins can recompute decorations without mutating the schema."),
            cb,
            pNode("Inline decorations highlight matching tokens while node decorations outline matching blocks in the view."),
        }) },
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
    const inl_attr = view_mod.DecorationAttrs.create();
    inl_attr.setClass("pm-match");
    const blk_attr = view_mod.DecorationAttrs.create();
    blk_attr.setClass("pm-hit-block");

    var inl_hits: usize = 0;
    var blk_hits: usize = 0;
    var block_pos: f64 = 0;
    const nb: usize = @intFromFloat(doc.getChildCount());
    for (0..nb) |i| {
        const blk = doc.child(@floatFromInt(i));
        var inline_pos: f64 = block_pos + 1.0;
        var matched = false;
        const nc: usize = @intFromFloat(blk.getChildCount());
        for (0..nc) |j| {
            const ch = blk.child(@floatFromInt(j));
            if (ch.getIsText()) {
                var tbuf: [1024]u8 = undefined;
                const txt = ch.getTextContent().read(&tbuf);
                var at: usize = 0;
                while (findCi(txt, query, at)) |pos_idx| {
                    const fp = inline_pos + @as(f64, @floatFromInt(pos_idx));
                    const tp = fp + @as(f64, @floatFromInt(query.len));
                    decos.push(view_mod.Decoration.@"inline"(fp, tp, inl_attr, .{}));
                    inl_hits += 1;
                    matched = true;
                    at = pos_idx + query.len;
                }
            }
            inline_pos += ch.getNodeSize();
        }
        if (matched) {
            decos.push(view_mod.Decoration.node(block_pos, block_pos + blk.getNodeSize(), blk_attr, .{}));
            blk_hits += 1;
        }
        block_pos += blk.getNodeSize();
    }
    return .{ .set = view_mod.DecorationSet.create(doc, decos), .inline_hits = inl_hits, .block_hits = blk_hits };
}

fn searchInit(_: ?*anyopaque, _: state_mod.EditorStateConfig, _: state_mod.EditorState) pm.JsValue {
    return pm.JsValue.string("state");
}
fn searchApply(_: ?*anyopaque, tr: state_mod.Transaction, prev: pm.JsValue, _: state_mod.EditorState, _: state_mod.EditorState) pm.JsValue {
    const mv = tr.getMeta2(.{ .string = pm.JsString.from("search") });
    if (mv.h != 0) return mv;
    return pm.jsRetain(prev);
}
fn searchDecos(_: ?*anyopaque, st: state_mod.EditorState) ?view_mod.DecorationSource {
    const qv = g_key.getState(st);
    var qbuf: [64]u8 = undefined;
    const q = if (qv.h != 0) (pm.JsString{ .h = qv.h }).read(&qbuf) else "";
    const dset = computeDecorations(st.getDoc(), q).set;
    return .{ .h = dset.h };
}

fn syncMeta(st: state_mod.EditorState) void {
    const qv = g_key.getState(st);
    var qbuf: [64]u8 = undefined;
    const q = if (qv.h != 0) (pm.JsString{ .h = qv.h }).read(&qbuf) else "";
    const stats = computeDecorations(st.getDoc(), q);
    const total = stats.set.find(.{}).len();

    var qdisp_buf: [80]u8 = undefined;
    const qdisp = if (q.len == 0) "(none)" else (std.fmt.bufPrint(&qdisp_buf, "\"{s}\"", .{q}) catch q);

    var buf: [256]u8 = undefined;
    const msg = std.fmt.bufPrint(&buf, "Query: {s} · inline matches={d} · highlighted blocks={d} · decorations={d}", .{
        qdisp,
        stats.inline_hits,
        stats.block_hits,
        total,
    }) catch "Updated search";
    g_meta.asNode().setTextContent(msg);

    const ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (ids, 0..) |id, i| {
        g_doc.getElementById(id).?.asElement().setClassName(if (i == g_active_btn) "active" else "");
    }
}

fn onDispatch(_: ?*anyopaque, tr: state_mod.Transaction) void {
    if (g_view.h == 0) return;
    const next = g_view.getState().apply(tr);
    g_view.updateState(next);
    syncMeta(next);
}

const BTN_IDS = [5]usize{ 0, 1, 2, 3, 4 };
const QUERIES = [5][]const u8{ "state", "schema", "transaction", "view", "" };
fn onClick(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const idx = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    g_active_btn = idx;
    const tr = g_view.getState().getTr();
    _ = tr.setMeta2(.{ .string = pm.JsString.from("search") }, pm.JsValue.string(QUERIES[idx]));
    pm.callTransactionFn(g_view.getDispatch(), tr);
}

export fn start() void {
    g_doc = dom.getDocument();
    _ = dom.jsRetain(g_doc);
    g_meta = g_doc.getElementById("meta").?;
    _ = dom.jsRetain(g_meta);
    g_schema = makeSchema();
    _ = pm.jsRetain(g_schema);

    g_key = state_mod.PluginKey.new("search");
    _ = pm.jsRetain(g_key);

    const field = state_mod.StateField.create();
    field.setInit(pm.StateField_init.from(searchInit, null));
    field.setApply(pm.StateField_apply.from(searchApply, null));

    const eprops = view_mod.EditorProps.create();
    eprops.setDecorations(pm.any_EditorState_to_opt_DecorationSource_fn.from(searchDecos, null));

    const pspec = state_mod.PluginSpec.create();
    pspec.setKey(g_key);
    pspec.setState(field);
    pspec.setProps(eprops);

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
    syncMeta(g_view.getState());

    const ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (ids, 0..) |id, i| {
        const btn = g_doc.getElementById(id).?;
        btn.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onClick, @constCast(&BTN_IDS[i])), .{});
    }
}
