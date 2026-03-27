use csv::{Reader, ReaderBuilder};
use rayon::prelude::*;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use std::time::Duration;

pub mod formatter; // Add this line to declare the module

// Re-export the public functions for convenience
pub use formatter::{print_chunk_results_spark_style, SparkStyleFormatter};

pub fn prepare_csv_reader(path: &Path) -> Result<(Vec<String>, Reader<File>), Box<dyn Error>> {
    let file = File::open(path)?;
    let mut rdr: csv::Reader<File> = ReaderBuilder::new().from_reader(file);

    // Get the headers and convert them to owned Strings
    let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();

    Ok((headers, rdr))
}

#[test]
fn test_csv_headers() {
    use std::path::PathBuf;
    // TODO: Set up a temp csv file that gets used for tests and gets cleaned up
    let mut path_buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path_buf.push("tests");
    path_buf.push("sample-warehouse-data.csv");

    let (headers, _reader) = prepare_csv_reader(&path_buf).unwrap();

    assert_eq!(headers.len(), 9);
    let expected_headers = vec![
        "customer_id".to_string(),
        "order_date".to_string(),
        "product_sku".to_string(),
        "quantity".to_string(),
        "unit_price".to_string(),
        "shipping_zip".to_string(),
        "email".to_string(),
        "last_updated_timestamp".to_string(),
        "".to_string(),
    ];

    assert_eq!(headers, expected_headers);
    assert!(!headers.is_empty(), "Headers should not be empty");
    println!("Headers: {:?}", headers);
}

// Trying out making an iterator that can read in chunks with csv Reader
pub struct CsvChunkIterator<'a, R: Read> {
    records: csv::StringRecordsIter<'a, R>, // 'a is the lifetime specifier compiler is asking for this
    chunk_size: usize,
}

impl<'a, R: Read> CsvChunkIterator<'a, R> {
    pub fn new(records: csv::StringRecordsIter<'a, R>, chunk_size: usize) -> Self {
        CsvChunkIterator {
            records,
            chunk_size,
        }
    }
}

impl<R: Read> Iterator for CsvChunkIterator<'_, R> {
    type Item = Result<Vec<csv::StringRecord>, csv::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let chunk: Result<Vec<_>, _> = self.records.by_ref().take(self.chunk_size).collect(); // this essentially allows for the iterator not to reset and read from top of file again

        match chunk {
            Ok(records) if records.is_empty() => None, // End of iterator, no more chunks
            Ok(records) => {
                // Print a message when a chunk is read
                println!("Chunk read with {} records", records.len());
                Some(Ok(records))
            }
            Err(e) => Some(Err(e)), // Propagate the error if there was one
        }
    }
}

// Each check must be Send + Sync to work with Rayon
pub trait PatternCheck: Send + Sync {
    // Name of the check (for reporting)
    fn name(&self) -> &str;

    // The actual check logic
    fn check(&self, value: &str) -> bool;

    // Example of what this check looks for (for reporting)
    fn show_check_pattern(&self) -> &str;
}

// Empty Check Strategy
pub struct EmptyCheck;

impl Default for EmptyCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl EmptyCheck {
    pub fn new() -> Self {
        Self
    }
}

impl PatternCheck for EmptyCheck {
    fn name(&self) -> &str {
        "Empty"
    }
    fn check(&self, value: &str) -> bool {
        value.is_empty()
    }
    fn show_check_pattern(&self) -> &str {
        "Empty string \"\""
    }
}

pub struct WhiteSpaceOnlyCheck;

impl Default for WhiteSpaceOnlyCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl WhiteSpaceOnlyCheck {
    pub fn new() -> Self {
        Self
    }
}

impl PatternCheck for WhiteSpaceOnlyCheck {
    fn name(&self) -> &str {
        "WhiteSpaceOnlyCheck"
    }
    fn check(&self, value: &str) -> bool {
        !value.is_empty() && value.trim().is_empty()
    }
    fn show_check_pattern(&self) -> &str {
        "WhiteSpaceOnlyCheck string ' ' "
    }
}

