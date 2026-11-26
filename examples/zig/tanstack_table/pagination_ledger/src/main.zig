const std = @import("std");
const dom = @import("dom");
const tt = @import("tanstack_table_core");

const NONE: tt.JsValue = std.mem.zeroes(tt.JsValue);

fn getProp(obj: tt.JsValue, key: []const u8) tt.JsValue {
    return obj.as(tt.RowData_map).getAt(key);
}

fn storeState(tbl: tt.Table_Core) tt.JsValue {
    const store_v = tbl.as(tt.Table_CoreProperties).getStore().as(tt.JsValue);
    return tt.functionalUpdate(getProp(store_v, "get"), NONE);
}

const AtomState = struct {
    value: tt.JsValue = NONE,
    derive: tt.void_to_any_fn = std.mem.zeroes(tt.void_to_any_fn),
    is_ro: bool = false,
};

const App = struct {
    table: tt.Table_Core = undefined,
    wrap: dom.HTMLElement = undefined,
    meta: dom.HTMLElement = undefined,
    atoms: [512]AtomState = undefined,
    atom_count: usize = 0,
    btns: [5]BtnCtx = undefined,
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};
var html_buf: [16384]u8 = undefined;
var meta_buf: [256]u8 = undefined;
var tmp_buf: [128]u8 = undefined;

fn atomGet(a: *AtomState) tt.JsValue {
    if (a.is_ro) return a.derive.call();
    return a.value.retain();
}

fn atomSet(a: *AtomState, updater: tt.JsValue) tt.JsValue {
    const next = tt.functionalUpdate(updater, a.value);
    if (a.value.h != 0) a.value.deinit();
    a.value = next;
    return NONE;
}

fn noUnsub() void {}

fn atomSub(_: tt.JsValue) tt.JsValue {
    const scope = tt.JsScope.open();
    defer scope.deinit();
    return tt.JsValue.obj(.{ .unsubscribe = tt.void_fn.fromFn(noUnsub) }).retain();
}

fn makeAtom(a: *AtomState) tt.JsValue {
    const scope = tt.JsScope.open();
    defer scope.deinit();
    if (!a.is_ro) {
        return tt.JsValue.obj(.{
            .get = tt.void_to_any_fn.from(atomGet, a),
            .set = tt.UpdaterFn.from(atomSet, a),
            .subscribe = tt.UpdaterFn.fromFn(atomSub),
        }).retain();
    }
    return tt.JsValue.obj(.{
        .get = tt.void_to_any_fn.from(atomGet, a),
        .subscribe = tt.UpdaterFn.fromFn(atomSub),
    }).retain();
}

fn createWritable(app: *App, initial: tt.JsValue, _: ?tt.TableAtomOptions) tt.Atom {
    const a = &app.atoms[app.atom_count];
    app.atom_count += 1;
    a.* = .{ .value = initial.retain() };
    return makeAtom(a).as(tt.Atom);
}

fn createReadonly(app: *App, fn_v: tt.void_to_any_fn, _: ?tt.TableAtomOptions) tt.ReadonlyAtom {
    const a = &app.atoms[app.atom_count];
    app.atom_count += 1;
    a.* = .{ .derive = fn_v.retain(), .is_ro = true };
    return makeAtom(a).as(tt.ReadonlyAtom);
}

fn addSub(_: tt.Subscription) void {}
fn unmountCb() void {}
fn batchCb(fn_v: tt.void_fn) void {
    fn_v.call();
}
fn untrackCb(fn_v: tt.void_to_any_fn) tt.JsValue {
    return fn_v.call();
}

fn makeReactivity(app: *App) tt.TableReactivityBindings {
    return tt.TableReactivityBindings.init(.{
        .createOptionsStore = false,
        .wrapExternalAtoms = false,
        .addSubscription = addSub,
        .unmount = unmountCb,
        .batch = batchCb,
        .schedule = batchCb,
        .untrack = untrackCb,
        .createWritableAtom = .{ createWritable, app },
        .createReadonlyAtom = .{ createReadonly, app },
    });
}

