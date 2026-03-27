use crate::ChunkProcessingResult;
use std::collections::HashMap;

// Spark-style table formatter
pub struct SparkStyleFormatter {
    max_col_width: usize,
    show_truncation: bool,
}

impl Default for SparkStyleFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl SparkStyleFormatter {
    pub fn new() -> Self {
        Self {
            max_col_width: 20,
            show_truncation: true,
        }
    }

    pub fn with_max_width(mut self, width: usize) -> Self {
        self.max_col_width = width;
        self
    }

    // Format chunk results in Spark style
    pub fn format_chunk_results(
        &self,
        results: &[ChunkProcessingResult],
        headers: &[String],
    ) -> String {
        let mut output = String::new();

        // Create summary table
        output.push_str(&self.format_summary_table(results, headers));
        output.push('\n');

        // Comprehensive table showing all checks for all columns
        output.push_str(&self.format_comprehensive_table(results, headers));
        output.push('\n');

        // Create detailed table for each issue type
        output
            .push_str(&self.format_issues_table(results, headers, "NULL-like", |r| &r.null_counts));
        output.push('\n');
        output.push_str(&self.format_issues_table(results, headers, "Empty", |r| &r.empty_counts));
        output.push('\n');
        output.push_str(
            &self.format_issues_table(results, headers, "Whitespace", |r| &r.whitespace_counts),
        );
        output.push('\n');
        output.push_str(
            &self.format_issues_table(results, headers, "Placeholder", |r| &r.placeholder_counts),
        );
        output.push('\n');
        output.push_str(
            &self.format_issues_table(results, headers, "Digits-Only", |r| &r.digits_only_counts),
        );
        output.push('\n');
        output.push_str(
            &self.format_issues_table(results, headers, "Dash-Only", |r| &r.dash_only_counts),
        );

        output
    }

    // Format summary table
    fn format_summary_table(
        &self,
        results: &[ChunkProcessingResult],
        headers: &[String],
    ) -> String {
        let mut output = String::new();

        // Calculate totals
        let total_rows: usize = results.iter().map(|r| r.rows_processed).sum();
        let total_chunks = results.len();
        let total_nulls: usize = results
            .iter()
            .map(|r| r.null_counts.values().sum::<usize>())
            .sum();
        let total_empty: usize = results
            .iter()
            .map(|r| r.empty_counts.values().sum::<usize>())
            .sum();
        let total_whitespace: usize = results
            .iter()
            .map(|r| r.whitespace_counts.values().sum::<usize>())
            .sum();
        let total_placeholder: usize = results
            .iter()
            .map(|r| r.placeholder_counts.values().sum::<usize>())
            .sum();
        let total_digits_only: usize = results
            .iter()
            .map(|r| r.digits_only_counts.values().sum::<usize>())
            .sum();
        let total_dash_only: usize = results
            .iter()
            .map(|r| r.dash_only_counts.values().sum::<usize>())
            .sum();

        let total_cells = total_rows * headers.len();

        // Create strings with better explanations
        let table_headers = vec![
            "Metric".to_string(),
            "Count".to_string(),
            "% of All Cells".to_string(),
        ];
        let rows = vec![
            vec![
                "Total Rows".to_string(),
                total_rows.to_string(),
                "-".to_string(),
            ],
            vec![
                "Total Chunks".to_string(),
                total_chunks.to_string(),
                "-".to_string(),
            ],
            vec![
                "Total Cells".to_string(),
                total_cells.to_string(),
                "100.000%".to_string(),
            ],
            vec![
                "NULL-like Values".to_string(),
                total_nulls.to_string(),
                format!(
                    "{:.3}%",
                    if total_cells > 0 {
                        (total_nulls as f64 / total_cells as f64) * 100.0
                    } else {
                        0.0
                    }
                ),
            ],
            vec![
                "Empty Values".to_string(),
                total_empty.to_string(),
                format!(
                    "{:.3}%",
                    if total_cells > 0 {
                        (total_empty as f64 / total_cells as f64) * 100.0
                    } else {
                        0.0
                    }
                ),
            ],
            vec![
                "Whitespace Values".to_string(),
                total_whitespace.to_string(),
                format!(
                    "{:.3}%",
                    if total_cells > 0 {
                        (total_whitespace as f64 / total_cells as f64) * 100.0
                    } else {
                        0.0
                    }
                ),
            ],
            vec![
                "Placeholder Values".to_string(),
                total_placeholder.to_string(),
                format!(
                    "{:.3}%",
                    if total_cells > 0 {
                        (total_placeholder as f64 / total_cells as f64) * 100.0
                    } else {
                        0.0
                    }
                ),
            ],
            vec![
                "Digits-Only Values".to_string(),
                total_digits_only.to_string(),
                format!(
                    "{:.3}%",
                    if total_cells > 0 {
                        (total_digits_only as f64 / total_cells as f64) * 100.0
                    } else {
                        0.0
                    }
                ),
            ],
            vec![
                "Dash-Only Values".to_string(),
                total_dash_only.to_string(),
                format!(
                    "{:.3}%",
                    if total_cells > 0 {
                        (total_dash_only as f64 / total_cells as f64) * 100.0
                    } else {
                        0.0
                    }
                ),
            ],
        ];

        output.push_str("=== PROCESSING SUMMARY ===\n");
        output.push_str(&self.format_table_owned(&table_headers, &rows));
        output.push_str(&format!(
            "Dataset: {} rows × {} columns = {} total cells\n",
            total_rows,
            headers.len(),
            total_cells
        ));

        output
    }

