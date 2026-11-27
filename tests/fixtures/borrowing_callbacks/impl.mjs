class Queue {
    #items = [];
    push(n) { this.#items.push(n); }
    drain(f) {
        const n = this.#items.length;
        for (const item of this.#items.splice(0)) f(item);
        return n;
    }
    drainOwned(f) {
        for (const item of this.#items.splice(0)) f(item);
    }
}
globalThis.Queue = Queue;
globalThis.runScoped = f => f(21);
let stashed = null;
globalThis.runScopedStash = f => {
    stashed = f;
    f(10);
};
globalThis.invokeStashed = () => {
    if (stashed) stashed(99);
};
