const std = @import("std");
const dom = @import("dom");
const tt = @import("tanstack_table_core");

const NONE = tt.JsValue{ .h = 0 };
fn getProp(obj: tt.JsValue, key: []const u8) tt.JsValue {
    return (tt.RowData_map{ .h = obj.h }).getAt(key);
}

const AtomState = struct {
    value: tt.JsValue = NONE,
    derive: tt.JsValue = NONE,
    is_ro: bool = false,
};
var atoms: [512]AtomState = undefined;
var atom_count: usize = 0;

fn atomGet(ctx: ?*anyopaque) tt.JsValue {
    const a: *AtomState = @ptrCast(@alignCast(ctx.?));
    if (a.is_ro) return tt.callVoidToAnyFn(.{ .h = a.derive.h });
    _ = tt.jsRetain(a.value);
    return a.value;
}
fn atomSet(ctx: ?*anyopaque, updater: tt.JsValue) tt.JsValue {
    const a: *AtomState = @ptrCast(@alignCast(ctx.?));
    const next = tt.functionalUpdate(updater, a.value);
    _ = tt.jsRetain(next);
    a.value = next;
    return NONE;
}
fn noUnsub(_: ?*anyopaque) void {}
fn atomSub(_: ?*anyopaque, _: tt.JsValue) tt.JsValue {
    const sub = tt.makeObjectMap();
    sub.setAt("unsubscribe", .{ .h = tt.void_fn.from(noUnsub, null).h });
    return .{ .h = sub.h };
}
fn makeAtom(a: *AtomState) tt.JsValue {
    const atom = tt.makeObjectMap();
    atom.setAt("get", .{ .h = tt.void_to_any_fn.from(atomGet, a).h });
    if (!a.is_ro) atom.setAt("set", .{ .h = tt.Updater_fn2.from(atomSet, a).h });
    atom.setAt("subscribe", .{ .h = tt.Updater_fn2.from(atomSub, null).h });
    return .{ .h = atom.h };
}
fn createWritable(_: ?*anyopaque, initial: tt.JsValue, _: ?tt.TableAtomOptions) tt.Atom {
    const a = &atoms[atom_count];
    atom_count += 1;
    a.* = .{ .value = initial };
    _ = tt.jsRetain(initial);
    return .{ .h = makeAtom(a).h };
}
fn createReadonly(_: ?*anyopaque, fn_v: tt.void_to_any_fn, _: ?tt.TableAtomOptions) tt.ReadonlyAtom {
    const a = &atoms[atom_count];
    atom_count += 1;
    a.* = .{ .derive = .{ .h = fn_v.h }, .is_ro = true };
    _ = tt.jsRetain(fn_v);
    return .{ .h = makeAtom(a).h };
}
fn addSub(_: ?*anyopaque, _: tt.Subscription) void {}
fn unmountCb(_: ?*anyopaque) void {}
fn batchCb(_: ?*anyopaque, fn_v: tt.void_fn) void {
    tt.callVoidFn(fn_v);
}
fn untrackCb(_: ?*anyopaque, fn_v: tt.void_to_any_fn) tt.JsValue {
    return tt.callVoidToAnyFn(fn_v);
}

fn makeReactivity() tt.TableReactivityBindings {
    const r = tt.TableReactivityBindings.create();
    r.setCreateOptionsStore(false);
    r.setWrapExternalAtoms(false);
    r.setAddSubscription(tt.TableReactivityBindings_addSubscription.from(addSub, null));
    r.setUnmount(tt.void_fn.from(unmountCb, null));
    r.setBatch(tt.fn_fn.from(batchCb, null));
    r.setSchedule(tt.fn_fn.from(batchCb, null));
    r.setUntrack(tt.TableReactivityBindings_untrack.from(untrackCb, null));
    r.setCreateWritableAtom(tt.TableReactivityBindings_createWritableAtom.from(createWritable, null));
    r.setCreateReadonlyAtom(tt.TableReactivityBindings_createReadonlyAtom.from(createReadonly, null));
    return r;
}

fn colDef(key: []const u8, header: []const u8) tt.JsValue {
    const c = tt.makeObjectMap();
    c.setAt("accessorKey", tt.JsValue.string(key));
    c.setAt("header", tt.JsValue.string(header));
    return .{ .h = c.h };
}
fn taskRow(id: []const u8, task: []const u8, owner: []const u8, zone: []const u8, cpu: f64, status: []const u8) tt.JsValue {
    const r = tt.makeObjectMap();
    r.setAt("id", tt.JsValue.string(id));
    r.setAt("task", tt.JsValue.string(task));
    r.setAt("owner", tt.JsValue.string(owner));
    r.setAt("zone", tt.JsValue.string(zone));
    r.setAt("cpu", tt.JsValue.number(cpu));
    r.setAt("status", tt.JsValue.string(status));
    return .{ .h = r.h };
}
fn appendEsc(w: *std.Io.Writer, s: []const u8) !void {
    for (s) |c| {
        switch (c) {
            '&' => try w.writeAll("&amp;"),
            '<' => try w.writeAll("&lt;"),
            '>' => try w.writeAll("&gt;"),
            else => try w.writeByte(c),
        }
    }
}