// Header Analysis
#[derive(Debug, Clone)]
pub enum HeaderIssue {
    EmptyHeader { index: usize },
    DuplicateHeader { name: String, indices: Vec<usize> },
    NullLikeHeader { index: usize, name: String },
    NumericHeader { index: usize, name: String },
}

#[derive(Debug, Clone)]
pub struct HeaderAnalysisResult {
    pub issues: Vec<HeaderIssue>,
    /// True when every non-empty header looks like a data value (all numeric),
    /// suggesting the file has no header row and the first data row was parsed as one.
    pub likely_missing_header: bool,
}

pub fn analyze_headers(headers: &[String]) -> HeaderAnalysisResult {
    let mut issues = Vec::new();

    // Empty headers
    for (i, h) in headers.iter().enumerate() {
        if h.is_empty() {
            issues.push(HeaderIssue::EmptyHeader { index: i });
        }
    }

    // Duplicate headers
    let mut seen: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, h) in headers.iter().enumerate() {
        seen.entry(h.as_str()).or_default().push(i);
    }
    let mut dups: Vec<_> = seen
        .into_iter()
        .filter(|(name, indices)| !name.is_empty() && indices.len() > 1)
        .collect();
    dups.sort_by_key(|(name, _)| name.to_string());
    for (name, indices) in dups {
        issues.push(HeaderIssue::DuplicateHeader {
            name: name.to_string(),
            indices,
        });
    }

    // NULL-like headers (e.g. a header cell containing "NULL" or "N/A")
    let null_check = NullLikeCheck::new();
    for (i, h) in headers.iter().enumerate() {
        if null_check.check(h) {
            issues.push(HeaderIssue::NullLikeHeader {
                index: i,
                name: h.clone(),
            });
        }
    }

    // Numeric headers — individual issues and a file-level "likely missing header" flag
    let digits_check = DigitsOnlyCheck::new();
    let numeric_indices: Vec<usize> = headers
        .iter()
        .enumerate()
        .filter(|(_, h)| !h.is_empty() && digits_check.check(h))
        .map(|(i, _)| i)
        .collect();

    for &i in &numeric_indices {
        issues.push(HeaderIssue::NumericHeader {
            index: i,
            name: headers[i].clone(),
        });
    }

    let non_empty_count = headers.iter().filter(|h| !h.is_empty()).count();
    let likely_missing_header = non_empty_count > 0 && numeric_indices.len() == non_empty_count;

    HeaderAnalysisResult {
        issues,
        likely_missing_header,
    }
}

#[test]
fn test_analyze_headers_clean() {
    let headers = vec![
        "customer_id".to_string(),
        "email".to_string(),
        "amount".to_string(),
    ];
    let result = analyze_headers(&headers);
    assert!(result.issues.is_empty());
    assert!(!result.likely_missing_header);
}

#[test]
fn test_analyze_headers_empty() {
    let headers = vec!["name".to_string(), "".to_string(), "email".to_string()];
    let result = analyze_headers(&headers);
    assert!(result
        .issues
        .iter()
        .any(|i| matches!(i, HeaderIssue::EmptyHeader { index: 1 })));
}

#[test]
fn test_analyze_headers_duplicate() {
    let headers = vec!["name".to_string(), "email".to_string(), "name".to_string()];
    let result = analyze_headers(&headers);
    assert!(result
        .issues
        .iter()
        .any(|i| matches!(i, HeaderIssue::DuplicateHeader { .. })));
}

#[test]
fn test_analyze_headers_null_like() {
    let headers = vec!["customer_id".to_string(), "NULL".to_string()];
    let result = analyze_headers(&headers);
    assert!(result
        .issues
        .iter()
        .any(|i| matches!(i, HeaderIssue::NullLikeHeader { .. })));
}

#[test]
fn test_analyze_headers_all_numeric_likely_missing() {
    let headers = vec!["1001".to_string(), "20240115".to_string(), "5".to_string()];
    let result = analyze_headers(&headers);
    assert!(result.likely_missing_header);
}

// Digits Only Check Strategy
pub struct DigitsOnlyCheck;