fn colDef(key: []const u8, header: []const u8) tt.JsValue {
    return tt.JsValue.obj(.{ .accessorKey = key, .header = header });
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

fn render(app: *App) void {
    const scope = tt.JsScope.open();
    defer scope.deinit();

    const rm = app.table.as(tt.Table_RowModels_Core).getRowModel();
    const core = app.table.as(tt.Table_RowModels_Core).getCoreRowModel();
    const rows = rm.getRows();
    const nrows = rows.len();
    const ntotal = core.getRows().len();

    var w = std.Io.Writer.fixed(&html_buf);
    w.writeAll("<table><thead><tr><th>Entry</th><th>Account</th><th>Category</th><th class=\"num\">Units</th><th class=\"num\">Amount (CHF)</th></tr></thead><tbody>") catch return;
    for (0..nrows) |i| {
        const row = rows.get(@intCast(i));
        const r_row = row.as(tt.Row_Row);
        w.writeAll("<tr><td>") catch return;
        appendEsc(&w, r_row.getValue("id").toString().copy(&tmp_buf)) catch return;
        w.writeAll("</td><td>") catch return;
        appendEsc(&w, r_row.getValue("account").toString().copy(&tmp_buf)) catch return;
        w.writeAll("</td><td>") catch return;
        appendEsc(&w, r_row.getValue("category").toString().copy(&tmp_buf)) catch return;
        w.print("</td><td class=\"num\">{d:.0}</td><td class=\"num\">{d:.0}</td></tr>", .{
            r_row.getValue("units").toNumber(),
            r_row.getValue("amount").toNumber(),
        }) catch return;
    }
    w.writeAll("</tbody></table>") catch return;
    app.wrap.setInnerHTML(w.buffered());

    const pg = getProp(storeState(app.table), "pagination");
    const page_idx: i32 = @intFromFloat(@round(getProp(pg, "pageIndex").toNumber()));
    const page_sz: i32 = @intFromFloat(@round(getProp(pg, "pageSize").toNumber()));
    const page_cnt: i32 = @intFromFloat(@round(app.table.as(tt.Table_RowPagination).getPageCount()));
    const m = std.fmt.bufPrint(&meta_buf, "Page {d} / {d} · pageSize {d} · showing {d} of {d} rows", .{
        page_idx + 1, page_cnt, page_sz, nrows, ntotal,
    }) catch "Page";
    app.meta.setTextContent(m);
}

fn onClick(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const scope = tt.JsScope.open();
    defer scope.deinit();

    const app = ctx.app;
    const pag = app.table.as(tt.Table_RowPagination);
    const pg = getProp(storeState(app.table), "pagination");
    const page_idx: f64 = getProp(pg, "pageIndex").toNumber();
    const max_idx: f64 = pag.getPageCount() - 1.0;

    if (ctx.id == 0) {
        if (page_idx > 0.0) pag.setPageIndex(page_idx - 1.0);
    } else if (ctx.id == 1) {
        if (page_idx < max_idx) pag.setPageIndex(page_idx + 1.0);
    } else {
        const new_sz: f64 = if (ctx.id == 2) 5.0 else if (ctx.id == 3) 6.0 else 9.0;
        pag.setPageIndex(0.0);
        pag.setPageSize(new_sz);
    }
    render(app);
}

export fn start() void {
    const scope = tt.JsScope.open();
    defer scope.deinit();

    const accounts = [3][]const u8{ "ops-pool", "clearing-ch", "reserve-de" };
    const cats = [4][]const u8{ "compute", "storage", "egress", "audit" };

    const data = tt.JsArray.new();
    var id_buf: [16]u8 = undefined;
    for (0..18) |i| {
        const id_s = std.fmt.bufPrint(&id_buf, "TX-{d:0>3}", .{i + 1}) catch "TX";
        data.push(.{
            .id = id_s,
            .account = accounts[i % 3],
            .category = cats[i % 4],
            .units = (i * 7) % 19 + 4,
            .amount = ((i * 37) % 90 + 15) * 12,
        });
    }

    const opts = tt.JsValue.obj(.{
        .features = .{
            .coreReactivityFeature = makeReactivity(&g_app),
            .rowPaginationFeature = tt.getRowPaginationFeature(),
            .coreRowModel = tt.createCoreRowModel(),
            .paginatedRowModel = tt.createPaginatedRowModel(),
        },
        .data = data,
        .columns = .{
            colDef("id", "Entry"),
            colDef("account", "Account"),
            colDef("category", "Category"),
            colDef("units", "Units"),
            colDef("amount", "Amount (CHF)"),
        },
        .initialState = .{
            .pagination = .{ .pageIndex = 0.0, .pageSize = 5.0 },
        },
    });

    g_app.table = tt.constructTable(opts.as(tt.TableOptions_Core)).retain();
    const doc = dom.getDocument();
    g_app.wrap = doc.getElementById("table-wrap").?.retain();
    g_app.meta = doc.getElementById("meta").?.retain();

    const btn_ids = [5][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (btn_ids, 0..) |bid, i| {
        g_app.btns[i] = .{ .app = &g_app, .id = i };
        doc.getElementById(bid).?.addEventListener(.click, .{ onClick, &g_app.btns[i] }, null);
    }
    render(&g_app);
}
