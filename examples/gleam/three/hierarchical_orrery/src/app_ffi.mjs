export const coerce = (x) => x;
export const newCell = (v) => ({ v });
export const getCell = (c) => c.v;
export const setCell = (c, v) => { c.v = v; };
export const roundInt = (x) => Math.round(x) | 0;
export const fmt2 = (x) => Number(x).toFixed(2);