impl Default for DigitsOnlyCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl DigitsOnlyCheck {
    pub fn new() -> Self {
        Self
    }
}

impl PatternCheck for DigitsOnlyCheck {
    fn name(&self) -> &str {
        "DigitsOnlyCheck"
    }
    fn check(&self, value: &str) -> bool {
        !value.is_empty() && value.chars().all(|c| c.is_ascii_digit())
    }
    fn show_check_pattern(&self) -> &str {
        "Fields containing only digit characters (0-9)"
    }
}

// Dash-Only Values Check Strategy
pub struct DashOnlyCheck;

impl Default for DashOnlyCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl DashOnlyCheck {
    pub fn new() -> Self {
        Self
    }
}

impl PatternCheck for DashOnlyCheck {
    fn name(&self) -> &str {
        "DASH_ONLY"
    }

    fn check(&self, value: &str) -> bool {
        let trimmed = value.trim();
        trimmed == "-" || trimmed == "--"
    }

    fn show_check_pattern(&self) -> &str {
        "Fields whose trimmed value is '-' or '--'"
    }
}

#[test]
fn test_dash_only_check() {
    let check = DashOnlyCheck::new();
    assert!(check.check("-"));
    assert!(check.check("--"));
    assert!(check.check("  -  "));
    assert!(check.check("  --  "));
    assert!(!check.check(""));
    assert!(!check.check("---"));
    assert!(!check.check("a-b"));
    assert!(!check.check("some value"));
}

// Placeholder-Like Values Check Strategy
pub struct PlaceholderCheck;

impl Default for PlaceholderCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaceholderCheck {
    pub const PLACEHOLDER_VALUES: [&'static str; 4] = ["TBD", "TODO", "PLACEHOLDER", "UNKNOWN"];

    pub fn new() -> Self {
        Self
    }
}

impl PatternCheck for PlaceholderCheck {
    fn name(&self) -> &str {
        "PLACEHOLDER_VALUES"
    }

    fn check(&self, value: &str) -> bool {
        let trimmed = value.trim();
        Self::PLACEHOLDER_VALUES
            .iter()
            .any(|&placeholder| trimmed.eq_ignore_ascii_case(placeholder))
    }

    fn show_check_pattern(&self) -> &str {
        "TBD, TODO, PLACEHOLDER, UNKNOWN"
    }
}

#[test]
fn test_digits_only_check() {
    let check = DigitsOnlyCheck::new();
    // Should match fields with only digit characters
    assert!(check.check("0"));
    assert!(check.check("123"));
    assert!(check.check("0009"));
    assert!(check.check("1001"));
    assert!(check.check("94105"));
    // Should not match empty strings
    assert!(!check.check(""));
    // Should not match strings with non-digit characters
    assert!(!check.check("12.34"));
    assert!(!check.check("123abc"));
    assert!(!check.check(" 123"));
    assert!(!check.check("123 "));
    assert!(!check.check("SKU123"));
    assert!(!check.check("N/A"));
}

#[test]
fn test_digits_only_check_name_and_pattern() {
    let check = DigitsOnlyCheck::new();
    assert_eq!(check.name(), "DigitsOnlyCheck");
    assert_eq!(
        check.show_check_pattern(),
        "Fields containing only digit characters (0-9)"
    );
}

#[test]
fn test_placeholder_check() {
    let check = PlaceholderCheck::new();
    assert!(check.check("TBD"));
    assert!(check.check("tbd"));
    assert!(check.check("TODO"));
    assert!(check.check("todo"));
    assert!(check.check("PLACEHOLDER"));
    assert!(check.check("placeholder"));
    assert!(check.check("UNKNOWN"));
    assert!(check.check("unknown"));
    assert!(check.check("  TBD  "));
    assert!(!check.check(""));
    assert!(!check.check("some value"));
    assert!(!check.check("NULL"));
}

// NULL Like Values Check Strategy
pub struct NullLikeCheck;

impl Default for NullLikeCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl NullLikeCheck {
    pub const NULL_LIKE_VALUES: [&'static str; 5] = ["NULL", "N/A", "NA", "NONE", "NaN"]; // use const since only checks a few strings

    pub fn new() -> Self {
        Self
    }
}

impl PatternCheck for NullLikeCheck {
    fn name(&self) -> &str {
        "NULL_LIKE_VALUES"
    }

