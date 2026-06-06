use eng_compiler::{CheckReport, Severity};

pub const REPORT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PLOT_SPEC_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct PlotSpec {
    pub title: String,
    pub plot_type: String,
    pub x_axis: PlotAxis,
    pub y_axis: PlotAxis,
    pub series: Vec<PlotSeries>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlotAxis {
    pub name: String,
    pub label: String,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlotSeries {
    pub name: String,
    pub quantity_kind: String,
    pub display_unit: String,
    pub points: Vec<PlotPoint>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlotPoint {
    pub x: f64,
    pub y: f64,
}

pub fn plot_spec_from_report(report: &CheckReport) -> PlotSpec {
    let series_binding = report
        .semantic_program
        .typed_bindings
        .iter()
        .find_map(|binding| {
            time_series_quantity(&binding.semantic_type.quantity_kind).map(|(axis, quantity)| {
                (
                    binding.name.clone(),
                    axis,
                    quantity,
                    binding.semantic_type.display_unit.clone(),
                )
            })
        });

    let (name, axis, quantity, unit) = series_binding.unwrap_or_else(|| {
        (
            "preview".to_owned(),
            "Time".to_owned(),
            "Value".to_owned(),
            "unit".to_owned(),
        )
    });

    PlotSpec {
        title: if name == "preview" {
            "EngLang preview plot".to_owned()
        } else {
            format!("{name} over {axis}")
        },
        plot_type: "line".to_owned(),
        x_axis: PlotAxis {
            name: axis.clone(),
            label: axis,
            unit: "sample".to_owned(),
        },
        y_axis: PlotAxis {
            name: quantity.clone(),
            label: quantity,
            unit: unit.clone(),
        },
        series: vec![PlotSeries {
            name,
            quantity_kind: "TimeSeries".to_owned(),
            display_unit: unit,
            points: preview_points(),
        }],
    }
}

pub fn render_svg(title: &str) -> String {
    render_svg_from_spec(&default_plot_spec(title))
}

pub fn render_svg_from_spec(spec: &PlotSpec) -> String {
    let title = xml_escape(&spec.title);
    let x_label = xml_escape(&axis_label(&spec.x_axis));
    let y_label = xml_escape(&axis_label(&spec.y_axis));
    let points = spec
        .series
        .first()
        .map(|series| svg_points(&series.points))
        .unwrap_or_default();
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="720" height="320" viewBox="0 0 720 320" role="img" aria-label="{title}">
  <rect width="720" height="320" fill="#f7f8fb"/>
  <line x1="72" y1="250" x2="660" y2="250" stroke="#222" stroke-width="2"/>
  <line x1="72" y1="40" x2="72" y2="250" stroke="#222" stroke-width="2"/>
  <polyline points="{points}" fill="none" stroke="#0b6bcb" stroke-width="4"/>
  <text x="72" y="26" font-family="Segoe UI, Arial, sans-serif" font-size="20" fill="#111">{title}</text>
  <text x="328" y="294" font-family="Segoe UI, Arial, sans-serif" font-size="14" fill="#333">{x_label}</text>
  <text x="18" y="156" transform="rotate(-90 18 156)" font-family="Segoe UI, Arial, sans-serif" font-size="14" fill="#333">{y_label}</text>
</svg>
"##
    )
}

pub fn plot_spec_json(spec: &PlotSpec) -> String {
    let mut points = String::new();
    for (index, point) in spec
        .series
        .first()
        .map(|series| series.points.as_slice())
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        if index > 0 {
            points.push_str(", ");
        }
        points.push_str(&format!("[{}, {}]", point.x, point.y));
    }

    let series = spec.series.first();
    format!(
        "{{\n  \"format\": \"eng-plotspec-v1\",\n  \"plot_spec_version\": {PLOT_SPEC_VERSION},\n  \"plot_type\": \"{}\",\n  \"title\": \"{}\",\n  \"x_axis\": {{ \"name\": \"{}\", \"label\": \"{}\", \"unit\": \"{}\" }},\n  \"y_axis\": {{ \"name\": \"{}\", \"label\": \"{}\", \"unit\": \"{}\" }},\n  \"series\": [\n    {{\n      \"name\": \"{}\",\n      \"quantity_kind\": \"{}\",\n      \"display_unit\": \"{}\",\n      \"points\": [{}]\n    }}\n  ]\n}}\n",
        json_escape(&spec.plot_type),
        json_escape(&spec.title),
        json_escape(&spec.x_axis.name),
        json_escape(&spec.x_axis.label),
        json_escape(&spec.x_axis.unit),
        json_escape(&spec.y_axis.name),
        json_escape(&spec.y_axis.label),
        json_escape(&spec.y_axis.unit),
        json_escape(series.map(|series| series.name.as_str()).unwrap_or("preview")),
        json_escape(
            series
                .map(|series| series.quantity_kind.as_str())
                .unwrap_or("Value")
        ),
        json_escape(
            series
                .map(|series| series.display_unit.as_str())
                .unwrap_or("unit")
        ),
        points
    )
}

pub fn plot_manifest_json(
    spec: &PlotSpec,
    svg_relative_path: &str,
    plot_spec_hash: &str,
    svg_hash: &str,
) -> String {
    format!(
        "{{\n  \"format\": \"eng-plot-manifest-v1\",\n  \"plot_spec_version\": {PLOT_SPEC_VERSION},\n  \"plots\": [\n    {{\n      \"title\": \"{}\",\n      \"plot_type\": \"{}\",\n      \"plot_spec\": \"plot_spec.json\",\n      \"plot_spec_hash\": \"{}\",\n      \"svg\": \"{}\",\n      \"svg_hash\": \"{}\",\n      \"x_axis_label\": \"{}\",\n      \"y_axis_label\": \"{}\"\n    }}\n  ]\n}}\n",
        json_escape(&spec.title),
        json_escape(&spec.plot_type),
        json_escape(plot_spec_hash),
        json_escape(svg_relative_path),
        json_escape(svg_hash),
        json_escape(&axis_label(&spec.x_axis)),
        json_escape(&axis_label(&spec.y_axis))
    )
}

pub fn render_html(report: &CheckReport, plot_relative_path: &str) -> String {
    let title = html_escape(&format!(
        "EngLang Review - {}",
        report
            .source_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("source.eng")
    ));
    let mut diagnostics = String::new();
    for diagnostic in &report.diagnostics {
        diagnostics.push_str("<tr>");
        diagnostics.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            diagnostic.line,
            html_escape(diagnostic.severity.as_str()),
            html_escape(&diagnostic.code),
            html_escape(&diagnostic.message)
        ));
        diagnostics.push_str("</tr>");
    }
    if diagnostics.is_empty() {
        diagnostics.push_str("<tr><td colspan=\"4\">No diagnostics.</td></tr>");
    }

    let mut inferred = String::new();
    for declaration in &report.inferred_declarations {
        inferred.push_str("<tr>");
        inferred.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td>",
            declaration.line,
            html_escape(&declaration.name),
            html_escape(&declaration.quantity_kind),
            html_escape(&declaration.display_unit),
            html_escape(&declaration.expression)
        ));
        inferred.push_str("</tr>");
    }
    if inferred.is_empty() {
        inferred.push_str("<tr><td colspan=\"5\">No inferred local declarations.</td></tr>");
    }

    let mut hover_hints = String::new();
    for hover in &report.semantic_program.hover_hints {
        hover_hints.push_str("<tr>");
        hover_hints.push_str(&format!(
            "<td>{}:{}</td><td>{}</td><td>{}</td><td>{}</td>",
            hover.line,
            hover.column,
            html_escape(&hover.name),
            html_escape(&hover.quantity_kind),
            html_escape(&hover.detail)
        ));
        hover_hints.push_str("</tr>");
    }
    if hover_hints.is_empty() {
        hover_hints.push_str("<tr><td colspan=\"4\">No hover hints.</td></tr>");
    }

    let mut type_info = String::new();
    for info in &report.semantic_program.type_infos {
        type_info.push_str("<tr>");
        type_info.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            info.line,
            html_escape(&info.name),
            html_escape(&info.quantity_kind),
            html_escape(&info.display_unit),
            html_escape(&info.canonical_unit),
            html_escape(&info.dimension)
        ));
        type_info.push_str("</tr>");
    }
    if type_info.is_empty() {
        type_info.push_str("<tr><td colspan=\"6\">No type info.</td></tr>");
    }

    let mut unit_derivations = String::new();
    for derivation in &report.semantic_program.unit_derivations {
        unit_derivations.push_str("<tr>");
        unit_derivations.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            derivation.line,
            html_escape(&derivation.name),
            html_escape(derivation.source_unit.as_deref().unwrap_or("not detected")),
            html_escape(&derivation.display_unit),
            html_escape(&derivation.canonical_unit)
        ));
        unit_derivations.push_str("</tr>");
    }
    if unit_derivations.is_empty() {
        unit_derivations.push_str("<tr><td colspan=\"5\">No unit derivations.</td></tr>");
    }

    let mut axis_info = String::new();
    for axis in &report.semantic_program.axis_infos {
        axis_info.push_str("<tr>");
        axis_info.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            axis.line,
            html_escape(&axis.binding),
            html_escape(&axis.axis),
            html_escape(&axis.role),
            html_escape(&axis.source)
        ));
        axis_info.push_str("</tr>");
    }
    if axis_info.is_empty() {
        axis_info.push_str("<tr><td colspan=\"5\">No axis metadata.</td></tr>");
    }

    let mut stats_info = String::new();
    for stats in &report.semantic_program.stats_infos {
        stats_info.push_str("<tr>");
        stats_info.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            stats.line,
            html_escape(&stats.source),
            html_escape(&stats.quantity_kind),
            html_escape(&stats.axis),
            html_escape(&stats.statistics.join(", ")),
            html_escape(&stats.cache_key)
        ));
        stats_info.push_str("</tr>");
    }
    if stats_info.is_empty() {
        stats_info.push_str("<tr><td colspan=\"6\">No statistics summaries.</td></tr>");
    }

    let mut integrations = String::new();
    for integration in &report.semantic_program.integrations {
        integrations.push_str("<tr>");
        integrations.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            integration.line,
            html_escape(&integration.binding),
            html_escape(&integration.source),
            html_escape(&integration.input_quantity),
            html_escape(&integration.over_axis),
            html_escape(&integration.result_quantity)
        ));
        integrations.push_str("</tr>");
    }
    if integrations.is_empty() {
        integrations.push_str("<tr><td colspan=\"6\">No integrations.</td></tr>");
    }

    let mut schemas = String::new();
    for schema in &report.semantic_program.schemas {
        schemas.push_str("<tr>");
        schemas.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            schema.line,
            html_escape(&schema.name),
            schema.columns.len(),
            schema.constraints.len(),
            schema.missing_policies.len()
        ));
        schemas.push_str("</tr>");
    }
    if schemas.is_empty() {
        schemas.push_str("<tr><td colspan=\"5\">No schemas.</td></tr>");
    }

    let mut csv_promotions = String::new();
    for promotion in &report.semantic_program.csv_promotions {
        csv_promotions.push_str("<tr>");
        csv_promotions.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            promotion.line,
            html_escape(&promotion.binding),
            html_escape(&promotion.schema_name),
            html_escape(&promotion.source_literal),
            promotion.row_count,
            html_escape(promotion.source_hash.as_deref().unwrap_or("not available"))
        ));
        csv_promotions.push_str("</tr>");
    }
    if csv_promotions.is_empty() {
        csv_promotions.push_str("<tr><td colspan=\"6\">No CSV promotions.</td></tr>");
    }

    let mut entry_points = String::new();
    for entry in &report.semantic_program.entry_points {
        entry_points.push_str("<tr>");
        entry_points.push_str(&format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            entry.line,
            html_escape(&entry.kind),
            html_escape(&entry.name),
            html_escape(entry.arg_type.as_deref().unwrap_or("Args")),
            html_escape(entry.return_type.as_deref().unwrap_or("Report"))
        ));
        entry_points.push_str("</tr>");
    }
    if entry_points.is_empty() {
        entry_points.push_str("<tr><td colspan=\"5\">No entry points.</td></tr>");
    }

    let error_count = report.diagnostic_count(Severity::Error);
    let warning_count = report.diagnostic_count(Severity::Warning);
    let syntax_items = report.syntax_summary.ast_items;
    let typed_bindings = report.semantic_program.typed_bindings.len();
    let expected_types = report.semantic_program.expected_types.len();
    let hover_count = report.semantic_program.hover_hints.len();
    let quantity_completion_count = report.quantity_completion_count;
    let unit_info_count = report.unit_info_count;
    let type_info_count = report.semantic_program.type_infos.len();
    let unit_derivation_count = report.semantic_program.unit_derivations.len();
    let axis_info_count = report.semantic_program.axis_infos.len();
    let stats_info_count = report.semantic_program.stats_infos.len();
    let integration_count = report.semantic_program.integrations.len();
    let schema_count = report.semantic_program.schemas.len();
    let csv_promotion_count = report.semantic_program.csv_promotions.len();
    let entry_point_count = report.semantic_program.entry_points.len();
    let plot_relative_path = html_escape(plot_relative_path);

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <style>
    :root {{
      color-scheme: light;
      font-family: "Segoe UI", Arial, sans-serif;
      background: #f5f6f8;
      color: #20242a;
    }}
    body {{
      margin: 0;
      padding: 32px;
    }}
    main {{
      max-width: 1040px;
      margin: 0 auto;
    }}
    h1, h2 {{
      letter-spacing: 0;
    }}
    h1 {{
      margin: 0 0 8px;
      font-size: 28px;
    }}
    h2 {{
      margin-top: 28px;
      font-size: 20px;
    }}
    .summary {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 12px;
      margin: 24px 0;
    }}
    .metric {{
      border: 1px solid #d9dee7;
      border-radius: 8px;
      padding: 14px;
      background: #fff;
    }}
    .metric strong {{
      display: block;
      font-size: 24px;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      background: #fff;
      border: 1px solid #d9dee7;
    }}
    th, td {{
      text-align: left;
      border-bottom: 1px solid #e7ebf0;
      padding: 10px 12px;
      vertical-align: top;
    }}
    th {{
      background: #eef2f7;
      font-weight: 600;
    }}
    code {{
      font-family: Consolas, "SFMono-Regular", monospace;
    }}
    .plot {{
      width: 100%;
      min-height: 320px;
      border: 1px solid #d9dee7;
      border-radius: 8px;
      background: #fff;
    }}
  </style>
