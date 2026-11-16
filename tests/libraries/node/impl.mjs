// The packages the test binds, by name, for the glue.
export const modules = { fs: await import("node:fs"), path: await import("node:path"), events: await import("node:events") };