    fn check(&self, value: &str) -> bool {
        let trimmed = value.trim(); // Borrowed slice, no allocation
        Self::NULL_LIKE_VALUES
            .iter()
            .any(|&null| trimmed.eq_ignore_ascii_case(null))
    }

    fn show_check_pattern(&self) -> &str {
        "NULL, N/A, NA, None, NaN" //TODO: Can we ref the const here? avoid hardcode would need to change in trait as well
    }
}

// Create a struct to hold statistics for each column
#[derive(Clone)]
pub struct ColumnStats {
    null_like_count: usize,
    empty_count: usize,
    white_space_only_count: usize, // Add other statistics as needed (pattern matches, etc.)
    digits_only_count: usize,
    placeholder_count: usize,
    dash_only_count: usize,
}

#[derive(Clone)]
pub struct CsvAggregator {
    headers: Vec<String>,
    column_stats: Vec<ColumnStats>,
    total_rows: usize,
    chunk_size: usize,
    processing_time: Option<Duration>,
}

impl CsvAggregator {
    // Initialize with headers
    pub fn new(headers: Vec<String>, chunk_size: usize) -> Self {
        let column_count = headers.len();
        let column_stats = vec![
            ColumnStats {
                null_like_count: 0,
                empty_count: 0,
                white_space_only_count: 0,
                digits_only_count: 0,
                placeholder_count: 0,
                dash_only_count: 0,
            };
            column_count
        ];

        CsvAggregator {
            headers,
            column_stats,
            total_rows: 0,
            chunk_size,
            processing_time: None,
        }
    }

    // Add chunk results to aggregator
    #[allow(clippy::too_many_arguments)]
    pub fn add_chunk_results(
        &mut self,
        null_map: &HashMap<usize, usize>,
        empty_map: &HashMap<usize, usize>,
        white_space_only_map: &HashMap<usize, usize>,
        digits_only_map: &HashMap<usize, usize>,
        placeholder_map: &HashMap<usize, usize>,
        dash_only_map: &HashMap<usize, usize>,
        chunk_size: usize,
    ) {
        // Update total row count
        self.total_rows += chunk_size;

        // Update null-like counts
        for (&col, &count) in null_map.iter() {
            if col < self.column_stats.len() {
                self.column_stats[col].null_like_count += count;
            }
        }

        // Update empty counts
        for (&col, &count) in empty_map.iter() {
            if col < self.column_stats.len() {
                self.column_stats[col].empty_count += count;
            }
        }

        // Update white_space_only_map counts
        for (&col, &count) in white_space_only_map.iter() {
            if col < self.column_stats.len() {
                self.column_stats[col].white_space_only_count += count;
            }
        }

        // Update digits_only_map counts
        for (&col, &count) in digits_only_map.iter() {
            if col < self.column_stats.len() {
                self.column_stats[col].digits_only_count += count;
            }
        }

        // Update placeholder_map counts
        for (&col, &count) in placeholder_map.iter() {
            if col < self.column_stats.len() {
                self.column_stats[col].placeholder_count += count;
            }
        }

        // Update dash_only_map counts
        for (&col, &count) in dash_only_map.iter() {
            if col < self.column_stats.len() {
                self.column_stats[col].dash_only_count += count;
            }
        }
    }

    pub fn set_processing_time(&mut self, duration: Duration) {
        self.processing_time = Some(duration);
    }

    // Generate final report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();

        report.push_str("\n=== CSV QUALITY REPORT ===\n");
        report.push_str(&format!("Total rows processed: {}\n", self.total_rows));
        report.push_str(&format!("Total columns: {}\n\n", self.headers.len()));
        report.push_str(&format!("Chunk size used: {} rows\n", self.chunk_size));

