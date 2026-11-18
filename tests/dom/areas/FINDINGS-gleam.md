# Findings: Gleam DOM area programs

**Status:** Fixed: 1, 5.

Numbers match `// FINDING n` in tests/dom/areas/*/test.gleam.

1. Runtime bug: `NodeListOf.to_list` (and `entries`, `values`, `keys`) throw `$c_TNode is not defined`: the JS uses a codec the Gleam signature doesn't take (`areas/core/node_list_of_ffi.mjs:33`).
2. Awkward: `ImageData.data` is an opaque `Uint8ClampedArray` with no accessors.
3. Can't express: no `Uint8ClampedArray` can be made, so `ImageData.new_with_data` can't be called.
4. Awkward: no JS object can be built for `Response.json(data)`.
5. Runtime bug: `URLSearchParamsIterator.to_list` throws `$c_T is not defined` (`areas/network/url_search_params_iterator_ffi.mjs:10`): same cause as 1.
6. Can't express: `set_timeout` takes `TimerHandler`; `Function` is opaque: no Gleam callback can be given.
