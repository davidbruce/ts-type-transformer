// TanStack Table v9 (table-core) from Zig, in Node. What the bindings can express of it: state objects,
// column definitions, features, the sort functions, and the failure of constructTable without reactivity.
// A table itself cannot be built (see the FINDINGs). Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

var buf: [1024]u8 = undefined;

fn has(s: b.JsString, needle: []const u8) bool {
    return std.mem.indexOf(u8, s.copy(&buf), needle) != null;
}

export fn run() void {
    // Sorting state: a ColumnSort is { id, desc }.
    const by_age = b.ColumnSort.create();
    by_age.setId("age");
    by_age.setDesc(true);
    expect(by_age.getId().eql("age") and by_age.getDesc(), @src());
    const by_name = b.ColumnSort.create();
    by_name.setId("name");
    by_name.setDesc(false);
    expect(!by_name.getDesc() and by_name.getId().eql("name"), @src());

    // Pagination state.
    const page = b.PaginationState.create();
    page.setPageIndex(2);
    page.setPageSize(25);
    expect(page.getPageIndex() == 2 and page.getPageSize() == 25, @src());

    // A filter's value is any value.
    const filter = b.ColumnFilter.create();
    filter.setId("status");
    filter.setValue(b.JsValue.string("active"));
    expect(filter.getId().eql("status") and filter.getValue().toString().eql("active"), @src());
    filter.setValue(b.JsValue.number(42));
    expect(filter.getValue().toNumber() == 42, @src());

    // Record-shaped state: visibility, selection, sizing.
    const visibility = b.ColumnVisibilityState.create();
    visibility.setAt("email", false);
    visibility.setAt("name", true);
    expect(visibility.getAt("email") == false and visibility.getAt("name") == true, @src());
    expect(visibility.getAt("never") == null, @src());
    const selection = b.RowSelectionState.create();
    selection.setAt("row-1", true);
    expect(selection.getAt("row-1") == true and selection.getAt("row-2") == null, @src());
    const sizing = b.ColumnSizingState.create();
    sizing.setAt("name", 180);
    expect(sizing.getAt("name").? == 180 and sizing.getAt("age") == null, @src());

    // Pinning state holds arrays of column ids.
    const pinning = b.ColumnPinningState.create();
    const left = b.JsArrayOf(b.JsString).new();
    left.pushString("name");
    left.pushString("age");
    pinning.setStart(left);
    expect(pinning.getStart().len() == 2 and pinning.getStart().get(1).eql("age"), @src());
    pinning.setEnd(b.JsArrayOf(b.JsString).new());
    expect(pinning.getEnd().len() == 0, @src());

    // Column definitions: a header or cell is a string or a function (a union).
    const name_col = b.ColumnDefBase_Core.create();
    name_col.setCell(.{ .string = b.JsString.from("{{value}}") });
    name_col.setFooter(.{ .string = b.JsString.from("total") });
    expect(name_col.getCell().?.string.eql("{{value}}"), @src());
    expect(name_col.getFooter().?.string.eql("total"), @src());
    const age_col = b.ColumnDefBase_Core.create();
    expect(age_col.getCell() == null and age_col.getFooter() == null, @src());
    name_col.setMeta(b.JsValue.number(7));
    expect(name_col.getMeta().toNumber() == 7, @src());

    // Options that hold the columns.
    const options = b.TableOptions_Columns.create();
    options.setColumns(b.JsArrayOf(b.ColumnDefBase_Core).of(&.{ name_col, age_col }));
    expect(options.getColumns().len() == 2, @src());
    expect(b.jsSame(options.getColumns().get(0), name_col), @src());

    // Features: the stock set, the core set, and the core row model's feature are one object each.
    const stock = b.getStockFeatures();
    expect(stock.getRowSortingFeature().h != 0, @src());
    expect(stock.getColumnFilteringFeature().h != 0 and stock.getColumnVisibilityFeature().h != 0, @src());
    expect(b.jsSame(stock.getRowSortingFeature(), b.getRowSortingFeature()), @src());
    const core = b.getCoreFeatures();
    expect(b.jsSame(core.getCoreCellsFeature(), b.getCoreCellsFeature()), @src());
    expect(b.jsSame(core.getCoreRowsFeature(), b.getCoreRowsFeature()), @src());
    // The core features carry no reactivity of their own; the host supplies it.
    expect(core.getCoreReactivityFeature() == null, @src());

    // cloneState copies deeply: a new array with the same contents.
    const original = b.JsArray.new();
    original.push(b.JsValue.number(1));
    original.push(b.JsValue.number(2));
    const copied = b.cloneState(.{ .h = original.h });
    const copy: b.JsArray = .{ .h = copied.h };
    expect(copy.len() == 2 and copy.get(1).toNumber() == 2, @src());
    expect(!b.jsSame(copied, original), @src());

    // A sort function is callable; given rows that are not rows it throws a TypeError.
    const basic = b.getSortFnBasic();
    const fake = b.Row_Core.create();
    _ = basic.invoke(fake, fake, "age");
    const err = b.jsTakeError();
    expect(err != null, @src());
    if (err) |e| {
        defer b.jsRelease(e);
        const name = b.jsErrorName(e);
        defer b.jsRelease(name);
        expect(name.eql("TypeError"), @src());
    }

    // A function the library returns can be called: createColumnHelper().accessor(key, column).
    const helper = b.createColumnHelper();
    expect(helper.getAccessor().h != 0 and helper.getDisplay().h != 0, @src());
    const column = b.ColumnHelper_accessor_column_header_string.create();
    const def = b.callColumnHelperAccessor(helper.getAccessor(), .{ .string = b.JsString.from("age") }, .{ .ColumnHelper_accessor_column_header_string = column });
    expect(def.h != 0, @src());
    const keyed: b.TableOptions_Core_defaultColumn_accessorKey = .{ .h = def.h };
    expect(keyed.getAccessorKey().toString().eql("age"), @src());

    // FINDING 13: a table cannot be built. constructTable needs options.features with a coreReactivityFeature
    // whose createWritableAtom returns an Atom, but Atom is an opaque handle (from @tanstack/store, not bound),
    // nothing here can make one, and the options interfaces expose `features`, `data` and `state` read-only.
    // The attempt fails inside the library:
    const attempt = b.constructTable(b.TableOptions_Core.create());
    expect(attempt.h == 0, @src());
    const failure = b.jsTakeError();
    expect(failure != null, @src());
    if (failure) |e| {
        defer b.jsRelease(e);
        const msg = b.jsErrorMessage(e);
        defer b.jsRelease(msg);
        expect(has(msg, "coreReactivityFeature"), @src());
    }
}
