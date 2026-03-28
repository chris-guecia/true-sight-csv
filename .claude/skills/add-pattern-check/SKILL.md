---
name: add-pattern-check
description: Add a new PatternCheck implementation to the true-sight-csv project. Use this skill whenever the user wants to detect a new kind of data quality issue in CSV files — for example, detecting numeric-only fields, date format issues, duplicate values, overly long strings, regex pattern violations, or any custom string check. Trigger on phrases like "add a check for", "detect", "flag fields that", "new pattern", "implement a check", or any request to catch a new category of bad/suspicious data.
---

# Adding a New PatternCheck

This skill guides adding a new data quality check to `true-sight-csv`. The codebase uses an **ownership-based parallel pipeline** — no `Arc`, no `Mutex`, no shared state. Each Rayon thread owns a `RecordResult` accumulator, accumulates records into it directly via `merge_record`, then all thread-local accumulators are merged once at the end via `reduce`.

**Do not introduce `Arc`, `Mutex`, or `.lock()` for new checks.** The pattern to follow is:

```rust
// WRONG — do not do this:
let my_counters = Arc::new(Mutex::new(HashMap::new()));
records.par_iter().for_each(|r| {
    let mut map = my_counters.lock().unwrap();  // contention on every record
    ...
});

// CORRECT — this is the pattern used in this codebase:
// 1. Add a field to RecordResult (thread-local, no sharing)
// 2. Accumulate directly in merge_record (called per-record, per-thread)
// 3. Thread-local accumulators are combined by RecordResult::merge at the end
```

Adding a check means threading it through `RecordResult`, `merge_record`, `merge`, `process_single_chunk`, and `process_csv_chunks` — described step by step below.

## Step 1 — Implement the struct in `src/lib.rs`

Add the struct and its `PatternCheck` impl after the existing checks (around line 185). Follow this exact shape:

```rust
pub struct MyCheck;

impl Default for MyCheck {
    fn default() -> Self { Self::new() }
}

impl MyCheck {
    pub fn new() -> Self { Self }
}

impl PatternCheck for MyCheck {
    fn name(&self) -> &str {
        "MY_CHECK_NAME"  // shown in output tables — short SCREAMING_SNAKE_CASE
    }

    fn check(&self, value: &str) -> bool {
        // Return true when the value IS the problem.
        // value is a raw CSV field string — missing fields arrive as "".
        todo!()
    }

    fn show_check_pattern(&self) -> &str {
        "human-readable description of what this flags"
    }
}
```

`PatternCheck` requires `Send + Sync`. Unit structs satisfy this automatically. If you add fields, use only `Send + Sync` types.

## Step 2 — Add a field to `RecordResult`

`RecordResult` (around line 695) is the thread-local accumulator. Add your count map:

```rust
struct RecordResult {
    null_counts: HashMap<usize, usize>,
    empty_counts: HashMap<usize, usize>,
    whitespace_counts: HashMap<usize, usize>,
    // ... existing fields ...
    my_check_counts: HashMap<usize, usize>,  // add this
    rows_processed: usize,
}
```

`RecordResult` derives `Default`, so the new field initializes to `HashMap::new()` automatically — no other constructor change needed.

## Step 3 — Add the check inside `merge_record`

`merge_record` is the hot path. It accumulates one record directly into `self` with no allocation. Add your check inside the field loop:

```rust
fn merge_record(&mut self, record: &csv::StringRecord, ..., my_check: &MyCheck) {
    self.rows_processed += 1;
    for (i, field) in record.iter().enumerate() {
        // ... existing checks ...
        if my_check.check(field) {
            *self.my_check_counts.entry(i).or_insert(0) += 1;
        }
    }
}
```

Also add the `my_check: &MyCheck` parameter to the `merge_record` signature.

## Step 4 — Add the merge in `RecordResult::merge`

`merge` combines two thread-local accumulators at the end of processing. Add one block per new field:

```rust
fn merge(mut self, other: Self) -> Self {
    // ... existing merges ...
    for (col, count) in other.my_check_counts {
        *self.my_check_counts.entry(col).or_insert(0) += count;
    }
    self
}
```

## Step 5 — Thread the check through `process_single_chunk`

Add a parameter and pass it to both `merge_record` calls (the `fold` path and the sequential `fold` path):

