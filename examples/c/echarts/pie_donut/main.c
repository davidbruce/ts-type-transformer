#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>

static Echarts_PieDataItemOption slice(const char* name, double value) {
    return Echarts_PieDataItemOption_create(.name = name, .value = js_number(value));
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    HTMLElement box = Document_getElementById(doc, "chart");
    ECharts chart = echarts_init(box, NULL);

    JsArray palette = js_values("#111111", "#404040", "#737373", "#a3a3a3", "#d4d4d4");

    EChartsOption opt = EChartsOption_create(
        .animation = false,
        .backgroundColor = "#ffffff",
        .color_array = palette,
        .legend_plain = Echarts_PlainLegendComponentOption_create(
            .bottom = 8, .textStyle = Echarts_LabelOption_create(.color = js_string("#111111"))),
        .tooltip = Echarts_TooltipComponentOption_create(.trigger = Echarts_TooltipComponentOption_trigger_item),
        .series_pie = Echarts_PieSeriesOption_create(
            .name = "Subsystems",
            .radius_strings = js_strings("44%", "72%"),
            .center_strings = js_strings("50%", "46%"),
            .itemStyle = Echarts_PieItemStyleOption_create(.borderColor = "#ffffff", .borderWidth = 2),
            .label = Echarts_PieLabelOption_create(.color = "#111111"),
            .data_items = Echarts_PieDataItemOption_list(
                slice("Runtime", 34), slice("Bindings", 26), slice("Glue", 18), slice("Parser", 14), slice("Tests", 8))));
    ECharts_setOption(chart, opt);

    char meta[96];
    snprintf(meta, sizeof meta, "Canvas: %dx%d \xC2\xB7 5 monochrome segments \xC2\xB7 Total 100",
        (int)ECharts_getWidth(chart), (int)ECharts_getHeight(chart));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
}
