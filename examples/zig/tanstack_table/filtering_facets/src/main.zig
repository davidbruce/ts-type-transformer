const std = @import("std");
const dom = @import("dom");
const tt = @import("tanstack_table_core");

const NONE = tt.JsValue{ .h = 0 };

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

fn colDef(key: []const u8, header: []const u8, filter_fn: ?[]const u8) tt.JsValue {
    const c = tt.makeObjectMap();
    c.setAt("accessorKey", tt.JsValue.string(key));
    c.setAt("header", tt.JsValue.string(header));
    if (filter_fn) |f| c.setAt("filterFn", tt.JsValue.string(f));
    return .{ .h = c.h };
}
fn svcRow(id: []const u8, svc: []const u8, tier: []const u8, region: []const u8, reqs: f64, errs: f64) tt.JsValue {
    const r = tt.makeObjectMap();
    r.setAt("id", tt.JsValue.string(id));
    r.setAt("service", tt.JsValue.string(svc));
    r.setAt("tier", tt.JsValue.string(tier));
    r.setAt("region", tt.JsValue.string(region));
    r.setAt("requests", tt.JsValue.number(reqs));
    r.setAt("errors", tt.JsValue.number(errs));
    return .{ .h = r.h };
}
fn filterSpec(id: []const u8, val: tt.JsValue) tt.JsValue {
    const f = tt.makeObjectMap();
    f.setAt("id", tt.JsValue.string(id));
    f.setAt("value", val);
    return .{ .h = f.h };
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
    const core = tt.callVoidToRowModelFn((tt.Table_RowModels_Core{ .h = g_table.h }).getGetCoreRowModel());
    const rows = rm.getRows();
    const nrows = rows.len();
    const ntotal = core.getRows().len();

    var w = std.Io.Writer.fixed(&html_buf);
    w.writeAll("<table><thead><tr><th>ID</th><th>Service</th><th>Tier</th><th>Region</th><th class=\"num\">Req/s</th><th class=\"num\">Errors</th></tr></thead><tbody>") catch return;
    for (0..nrows) |i| {
        const row = rows.get(@intCast(i));
        const gv = (tt.Row_Row{ .h = row.h }).getGetValue();
        w.writeAll("<tr><td>") catch return;
        appendEsc(&w, tt.callStringFn(gv, "id").toString().copy(&tmp_buf)) catch return;
        w.writeAll("</td><td>") catch return;
        appendEsc(&w, tt.callStringFn(gv, "service").toString().copy(&tmp_buf)) catch return;
        w.writeAll("</td><td>") catch return;
        appendEsc(&w, tt.callStringFn(gv, "tier").toString().copy(&tmp_buf)) catch return;
        w.writeAll("</td><td>") catch return;
        appendEsc(&w, tt.callStringFn(gv, "region").toString().copy(&tmp_buf)) catch return;
        w.print("</td><td class=\"num\">{d:.0}</td><td class=\"num\">{d:.0}</td></tr>", .{
            tt.callStringFn(gv, "requests").toNumber(),
            tt.callStringFn(gv, "errors").toNumber(),
        }) catch return;
    }
    w.writeAll("</tbody></table>") catch return;
    g_wrap.asElement().setInnerHTML(w.buffered());

    const m = std.fmt.bufPrint(&meta_buf, "{s} · showing {d} / {d} services", .{ label, nrows, ntotal }) catch label;
    g_meta.asNode().setTextContent(m);
}

