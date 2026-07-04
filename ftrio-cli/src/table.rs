//! A tiny fixed-width text table renderer for console reports.

/// Render an aligned table with a header row and a separator.
pub fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let column_count = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate().take(column_count) {
            widths[index] = widths[index].max(cell.len());
        }
    }

    let mut output = String::new();
    let header_cells: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
    output.push_str(&format_row(&header_cells, &widths));
    output.push('\n');
    output.push_str(&separator(&widths));
    for row in rows {
        output.push('\n');
        output.push_str(&format_row(row, &widths));
    }
    output
}

fn format_row(cells: &[String], widths: &[usize]) -> String {
    let mut parts = Vec::with_capacity(cells.len());
    for (index, cell) in cells.iter().enumerate() {
        let width = widths.get(index).copied().unwrap_or(cell.len());
        parts.push(format!("{cell:<width$}"));
    }
    parts.join(" | ")
}

fn separator(widths: &[usize]) -> String {
    widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join("-+-")
}
