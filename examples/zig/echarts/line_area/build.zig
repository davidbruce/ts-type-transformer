const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .freestanding,
    });
    const optimize = b.standardOptimizeOption(.{
        .preferred_optimize_mode = .ReleaseSmall,
    });
    const dom_mod = b.createModule(.{
        .root_source_file = b.path("../../bindings/dom/bindings.zig"),
        .target = target,
        .optimize = optimize,
    });
    const echarts_mod = b.createModule(.{
        .root_source_file = b.path("../../bindings/echarts/bindings.zig"),
        .target = target,
        .optimize = optimize,
    });
    echarts_mod.addImport("dom", dom_mod);
    const root_mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    root_mod.addImport("dom", dom_mod);
    root_mod.addImport("echarts", echarts_mod);
    const exe = b.addExecutable(.{
        .name = "app",
        .root_module = root_mod,
    });
    exe.entry = .disabled;
    exe.rdynamic = true;
    exe.export_table = true;
    b.installArtifact(exe);
}
