export const coerce = (x) => x;
export const newCell = (v) => ({ v });
export const getCell = (c) => c.v;
export const setCell = (c, v) => { c.v = v; };
export const sin = Math.sin;
export const cos = Math.cos;
export const fmt2 = (x) => Number(x).toFixed(2);
