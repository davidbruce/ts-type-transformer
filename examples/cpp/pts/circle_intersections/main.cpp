#include <algorithm>
#include <cmath>
#include <string>
#include <vector>

#include "pts/pts.hpp"

using namespace pts;

static Pt mk_pt(double x, double y) {
    Pt p = Pt::make(2.0);
    p.x(x);
    p.y(y);
    return p;
}

static Group mk_group2(const Pt& a, const Pt& b) {
    return Group::create(ts::Array<Pt>{ a, b });
}

extern "C" __attribute__((export_name("start"))) void start() {
    CanvasSpaceOptions opts = CanvasSpaceOptions::create();
    opts.bgcolor("#ffffff");
    opts.resize(true);
    opts.retina(true);

    CanvasSpace space = CanvasSpace::create(std::string("#pt")).setup(opts);
    CanvasForm<CanvasSpace> form = space.getForm();

    IPlayer player = IPlayer::create();
    player.animate(AnimateCallbackFn([space, form](double time_ms, double, Space) {
        Pt c = space.center();
        Pt size = space.size();
        Pt pointer = space.pointer();
        double min_dim = std::max(120.0, std::min(size.x(), size.y()));

        double ang = time_ms * 0.00035;
        double half_len = min_dim * 0.44;
        double ca = std::cos(ang), sa = std::sin(ang);
        Group chord = mk_group2(
            mk_pt(c.x() - ca * half_len, c.y() - sa * half_len),
            mk_pt(c.x() + ca * half_len, c.y() + sa * half_len)
        );
        form.strokeOnly(CanvasForm_fill_c(std::string("#999999")), 1.2).line(PtLikeIterable(chord));

        Pt probe_center = (pointer.x() > 1.0 || pointer.y() > 1.0) ? pointer : c;
        Group probe = Circle::fromCenter(PtLike(probe_center), min_dim * 0.20);

        std::vector<Group> targets;
        for (int i = 0; i < 5; i++) {
            double fi = double(i);
            double orbit_r = min_dim * (0.14 + 0.04 * fi);
            double theta = time_ms * (0.00045 + 0.00012 * fi) + fi * 1.256;
            Pt tc = mk_pt(c.x() + std::cos(theta) * orbit_r, c.y() + std::sin(theta) * orbit_r);
            double tr = min_dim * (0.11 + 0.025 * double(i % 3));
            Group circle = Circle::fromCenter(PtLike(tc), tr);
            targets.push_back(circle);

            form.fill(CanvasForm_fill_c(std::string("#f7f7f7")))
                .stroke(CanvasForm_fill_c(std::string("#737373")), 1.4)
                .circle(PtLikeIterable(circle));
            if (auto foot = Line::perpendicularFromPt(PtIterable(chord), PtLike(tc), false)) {
                form.strokeOnly(CanvasForm_fill_c(std::string("#b5b5b5")), 1.0)
                    .line(PtLikeIterable(mk_group2(tc, *foot)));
                form.fillOnly(CanvasForm_fill_c(std::string("#555555")))
                    .point(PtLike(*foot), 2.5, "circle");
            }
        }

        form.strokeOnly(CanvasForm_fill_c(std::string("#111111")), 2.0).circle(PtLikeIterable(probe));
        form.strokeOnly(CanvasForm_fill_c(std::string("#111111")), 1.4)
            .line(PtLikeIterable(mk_group2(mk_pt(probe_center.x() - 6.0, probe_center.y()), mk_pt(probe_center.x() + 6.0, probe_center.y()))));
        form.strokeOnly(CanvasForm_fill_c(std::string("#111111")), 1.4)
            .line(PtLikeIterable(mk_group2(mk_pt(probe_center.x(), probe_center.y() - 6.0), mk_pt(probe_center.x(), probe_center.y() + 6.0))));

        for (const auto& circle : targets) {
            Group hits = Circle::intersectCircle2D(PtIterable(probe), PtIterable(circle));
            if (hits.size() == 2) {
                form.strokeOnly(CanvasForm_fill_c(std::string("#111111")), 1.4).line(PtLikeIterable(hits));
            }
            for (Pt h : hits) {
                form.fill(CanvasForm_fill_c(std::string("#ffffff")))
                    .stroke(CanvasForm_fill_c(std::string("#111111")), 2.0)
                    .point(PtLike(h), 5.0, "circle");
            }
        }
    }));

    space.add(player);
    space.bindMouse().bindTouch().play();
}
