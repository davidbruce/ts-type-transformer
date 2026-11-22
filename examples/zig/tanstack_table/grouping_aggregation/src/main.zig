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
    btns: [4]BtnCtx = undefined,
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};
var html_buf: [16384]u8 = undefined;
var meta_buf: [256]u8 = undefined;
var grp_col_buf: [32]u8 = undefined;
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
            .set = tt.Updater_fn2.from(atomSet, a),
            .subscribe = tt.Updater_fn2.fromFn(atomSub),
        }).retain();
    }
    return tt.JsValue.obj(.{
        .get = tt.void_to_any_fn.from(atomGet, a),
        .subscribe = tt.Updater_fn2.fromFn(atomSub),
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

fn colDef(key: []const u8, header: []const u8, agg_fn: ?[]const u8) tt.JsValue {
    if (agg_fn) |a| {
        return tt.JsValue.obj(.{ .accessorKey = key, .header = header, .aggregationFn = a });
    }
    return tt.JsValue.obj(.{ .accessorKey = key, .header = header });
}

fn hostRow(region: []const u8, host: []const u8, role: []const u8, cpu: f64, mem: f64) tt.JsValue {
    return tt.JsValue.obj(.{
        .region = region,
        .host = host,
        .role = role,
        .cpu = cpu,
        .mem = mem,
        .nodes = 1.0,
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

fn render(app: *App) void {
    const scope = tt.JsScope.open();
    defer scope.deinit();

    const rm = app.table.as(tt.Table_RowModels_Core).getRowModel();
    const rows = rm.getRows();
    const nrows = rows.len();

    const grp_arr = getProp(storeState(app.table), "grouping").as(tt.JsArray);
    const grp_col: []const u8 = if (grp_arr.len() > 0)
        grp_arr.get(0).toString().copy(&grp_col_buf)
    else
        "region";

    var w = std.Io.Writer.fixed(&html_buf);
    w.writeAll("<table><thead><tr><th>Group / Host</th><th>Region</th><th>Role</th><th class=\"num\">CPU %</th><th class=\"num\">Mem GB</th><th class=\"num\">Nodes</th></tr></thead><tbody>") catch return;
    var groups: usize = 0;
    for (0..nrows) |i| {
        const row = rows.get(@intCast(i));
        const r_row = row.as(tt.Row_Row);
        const is_grp = row.as(tt.Row_ColumnGrouping).getIsGrouped();
        const cpu = r_row.getValue("cpu").toNumber();
        const mem = r_row.getValue("mem").toNumber();
        const nodes = r_row.getValue("nodes").toNumber();
        if (is_grp) {
            groups += 1;
            const is_exp = row.as(tt.Row_RowExpanding).getIsExpanded();
            const sub_n = row.as(tt.Row_CoreProperties).getSubRows().len();
            w.writeAll("<tr class=\"group-row\"><td>") catch return;
            w.writeAll(if (is_exp) "[-] " else "[+] ") catch return;
            appendEsc(&w, grp_col) catch return;
            w.writeAll("=") catch return;
            appendEsc(&w, r_row.getValue(grp_col).toString().copy(&tmp_buf)) catch return;
            w.print(" ({d})</td><td>", .{sub_n}) catch return;
            appendEsc(&w, r_row.getValue("region").toString().copy(&tmp_buf)) catch return;
            w.writeAll("</td><td>") catch return;
            appendEsc(&w, r_row.getValue("role").toString().copy(&tmp_buf)) catch return;
            w.print("</td><td class=\"num\">{d:.1}</td><td class=\"num\">{d:.0}</td><td class=\"num\">{d:.0}</td></tr>", .{ cpu, mem, nodes }) catch return;
        } else {
            w.writeAll("<tr><td class=\"child-indent\">") catch return;
            appendEsc(&w, r_row.getValue("host").toString().copy(&tmp_buf)) catch return;
            w.writeAll("</td><td>") catch return;
            appendEsc(&w, r_row.getValue("region").toString().copy(&tmp_buf)) catch return;
            w.writeAll("</td><td>") catch return;
            appendEsc(&w, r_row.getValue("role").toString().copy(&tmp_buf)) catch return;
            w.print("</td><td class=\"num\">{d:.0}</td><td class=\"num\">{d:.0}</td><td class=\"num\">{d:.0}</td></tr>", .{ cpu, mem, nodes }) catch return;
        }
    }
    w.writeAll("</tbody></table>") catch return;
    app.wrap.setInnerHTML(w.buffered());

    const m = std.fmt.bufPrint(&meta_buf, "Grouped by {s} · {d} groups · {d} visible rows", .{ grp_col, groups, nrows }) catch "Grouped";
    app.meta.setTextContent(m);
}

fn onClick(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const scope = tt.JsScope.open();
    defer scope.deinit();

    const app = ctx.app;
    const grp = app.table.as(tt.Table_ColumnGrouping);
    const exp = app.table.as(tt.Table_RowExpanding);
    if (ctx.id == 0) {
        exp.toggleAllRowsExpanded(true);
    } else if (ctx.id == 1) {
        exp.toggleAllRowsExpanded(false);
    } else {
        grp.setGrouping(&[_][]const u8{if (ctx.id == 2) "region" else "role"});
        exp.setExpanded(true);
    }
    render(app);
}

export fn start() void {
    const scope = tt.JsScope.open();
    defer scope.deinit();

    const opts = tt.JsValue.obj(.{
        .features = .{
            .coreReactivityFeature = makeReactivity(&g_app),
            .columnGroupingFeature = tt.getColumnGroupingFeature(),
            .rowAggregationFeature = tt.getRowAggregationFeature(),
            .rowExpandingFeature = tt.getRowExpandingFeature(),
            .coreRowModel = tt.createCoreRowModel(),
            .groupedRowModel = tt.createGroupedRowModel(),
            .expandedRowModel = tt.createExpandedRowModel(),
            .aggregationFns = .{
                .sum = tt.getAggregationFnSum(),
                .mean = tt.getAggregationFnMean(),
                .count = tt.getAggregationFnCount(),
            },
        },
        .data = .{
            hostRow("CH", "zurich-api-01", "api", 72, 64),
            hostRow("CH", "basel-db-01", "db", 58, 128),
            hostRow("CH", "geneva-edge-01", "edge", 35, 32),
            hostRow("DE", "munich-api-02", "api", 84, 64),
            hostRow("DE", "berlin-db-02", "db", 66, 128),
            hostRow("DE", "frankfurt-edge-02", "edge", 42, 32),
            hostRow("FR", "paris-api-03", "api", 60, 64),
            hostRow("FR", "lyon-db-03", "db", 48, 128),
            hostRow("FR", "marseille-edge-03", "edge", 30, 32),
        },
        .columns = .{
            colDef("region", "Region", null),
            colDef("host", "Host", null),
            colDef("role", "Role", null),
            colDef("cpu", "CPU % (mean)", "mean"),
            colDef("mem", "Mem GB (sum)", "sum"),
            colDef("nodes", "Nodes (count)", "count"),
        },
        .initialState = .{
            .grouping = &.{"region"},
            .expanded = true,
        },
    });

    g_app.table = tt.constructTable(opts.as(tt.TableOptions_Core)).retain();
    const doc = dom.getDocument();
    g_app.wrap = doc.getElementById("table-wrap").?.retain();
    g_app.meta = doc.getElementById("meta").?.retain();

    const btn_ids = [4][]const u8{ "btn-0", "btn-1", "btn-2", "btn-3" };
    for (btn_ids, 0..) |bid, i| {
        g_app.btns[i] = .{ .app = &g_app, .id = i };
        doc.getElementById(bid).?.addEventListener(.click, .{ onClick, &g_app.btns[i] }, null);
    }
    render(&g_app);
}