    // Comprehensive table showing all checks for all columns
    fn format_comprehensive_table(
        &self,
        results: &[ChunkProcessingResult],
        headers: &[String],
    ) -> String {
        let mut output = String::new();

        // Aggregate counts across all chunks for each issue type
        let mut total_null_counts = HashMap::new();
        let mut total_empty_counts = HashMap::new();
        let mut total_whitespace_counts = HashMap::new();
        let mut total_placeholder_counts = HashMap::new();
        let mut total_dash_only_counts = HashMap::new();
        let mut total_digits_only_counts = HashMap::new();

        for result in results {
            for (col, count) in &result.null_counts {
                *total_null_counts.entry(*col).or_insert(0) += count;
            }
            for (col, count) in &result.empty_counts {
                *total_empty_counts.entry(*col).or_insert(0) += count;
            }
            for (col, count) in &result.whitespace_counts {
                *total_whitespace_counts.entry(*col).or_insert(0) += count;
            }
            for (col, count) in &result.placeholder_counts {
                *total_placeholder_counts.entry(*col).or_insert(0) += count;
            }
            for (col, count) in &result.dash_only_counts {
                *total_dash_only_counts.entry(*col).or_insert(0) += count;
            }
            for (col, count) in &result.digits_only_counts {
                *total_digits_only_counts.entry(*col).or_insert(0) += count;
            }
        }

        let total_rows: usize = results.iter().map(|r| r.rows_processed).sum();

        // Create comprehensive table headers
        let table_headers = vec![
            "Column".to_string(),
            "Column Name".to_string(),
            "NULL Count".to_string(),
            "NULL % of Column".to_string(),
            "Empty Count".to_string(),
            "Empty % of Column".to_string(),
            "Whitespace Count".to_string(),
            "Whitespace % of Column".to_string(),
            "Placeholder Count".to_string(),
            "Placeholder % of Column".to_string(),
            "Dash-Only Count".to_string(),
            "Dash-Only % of Column".to_string(),
            "Digits-Only Count".to_string(),
            "Digits-Only % of Column".to_string(),
        ];

        let mut rows = Vec::new();

        // Show ALL columns
        for (col_idx, header) in headers.iter().enumerate() {
            let column_name = self.truncate_string(header);

            let null_count = total_null_counts.get(&col_idx).copied().unwrap_or(0);
            let empty_count = total_empty_counts.get(&col_idx).copied().unwrap_or(0);
            let whitespace_count = total_whitespace_counts.get(&col_idx).copied().unwrap_or(0);
            let placeholder_count = total_placeholder_counts.get(&col_idx).copied().unwrap_or(0);
            let dash_only_count = total_dash_only_counts.get(&col_idx).copied().unwrap_or(0);
            let digits_only_count = total_digits_only_counts.get(&col_idx).copied().unwrap_or(0);

            // Calculate percentage of this column's cells (not all rows)
            let null_percentage = if total_rows > 0 {
                (null_count as f64 / total_rows as f64) * 100.0
            } else {
                0.0
            };
            let empty_percentage = if total_rows > 0 {
                (empty_count as f64 / total_rows as f64) * 100.0
            } else {
                0.0
            };
            let whitespace_percentage = if total_rows > 0 {
                (whitespace_count as f64 / total_rows as f64) * 100.0
            } else {
                0.0
            };
            let placeholder_percentage = if total_rows > 0 {
                (placeholder_count as f64 / total_rows as f64) * 100.0
            } else {
                0.0
            };
            let dash_only_percentage = if total_rows > 0 {
                (dash_only_count as f64 / total_rows as f64) * 100.0
            } else {
                0.0
            };
            let digits_only_percentage = if total_rows > 0 {
                (digits_only_count as f64 / total_rows as f64) * 100.0
            } else {
                0.0
            };

            rows.push(vec![
                col_idx.to_string(),
                column_name,
                null_count.to_string(),
                format!("{:.1}%", null_percentage),
                empty_count.to_string(),
                format!("{:.1}%", empty_percentage),
                whitespace_count.to_string(),
                format!("{:.1}%", whitespace_percentage),
                placeholder_count.to_string(),
                format!("{:.1}%", placeholder_percentage),
                dash_only_count.to_string(),
                format!("{:.1}%", dash_only_percentage),
                digits_only_count.to_string(),
                format!("{:.1}%", digits_only_percentage),
            ]);
        }

        output.push_str("=== DATA QUALITY SUMMARY BY COLUMN ===\n");
        output.push_str(&self.format_table_owned(&table_headers, &rows));

        output
    }