        // Add processing time to report if available
        if let Some(duration) = self.processing_time {
            let seconds = duration.as_secs();
            let millis = duration.subsec_millis();

            // Format processing time nicely
            if seconds > 60 {
                let minutes = seconds / 60;
                let remaining_secs = seconds % 60;
                report.push_str(&format!(
                    "Processing time: {}m {}s {}ms\n",
                    minutes, remaining_secs, millis
                ));
            } else {
                report.push_str(&format!("Processing time: {}s {}ms\n", seconds, millis));
            }

            // Add processing rate (rows per second)
            if seconds > 0 || millis > 0 {
                let total_seconds = seconds as f64 + (millis as f64 / 1000.0);
                let rows_per_second = self.total_rows as f64 / total_seconds;
                report.push_str(&format!(
                    "Processing rate: {:.2} rows/second\n",
                    rows_per_second
                ));
            }
        }

        report.push_str("COLUMN STATISTICS:\n");
        for (i, header) in self.headers.iter().enumerate() {
            let stats = &self.column_stats[i];

            // Skip columns with no issues if desired
            // if stats.null_like_count == 0 && stats.empty_count == 0 { continue; }

            report.push_str(&format!("col_{} ('{}'):\n", i, header));

            // Calculate percentages
            let null_percent = if self.total_rows > 0 {
                (stats.null_like_count as f64 / self.total_rows as f64) * 100.0
            } else {
                0.0
            };

            let empty_percent = if self.total_rows > 0 {
                (stats.empty_count as f64 / self.total_rows as f64) * 100.0
            } else {
                0.0
            };

            let white_space_only_percent = if self.total_rows > 0 {
                (stats.white_space_only_count as f64 / self.total_rows as f64) * 100.0
            } else {
                0.0
            };

            let digits_only_percent = if self.total_rows > 0 {
                (stats.digits_only_count as f64 / self.total_rows as f64) * 100.0
            } else {
                0.0
            };

            report.push_str(&format!(
                "  NULL-like values: {} ({:.2}%)\n",
                stats.null_like_count, null_percent
            ));

            report.push_str(&format!(
                "  Empty values: {} ({:.2}%)\n",
                stats.empty_count, empty_percent
            ));

            report.push_str(&format!(
                "  White-Space-Only values: {} ({:.2}%)\n",
                stats.white_space_only_count, white_space_only_percent
            ));

            let placeholder_percent = if self.total_rows > 0 {
                (stats.placeholder_count as f64 / self.total_rows as f64) * 100.0
            } else {
                0.0
            };

            report.push_str(&format!(
                "  Digits-Only values: {} ({:.2}%)\n",
                stats.digits_only_count, digits_only_percent
            ));

            report.push_str(&format!(
                "  Placeholder values: {} ({:.2}%)\n",
                stats.placeholder_count, placeholder_percent
            ));

            let dash_only_percent = if self.total_rows > 0 {
                (stats.dash_only_count as f64 / self.total_rows as f64) * 100.0
            } else {
                0.0
            };

            report.push_str(&format!(
                "  Dash-Only values: {} ({:.2}%)\n",
                stats.dash_only_count, dash_only_percent
            ));

            report.push('\n');
        }

        report
    }
}

// Owned per-record result — no shared state, no locking
#[derive(Debug, Default)]
struct RecordResult {
    null_counts: HashMap<usize, usize>,
    empty_counts: HashMap<usize, usize>,
    whitespace_counts: HashMap<usize, usize>,
    digits_only_counts: HashMap<usize, usize>,
    placeholder_counts: HashMap<usize, usize>,
    dash_only_counts: HashMap<usize, usize>,
    rows_processed: usize,
}

impl RecordResult {
    fn new() -> Self {
        Self::default()
    }

