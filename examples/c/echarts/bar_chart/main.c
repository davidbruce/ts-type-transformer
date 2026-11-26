#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>


static Echarts_LineStyleOption line_color(const char* hex) {
    return Echarts_LineStyleOption_create(.color = hex);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    ECharts chart = echarts_init(Document_getElementById(doc, "chart"), NULL);

    EChartsOption opt = EChartsOption_create(
        .animation = false,
        .backgroundColor = "#ffffff",
        .title = Echarts_TitleComponentOption_create(
            .text = "Monthly Output", .left = 16, .top = 12,
            .textStyle = Echarts_TitleTextStyleOption_create(.color = js_string("#111111"), .fontSize = 13)),
        .grid = Echarts_GridComponentOption_create(
            .left = 48, .right = 24, .top = 48, .bottom = 36),
        .xAxis_category = Echarts_XAXisComponentOption_xAxis_create(
            .data_strings = js_strings("Jan", "Feb", "Mar", "Apr", "May", "Jun"),
            .axisLine = Echarts_AxisLineOption_create(.lineStyle = line_color("#111111")),
            .axisLabel = Echarts_AxisLabelOption_create(.color = js_string("#555555"))),
        .yAxis_value = Echarts_YAXisComponentOption_yAxis_value_create(
            .min = 0, .max = 100,
            .splitLine = Echarts_SplitLineOption_create(.lineStyle = line_color("#e5e5e5")),
            .axisLabel = Echarts_AxisLabelOption_create(.color = js_string("#555555"))),
        .series_bar = Echarts_BarSeriesOption_create(
            .name = "Units", .barWidth_string = "46%", .data = js_values(42, 68, 55, 84, 71, 93),
            .itemStyle = Echarts_BarItemStyleOption_create(.color = "#111111")));
    ECharts_setOption(chart, opt);

    char meta[96];
    snprintf(meta, sizeof meta, "Canvas: %dx%d \xC2\xB7 6 bars \xC2\xB7 Monochrome #111111",
        (int)ECharts_getWidth(chart), (int)ECharts_getHeight(chart));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
}