    // Format issues table
    fn format_issues_table<F>(
        &self,
        results: &[ChunkProcessingResult],
        headers: &[String],
        issue_type: &str,
        extractor: F,
    ) -> String
    where
        F: Fn(&ChunkProcessingResult) -> &HashMap<usize, usize>,
    {
        let mut output = String::new();

        // Aggregate counts across all chunks
        let mut total_counts = HashMap::new();
        for result in results {
            for (col, count) in extractor(result) {
                *total_counts.entry(*col).or_insert(0) += count;
            }
        }

        let total_rows: usize = results.iter().map(|r| r.rows_processed).sum();
        let total_issues: usize = total_counts.values().sum();

        // Create table
        let table_headers = vec![
            "Column".to_string(),
            "Column Name".to_string(),
            format!("{} Count", issue_type),
            format!("% of All {}", issue_type),
            "% of Column Rows".to_string(),
        ];

        let mut rows = Vec::new();

        // Show ALL columns (0 to headers.len())
        for col_idx in 0..headers.len() {
            let column_name = if col_idx < headers.len() {
                self.truncate_string(&headers[col_idx])
            } else {
                "Unknown Column".to_string()
            };

            let count = total_counts.get(&col_idx).copied().unwrap_or(0);

            // Percentage of this specific issue type (e.g., 40.5% of all NULL values)
            let percentage_of_issue_type = if total_issues > 0 && count > 0 {
                (count as f64 / total_issues as f64) * 100.0
            } else {
                0.0
            };

            // This matches the summary table calculation
            let percentage_of_column_rows = if total_rows > 0 {
                (count as f64 / total_rows as f64) * 100.0
            } else {
                0.0
            };

            rows.push(vec![
                col_idx.to_string(),
                column_name,
                count.to_string(),
                if count > 0 {
                    format!("{:.1}%", percentage_of_issue_type)
                } else {
                    "-".to_string()
                },
                format!("{:.3}%", percentage_of_column_rows), // Same precision as summary table
            ]);
        }

        output.push_str(&format!("=== {} VALUES ===\n", issue_type.to_uppercase()));
        output.push_str(&self.format_table_owned(&table_headers, &rows));

        // Better explanation of the totals
        let percentage_of_all_cells = if total_rows * headers.len() > 0 {
            (total_issues as f64 / (total_rows * headers.len()) as f64) * 100.0
        } else {
            0.0
        };

        output.push_str(&format!(
            "Total {} values: {} ({:.3}% of all cells in dataset)\n",
            issue_type.to_lowercase(),
            total_issues,
            percentage_of_all_cells
        ));

        output
    }