    fn merge(mut self, other: Self) -> Self {
        self.rows_processed += other.rows_processed;
        for (col, count) in other.null_counts {
            *self.null_counts.entry(col).or_insert(0) += count;
        }
        for (col, count) in other.empty_counts {
            *self.empty_counts.entry(col).or_insert(0) += count;
        }
        for (col, count) in other.whitespace_counts {
            *self.whitespace_counts.entry(col).or_insert(0) += count;
        }
        for (col, count) in other.digits_only_counts {
            *self.digits_only_counts.entry(col).or_insert(0) += count;
        }
        for (col, count) in other.placeholder_counts {
            *self.placeholder_counts.entry(col).or_insert(0) += count;
        }
        for (col, count) in other.dash_only_counts {
            *self.dash_only_counts.entry(col).or_insert(0) += count;
        }
        self
    }
}

// Struct to hold processing results for a single chunk
#[derive(Debug, Clone)]
pub struct ChunkProcessingResult {
    pub chunk_number: usize,
    pub rows_processed: usize,
    pub null_counts: HashMap<usize, usize>,
    pub empty_counts: HashMap<usize, usize>,
    pub whitespace_counts: HashMap<usize, usize>,
    pub digits_only_counts: HashMap<usize, usize>,
    pub placeholder_counts: HashMap<usize, usize>,
    pub dash_only_counts: HashMap<usize, usize>,
}

// Struct to hold overall processing configuration
pub struct ProcessingConfig {
    pub chunk_size: usize,
    pub enable_parallel: bool,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1_000_000,
            enable_parallel: true,
        }
    }
}