</head>
<body>
  <main>
    <h1>{title}</h1>
    <p>Reviewable EngLang preview artifact with source hash <code>{source_hash}</code>.</p>
    <section class="summary" aria-label="Run summary">
      <div class="metric"><span>Errors</span><strong>{error_count}</strong></div>
      <div class="metric"><span>Warnings</span><strong>{warning_count}</strong></div>
      <div class="metric"><span>AST Items</span><strong>{syntax_items}</strong></div>
      <div class="metric"><span>Typed Bindings</span><strong>{typed_bindings}</strong></div>
      <div class="metric"><span>Expected Types</span><strong>{expected_types}</strong></div>
      <div class="metric"><span>Hover Hints</span><strong>{hover_count}</strong></div>
      <div class="metric"><span>Quantity Completions</span><strong>{quantity_completion_count}</strong></div>
      <div class="metric"><span>Unit Infos</span><strong>{unit_info_count}</strong></div>
      <div class="metric"><span>Type Info</span><strong>{type_info_count}</strong></div>
      <div class="metric"><span>Unit Derivations</span><strong>{unit_derivation_count}</strong></div>
      <div class="metric"><span>Axis Info</span><strong>{axis_info_count}</strong></div>
      <div class="metric"><span>Stats Info</span><strong>{stats_info_count}</strong></div>
      <div class="metric"><span>Integrations</span><strong>{integration_count}</strong></div>
      <div class="metric"><span>Schemas</span><strong>{schema_count}</strong></div>
      <div class="metric"><span>CSV Promotions</span><strong>{csv_promotion_count}</strong></div>
      <div class="metric"><span>Entry Points</span><strong>{entry_point_count}</strong></div>
      <div class="metric"><span>Compiler</span><strong>{compiler_version}</strong></div>
      <div class="metric"><span>Report</span><strong>{report_version}</strong></div>
    </section>
    <h2>Entry Points</h2>
    <table>
      <thead><tr><th>Line</th><th>Kind</th><th>Name</th><th>Args</th><th>Returns</th></tr></thead>
      <tbody>{entry_points}</tbody>
    </table>
    <h2>Inferred Declarations</h2>
    <table>
      <thead><tr><th>Line</th><th>Name</th><th>Quantity</th><th>Display Unit</th><th>Expression</th></tr></thead>
      <tbody>{inferred}</tbody>
    </table>
    <h2>Hover Hints</h2>
    <table>
      <thead><tr><th>Position</th><th>Name</th><th>Quantity</th><th>Detail</th></tr></thead>
      <tbody>{hover_hints}</tbody>
    </table>
    <h2>Type Info</h2>
    <table>
      <thead><tr><th>Line</th><th>Name</th><th>Quantity</th><th>Display Unit</th><th>Canonical Unit</th><th>Dimension</th></tr></thead>
      <tbody>{type_info}</tbody>
    </table>
    <h2>Unit Derivations</h2>
    <table>
      <thead><tr><th>Line</th><th>Name</th><th>Source Unit</th><th>Display Unit</th><th>Canonical Unit</th></tr></thead>
      <tbody>{unit_derivations}</tbody>
    </table>
    <h2>Axis Info</h2>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Axis</th><th>Role</th><th>Source</th></tr></thead>
      <tbody>{axis_info}</tbody>
    </table>
    <h2>Statistics</h2>
    <table>
      <thead><tr><th>Line</th><th>Source</th><th>Quantity</th><th>Axis</th><th>Statistics</th><th>Cache Key</th></tr></thead>
      <tbody>{stats_info}</tbody>
    </table>
    <h2>Integrations</h2>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Source</th><th>Input</th><th>Axis</th><th>Result</th></tr></thead>
      <tbody>{integrations}</tbody>
    </table>
    <h2>Schemas</h2>
    <table>
      <thead><tr><th>Line</th><th>Name</th><th>Columns</th><th>Constraints</th><th>Missing Policies</th></tr></thead>
      <tbody>{schemas}</tbody>
    </table>
    <h2>CSV Promotions</h2>
    <table>
      <thead><tr><th>Line</th><th>Binding</th><th>Schema</th><th>Source</th><th>Rows</th><th>Source Hash</th></tr></thead>
      <tbody>{csv_promotions}</tbody>
    </table>
    <h2>Diagnostics</h2>
    <table>
      <thead><tr><th>Line</th><th>Severity</th><th>Code</th><th>Message</th></tr></thead>
      <tbody>{diagnostics}</tbody>
    </table>
    <h2>Plot</h2>
    <iframe class="plot" src="{plot_relative_path}" title="Generated plot"></iframe>
  </main>
