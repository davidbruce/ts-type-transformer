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
    zone_vis: bool = false,
    owner_vis: bool = true,
    atoms: [512]AtomState = undefined,
    atom_count: usize = 0,
    btns: [5]BtnCtx = undefined,
};

const BtnCtx = struct { app: *App, id: usize };

var g_app: App = .{};
var html_buf: [16384]u8 = undefined;
var meta_buf: [256]u8 = undefined;
var col_ids_buf: [8][16]u8 = undefined;
var col_lens: [8]usize = undefined;
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

fn taskRow(id: []const u8, task: []const u8, owner: []const u8, zone: []const u8, cpu: f64, status: []const u8) tt.JsValue {
    return tt.JsValue.obj(.{
        .id = id,
        .task = task,
        .owner = owner,
        .zone = zone,
        .cpu = cpu,
        .status = status,
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

    const pin = app.table.as(tt.Table_ColumnPinning);
    const vis = app.table.as(tt.Table_ColumnVisibility);
    const sel = app.table.as(tt.Table_RowSelection);

    const groups = [3]tt.JsArrayOf(tt.Column_Core){
        pin.getStartVisibleLeafColumns(),
        pin.getCenterVisibleLeafColumns(),
        pin.getEndVisibleLeafColumns(),
    };

    var ncol: usize = 0;
    var w = std.Io.Writer.fixed(&html_buf);
    w.writeAll("<table><thead><tr><th>Sel</th>") catch return;
    for (groups, 0..) |grp, g_idx| {
        const gn = grp.len();
        for (0..gn) |j| {
            if (ncol >= 8) break;
            const col = grp.get(@intCast(j));
            const cp = col.as(tt.Column_CoreProperties);
            const cid = cp.getId().copy(&col_ids_buf[ncol]);
            col_lens[ncol] = cid.len;
            const is_cpu = std.mem.eql(u8, cid, "cpu");
            w.writeAll(if (is_cpu) "<th class=\"num\">" else "<th>") catch return;
            const hdr = getProp(cp.getColumnDef().as(tt.JsValue), "header").toString().copy(&tmp_buf);
            appendEsc(&w, hdr) catch return;
            if (g_idx != 1) w.writeAll("<span class=\"pin-tag\">PIN</span>") catch return;
            w.writeAll("</th>") catch return;
            ncol += 1;
        }
    }
    w.writeAll("</tr></thead><tbody>") catch return;

    const rm = app.table.as(tt.Table_RowModels_Core).getRowModel();
    const rows = rm.getRows();
    const nrows = rows.len();
    for (0..nrows) |i| {
        const row = rows.get(@intCast(i));
        const r_row = row.as(tt.Row_Row);
        const is_sel = row.as(tt.Row_RowSelection).getIsSelected();
        w.writeAll(if (is_sel) "<tr class=\"selected-row\"><td>&#9632;</td>" else "<tr><td>&#9633;</td>") catch return;
        for (0..ncol) |c| {
            const cid = col_ids_buf[c][0..col_lens[c]];
            if (std.mem.eql(u8, cid, "cpu")) {
                w.print("<td class=\"num\">{d:.0}%</td>", .{r_row.getValue("cpu").toNumber()}) catch return;
            } else {
                w.writeAll("<td>") catch return;
                appendEsc(&w, r_row.getValue(cid).toString().copy(&tmp_buf)) catch return;
                w.writeAll("</td>") catch return;
            }
        }
        w.writeAll("</tr>") catch return;
    }
    w.writeAll("</tbody></table>") catch return;
    app.wrap.setInnerHTML(w.buffered());

    const vis_n = vis.getVisibleLeafColumns().len();
    const all_n = app.table.as(tt.Table_Columns).getAllLeafColumns().len();
    const sel_n = sel.getSelectedRowModel().getRows().len();
    const m = std.fmt.bufPrint(&meta_buf, "{d} / {d} visible columns · start:id end:status pinned · {d} / {d} rows selected", .{
        vis_n, all_n, sel_n, nrows,
    }) catch "Layout";
    app.meta.setTextContent(m);
}

fn applyVisibility(app: *App) void {
    app.table.as(tt.Table_ColumnVisibility).setColumnVisibility(.{
        .zone = app.zone_vis,
        .owner = app.owner_vis,
    });
}

fn onClick(ctx: *const BtnCtx, _: dom.PointerEvent) void {
    const scope = tt.JsScope.open();
    defer scope.deinit();

    const app = ctx.app;
    const pin = app.table.as(tt.Table_ColumnPinning);
    const sel = app.table.as(tt.Table_RowSelection);
    if (ctx.id == 0) {
        app.zone_vis = !app.zone_vis;
        applyVisibility(app);
    } else if (ctx.id == 1) {
        app.owner_vis = !app.owner_vis;
        applyVisibility(app);
    } else if (ctx.id == 2) {
        sel.toggleAllRowsSelected(.{ .value = true });
    } else if (ctx.id == 3) {
        sel.toggleAllRowsSelected(.{ .value = false });
    } else if (ctx.id == 4) {
        app.zone_vis = false;
        app.owner_vis = true;
        applyVisibility(app);
        pin.setColumnPinning(.{ .start = &.{"id"}, .end = &.{"status"} });
        sel.setRowSelection(.{ .@"0" = true, .@"2" = true, .@"4" = true });
    }
    render(app);
}

export fn start() void {
    const scope = tt.JsScope.open();
    defer scope.deinit();

    const opts = tt.JsValue.obj(.{
        .features = .{
            .coreReactivityFeature = makeReactivity(&g_app),
            .columnVisibilityFeature = tt.getColumnVisibilityFeature(),
            .columnPinningFeature = tt.getColumnPinningFeature(),
            .rowSelectionFeature = tt.getRowSelectionFeature(),
            .coreRowModel = tt.createCoreRowModel(),
        },
        .data = .{
            taskRow("T-101", "ingest-wal-sync", "a.meyer", "eu-ch-1", 42, "running"),
            taskRow("T-102", "index-compact", "b.rossi", "eu-ch-2", 68, "running"),
            taskRow("T-103", "snapshot-verify", "c.keller", "eu-de-1", 15, "queued"),
            taskRow("T-104", "ledger-reconcile", "d.weber", "eu-de-2", 79, "running"),
            taskRow("T-105", "cert-rotation", "e.morel", "eu-fr-1", 24, "ready"),
            taskRow("T-106", "shard-rebalance", "f.novak", "eu-fr-2", 55, "running"),
            taskRow("T-107", "backup-archive", "g.steiner", "eu-ch-1", 31, "ready"),
        },
        .columns = .{
            colDef("id", "Task ID"),
            colDef("task", "Workload"),
            colDef("owner", "Owner"),
            colDef("zone", "Zone"),
            colDef("cpu", "CPU %"),
            colDef("status", "Status"),
        },
        .initialState = .{
            .columnVisibility = .{ .zone = false },
            .columnPinning = .{ .start = &.{"id"}, .end = &.{"status"} },
            .rowSelection = .{ .@"0" = true, .@"2" = true, .@"4" = true },
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
