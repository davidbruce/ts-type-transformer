#include <algorithm>
#include <cstdint>
#include <string>
#include <string_view>
#include <vector>

#include "node/node.hpp"

using namespace node;

extern "C" __attribute__((import_module("env"), import_name("print")))
void host_print(const char* ptr, uint32_t len);

static int g_passed = 0;

static void check(bool ok) {
    if (!ok) __builtin_trap();
    g_passed++;
}

static void println(std::string_view s) {
    host_print(s.data(), static_cast<uint32_t>(s.size()));
    host_print("\n", 1);
}

extern "C" __attribute__((export_name("start"))) void start() {
    EventEmitter<ts::Value> bus = EventEmitter<ts::Value>::create();
    int stages = 0;
    int done_count = 0;

    ts::Value ev_stage = ts::Value::string("stage");
    ts::Value ev_done = ts::Value::string("done");

    any_array_fn on_stage = [&](ts::Array<>) { stages++; };
    any_array_fn on_done = [&](ts::Array<>) { done_count++; };
    bus.on<void>(ev_stage, on_stage);
    bus.once<void>(ev_done, on_done);

    std::string root = fs::mkdtempSync("/tmp/tsrs-node-pipe-");
    std::string in_dir = path::join({root, "in"});
    std::string out_dir = path::join({root, "out"});
    mkdir_options mk = mkdir_options::create();
    fs::mkdirSync(in_dir, mk);
    fs::mkdirSync(out_dir, mk);

    // Stage 1: seed
    fs::writeFileSync(path::join({in_dir, "a.log"}), std::string("100 OK\n"));
    fs::writeFileSync(path::join({in_dir, "b.log"}), std::string("200 OK\n"));
    fs::writeFileSync(path::join({in_dir, "c.log"}), std::string("300 OK\n"));
    bus.emit<void>(ev_stage, ts::Array<>{ts::Value::string("seed")});
    check(stages == 1);

    // Stage 2: bundle
    ts::Array<std::string> raw_logs = fs::readdirSync(in_dir);
    std::vector<std::string> logs;
    for (std::string n : raw_logs) logs.push_back(n);
    std::sort(logs.begin(), logs.end());
    check(logs.size() == 3 && logs[0] == "a.log" && logs[1] == "b.log" && logs[2] == "c.log");

    std::string combined = path::join({out_dir, "combined.log"});
    for (const std::string& name : logs) {
        check(path::extname(name) == ".log");
        std::string chunk = fs::readFileSync(path::join({in_dir, name})).toString();
        fs::appendFileSync(combined, chunk);
    }
    bus.emit<void>(ev_stage, ts::Array<>{ts::Value::string("bundle")});
    check(stages == 2);

    // Stage 3: verify
    std::string comb_text = fs::readFileSync(combined).toString();
    check(fs::statSync(combined).size() == 21.0 && comb_text.find("200 OK") != std::string::npos);

    ts::Result<Buffer<ArrayBuffer>> missing = ts::attempt([&] {
        return fs::readFileSync(path::join({out_dir, "missing.log"}));
    });
    bool enoent_caught = !missing && missing.error().message().find("ENOENT") != std::string::npos;
    check(enoent_caught);

    bus.emit<void>(ev_stage, ts::Array<>{ts::Value::string("verify")});
    bus.emit<void>(ev_done, ts::Array<>{ts::Value::number(static_cast<double>(comb_text.size()))});
    check(stages == 3 && done_count == 1);

    RmOptions rm = RmOptions::create();
    rm.recursive(true);
    rm.force(true);
    fs::rmSync(root, rm);
    check(!fs::existsSync(root));

    on_stage.release();
    on_done.release();

    println("pipeline_root=" + root + " combined_bytes=" + std::to_string(comb_text.size()));
    println("stages=" + std::to_string(stages) + " done=" + std::to_string(done_count) +
            " enoent=" + std::string(enoent_caught ? "true" : "false"));
    println("PASS: file_pipeline (" + std::to_string(g_passed) + " checks)");
}
