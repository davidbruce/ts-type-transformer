const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.resolveTargetQuery(.{
        .cpu_arch = .wasm32,
        .os_tag = .freestanding,
    });
    const optimize = b.standardOptimizeOption(.{
        .preferred_optimize_mode = .ReleaseSmall,
    });
    const pts_mod = b.createModule(.{
        .root_source_file = b.path("bindings/pts/bindings.zig"),
        .target = target,
        .optimize = optimize,
    });
    const root_mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    root_mod.addImport("pts", pts_mod);
    const exe = b.addExecutable(.{
        .name = "app",
        .root_module = root_mod,
    });
    exe.entry = .disabled;
    exe.rdynamic = true;
    exe.export_table = true;
    b.installArtifact(exe);
}
