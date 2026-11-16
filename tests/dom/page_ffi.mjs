// What the Gleam DOM program needs of the page itself: identity, and its report.
export const same = (a, b) => a === b;
export const report = (passed, failed) => {
  window.__gleam = { passed, failed: [...failed] };
};
