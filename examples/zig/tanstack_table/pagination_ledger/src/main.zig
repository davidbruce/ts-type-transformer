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

fn render() void {
    const rm = tt.callVoidToRowModelFn((tt.Table_RowModels_Core{ .h = g_table.h }).getGetRowModel());
    const core = tt.callVoidToRowModelFn((tt.Table_RowModels_Core{ .h = g_table.h }).getGetCoreRowModel());
    const rows = rm.getRows();
    const nrows = rows.len();
    const ntotal = core.getRows().len();

    var w = std.Io.Writer.fixed(&html_buf);
    w.writeAll("<table><thead><tr><th>Entry</th><th>Account</th><th>Category</th><th class=\"num\">Units</th><th class=\"num\">Amount (CHF)</th></tr></thead><tbody>") catch return;
    for (0..nrows) |i| {
        const row = rows.get(@intCast(i));
        const gv = (tt.Row_Row{ .h = row.h }).getGetValue();
        w.writeAll("<tr><td>") catch return;
        appendEsc(&w, tt.callStringFn(gv, "id").toString().copy(&tmp_buf)) catch return;
        w.writeAll("</td><td>") catch return;
        appendEsc(&w, tt.callStringFn(gv, "account").toString().copy(&tmp_buf)) catch return;
        w.writeAll("</td><td>") catch return;
        appendEsc(&w, tt.callStringFn(gv, "category").toString().copy(&tmp_buf)) catch return;
        w.print("</td><td class=\"num\">{d:.0}</td><td class=\"num\">{d:.0}</td></tr>", .{
            tt.callStringFn(gv, "units").toNumber(),
            tt.callStringFn(gv, "amount").toNumber(),
        }) catch return;
    }
    w.writeAll("</tbody></table>") catch return;
    g_wrap.asElement().setInnerHTML(w.buffered());

    const pg = getProp(storeState(g_table), "pagination");
    const page_idx: i32 = @intFromFloat(@round(getProp(pg, "pageIndex").toNumber()));
    const page_sz: i32 = @intFromFloat(@round(getProp(pg, "pageSize").toNumber()));
    const page_cnt: i32 = @intFromFloat(@round(tt.callVoidToNumberFn((tt.Table_RowPagination{ .h = g_table.h }).getGetPageCount())));
    const m = std.fmt.bufPrint(&meta_buf, "Page {d} / {d} · pageSize {d} · showing {d} of {d} rows", .{
        page_idx + 1, page_cnt, page_sz, nrows, ntotal,
    }) catch "Page";
    g_meta.asNode().setTextContent(m);
}

const BTN_IDS = [5]usize{ 0, 1, 2, 3, 4 };
fn onClick(ctx: ?*anyopaque, _: dom.PointerEvent) void {
    const idx = @as(*const usize, @ptrCast(@alignCast(ctx.?))).*;
    const pag = tt.Table_RowPagination{ .h = g_table.h };
    const pg = getProp(storeState(g_table), "pagination");
    const page_idx: f64 = getProp(pg, "pageIndex").toNumber();
    const max_idx: f64 = tt.callVoidToNumberFn(pag.getGetPageCount()) - 1.0;

    if (idx == 0) {
        if (page_idx > 0.0) tt.callTableRowPaginationSetPageIndex(pag.getSetPageIndex(), tt.JsValue.number(page_idx - 1.0));
    } else if (idx == 1) {
        if (page_idx < max_idx) tt.callTableRowPaginationSetPageIndex(pag.getSetPageIndex(), tt.JsValue.number(page_idx + 1.0));
    } else {
        const new_sz: f64 = if (idx == 2) 5.0 else if (idx == 3) 6.0 else 9.0;
        tt.callTableRowPaginationSetPageIndex(pag.getSetPageIndex(), tt.JsValue.number(0.0));
        tt.callTableRowPaginationSetPageIndex(pag.getSetPageSize(), tt.JsValue.number(new_sz));
    }
    render();
}

export fn start() void {
    const accounts = [3][]const u8{ "ops-pool", "clearing-ch", "reserve-de" };
    const cats = [4][]const u8{ "compute", "storage", "egress", "audit" };

    const features = tt.makeObjectMap();
    features.setAt("coreReactivityFeature", .{ .h = makeReactivity().h });
    features.setAt("rowPaginationFeature", .{ .h = tt.getRowPaginationFeature().h });
    features.setAt("coreRowModel", .{ .h = tt.createCoreRowModel().h });
    features.setAt("paginatedRowModel", .{ .h = tt.createPaginatedRowModel().h });

    const data = tt.JsArray.new();
    var id_buf: [16]u8 = undefined;
    for (0..18) |i| {
        const id_s = std.fmt.bufPrint(&id_buf, "TX-{d:0>3}", .{i + 1}) catch "TX";
        const row = tt.makeObjectMap();
        row.setAt("id", tt.JsValue.string(id_s));
        row.setAt("account", tt.JsValue.string(accounts[i % 3]));
        row.setAt("category", tt.JsValue.string(cats[i % 4]));
        row.setAt("units", tt.JsValue.number(@floatFromInt((i * 7) % 19 + 4)));
        row.setAt("amount", tt.JsValue.number(@floatFromInt(((i * 37) % 90 + 15) * 12)));
        data.push(.{ .h = row.h });
    }

    const cols = tt.JsArray.new();
    cols.push(colDef("id", "Entry"));
    cols.push(colDef("account", "Account"));
    cols.push(colDef("category", "Category"));
    cols.push(colDef("units", "Units"));
    cols.push(colDef("amount", "Amount (CHF)"));

    const pg = tt.makeObjectMap();
    pg.setAt("pageIndex", tt.JsValue.number(0.0));
    pg.setAt("pageSize", tt.JsValue.number(5.0));
    const init_state = tt.makeObjectMap();
    init_state.setAt("pagination", .{ .h = pg.h });

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
