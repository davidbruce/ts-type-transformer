export const make = (value) => ({ value });
export const get = (cell) => cell.value;
export const set = (cell, value) => {
  cell.value = value;
};
