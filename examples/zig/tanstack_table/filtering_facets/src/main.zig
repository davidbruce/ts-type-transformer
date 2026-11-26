const std = @import("std");
const dom = @import("dom");
const tt = @import("tanstack_table_core");

const NONE: tt.JsValue = std.mem.zeroes(tt.JsValue);

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

fn colDef(key: []const u8, header: []const u8, filter_fn: ?[]const u8) tt.JsValue {
    if (filter_fn) |f| {
        return tt.JsValue.obj(.{ .accessorKey = key, .header = header, .filterFn = f });
    }
    return tt.JsValue.obj(.{ .accessorKey = key, .header = header });
}

fn svcRow(id: []const u8, svc: []const u8, tier: []const u8, region: []const u8, reqs: f64, errs: f64) tt.JsValue {
    return tt.JsValue.obj(.{
        .id = id,
        .service = svc,
        .tier = tier,
        .region = region,
        .requests = reqs,
        .errors = errs,
    });
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

fn render(app: *App, label: []const u8) void {
    const scope = tt.JsScope.open();
    defer scope.deinit();

    const rm = app.table.as(tt.Table_RowModels_Core).getRowModel();
    const core = app.table.as(tt.Table_RowModels_Core).getCoreRowModel();
    const rows = rm.getRows();
    const nrows = rows.len();
    const ntotal = core.getRows().len();

    var w = std.Io.Writer.fixed(&html_buf);
    w.writeAll("<table><thead><tr><th>ID</th><th>Service</th><th>Tier</th><th>Region</th><th class=\"num\">Req/s</th><th class=\"num\">Errors</th></tr></thead><tbody>") catch return;
    for (0..nrows) |i| {
        const row = rows.get(@intCast(i));
        const r_row = row.as(tt.Row_Row);
        w.writeAll("<tr><td>") catch return;
        appendEsc(&w, r_row.getValue("id").toString().copy(&tmp_buf)) catch return;
        w.writeAll("</td><td>") catch return;
        appendEsc(&w, r_row.getValue("service").toString().copy(&tmp_buf)) catch return;
        w.writeAll("</td><td>") catch return;
        appendEsc(&w, r_row.getValue("tier").toString().copy(&tmp_buf)) catch return;
        w.writeAll("</td><td>") catch return;
        appendEsc(&w, r_row.getValue("region").toString().copy(&tmp_buf)) catch return;
        w.print("</td><td class=\"num\">{d:.0}</td><td class=\"num\">{d:.0}</td></tr>", .{
            r_row.getValue("requests").toNumber(),
            r_row.getValue("errors").toNumber(),
        }) catch return;
    }
    w.writeAll("</tbody></table>") catch return;
    app.wrap.setInnerHTML(w.buffered());

    const m = std.fmt.bufPrint(&meta_buf, "{s} · showing {d} / {d} services", .{ label, nrows, ntotal }) catch label;
    app.meta.setTextContent(m);
}

fn onClick(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const scope = tt.JsScope.open();
    defer scope.deinit();

    const app = ctx.app;
    const cf = app.table.as(tt.Table_ColumnFiltering);
    const gf = app.table.as(tt.Table_GlobalFiltering);
    const filters = tt.JsArray.new();
    var label: []const u8 = "Filter: all services";
    if (ctx.id == 0) {
        gf.setGlobalFilter("");
        filters.push(.{ .id = "tier", .value = "core" });
        filters.push(.{ .id = "requests", .value = 400.0 });
        cf.setColumnFilters(filters);
        label = "Filter: tier = core & requests >= 400";
    } else if (ctx.id == 1) {
        gf.setGlobalFilter("");
        cf.setColumnFilters(filters);
        label = "Filter: all services";
    } else if (ctx.id == 2) {
        gf.setGlobalFilter("");
        filters.push(.{ .id = "tier", .value = "edge" });
        cf.setColumnFilters(filters);
        label = "Filter: tier = edge";
    } else if (ctx.id == 3) {
        cf.setColumnFilters(filters);
        gf.setGlobalFilter("auth");
        label = "Global search: \"auth\"";
    } else if (ctx.id == 4) {
        gf.setGlobalFilter("");
        filters.push(.{ .id = "errors", .value = 5.0 });
        cf.setColumnFilters(filters);
        label = "Filter: errors >= 5";
    }
    render(app, label);
}

export fn start() void {
    const scope = tt.JsScope.open();
    defer scope.deinit();

    const opts = tt.JsValue.obj(.{
        .features = .{
            .coreReactivityFeature = makeReactivity(&g_app),
            .columnFilteringFeature = tt.getColumnFilteringFeature(),
            .globalFilteringFeature = tt.getGlobalFilteringFeature(),
            .coreRowModel = tt.createCoreRowModel(),
            .filteredRowModel = tt.createFilteredRowModel(),
            .filterFns = .{
                .includesString = tt.getFilterFnIncludesString(),
                .equalsString = tt.getFilterFnEqualsString(),
                .greaterThanOrEqualTo = tt.getFilterFnGreaterThanOrEqualTo(),
            },
        },
        .data = .{
            svcRow("S-01", "auth-gateway", "core", "CH", 620, 2),
            svcRow("S-02", "session-cache", "core", "CH", 480, 1),
            svcRow("S-03", "billing-ledger", "core", "DE", 510, 6),
            svcRow("S-04", "oauth-signer", "core", "FR", 430, 3),
            svcRow("S-05", "trace-ingestor", "edge", "DE", 290, 8),
            svcRow("S-06", "cdn-router", "edge", "FR", 840, 5),
            svcRow("S-07", "static-assets", "edge", "CH", 710, 0),
            svcRow("S-08", "mail-relay", "batch", "DE", 140, 7),
            svcRow("S-09", "report-export", "batch", "FR", 210, 4),
            svcRow("S-10", "audit-auth-log", "core", "US", 360, 9),
        },
        .columns = .{
            colDef("id", "ID", null),
            colDef("service", "Service", null),
            colDef("tier", "Tier", "equalsString"),
            colDef("region", "Region", null),
            colDef("requests", "Req/s", "greaterThanOrEqualTo"),
            colDef("errors", "Errors", "greaterThanOrEqualTo"),
        },
        .initialState = .{
            .columnFilters = .{
                .{ .id = "tier", .value = "core" },
                .{ .id = "requests", .value = 400.0 },
            },
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
    render(&g_app, "Filter: tier = core & requests >= 400");
}
