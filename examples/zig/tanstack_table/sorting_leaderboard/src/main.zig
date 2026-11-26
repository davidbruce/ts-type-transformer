const std = @import("std");
const dom = @import("dom");
const tt = @import("tanstack_table_core");

const NONE: tt.JsValue = std.mem.zeroes(tt.JsValue);

fn getProp(obj: tt.JsValue, key: []const u8) tt.JsValue {
    return obj.as(tt.RowData_map).getAt(key);
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

fn nodeRow(id: []const u8, host: []const u8, region: []const u8, lat: f64, thr: f64, up: f64) tt.JsValue {
    return tt.JsValue.obj(.{
        .id = id,
        .host = host,
        .region = region,
        .latency = lat,
        .throughput = thr,
        .uptime = up,
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
    const rows = rm.getRows();
    const cols = app.table.as(tt.Table_Columns).getAllLeafColumns();

    var w = std.Io.Writer.fixed(&html_buf);
    w.writeAll("<table><thead><tr>") catch return;
    const ncols = cols.len();
    for (0..ncols) |i| {
        const col = cols.get(@intCast(i));
        const cp = col.as(tt.Column_CoreProperties);
        const cid = cp.getId().copy(&tmp_buf);
        const is_num = std.mem.eql(u8, cid, "latency") or std.mem.eql(u8, cid, "throughput") or std.mem.eql(u8, cid, "uptime");
        w.writeAll(if (is_num) "<th class=\"num\">" else "<th>") catch return;
        const hdr = getProp(cp.getColumnDef().as(tt.JsValue), "header").toString().copy(&tmp_buf);
        appendEsc(&w, hdr) catch return;
        if (col.as(tt.Column_RowSorting).getIsSorted()) |sd| switch (sd) {
            .asc => w.writeAll(" &#9650;") catch return,
            .desc => w.writeAll(" &#9660;") catch return,
        };
        w.writeAll("</th>") catch return;
    }
    w.writeAll("</tr></thead><tbody>") catch return;

    const nrows = rows.len();
    var top_host_buf: [64]u8 = undefined;
    var top_host: []const u8 = "-";
    for (0..nrows) |i| {
        const row = rows.get(@intCast(i));
        const r_row = row.as(tt.Row_Row);
        w.writeAll("<tr><td>") catch return;
        appendEsc(&w, r_row.getValue("id").toString().copy(&tmp_buf)) catch return;
        w.writeAll("</td><td>") catch return;
        const host = r_row.getValue("host").toString().copy(&tmp_buf);
        if (i == 0) {
            const cpy_len = @min(host.len, top_host_buf.len);
            @memcpy(top_host_buf[0..cpy_len], host[0..cpy_len]);
            top_host = top_host_buf[0..cpy_len];
        }
        appendEsc(&w, host) catch return;
        w.writeAll("</td><td>") catch return;
        appendEsc(&w, r_row.getValue("region").toString().copy(&tmp_buf)) catch return;
        w.print("</td><td class=\"num\">{d:.1}</td><td class=\"num\">{d:.0}</td><td class=\"num\">{d:.2}%</td></tr>", .{
            r_row.getValue("latency").toNumber(),
            r_row.getValue("throughput").toNumber(),
            r_row.getValue("uptime").toNumber(),
        }) catch return;
    }
    w.writeAll("</tbody></table>") catch return;
    app.wrap.setInnerHTML(w.buffered());

    const m = std.fmt.bufPrint(&meta_buf, "{s} · {d} rows · top node: {s}", .{ label, nrows, top_host }) catch label;
    app.meta.setTextContent(m);
}

fn onClick(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const scope = tt.JsScope.open();
    defer scope.deinit();

    const app = ctx.app;
    const next = tt.JsArray.new();
    var label: []const u8 = "Sort: unsorted (data order)";
    if (ctx.id == 0) {
        next.push(.{ .id = "region", .desc = false });
        next.push(.{ .id = "latency", .desc = false });
        label = "Sort: region asc, latency asc";
    } else if (ctx.id == 1) {
        next.push(.{ .id = "latency", .desc = false });
        label = "Sort: latency asc";
    } else if (ctx.id == 2) {
        next.push(.{ .id = "throughput", .desc = true });
        label = "Sort: throughput desc";
    } else if (ctx.id == 3) {
        next.push(.{ .id = "host", .desc = false });
        label = "Sort: host asc";
    }
    app.table.as(tt.Table_RowSorting).setSorting(next);
    render(app, label);
}

export fn start() void {
    const scope = tt.JsScope.open();
    defer scope.deinit();

    const opts = tt.JsValue.obj(.{
        .features = .{
            .coreReactivityFeature = makeReactivity(&g_app),
            .rowSortingFeature = tt.getRowSortingFeature(),
            .coreRowModel = tt.createCoreRowModel(),
            .sortedRowModel = tt.createSortedRowModel(),
            .sortFns = .{
                .alphanumeric = tt.getSortFnAlphanumeric(),
                .basic = tt.getSortFnBasic(),
                .text = tt.getSortFnText(),
            },
        },
        .data = .{
            nodeRow("N-01", "eu-west-1a", "CH", 12.4, 4200, 99.98),
            nodeRow("N-02", "eu-west-1b", "CH", 8.7, 5100, 99.99),
            nodeRow("N-03", "eu-central-1a", "DE", 15.2, 3900, 99.91),
            nodeRow("N-04", "eu-central-1b", "DE", 6.3, 6400, 99.97),
            nodeRow("N-05", "eu-north-1a", "SE", 19.8, 2800, 99.85),
            nodeRow("N-06", "eu-north-1b", "SE", 11.1, 4750, 99.95),
            nodeRow("N-07", "us-east-1a", "US", 28.5, 7100, 99.94),
            nodeRow("N-08", "us-east-1b", "US", 21.0, 5900, 99.96),
        },
        .columns = .{
            colDef("id", "Node"),
            colDef("host", "Host"),
            colDef("region", "Region"),
            colDef("latency", "Latency (ms)"),
            colDef("throughput", "Req/s"),
            colDef("uptime", "Uptime %"),
        },
        .initialState = .{
            .sorting = .{
                .{ .id = "region", .desc = false },
                .{ .id = "latency", .desc = false },
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
    render(&g_app, "Sort: region asc, latency asc");
}