    // Table formatting function that works with owned Strings
    fn format_table_owned(&self, headers: &[String], rows: &[Vec<String>]) -> String {
        if rows.is_empty() {
            return String::new();
        }

        // Calculate column widths
        let mut col_widths = Vec::new();
        for (i, header) in headers.iter().enumerate() {
            let mut max_width = header.len();
            for row in rows {
                if i < row.len() {
                    max_width = max_width.max(row[i].len());
                }
            }
            col_widths.push(max_width.min(self.max_col_width));
        }

        let mut output = String::new();

        // Top border
        output.push('+');
        for &width in &col_widths {
            output.push_str(&"-".repeat(width + 2));
            output.push('+');
        }
        output.push('\n');

        // Headers
        output.push('|');
        for (i, header) in headers.iter().enumerate() {
            let truncated = self.truncate_to_width(header, col_widths[i]);
            output.push_str(&format!(" {:^width$} |", truncated, width = col_widths[i]));
        }
        output.push('\n');

        // Header separator
        output.push('+');
        for &width in &col_widths {
            output.push_str(&"-".repeat(width + 2));
            output.push('+');
        }
        output.push('\n');

        // Data rows
        for row in rows {
            output.push('|');
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() {
                    let truncated = self.truncate_to_width(cell, col_widths[i]);
                    // Right-align numbers, left-align text
                    if cell
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                    {
                        output.push_str(&format!(
                            " {:>width$} |",
                            truncated,
                            width = col_widths[i]
                        ));
                    } else {
                        output.push_str(&format!(
                            " {:<width$} |",
                            truncated,
                            width = col_widths[i]
                        ));
                    }
                }
            }
            output.push('\n');
        }

        // Bottom border
        output.push('+');
        for &width in &col_widths {
            output.push_str(&"-".repeat(width + 2));
            output.push('+');
        }
        output.push('\n');

        output
    }

    fn truncate_to_width(&self, s: &str, width: usize) -> String {
        if s.len() <= width {
            s.to_string()
        } else if self.show_truncation && width > 3 {
            format!("{}...", &s[..width - 3])
        } else {
            s.chars().take(width).collect()
        }
    }

    fn truncate_string(&self, s: &str) -> String {
        self.truncate_to_width(s, self.max_col_width)
    }
}

// Console formatter for original style output
pub struct ConsoleFormatter;

impl Default for ConsoleFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleFormatter {
    pub fn new() -> Self {
        Self
    }

    pub fn print_chunk_results(&self, result: &ChunkProcessingResult, headers: &[String]) {
        println!(
            "\nProcessed chunk #{} with {} rows",
            result.chunk_number, result.rows_processed
        );

        println!("--- Statistics for chunk {}:", result.chunk_number);

        self.print_null_results(result, headers);
        self.print_empty_results(result, headers);
        self.print_whitespace_results(result, headers);
    }

    fn print_null_results(&self, result: &ChunkProcessingResult, headers: &[String]) {
        if !result.null_counts.is_empty() {
            println!("NULL-like values:");
            for (col, count) in result.null_counts.iter().filter(|(_, &count)| count > 0) {
                let header_name = if *col < headers.len() {
                    &headers[*col]
                } else {
                    "Unkown Column"
                };

                println!(
                    "   col_{} column_name={}: {} NULL-like values",
                    col, header_name, count
                );
            }
        } else {
            println!("No NULL-like values found in this chunk");
        }
    }

    fn print_empty_results(&self, result: &ChunkProcessingResult, headers: &[String]) {
        if !result.empty_counts.is_empty() {
            println!("Empty values:");
            for (col, count) in result.empty_counts.iter().filter(|(_, &count)| count > 0) {
                let header_name = if *col < headers.len() {
                    &headers[*col]
                } else {
                    "Unkown Column"
                };

                println!(
                    "   col_{} column_name={}: {} empty values",
                    col, header_name, count
                );
            }
        } else {
            println!("No empty values found in this chunk");
        }
    }