```rust
pub fn process_single_chunk(
    records: &[csv::StringRecord],
    chunk_number: usize,
    null_check: &NullLikeCheck,
    // ... existing checks ...
    my_check: &MyCheck,       // add this
    enable_parallel: bool,
) -> Result<ChunkProcessingResult, Box<dyn std::error::Error>> {
    let combined = if enable_parallel {
        records
            .par_iter()
            .fold(RecordResult::new, |mut acc, record| {
                acc.merge_record(record, null_check, ..., my_check);  // add my_check
                acc
            })
            .reduce(RecordResult::new, RecordResult::merge)
    } else {
        records.iter().fold(RecordResult::new(), |mut acc, record| {
            acc.merge_record(record, null_check, ..., my_check);      // add my_check
            acc
        })
    };

    Ok(ChunkProcessingResult {
        chunk_number,
        rows_processed: combined.rows_processed,
        // ... existing fields ...
        my_check_counts: combined.my_check_counts,   // add this
    })
}
```

If the number of parameters hits ~9+, add `#[allow(clippy::too_many_arguments)]` above the function.

## Step 6 — Instantiate in `process_csv_chunks`

Checks are stateless and `Send + Sync` — just instantiate by value and pass by reference. No `Arc` needed:

```rust
pub fn process_csv_chunks<R: Read>(...) {
    let null_check = NullLikeCheck::new();
    // ... existing checks ...
    let my_check = MyCheck::new();   // add this — no Arc::new() wrapper

    // inside the chunk loop:
    let result = process_single_chunk(
        &records, chunk_number,
        &null_check, ..., &my_check,   // add &my_check
        config.enable_parallel,
    )?;
}
```

## Step 7 — Extend `ChunkProcessingResult`

Add the public field:

```rust
pub struct ChunkProcessingResult {
    pub chunk_number: usize,
    pub rows_processed: usize,
    // ... existing fields ...
    pub my_check_counts: HashMap<usize, usize>,   // add this
}
```

## Step 8 — Extend `CsvAggregator` / `ColumnStats`

Add a field to `ColumnStats`:

```rust
my_check_count: usize,
```

Update `add_chunk_results` to accept and merge the new map:

```rust
pub fn add_chunk_results(
    &mut self,
    null_map: &HashMap<usize, usize>,
    // ... existing maps ...
    my_check_map: &HashMap<usize, usize>,   // add this
    chunk_size: usize,
) {
    // ... existing merge loops ...
    for (&col, &count) in my_check_map.iter() {
        if col < self.column_stats.len() {
            self.column_stats[col].my_check_count += count;
        }
    }
}
```

Update `generate_report` to include totals for the new check.

## Step 9 — Update `src/main.rs`

The `add_chunk_results` call in the result aggregation loop must be updated:

```rust
aggregator.add_chunk_results(
    &result.null_counts,
    &result.empty_counts,
    &result.whitespace_counts,
    // ... existing maps ...
    &result.my_check_counts,   // add this
    result.rows_processed,
);
```

## Step 10 — Extend `SparkStyleFormatter` in `src/formatter.rs`

Add your new check column to the summary table and per-column breakdown. Use the whitespace column as a template — the pattern is consistent across all checks.

## Step 11 — Write tests

### Unit test (inline in `src/lib.rs`):

```rust
#[test]
fn test_my_check() {
    let check = MyCheck::new();
    assert!(check.check("value that should match"));
    assert!(!check.check("value that should not match"));
    assert_eq!(check.name(), "MY_CHECK_NAME");
}
```

### Integration test in `tests/integration_tests.rs`:

```rust
use true_sight_csv::MyCheck;

#[test]
fn test_my_check_patterns() {
    let check = MyCheck::new();
    assert!(check.check("..."));
    assert!(!check.check("..."));
}
```

If your check should detect values in `tests/sample-warehouse-data.csv`, add an assertion to `test_process_csv_chunks` by summing the new count field across all chunk results — follow the `total_empty_found` / `total_null_found` pattern.

## Checklist

- [ ] Struct + `PatternCheck` impl in `src/lib.rs`
- [ ] `my_check_counts` field added to `RecordResult`
- [ ] Check added inside `merge_record` field loop (+ parameter added to signature)
- [ ] Merge added to `RecordResult::merge`
- [ ] `process_single_chunk` parameter added + both `merge_record` call sites updated
- [ ] `ChunkProcessingResult` has new public field
- [ ] `process_csv_chunks` instantiates `MyCheck::new()` (no Arc) and passes `&my_check`
- [ ] `ColumnStats` + `CsvAggregator::add_chunk_results` updated
- [ ] `src/main.rs` `add_chunk_results` call updated with new map argument
- [ ] `SparkStyleFormatter` renders the new column
- [ ] Unit test passes (`cargo test test_my_check`)
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean
