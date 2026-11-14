// Calls Pts, a real library, through its generated C++ bindings; every CHECK must hold.
#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// Coordinates: a Pt's constructor, and its arithmetic, take any number of them.
static ts::Array<> xy(double x, double y) {
    ts::Array<> a = ts::Array<>::create();
    a.push(x);
    a.push(y);
    return a;
}

static Pt pt(double x, double y) { return Pt::create(xy(x, y)); }

extern "C" __attribute__((export_name("run"))) void run() {
    uint32_t live = ts::live_handles();
    {
        // A Pt is an array of coordinates, with names for the first four.
        Pt p = pt(3, 4);
        CHECK(p.x() == 3 && p.y() == 4);
        CHECK(p.magnitude() == 5);
        CHECK(p.toString() == "Pt(3, 4)");
        p.id("corner");
        CHECK(p.id() == "corner");

        // A method that changes a Pt returns it; its `$` twin returns another.
        Pt moved = p._add(xy(1, 1));
        CHECK(!(moved == p) && moved.x() == 4 && p.x() == 3);
        CHECK(p.add(xy(1, 1)) == p && p.x() == 4);
        // A union is a variant: any member converts. Optional arguments are overloads.
        CHECK(p.equals(moved));
        CHECK(p.clone().equals(p, 0.001) && !(p.clone() == p));

        // Statics.
        CHECK(Num::lerp(0, 10, 0.5) == 5);
        CHECK(Num::clamp(12, 0, 10) == 10);
        CHECK(Geom::toDegree(3.141592653589793) == 180);
        CHECK(Pt::make(3, 2).toString() == "Pt(2, 2, 2)");

        // A Group is an array of Pt: indexed and iterated as one.
        Group square = Group::create({pt(0, 0), pt(4, 0), pt(4, 4), pt(0, 4)});
        CHECK(square.size() == 4 && square[2].y() == 4);
        double sum = 0;
        for (Pt corner : square) sum += corner.x();
        CHECK(sum == 8);
        CHECK(square.p1().x() == 0);
        CHECK(square.centroid().toString() == "Pt(2, 2)");
        CHECK(square.boundingBox().p2().x() == 4);

        // A union parameter takes any of its members: a Group where points are wanted.
        CHECK(Geom::centroid(square).x() == 2);
        CHECK(Line::magnitude(Group::create({pt(0, 0), pt(3, 4)})) == 5);
        Group ray = Line::fromAngle(pt(1, 1), 0, 5);
        CHECK(ray.p2().x() == 6 && ray.p2().y() == 1);
    }
    // Handles are counted: what goes out of scope is freed.
    CHECK(ts::live_handles() == live);
}
