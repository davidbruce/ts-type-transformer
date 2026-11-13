// The JavaScript the generated bindings call.
class Counter {
    count = 0;
    onchange = null;
    #listeners = [];
    subscribe(listener) { this.#listeners.push(listener); }
    // Removes by identity, as removeEventListener does.
    unsubscribe(listener) { this.#listeners = this.#listeners.filter(l => l !== listener); }
    increment(by = 1) {
        this.count += by;
        for (const l of this.#listeners) l(this.count);
        if (this.onchange) this.onchange(this.count);
        return this.count;
    }
    fold(f) { return f(this.count, "count"); }
    visit(f) { return f(this); }
}
globalThis.Counter = Counter;
const pending = [];
globalThis.later = f => { pending.push(f); };
globalThis.flush = () => {
    const n = pending.length;
    for (const f of pending.splice(0)) f();
    return n;
};
globalThis.eachId = f => {
    f(1);
    f("two");
    return 2;
};
