const std = @import("std");
const dom = @import("dom");
const tt = @import("tanstack_table_core");

const NONE = tt.JsValue{ .h = 0 };
fn getProp(obj: tt.JsValue, key: []const u8) tt.JsValue {
    return (tt.RowData_map{ .h = obj.h }).getAt(key);
}
fn storeState(tbl: tt.Table_Core) tt.JsValue {
    const store_v = tt.JsValue{ .h = (tt.Table_CoreProperties{ .h = tbl.h }).getStore().h };
    return tt.functionalUpdate(getProp(store_v, "get"), NONE);
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

fn colDef(key: []const u8, header: []const u8, agg_fn: ?[]const u8) tt.JsValue {
    const c = tt.makeObjectMap();
    c.setAt("accessorKey", tt.JsValue.string(key));
    c.setAt("header", tt.JsValue.string(header));
    if (agg_fn) |a| c.setAt("aggregationFn", tt.JsValue.string(a));
    return .{ .h = c.h };
}
fn hostRow(region: []const u8, host: []const u8, role: []const u8, cpu: f64, mem: f64) tt.JsValue {
    const r = tt.makeObjectMap();
    r.setAt("region", tt.JsValue.string(region));
    r.setAt("host", tt.JsValue.string(host));
    r.setAt("role", tt.JsValue.string(role));
    r.setAt("cpu", tt.JsValue.number(cpu));
    r.setAt("mem", tt.JsValue.number(mem));
    r.setAt("nodes", tt.JsValue.number(1.0));
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
var html_buf: [16384]u8 = undefined;
var meta_buf: [256]u8 = undefined;
var grp_col_buf: [32]u8 = undefined;
var tmp_buf: [128]u8 = undefined;

fn render() void {
    const rm = tt.callVoidToRowModelFn((tt.Table_RowModels_Core{ .h = g_table.h }).getGetRowModel());
    const rows = rm.getRows();
    const nrows = rows.len();

    const grp_arr = tt.JsArray{ .h = getProp(storeState(g_table), "grouping").h };
    const grp_col: []const u8 = if (grp_arr.len() > 0)
        grp_arr.get(0).toString().copy(&grp_col_buf)
    else
        "region";

    var w = std.Io.Writer.fixed(&html_buf);
    w.writeAll("<table><thead><tr><th>Group / Host</th><th>Region</th><th>Role</th><th class=\"num\">CPU %</th><th class=\"num\">Mem GB</th><th class=\"num\">Nodes</th></tr></thead><tbody>") catch return;
    var groups: usize = 0;
    for (0..nrows) |i| {
        const row = rows.get(@intCast(i));
        const gv = (tt.Row_Row{ .h = row.h }).getGetValue();
        const is_grp = tt.callVoidToBooleanFn((tt.Row_ColumnGrouping{ .h = row.h }).getGetIsGrouped());
        const cpu = tt.callStringFn(gv, "cpu").toNumber();
        const mem = tt.callStringFn(gv, "mem").toNumber();
        const nodes = tt.callStringFn(gv, "nodes").toNumber();
        if (is_grp) {
            groups += 1;
            const is_exp = tt.callVoidToBooleanFn((tt.Row_RowExpanding{ .h = row.h }).getGetIsExpanded());
            const sub_n = (tt.Row_CoreProperties{ .h = row.h }).getSubRows().len();
            w.writeAll("<tr class=\"group-row\"><td>") catch return;
            w.writeAll(if (is_exp) "[-] " else "[+] ") catch return;
            appendEsc(&w, grp_col) catch return;
            w.writeAll("=") catch return;
            appendEsc(&w, tt.callStringFn(gv, grp_col).toString().copy(&tmp_buf)) catch return;
            w.print(" ({d})</td><td>", .{sub_n}) catch return;
            appendEsc(&w, tt.callStringFn(gv, "region").toString().copy(&tmp_buf)) catch return;
            w.writeAll("</td><td>") catch return;
            appendEsc(&w, tt.callStringFn(gv, "role").toString().copy(&tmp_buf)) catch return;
            w.print("</td><td class=\"num\">{d:.1}</td><td class=\"num\">{d:.0}</td><td class=\"num\">{d:.0}</td></tr>", .{ cpu, mem, nodes }) catch return;
        } else {
            w.writeAll("<tr><td class=\"child-indent\">") catch return;
            appendEsc(&w, tt.callStringFn(gv, "host").toString().copy(&tmp_buf)) catch return;
            w.writeAll("</td><td>") catch return;
            appendEsc(&w, tt.callStringFn(gv, "region").toString().copy(&tmp_buf)) catch return;
            w.writeAll("</td><td>") catch return;
            appendEsc(&w, tt.callStringFn(gv, "role").toString().copy(&tmp_buf)) catch return;
            w.print("</td><td class=\"num\">{d:.0}</td><td class=\"num\">{d:.0}</td><td class=\"num\">{d:.0}</td></tr>", .{ cpu, mem, nodes }) catch return;
        }
    }
    w.writeAll("</tbody></table>") catch return;
    g_wrap.asElement().setInnerHTML(w.buffered());

    const m = std.fmt.bufPrint(&meta_buf, "Grouped by {s} · {d} groups · {d} visible rows", .{ grp_col, groups, nrows }) catch "Grouped";
    g_meta.asNode().setTextContent(m);
}

const BTN_IDS = [4]usize{ 0, 1, 2, 3 };
fn onClick(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const idx = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    const grp = tt.Table_ColumnGrouping{ .h = g_table.h };
    const exp = tt.Table_RowExpanding{ .h = g_table.h };
    if (idx == 0) {
        tt.callRowRowExpandingToggleExpanded(exp.getToggleAllRowsExpanded(), .{ .expanded = true });
    } else if (idx == 1) {
        tt.callRowRowExpandingToggleExpanded(exp.getToggleAllRowsExpanded(), .{ .expanded = false });
    } else {
        const next = tt.JsArray.new();
        next.push(tt.JsValue.string(if (idx == 2) "region" else "role"));
        tt.callTableColumnGroupingSetGrouping(grp.getSetGrouping(), .{ .h = next.h });
        tt.callTableRowExpandingSetExpanded(exp.getSetExpanded(), tt.JsValue.boolean(true));
    }
    render();
}

export fn start() void {
    const agg_fns = tt.makeObjectMap();
    agg_fns.setAt("sum", .{ .h = tt.getAggregationFnSum().h });
    agg_fns.setAt("mean", .{ .h = tt.getAggregationFnMean().h });
    agg_fns.setAt("count", .{ .h = tt.getAggregationFnCount().h });

    const features = tt.makeObjectMap();
    features.setAt("coreReactivityFeature", .{ .h = makeReactivity().h });
    features.setAt("columnGroupingFeature", .{ .h = tt.getColumnGroupingFeature().h });
    features.setAt("rowAggregationFeature", .{ .h = tt.getRowAggregationFeature().h });
    features.setAt("rowExpandingFeature", .{ .h = tt.getRowExpandingFeature().h });
    features.setAt("coreRowModel", .{ .h = tt.createCoreRowModel().h });
    features.setAt("groupedRowModel", .{ .h = tt.createGroupedRowModel().h });
    features.setAt("expandedRowModel", .{ .h = tt.createExpandedRowModel().h });
    features.setAt("aggregationFns", .{ .h = agg_fns.h });

    const data = tt.JsArray.new();
    data.push(hostRow("CH", "zurich-api-01", "api", 72, 64));
    data.push(hostRow("CH", "basel-db-01", "db", 58, 128));
    data.push(hostRow("CH", "geneva-edge-01", "edge", 35, 32));
    data.push(hostRow("DE", "munich-api-02", "api", 84, 64));
    data.push(hostRow("DE", "berlin-db-02", "db", 66, 128));
    data.push(hostRow("DE", "frankfurt-edge-02", "edge", 42, 32));
    data.push(hostRow("FR", "paris-api-03", "api", 60, 64));
    data.push(hostRow("FR", "lyon-db-03", "db", 48, 128));
    data.push(hostRow("FR", "marseille-edge-03", "edge", 30, 32));

    const cols = tt.JsArray.new();
    cols.push(colDef("region", "Region", null));
    cols.push(colDef("host", "Host", null));
    cols.push(colDef("role", "Role", null));
    cols.push(colDef("cpu", "CPU % (mean)", "mean"));
    cols.push(colDef("mem", "Mem GB (sum)", "sum"));
    cols.push(colDef("nodes", "Nodes (count)", "count"));

    const init_grp = tt.JsArray.new();
    init_grp.push(tt.JsValue.string("region"));
    const init_state = tt.makeObjectMap();
    init_state.setAt("grouping", .{ .h = init_grp.h });
    init_state.setAt("expanded", tt.JsValue.boolean(true));

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

    const btn_ids = [4][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3" };
    for (btn_ids, 0..) |bid, i| {
        const btn = doc.getElementById(bid).?;
        btn.addEventListener(.click, dom.JsListener(dom.PointerEvent).from(onClick, @constCast(&BTN_IDS[i])), .{});
    }
    render();
}