    fn print_whitespace_results(&self, result: &ChunkProcessingResult, headers: &[String]) {
        if !result.whitespace_counts.is_empty() {
            println!("White Space Only values:");
            for (col, count) in result
                .whitespace_counts
                .iter()
                .filter(|(_, &count)| count > 0)
            {
                let header_name = if *col < headers.len() {
                    &headers[*col]
                } else {
                    "Unkown Column"
                };

                println!(
                    "   col_{} column_name={}: {} white space only values",
                    col, header_name, count
                );
            }
        } else {
            println!("No white space only values found in this chunk");
        }
    }
}

// Public convenience functions
pub fn print_chunk_results_spark_style(results: &[ChunkProcessingResult], headers: &[String]) {
    let formatter = SparkStyleFormatter::new().with_max_width(25);
    let formatted_output = formatter.format_chunk_results(results, headers);
    println!("{}", formatted_output);
}

pub fn print_chunk_results_console_style(result: &ChunkProcessingResult, headers: &[String]) {
    let formatter = ConsoleFormatter::new();
    formatter.print_chunk_results(result, headers);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentage_calculations() {
        let results = vec![ChunkProcessingResult {
            chunk_number: 1,
            rows_processed: 100,
            null_counts: [(0, 5), (1, 15)].into_iter().collect(), // 20 total nulls
            empty_counts: [(0, 10), (1, 30), (2, 60)].into_iter().collect(), // 100 total empty
            whitespace_counts: [(0, 2)].into_iter().collect(),    // 2 total whitespace
            digits_only_counts: HashMap::new(),
            placeholder_counts: HashMap::new(),
            dash_only_counts: HashMap::new(),
        }];

        let headers = vec!["col1".to_string(), "col2".to_string(), "col3".to_string()];
        let formatter = SparkStyleFormatter::new();
        let output = formatter.format_chunk_results(&results, &headers);

        // With 100 rows × 3 columns = 300 total cells:
        // - 20 nulls / 300 cells = 6.667%
        // - 100 empty / 300 cells = 33.333%
        // - 2 whitespace / 300 cells = 0.667%

        assert!(output.contains("6.667%")); // NULL percentage of all cells
        assert!(output.contains("33.333%")); // Empty percentage of all cells
        assert!(output.contains("0.667%")); // Whitespace percentage of all cells

        // Individual column percentages should be based on rows:
        // col1 empty: 10/100 = 10%
        // col2 empty: 30/100 = 30%
        // col3 empty: 60/100 = 60%

        println!("{}", output); // For visual verification
    }

    #[test]
    fn test_empty_values_dont_exceed_100_percent() {
        // Simulate your real data scenario
        let results = vec![ChunkProcessingResult {
            chunk_number: 1,
            rows_processed: 1000,
            null_counts: HashMap::new(),
            empty_counts: [
                (0, 900), // 90% of column 0 is empty
                (1, 800), // 80% of column 1 is empty
                (2, 700), // 70% of column 2 is empty
            ]
            .into_iter()
            .collect(), // 2400 total empty values
            whitespace_counts: HashMap::new(),
            digits_only_counts: HashMap::new(),
            placeholder_counts: HashMap::new(),
            dash_only_counts: HashMap::new(),
        }];

        let headers = vec!["col1".to_string(), "col2".to_string(), "col3".to_string()];
        let formatter = SparkStyleFormatter::new();
        let output = formatter.format_chunk_results(&results, &headers);

        // With 1000 rows × 3 columns = 3000 total cells:
        // 2400 empty / 3000 cells = 80.0%
        assert!(output.contains("80.0%"));

        // Individual columns should show reasonable percentages
        assert!(output.contains("90.0%")); // col1: 90% empty
        assert!(output.contains("80.0%")); // col2: 80% empty
        assert!(output.contains("70.0%")); // col3: 70% empty

        // Should NOT contain any percentage > 100%
        assert!(!output.contains("240.0%")); // This would be wrong
        assert!(!output.contains("632.0%")); // This would be very wrong

        println!("Fixed output:\n{}", output);
    }
}
