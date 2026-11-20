#include <algorithm>
#include <cmath>
#include <string>

#include "pts/pts.hpp"

using namespace pts;

static Pt mk_pt(double x, double y) {
    Pt p = Pt::make(2.0);
    return p.x(x).y(y);
}

static const char* SHADES[9] = {
    "#f7f7f7", "#efefef", "#e5e5e5",
    "#d4d4d4", "#b5b5b5", "#999999",
    "#737373", "#404040", "#262626"
};

TS_EXPORT(start) void start() {
    auto opts = CanvasSpaceOptions::create()
        .bgcolor("#ffffff")
        .resize(true)
        .retina(true);

    auto space = CanvasSpace::create("#pt").setup(opts);
    auto form = space.getForm();

    auto player = IPlayer::create();
    player.animate([space, form](double time_ms, double, Space) {
        Pt c = space.center();
        Pt size = space.size();
        Pt pointer = space.pointer();
        double min_dim = std::min(size.x(), size.y());
        double radius = std::max(80.0, min_dim * 0.40);

        double px_norm = (pointer.x() / std::max(1.0, size.x())) - 0.5;
        double t = Num::clamp(0.22 + 0.16 * std::sin(time_ms * 0.0012) + px_norm * 0.24, 0.10, 0.48);

        Group poly = Polygon::fromCenter(c, radius, 6.0);
        poly.rotate2D(time_ms * 0.00035, c);
        Pt centroid = Polygon::centroid(poly);

        for (int depth = 0; depth < 9; depth++) {
            double sw = depth == 0 ? 1.6 : 1.0;
            form.fill(SHADES[depth])
                .stroke("#111111", sw)
                .polygon(poly);
            if (depth == 0) {
                for (Pt pt : poly) {
                    form.strokeOnly("#d4d4d4", 1.0)
                        .line(Group::create(centroid, pt));
                    form.fillOnly("#111111")
                        .point(pt, 3.5, "circle");
                }
            }
            poly = Polygon::midpoints(poly, true, t);
        }

        form.strokeOnly("#ffffff", 1.5)
            .line(Group::create(mk_pt(c.x() - 10.0, c.y()), mk_pt(c.x() + 10.0, c.y())));
        form.strokeOnly("#ffffff", 1.5)
            .line(Group::create(mk_pt(c.x(), c.y() - 10.0), mk_pt(c.x(), c.y() + 10.0)));
        form.fill("#ffffff")
            .stroke("#111111", 1.5)
            .point(centroid, 5.0, "circle");

        Group ring = Circle::fromCenter(pointer, 10.0);
        form.strokeOnly("#737373", 1.5).circle(ring);
    });

    space.add(player);
    space.bindMouse().bindTouch().play();
}
