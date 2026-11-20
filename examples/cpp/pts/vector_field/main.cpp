#include <algorithm>
#include <cmath>
#include <cstdio>
#include <string>

#include "pts/pts.hpp"

using namespace pts;

TS_EXPORT(start) void start() {
    auto opts = CanvasSpaceOptions::create()
        .bgcolor("#ffffff")
        .resize(true)
        .retina(true);

    auto space = CanvasSpace::create("#pt").setup(opts);
    auto form = space.getForm();

    auto player = IPlayer::create();
    player.animate([space, form](double time_ms, double, Space) {
        Pt pointer = space.pointer();
        Group grid = Create::gridPts(space.innerBound(), 18.0, 11.0);
        Pt size = space.size();
        double max_dist = std::max(1.0, std::max(size.x(), size.y()) * 0.55);

        for (Pt pt : grid) {
            Pt delta = pt._subtract(pointer);
            double dist = delta.magnitude();
            double t = 1.0 - Num::clamp(dist / max_dist, 0.0, 1.0);
            double angle = delta.angle() + std::sin(time_ms * 0.0015 + dist * 0.01) * 0.35;
            double len = 8.0 + 34.0 * t;
            Group seg = Line::fromAngle(pt, angle, len);

            double light = 65.0 - 57.0 * t;
            double alpha = 0.25 + 0.75 * t;
            char buf[48];
            std::snprintf(buf, sizeof(buf), "hsla(0, 0%%, %.0f%%, %.2f)", light, alpha);

            form.strokeOnly(buf, 1.5 + 2.5 * t).line(seg);
            Group dot = Circle::fromCenter(pt, 1.5 + 4.5 * t * t);
            form.fillOnly(buf).circle(dot);
        }

        double ring_radius = 18.0 + 6.0 * std::sin(time_ms * 0.004);
        Group ring = Circle::fromCenter(pointer, ring_radius);
        form.strokeOnly("#11111199", 2.0).circle(ring);
    });

    space.add(player);
    space.bindMouse().bindTouch().play();
}