var g_table: tt.Table_Core = undefined;
var g_wrap: dom.HTMLElement = undefined;
var g_meta: dom.HTMLElement = undefined;
var g_zone_vis: bool = false;
var g_owner_vis: bool = true;
var html_buf: [16384]u8 = undefined;
var meta_buf: [256]u8 = undefined;
var col_ids_buf: [8][16]u8 = undefined;
var col_lens: [8]usize = undefined;
var tmp_buf: [128]u8 = undefined;

fn render() void {
    const pin = tt.Table_ColumnPinning{ .h = g_table.h };
    const vis = tt.Table_ColumnVisibility{ .h = g_table.h };
    const sel = tt.Table_RowSelection{ .h = g_table.h };

    const groups = [3]tt.JsArrayOf(tt.Column_Core){
        tt.callTableColumnPinningGetCenterLeafColumns(pin.getGetStartVisibleLeafColumns()),
        tt.callTableColumnPinningGetCenterLeafColumns(pin.getGetCenterVisibleLeafColumns()),
        tt.callTableColumnPinningGetCenterLeafColumns(pin.getGetEndVisibleLeafColumns()),
    };

    var ncol: usize = 0;
    var w = std.Io.Writer.fixed(&html_buf);
    w.writeAll("<table><thead><tr><th>Sel</th>") catch return;
    for (groups, 0..) |grp, g_idx| {
        const gn = grp.len();
        for (0..gn) |j| {
            if (ncol >= 8) break;
            const col = grp.get(@intCast(j));
            const cp = tt.Column_CoreProperties{ .h = col.h };
            const cid = cp.getId().copy(&col_ids_buf[ncol]);
            col_lens[ncol] = cid.len;
            const is_cpu = std.mem.eql(u8, cid, "cpu");
            w.writeAll(if (is_cpu) "<th class=\"num\">" else "<th>") catch return;
            const hdr = getProp(.{ .h = cp.getColumnDef().h }, "header").toString().copy(&tmp_buf);
            appendEsc(&w, hdr) catch return;
            if (g_idx != 1) w.writeAll("<span class=\"pin-tag\">PIN</span>") catch return;
            w.writeAll("</th>") catch return;
            ncol += 1;
        }
    }
    w.writeAll("</tr></thead><tbody>") catch return;

    const rm = tt.callVoidToRowModelFn((tt.Table_RowModels_Core{ .h = g_table.h }).getGetRowModel());
    const rows = rm.getRows();
    const nrows = rows.len();
    for (0..nrows) |i| {
        const row = rows.get(@intCast(i));
        const gv = (tt.Row_Row{ .h = row.h }).getGetValue();
        const is_sel = tt.callVoidToBooleanFn((tt.Row_RowSelection{ .h = row.h }).getGetIsSelected());
        w.writeAll(if (is_sel) "<tr class=\"selected-row\"><td>&#9632;</td>" else "<tr><td>&#9633;</td>") catch return;
        for (0..ncol) |c| {
            const cid = col_ids_buf[c][0..col_lens[c]];
            if (std.mem.eql(u8, cid, "cpu")) {
                w.print("<td class=\"num\">{d:.0}%</td>", .{tt.callStringFn(gv, "cpu").toNumber()}) catch return;
            } else {
                w.writeAll("<td>") catch return;
                appendEsc(&w, tt.callStringFn(gv, cid).toString().copy(&tmp_buf)) catch return;
                w.writeAll("</td>") catch return;
            }
        }
        w.writeAll("</tr>") catch return;
    }
    w.writeAll("</tbody></table>") catch return;
    g_wrap.asElement().setInnerHTML(w.buffered());

    const vis_n = tt.callTableColumnPinningGetCenterLeafColumns(vis.getGetVisibleLeafColumns()).len();
    const all_n = tt.callTableColumnPinningGetCenterLeafColumns((tt.Table_Columns{ .h = g_table.h }).getGetAllLeafColumns()).len();
    const sel_n = tt.callVoidToRowModelFn(sel.getGetSelectedRowModel()).getRows().len();
    const m = std.fmt.bufPrint(&meta_buf, "{d} / {d} visible columns · start:id end:status pinned · {d} / {d} rows selected", .{
        vis_n, all_n, sel_n, nrows,
    }) catch "Layout";
    g_meta.asNode().setTextContent(m);
}

fn applyVisibility() void {
    const cv = tt.makeObjectMap();
    cv.setAt("zone", tt.JsValue.boolean(g_zone_vis));
    cv.setAt("owner", tt.JsValue.boolean(g_owner_vis));
    tt.callTableColumnVisibilitySetColumnVisibility((tt.Table_ColumnVisibility{ .h = g_table.h }).getSetColumnVisibility(), .{ .h = cv.h });
}