// Main processing function
pub fn process_csv_chunks<R: Read>(
    chunk_iterator: CsvChunkIterator<'_, R>,
    config: ProcessingConfig,
) -> Result<Vec<ChunkProcessingResult>, Box<dyn std::error::Error>> {
    // Checks are stateless and Send+Sync — share by reference, no Arc needed
    let null_check = NullLikeCheck::new();
    let empty_check = EmptyCheck::new();
    let whitespace_check = WhiteSpaceOnlyCheck::new();
    let digits_only_check = DigitsOnlyCheck::new();
    let placeholder_check = PlaceholderCheck::new();
    let dash_only_check = DashOnlyCheck::new();

    let mut results = Vec::new();
    let mut chunk_number = 0;

    for chunk in chunk_iterator {
        match chunk {
            Ok(records) => {
                chunk_number += 1;
                let result = process_single_chunk(
                    &records,
                    chunk_number,
                    &null_check,
                    &empty_check,
                    &whitespace_check,
                    &digits_only_check,
                    &placeholder_check,
                    &dash_only_check,
                    config.enable_parallel,
                )?;
                results.push(result);
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    Ok(results)
}

// Process a single record — pure function, returns owned result, no shared state
fn process_record_owned(
    record: &csv::StringRecord,
    null_check: &NullLikeCheck,
    empty_check: &EmptyCheck,
    whitespace_check: &WhiteSpaceOnlyCheck,
    digits_only_check: &DigitsOnlyCheck,
    placeholder_check: &PlaceholderCheck,
    dash_only_check: &DashOnlyCheck,
) -> RecordResult {
    let mut result = RecordResult {
        rows_processed: 1,
        ..Default::default()
    };

    for (i, field) in record.iter().enumerate() {
        if null_check.check(field) {
            *result.null_counts.entry(i).or_insert(0) += 1;
        }
        if empty_check.check(field) {
            *result.empty_counts.entry(i).or_insert(0) += 1;
        }
        if whitespace_check.check(field) {
            *result.whitespace_counts.entry(i).or_insert(0) += 1;
        }
        if digits_only_check.check(field) {
            *result.digits_only_counts.entry(i).or_insert(0) += 1;
        }
        if placeholder_check.check(field) {
            *result.placeholder_counts.entry(i).or_insert(0) += 1;
        }
        if dash_only_check.check(field) {
            *result.dash_only_counts.entry(i).or_insert(0) += 1;
        }
    }

    result
}

// Process a single chunk using ownership + fold/reduce — zero locks during processing
#[allow(clippy::too_many_arguments)]
pub fn process_single_chunk(
    records: &[csv::StringRecord],
    chunk_number: usize,
    null_check: &NullLikeCheck,
    empty_check: &EmptyCheck,
    whitespace_check: &WhiteSpaceOnlyCheck,
    digits_only_check: &DigitsOnlyCheck,
    placeholder_check: &PlaceholderCheck,
    dash_only_check: &DashOnlyCheck,
    enable_parallel: bool,
) -> Result<ChunkProcessingResult, Box<dyn std::error::Error>> {
    let combined = if enable_parallel {
        // Each Rayon thread owns its fold accumulator; merge happens once at the end
        records
            .par_iter()
            .map(|record| {
                process_record_owned(
                    record,
                    null_check,
                    empty_check,
                    whitespace_check,
                    digits_only_check,
                    placeholder_check,
                    dash_only_check,
                )
            })
            .reduce(RecordResult::new, RecordResult::merge)
    } else {
        records
            .iter()
            .map(|record| {
                process_record_owned(
                    record,
                    null_check,
                    empty_check,
                    whitespace_check,
                    digits_only_check,
                    placeholder_check,
                    dash_only_check,
                )
            })
            .fold(RecordResult::new(), RecordResult::merge)
    };

    Ok(ChunkProcessingResult {
        chunk_number,
        rows_processed: combined.rows_processed,
        null_counts: combined.null_counts,
        empty_counts: combined.empty_counts,
        whitespace_counts: combined.whitespace_counts,
        digits_only_counts: combined.digits_only_counts,
        placeholder_counts: combined.placeholder_counts,
        dash_only_counts: combined.dash_only_counts,
    })
}

// Print results function
pub fn print_chunk_results(result: &ChunkProcessingResult, headers: &[String]) {
    println!(
        "\nProcessed chunk #{} with {} rows",
        result.chunk_number, result.rows_processed
    );

    // Print statistics for this chunk
    println!("--- Statistics for chunk {}:", result.chunk_number);

    // NULL-like values
    if !result.null_counts.is_empty() {
        println!("NULL-like values:");
        for (col, count) in result.null_counts.iter().filter(|(_, &count)| count > 0) {
            let header_name = if *col < headers.len() {
                &headers[*col]
            } else {
                "Unknown Column"
            };

            println!(
                "   col_{} column_name={}: {} NULL-like values",
                col, header_name, count
            );
        }
    } else {
        println!("No NULL-like values found in this chunk");
    }

    // Empty values
    if !result.empty_counts.is_empty() {
        println!("Empty values:");
        for (col, count) in result.empty_counts.iter().filter(|(_, &count)| count > 0) {
            let header_name = if *col < headers.len() {
                &headers[*col]
            } else {
                "Unknown Column"
            };

            println!(
                "   col_{} column_name={}: {} empty values",
                col, header_name, count
            );
        }
    } else {
        println!("No empty values found in this chunk");
    }

    // White space only values
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
                "Unknown Column"
            };

            println!(
                "   col_{} column_name={}: {} white space only values",
                col, header_name, count
            );
        }
    } else {
        println!("No white space only values found in this chunk");
    }

    // Digits only values
    if !result.digits_only_counts.is_empty() {
        println!("Digits Only values:");
        for (col, count) in result
            .digits_only_counts
            .iter()
            .filter(|(_, &count)| count > 0)
        {
            let header_name = if *col < headers.len() {
                &headers[*col]
            } else {
                "Unknown Column"
            };

            println!(
                "   col_{} column_name={}: {} digits only values",
                col, header_name, count
            );
        }
    } else {
        println!("No digits only values found in this chunk");
    }
}

type QualityMetrics = (
    HashMap<usize, usize>, // null_counts
    HashMap<usize, usize>, // empty_counts
    HashMap<usize, usize>, // whitespace_counts
    HashMap<usize, usize>, // placeholder_counts
    usize,                 // total_rows
);

// Aggregate results function
pub fn aggregate_results(results: &[ChunkProcessingResult]) -> QualityMetrics {
    let mut total_null_counts = HashMap::new();
    let mut total_empty_counts = HashMap::new();
    let mut total_whitespace_counts = HashMap::new();
    let mut total_placeholder_counts = HashMap::new();
    let mut total_rows = 0;

    for result in results {
        total_rows += result.rows_processed;

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
    }

    (
        total_null_counts,
        total_empty_counts,
        total_whitespace_counts,
        total_placeholder_counts,
        total_rows,
    )
}
