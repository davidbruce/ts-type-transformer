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
    if (a.is_ro) return tt.callVoidFn(.{ .h = a.derive.h });
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
fn noUnsub(_: ?*anyopaque) tt.JsValue {
    return NONE;
}
fn atomSub(_: ?*anyopaque, _: tt.JsValue) tt.JsValue {
    const sub = tt.makeObjectMap();
    sub.setAt("unsubscribe", .{ .h = tt.void_fn.from(noUnsub, null).h });
    return .{ .h = sub.h };
}
fn makeAtom(a: *AtomState) tt.JsValue {
    const atom = tt.makeObjectMap();
    atom.setAt("get", .{ .h = tt.void_fn.from(atomGet, a).h });
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
fn createReadonly(_: ?*anyopaque, fn_v: tt.void_fn, _: ?tt.TableAtomOptions) tt.ReadonlyAtom {
    const a = &atoms[atom_count];
    atom_count += 1;
    a.* = .{ .derive = .{ .h = fn_v.h }, .is_ro = true };
    _ = tt.jsRetain(fn_v);
    return .{ .h = makeAtom(a).h };
}
fn addSub(_: ?*anyopaque, _: tt.Subscription) void {}
fn unmountCb(_: ?*anyopaque) void {}
fn batchCb(_: ?*anyopaque, fn_v: tt.Column_ColumnGrouping_toggleGrouping) void {
    tt.callColumnColumnGroupingToggleGrouping(fn_v);
}
fn untrackCb(_: ?*anyopaque, fn_v: tt.void_fn) tt.JsValue {
    return tt.callVoidFn(fn_v);
}

fn makeReactivity() tt.TableReactivityBindings {
    const r = tt.TableReactivityBindings.create();
    r.setCreateOptionsStore(false);
    r.setWrapExternalAtoms(false);
    r.setAddSubscription(tt.TableReactivityBindings_addSubscription.from(addSub, null));
    r.setUnmount(tt.Column_ColumnGrouping_toggleGrouping.from(unmountCb, null));
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
fn nodeRow(id: []const u8, host: []const u8, region: []const u8, lat: f64, thr: f64, up: f64) tt.JsValue {
    const r = tt.makeObjectMap();
    r.setAt("id", tt.JsValue.string(id));
    r.setAt("host", tt.JsValue.string(host));
    r.setAt("region", tt.JsValue.string(region));
    r.setAt("latency", tt.JsValue.number(lat));
    r.setAt("throughput", tt.JsValue.number(thr));
    r.setAt("uptime", tt.JsValue.number(up));
    return .{ .h = r.h };
}
fn sortSpec(id: []const u8, desc: bool) tt.JsValue {
    const s = tt.makeObjectMap();
    s.setAt("id", tt.JsValue.string(id));
    s.setAt("desc", tt.JsValue.boolean(desc));
    return .{ .h = s.h };
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
var tmp_buf: [128]u8 = undefined;

fn render(label: []const u8) void {
    const rm = tt.callVoidToRowModelFn((tt.Table_RowModels_Core{ .h = g_table.h }).getGetRowModel());
    const rows = rm.getRows();
    const cols = tt.callTableColumnPinningGetCenterLeafColumns((tt.Table_Columns{ .h = g_table.h }).getGetAllLeafColumns());

    var w = std.Io.Writer.fixed(&html_buf);
    w.writeAll("<table><thead><tr>") catch return;
    const ncols = cols.len();
    for (0..ncols) |i| {
        const col = cols.get(@intCast(i));
        const cp = tt.Column_CoreProperties{ .h = col.h };
        const cid = cp.getId().copy(&tmp_buf);
        const is_num = std.mem.eql(u8, cid, "latency") or std.mem.eql(u8, cid, "throughput") or std.mem.eql(u8, cid, "uptime");
        w.writeAll(if (is_num) "<th class=\"num\">" else "<th>") catch return;
        const hdr = getProp(.{ .h = cp.getColumnDef().h }, "header").toString().copy(&tmp_buf);
        appendEsc(&w, hdr) catch return;
        const sd = tt.callColumnRowSortingGetIsSorted((tt.Column_RowSorting{ .h = col.h }).getGetIsSorted());
        if (sd == .string) {
            if (sd.string.eql("asc")) {
                w.writeAll(" &#9650;") catch return;
            } else if (sd.string.eql("desc")) {
                w.writeAll(" &#9660;") catch return;
            }
        }
        w.writeAll("</th>") catch return;
    }
    w.writeAll("</tr></thead><tbody>") catch return;

    const nrows = rows.len();
    var top_host_buf: [64]u8 = undefined;
    var top_host: []const u8 = "-";
    for (0..nrows) |i| {
        const row = rows.get(@intCast(i));
        const gv = (tt.Row_Row{ .h = row.h }).getGetValue();
        w.writeAll("<tr><td>") catch return;
        appendEsc(&w, tt.callStringFn(gv, "id").toString().copy(&tmp_buf)) catch return;
        w.writeAll("</td><td>") catch return;
        const host = tt.callStringFn(gv, "host").toString().copy(&tmp_buf);
        if (i == 0) {
            const cpy_len = @min(host.len, top_host_buf.len);
            @memcpy(top_host_buf[0..cpy_len], host[0..cpy_len]);
            top_host = top_host_buf[0..cpy_len];
        }
        appendEsc(&w, host) catch return;
        w.writeAll("</td><td>") catch return;
        appendEsc(&w, tt.callStringFn(gv, "region").toString().copy(&tmp_buf)) catch return;
        w.print("</td><td class=\"num\">{d:.1}</td><td class=\"num\">{d:.0}</td><td class=\"num\">{d:.2}%</td></tr>", .{
            tt.callStringFn(gv, "latency").toNumber(),
            tt.callStringFn(gv, "throughput").toNumber(),
            tt.callStringFn(gv, "uptime").toNumber(),
        }) catch return;
    }
    w.writeAll("</tbody></table>") catch return;
    g_wrap.asElement().setInnerHTML(w.buffered());

    const m = std.fmt.bufPrint(&meta_buf, "{s} · {d} rows · top node: {s}", .{ label, nrows, top_host }) catch label;
    g_meta.asNode().setTextContent(m);
}

const BTN_IDS = [5]usize{ 0, 1, 2, 3, 4 };
fn onClick(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const idx = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    const next = tt.JsArray.new();
    var label: []const u8 = "Sort: unsorted (data order)";
    if (idx == 0) {
        next.push(sortSpec("region", false));
        next.push(sortSpec("latency", false));
        label = "Sort: region asc, latency asc";
    } else if (idx == 1) {
        next.push(sortSpec("latency", false));
        label = "Sort: latency asc";
    } else if (idx == 2) {
        next.push(sortSpec("throughput", true));
        label = "Sort: throughput desc";
    } else if (idx == 3) {
        next.push(sortSpec("host", false));
        label = "Sort: host asc";
    }
    tt.callTableRowSortingSetSorting((tt.Table_RowSorting{ .h = g_table.h }).getSetSorting(), .{ .h = next.h });
    render(label);
}

export fn start() void {
    const sort_fns = tt.makeObjectMap();
    sort_fns.setAt("alphanumeric", .{ .h = tt.getSortFnAlphanumeric().h });
    sort_fns.setAt("basic", .{ .h = tt.getSortFnBasic().h });
    sort_fns.setAt("text", .{ .h = tt.getSortFnText().h });

    const features = tt.makeObjectMap();
    features.setAt("coreReactivityFeature", .{ .h = makeReactivity().h });
    features.setAt("rowSortingFeature", .{ .h = tt.getRowSortingFeature().h });
    features.setAt("coreRowModel", .{ .h = tt.createCoreRowModel().h });
    features.setAt("sortedRowModel", .{ .h = tt.createSortedRowModel().h });
    features.setAt("sortFns", .{ .h = sort_fns.h });

    const data = tt.JsArray.new();
    data.push(nodeRow("N-01", "eu-west-1a", "CH", 12.4, 4200, 99.98));
    data.push(nodeRow("N-02", "eu-west-1b", "CH", 8.7, 5100, 99.99));
    data.push(nodeRow("N-03", "eu-central-1a", "DE", 15.2, 3900, 99.91));
    data.push(nodeRow("N-04", "eu-central-1b", "DE", 6.3, 6400, 99.97));
    data.push(nodeRow("N-05", "eu-north-1a", "SE", 19.8, 2800, 99.85));
    data.push(nodeRow("N-06", "eu-north-1b", "SE", 11.1, 4750, 99.95));
    data.push(nodeRow("N-07", "us-east-1a", "US", 28.5, 7100, 99.94));
    data.push(nodeRow("N-08", "us-east-1b", "US", 21.0, 5900, 99.96));

    const cols = tt.JsArray.new();
    cols.push(colDef("id", "Node"));
    cols.push(colDef("host", "Host"));
    cols.push(colDef("region", "Region"));
    cols.push(colDef("latency", "Latency (ms)"));
    cols.push(colDef("throughput", "Req/s"));
    cols.push(colDef("uptime", "Uptime %"));

    const init_sort = tt.JsArray.new();
    init_sort.push(sortSpec("region", false));
    init_sort.push(sortSpec("latency", false));
    const init_state = tt.makeObjectMap();
    init_state.setAt("sorting", .{ .h = init_sort.h });

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
    render("Sort: region asc, latency asc");
}
