#include <algorithm>
#include <cmath>
#include <string>
#include <vector>

#include "pts/pts.hpp"

using namespace pts;

static Pt mk_pt(double x, double y) {
    Pt p = Pt::make(2.0);
    return p.x(x).y(y);
}

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
        double min_dim = std::max(120.0, std::min(size.x(), size.y()));

        double ang = time_ms * 0.00035;
        double half_len = min_dim * 0.44;
        double ca = std::cos(ang), sa = std::sin(ang);
        Group chord = Group::create(
            mk_pt(c.x() - ca * half_len, c.y() - sa * half_len),
            mk_pt(c.x() + ca * half_len, c.y() + sa * half_len)
        );
        form.strokeOnly("#999999", 1.2).line(chord);

        Pt probe_center = (pointer.x() > 1.0 || pointer.y() > 1.0) ? pointer : c;
        Group probe = Circle::fromCenter(probe_center, min_dim * 0.20);

        std::vector<Group> targets;
        for (int i = 0; i < 5; i++) {
            double fi = double(i);
            double orbit_r = min_dim * (0.14 + 0.04 * fi);
            double theta = time_ms * (0.00045 + 0.00012 * fi) + fi * 1.256;
            Pt tc = mk_pt(c.x() + std::cos(theta) * orbit_r, c.y() + std::sin(theta) * orbit_r);
            double tr = min_dim * (0.11 + 0.025 * double(i % 3));
            Group circle = Circle::fromCenter(tc, tr);
            targets.push_back(circle);

            form.fill("#f7f7f7")
                .stroke("#737373", 1.4)
                .circle(circle);
            if (auto foot = Line::perpendicularFromPt(chord, tc, false)) {
                form.strokeOnly("#b5b5b5", 1.0)
                    .line(Group::create(tc, *foot));
                form.fillOnly("#555555")
                    .point(*foot, 2.5, "circle");
            }
        }

        form.strokeOnly("#111111", 2.0).circle(probe);
        form.strokeOnly("#111111", 1.4)
            .line(Group::create(mk_pt(probe_center.x() - 6.0, probe_center.y()), mk_pt(probe_center.x() + 6.0, probe_center.y())));
        form.strokeOnly("#111111", 1.4)
            .line(Group::create(mk_pt(probe_center.x(), probe_center.y() - 6.0), mk_pt(probe_center.x(), probe_center.y() + 6.0)));

        for (const auto& circle : targets) {
            Group hits = Circle::intersectCircle2D(probe, circle);
            if (hits.size() == 2) {
                form.strokeOnly("#111111", 1.4).line(hits);
            }
            for (Pt h : hits) {
                form.fill("#ffffff")
                    .stroke("#111111", 2.0)
                    .point(h, 5.0, "circle");
            }
        }
    });

    space.add(player);
    space.bindMouse().bindTouch().play();
}