</body>
</html>
"#,
        source_hash = html_escape(&report.source_hash),
        compiler_version = html_escape(eng_compiler::COMPILER_VERSION),
        report_version = html_escape(REPORT_VERSION)
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn xml_escape(value: &str) -> String {
    html_escape(value)
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn default_plot_spec(title: &str) -> PlotSpec {
    PlotSpec {
        title: title.to_owned(),
        plot_type: "line".to_owned(),
        x_axis: PlotAxis {
            name: "Time".to_owned(),
            label: "Time".to_owned(),
            unit: "sample".to_owned(),
        },
        y_axis: PlotAxis {
            name: "Value".to_owned(),
            label: "unit-aware value".to_owned(),
            unit: "preview".to_owned(),
        },
        series: vec![PlotSeries {
            name: "preview".to_owned(),
            quantity_kind: "Value".to_owned(),
            display_unit: "preview".to_owned(),
            points: preview_points(),
        }],
    }
}

fn preview_points() -> Vec<PlotPoint> {
    vec![
        PlotPoint { x: 0.0, y: 20.0 },
        PlotPoint { x: 1.0, y: 32.0 },
        PlotPoint { x: 2.0, y: 36.0 },
        PlotPoint { x: 3.0, y: 54.0 },
        PlotPoint { x: 4.0, y: 61.0 },
        PlotPoint { x: 5.0, y: 78.0 },
        PlotPoint { x: 6.0, y: 74.0 },
        PlotPoint { x: 7.0, y: 96.0 },
    ]
}

fn axis_label(axis: &PlotAxis) -> String {
    if axis.unit.is_empty() {
        axis.label.clone()
    } else {
        format!("{} ({})", axis.label, axis.unit)
    }
}

fn svg_points(points: &[PlotPoint]) -> String {
    if points.is_empty() {
        return String::new();
    }

    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let x_span = (max_x - min_x).max(1.0);
    let y_span = (max_y - min_y).max(1.0);

    points
        .iter()
        .map(|point| {
            let x = 72.0 + ((point.x - min_x) / x_span) * 588.0;
            let y = 250.0 - ((point.y - min_y) / y_span) * 210.0;
            format!("{x:.0},{y:.0}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn time_series_quantity(quantity_kind: &str) -> Option<(String, String)> {
    let rest = quantity_kind.strip_prefix("TimeSeries[")?;
    let (axis, after_axis) = rest.split_once(']')?;
    let quantity = after_axis.trim().strip_prefix("of ")?;
    Some((axis.trim().to_owned(), quantity.trim().to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eng_compiler::{check_source, CheckOptions};

    #[test]
    fn plotspec_uses_timeseries_axis_unit_labels() {
        let report = check_source(
            "ok.eng",
            "script main(args: Args) -> Report {\n    sensor = promote csv \"data/sensor.csv\" as SensorData\n    cp = 4180 J/kg/K\n    Q_coil = sensor.m_dot * cp * (sensor.T_return - sensor.T_supply)\n}\n",
            &CheckOptions::default(),
        );

        let spec = plot_spec_from_report(&report);
        let json = plot_spec_json(&spec);
        let svg = render_svg_from_spec(&spec);

        assert_eq!(spec.plot_type, "line");
        assert_eq!(spec.x_axis.label, "Time");
        assert_eq!(spec.y_axis.unit, "W");
        assert!(json.contains("\"format\": \"eng-plotspec-v1\""));
        assert!(svg.contains("HeatRate (W)"));
    }
}
