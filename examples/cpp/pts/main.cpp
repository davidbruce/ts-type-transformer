#include <algorithm>
#include <cmath>
#include <cstdio>
#include <string>

#include "pts/pts.hpp"

using namespace pts;

extern "C" __attribute__((export_name("start"))) void start() {
    CanvasSpaceOptions opts = CanvasSpaceOptions::create();
    opts.bgcolor("#101622");
    opts.resize(true);
    opts.retina(true);

    CanvasSpace space = CanvasSpace::create(std::string("#pt")).setup(opts);
    CanvasForm<CanvasSpace> form = space.getForm();

    IPlayer player = IPlayer::create();
    player.animate(AnimateCallbackFn([space, form](double time_ms, double, Space) {
        Pt pointer = space.pointer();
        Group grid = Create::gridPts(space.innerBound(), 18.0, 11.0);
        Pt size = space.size();
        double max_dist = std::max(1.0, std::max(size.x(), size.y()) * 0.55);

        for (Pt pt : grid) {
            Pt delta = pt._subtract(ts::Array<>{ ts::Value::of(pointer) });
            double dist = delta.magnitude();
            double t = 1.0 - Num::clamp(dist / max_dist, 0.0, 1.0);
            double angle = delta.angle() + std::sin(time_ms * 0.0015 + dist * 0.01) * 0.35;
            double len = 8.0 + 34.0 * t;
            Group seg = Line::fromAngle(pt, angle, len);

            double hue = 155.0 + 185.0 * t;
            double alpha = 0.25 + 0.75 * t;
            char buf[48];
            std::snprintf(buf, sizeof(buf), "hsla(%.0f, 85%%, 62%%, %.2f)", hue, alpha);
            std::string color(buf);

            form.strokeOnly(CanvasForm_fill_c(color), 1.5 + 2.5 * t).line(PtLikeIterable(seg));
            Group dot = Circle::fromCenter(pt, 1.5 + 4.5 * t * t);
            form.fillOnly(CanvasForm_fill_c(color)).circle(PtLikeIterable(dot));
        }

        double ring_radius = 18.0 + 6.0 * std::sin(time_ms * 0.004);
        Group ring = Circle::fromCenter(pointer, ring_radius);
        form.strokeOnly(CanvasForm_fill_c(std::string("#ffffffaa")), 2.0).circle(PtLikeIterable(ring));
    }));

    space.add(player);
    space.bindMouse().bindTouch().play();
}
