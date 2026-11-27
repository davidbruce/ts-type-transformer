#![no_main]
use bindings::*;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check { ($e:expr) => { unsafe { check_at($e as i32, line!()) } }; }

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    {
        let q = Queue::new();
        q.push(10.0);
        q.push(20.0);
        q.push(12.0);
        // Borrowing closure mutates local stack state directly without RefCell or 'static.
        let mut sum = 0.0;
        let drained = q.drain(|n| {
            sum += n;
        });
        check!(drained == 3.0 && sum == 42.0);

        let factor = 3.0;
        let mut calls = 0;
        let res = run_scoped(|x| {
            calls += 1;
            x * factor
        });
        check!(calls == 1 && res == 63.0);

        let mut stashed_sum = 0.0;
        run_scoped_stash(|x| {
            stashed_sum += x;
        });
        check!(stashed_sum == 10.0);

        // Non-borrowing method remains 'static + owned.
        q.push(5.0);
        q.drain_owned(&DrainOwnedFn::new(|n| {
            let _ = n;
        }));

        // Calling a stashed borrowed callback after return traps cleanly.
        let err = js::attempt(|| invoke_stashed()).unwrap_err();
        check!(err.message().contains("callback called after release"));
    }
    check!(js::live_handles() == 0);
}