const BTN_IDS = [5]usize{ 0, 1, 2, 3, 4 };
fn onClick(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const idx = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    const cf = tt.Table_ColumnFiltering{ .h = g_table.h };
    const gf = tt.Table_GlobalFiltering{ .h = g_table.h };
    const filters = tt.JsArray.new();
    var label: []const u8 = "Filter: all services";
    if (idx == 0) {
        tt.callUpdaterFn(tt.Updater_fn{ .h = gf.getSetGlobalFilter().h }, tt.JsValue.string(""));
        filters.push(filterSpec("tier", tt.JsValue.string("core")));
        filters.push(filterSpec("requests", tt.JsValue.number(400.0)));
        tt.callTableColumnFilteringSetColumnFilters(cf.getSetColumnFilters(), .{ .h = filters.h });
        label = "Filter: tier = core & requests >= 400";
    } else if (idx == 1) {
        tt.callUpdaterFn(tt.Updater_fn{ .h = gf.getSetGlobalFilter().h }, tt.JsValue.string(""));
        tt.callTableColumnFilteringSetColumnFilters(cf.getSetColumnFilters(), .{ .h = filters.h });
        label = "Filter: all services";
    } else if (idx == 2) {
        tt.callUpdaterFn(tt.Updater_fn{ .h = gf.getSetGlobalFilter().h }, tt.JsValue.string(""));
        filters.push(filterSpec("tier", tt.JsValue.string("edge")));
        tt.callTableColumnFilteringSetColumnFilters(cf.getSetColumnFilters(), .{ .h = filters.h });
        label = "Filter: tier = edge";
    } else if (idx == 3) {
        tt.callTableColumnFilteringSetColumnFilters(cf.getSetColumnFilters(), .{ .h = filters.h });
        tt.callUpdaterFn(tt.Updater_fn{ .h = gf.getSetGlobalFilter().h }, tt.JsValue.string("auth"));
        label = "Global search: \"auth\"";
    } else if (idx == 4) {
        tt.callUpdaterFn(tt.Updater_fn{ .h = gf.getSetGlobalFilter().h }, tt.JsValue.string(""));
        filters.push(filterSpec("errors", tt.JsValue.number(5.0)));
        tt.callTableColumnFilteringSetColumnFilters(cf.getSetColumnFilters(), .{ .h = filters.h });
        label = "Filter: errors >= 5";
    }
    render(label);
}

export fn start() void {
    const filter_fns = tt.makeObjectMap();
    filter_fns.setAt("includesString", .{ .h = tt.getFilterFnIncludesString().h });
    filter_fns.setAt("equalsString", .{ .h = tt.getFilterFnEqualsString().h });
    filter_fns.setAt("greaterThanOrEqualTo", .{ .h = tt.getFilterFnGreaterThanOrEqualTo().h });

    const features = tt.makeObjectMap();
    features.setAt("coreReactivityFeature", .{ .h = makeReactivity().h });
    features.setAt("columnFilteringFeature", .{ .h = tt.getColumnFilteringFeature().h });
    features.setAt("globalFilteringFeature", .{ .h = tt.getGlobalFilteringFeature().h });
    features.setAt("coreRowModel", .{ .h = tt.createCoreRowModel().h });
    features.setAt("filteredRowModel", .{ .h = tt.createFilteredRowModel().h });
    features.setAt("filterFns", .{ .h = filter_fns.h });

    const data = tt.JsArray.new();
    data.push(svcRow("S-01", "auth-gateway", "core", "CH", 620, 2));
    data.push(svcRow("S-02", "session-cache", "core", "CH", 480, 1));
    data.push(svcRow("S-03", "billing-ledger", "core", "DE", 510, 6));
    data.push(svcRow("S-04", "oauth-signer", "core", "FR", 430, 3));
    data.push(svcRow("S-05", "trace-ingestor", "edge", "DE", 290, 8));
    data.push(svcRow("S-06", "cdn-router", "edge", "FR", 840, 5));
    data.push(svcRow("S-07", "static-assets", "edge", "CH", 710, 0));
    data.push(svcRow("S-08", "mail-relay", "batch", "DE", 140, 7));
    data.push(svcRow("S-09", "report-export", "batch", "FR", 210, 4));
    data.push(svcRow("S-10", "audit-auth-log", "core", "US", 360, 9));

    const cols = tt.JsArray.new();
    cols.push(colDef("id", "ID", null));
    cols.push(colDef("service", "Service", null));
    cols.push(colDef("tier", "Tier", "equalsString"));
    cols.push(colDef("region", "Region", null));
    cols.push(colDef("requests", "Req/s", "greaterThanOrEqualTo"));
    cols.push(colDef("errors", "Errors", "greaterThanOrEqualTo"));

    const init_filters = tt.JsArray.new();
    init_filters.push(filterSpec("tier", tt.JsValue.string("core")));
    init_filters.push(filterSpec("requests", tt.JsValue.number(400.0)));
    const init_state = tt.makeObjectMap();
    init_state.setAt("columnFilters", .{ .h = init_filters.h });

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
    render("Filter: tier = core & requests >= 400");
}