const BTN_IDS = [5]usize{ 0, 1, 2, 3, 4 };
fn onClick(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const idx = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    const pin = tt.Table_ColumnPinning{ .h = g_table.h };
    const sel = tt.Table_RowSelection{ .h = g_table.h };
    if (idx == 0) {
        g_zone_vis = !g_zone_vis;
        applyVisibility();
    } else if (idx == 1) {
        g_owner_vis = !g_owner_vis;
        applyVisibility();
    } else if (idx == 2) {
        tt.callTableRowSelectionToggleAllPageRowsSelected(sel.getToggleAllRowsSelected(), .{ .value = true });
    } else if (idx == 3) {
        tt.callTableRowSelectionToggleAllPageRowsSelected(sel.getToggleAllRowsSelected(), .{ .value = false });
    } else if (idx == 4) {
        g_zone_vis = false;
        g_owner_vis = true;
        applyVisibility();
        const st = tt.JsArray.new();
        st.push(tt.JsValue.string("id"));
        const en = tt.JsArray.new();
        en.push(tt.JsValue.string("status"));
        const cp = tt.makeObjectMap();
        cp.setAt("start", .{ .h = st.h });
        cp.setAt("end", .{ .h = en.h });
        tt.callTableColumnPinningSetColumnPinning(pin.getSetColumnPinning(), .{ .h = cp.h });
        const rs = tt.makeObjectMap();
        rs.setAt("0", tt.JsValue.boolean(true));
        rs.setAt("2", tt.JsValue.boolean(true));
        rs.setAt("4", tt.JsValue.boolean(true));
        tt.callTableRowSelectionSetRowSelection(sel.getSetRowSelection(), .{ .h = rs.h });
    }
    render();
}

export fn start() void {
    const features = tt.makeObjectMap();
    features.setAt("coreReactivityFeature", .{ .h = makeReactivity().h });
    features.setAt("columnVisibilityFeature", .{ .h = tt.getColumnVisibilityFeature().h });
    features.setAt("columnPinningFeature", .{ .h = tt.getColumnPinningFeature().h });
    features.setAt("rowSelectionFeature", .{ .h = tt.getRowSelectionFeature().h });
    features.setAt("coreRowModel", .{ .h = tt.createCoreRowModel().h });

    const data = tt.JsArray.new();
    data.push(taskRow("T-101", "ingest-wal-sync", "a.meyer", "eu-ch-1", 42, "running"));
    data.push(taskRow("T-102", "index-compact", "b.rossi", "eu-ch-2", 68, "running"));
    data.push(taskRow("T-103", "snapshot-verify", "c.keller", "eu-de-1", 15, "queued"));
    data.push(taskRow("T-104", "ledger-reconcile", "d.weber", "eu-de-2", 79, "running"));
    data.push(taskRow("T-105", "cert-rotation", "e.morel", "eu-fr-1", 24, "ready"));
    data.push(taskRow("T-106", "shard-rebalance", "f.novak", "eu-fr-2", 55, "running"));
    data.push(taskRow("T-107", "backup-archive", "g.steiner", "eu-ch-1", 31, "ready"));

    const cols = tt.JsArray.new();
    cols.push(colDef("id", "Task ID"));
    cols.push(colDef("task", "Workload"));
    cols.push(colDef("owner", "Owner"));
    cols.push(colDef("zone", "Zone"));
    cols.push(colDef("cpu", "CPU %"));
    cols.push(colDef("status", "Status"));

    const cv = tt.makeObjectMap();
    cv.setAt("zone", tt.JsValue.boolean(false));

    const st = tt.JsArray.new();
    st.push(tt.JsValue.string("id"));
    const en = tt.JsArray.new();
    en.push(tt.JsValue.string("status"));
    const cp = tt.makeObjectMap();
    cp.setAt("start", .{ .h = st.h });
    cp.setAt("end", .{ .h = en.h });

    const rs = tt.makeObjectMap();
    rs.setAt("0", tt.JsValue.boolean(true));
    rs.setAt("2", tt.JsValue.boolean(true));
    rs.setAt("4", tt.JsValue.boolean(true));

    const init_state = tt.makeObjectMap();
    init_state.setAt("columnVisibility", .{ .h = cv.h });
    init_state.setAt("columnPinning", .{ .h = cp.h });
    init_state.setAt("rowSelection", .{ .h = rs.h });

    const opts = tt.makeObjectMap();
    opts.setAt("features", .{ .h = features.h });
    opts.setAt("data", .{ .h = data.h });
    opts.setAt("columns", .{ .h = cols.h });
    opts.setAt("initialState", .{ .h = init_state.h });

    g_table = tt.constructTable(.{ .h = opts.h });
    _ = tt.jsRetain(g_table);
    const doc = dom.getDocument();
    g_wrap = doc.getElementById("table-wrap").?;
    g_meta = doc.getElementById("meta").?;

    const btn_ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (btn_ids, 0..) |bid, i| {
        const btn = doc.getElementById(bid).?;
        btn.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onClick, @constCast(&BTN_IDS[i])), .{});
    }
    render();
}
