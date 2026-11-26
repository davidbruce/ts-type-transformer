#include <algorithm>
#include <cstdint>
#include <string>
#include <string_view>
#include <vector>

#include "node/node.hpp"

using namespace node;

extern "C" TS_IMPORT(print) void host_print(const char* ptr, uint32_t len);

static void println(std::string_view s) {
    host_print(s.data(), static_cast<uint32_t>(s.size()));
    host_print("\n", 1);
}

TS_EXPORT(start) void start() {
    int passed = 0;
    auto check = [&](bool ok) {
        if (!ok) __builtin_trap();
        passed++;
    };

    auto bus = EventEmitter::create();
    int stages = 0;
    int done_count = 0;

    auto ev_stage = ts::Value::string("stage");
    auto ev_done = ts::Value::string("done");

    ts::Scoped<Listener> on_stage = [&](ts::Array<>) { stages++; };
    ts::Scoped<Listener> on_done = [&](ts::Array<>) { done_count++; };
    bus.on(ev_stage, on_stage);
    bus.once(ev_done, on_done);

    std::string root = fs::mkdtempSync("/tmp/tsrs-node-pipe-");
    std::string in_dir = path::join(root, "in");
    std::string out_dir = path::join(root, "out");
    auto mk = mkdir_options::create();
    fs::mkdirSync(in_dir, mk);
    fs::mkdirSync(out_dir, mk);

    // Stage 1: seed
    fs::writeFileSync(path::join(in_dir, "a.log"), "100 OK\n");
    fs::writeFileSync(path::join(in_dir, "b.log"), "200 OK\n");
    fs::writeFileSync(path::join(in_dir, "c.log"), "300 OK\n");
    bus.emit(ev_stage, "seed");
    check(stages == 1);

    // Stage 2: bundle
    auto raw_logs = fs::readdirSync(in_dir);
    std::vector<std::string> logs;
    for (std::string n : raw_logs) logs.push_back(n);
    std::sort(logs.begin(), logs.end());
    check(logs.size() == 3 && logs[0] == "a.log" && logs[1] == "b.log" && logs[2] == "c.log");

    std::string combined = path::join(out_dir, "combined.log");
    for (const std::string& name : logs) {
        check(path::extname(name) == ".log");
        std::string chunk = fs::readFileSync(path::join(in_dir, name)).toString();
        fs::appendFileSync(combined, chunk);
    }
    bus.emit(ev_stage, "bundle");
    check(stages == 2);

    // Stage 3: verify
    std::string comb_text = fs::readFileSync(combined).toString();
    check(fs::statSync(combined).size() == 21.0 && comb_text.find("200 OK") != std::string::npos);

    auto missing = ts::attempt([&] {
        return fs::readFileSync(path::join(out_dir, "missing.log"));
    });
    bool enoent_caught = !missing && missing.error().message().find("ENOENT") != std::string::npos;
    check(enoent_caught);

    bus.emit(ev_stage, "verify");
    bus.emit(ev_done, static_cast<double>(comb_text.size()));
    check(stages == 3 && done_count == 1);

    fs::rmSync(root, {{.force = true, .recursive = true}});
    check(!fs::existsSync(root));

    println("pipeline_root=" + root + " combined_bytes=" + std::to_string(comb_text.size()));
    println("stages=" + std::to_string(stages) + " done=" + std::to_string(done_count) +
            " enoent=" + std::string(enoent_caught ? "true" : "false"));
    println("PASS: file_pipeline (" + std::to_string(passed) + " checks)");
}
