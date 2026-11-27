#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

extern "C" __attribute__((export_name("run"))) void run() {
    {
        Queue q = Queue::create();
        q.push(10);
        q.push(20);
        q.push(12);
        // Borrowing lambda captures by reference safely for the synchronous call.
        double sum = 0;
        double drained = q.drain([&](double n) { sum += n; });
        CHECK(drained == 3 && sum == 42);

        double factor = 3;
        int calls = 0;
        double res = runScoped([&](double x) {
            calls++;
            return x * factor;
        });
        CHECK(calls == 1 && res == 63);

        double stashed_sum = 0;
        runScopedStash([&](double x) { stashed_sum += x; });
        CHECK(stashed_sum == 10);

        // Calling a stashed borrowed lambda after return fails cleanly.
        auto tried = ts::attempt([] { invokeStashed(); });
        CHECK(!tried);
        CHECK(tried.error().message().find("callback called after release") != std::string::npos);
    }
    CHECK(ts::live_handles() == 0);
}
