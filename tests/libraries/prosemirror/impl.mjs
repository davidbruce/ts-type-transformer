// The packages the test binds, by name, for the glue.
export const modules = { "prosemirror-model": await import("prosemirror-model"), "prosemirror-transform": await import("prosemirror-transform"), "prosemirror-state": await import("prosemirror-state"), "prosemirror-view": await import("prosemirror-view"), "test-page": await import("test-page") };
