---
name: add-pattern-check
description: Add a new PatternCheck implementation to the true-sight-csv project. Use this skill whenever the user wants to detect a new kind of data quality issue in CSV files — for example, detecting numeric-only fields, date format issues, duplicate values, overly long strings, regex pattern violations, or any custom string check. Trigger on phrases like "add a check for", "detect", "flag fields that", "new pattern", "implement a check", or any request to catch a new category of bad/suspicious data.
---

# Adding a New PatternCheck

This skill guides adding a new data quality check to `true-sight-csv`. A check is a struct that implements the `PatternCheck` trait and gets wired into the processing pipeline.

## Step 1 — Implement the struct in `src/lib.rs`

Add the struct and its `PatternCheck` impl after the existing checks (around line 185). Follow this exact shape:

```rust
pub struct MyCheck;  // name it after what it detects

impl Default for MyCheck {
    fn default() -> Self { Self::new() }
}

impl MyCheck {
    pub fn new() -> Self { Self }
}

impl PatternCheck for MyCheck {
    fn name(&self) -> &str {
        "MY_CHECK_NAME"  // used in output tables — keep it short and SCREAMING_SNAKE_CASE
    }

    fn check(&self, value: &str) -> bool {
        // Return true when the value IS the problem
        // value is a raw CSV field string (never None — missing fields are "")
        todo!()
    }

    fn show_check_pattern(&self) -> &str {
        "human-readable description of what this flags"
    }
}
```

**`PatternCheck` requires `Send + Sync`** (line 89). Unit structs satisfy this automatically. If you add fields, use only `Send + Sync` types.

## Step 2 — Wire into `process_csv_chunks` and `process_single_chunk`

These two functions in `src/lib.rs` are currently hardcoded to three checks. You need to extend both.

### In `process_csv_chunks` (around line 376):

```rust
// Add alongside the existing Arc::new(...) lines:
let my_check = Arc::new(MyCheck::new());
```

Then pass it to `process_single_chunk`:

```rust
let result = process_single_chunk(
    &records,
    chunk_number,
    &null_check,
    &empty_check,
    &white_space_only_check,
    &my_check,          // add this
    config.enable_parallel,
)?;
```

### In `process_single_chunk` signature (around line 413):

Add the parameter:

```rust
my_check: &Arc<MyCheck>,
```

Add a counter inside the function body:

```rust
let my_check_counters = Arc::new(Mutex::new(HashMap::<usize, usize>::new()));
```

Pass it and the check into both the `par_iter` and `iter` calls to `process_record`.

Return it in `ChunkProcessingResult`:

```rust
Ok(ChunkProcessingResult {
    chunk_number,
    rows_processed: records.len(),
    null_counts,
    empty_counts,
    whitespace_counts,
    my_check_counts,    // add this
})
```

## Step 3 — Extend `process_record`

Add a local findings vec and a counter update block, mirroring the existing three:

```rust
let mut local_my_check_findings = Vec::new();

// inside the field loop:
if my_check.check(field) {
    local_my_check_findings.push(i);
}

// after the loop:
if !local_my_check_findings.is_empty() {
    let mut my_map = my_check_counters.lock().unwrap();
    for col in local_my_check_findings {
        *my_map.entry(col).or_insert(0) += 1;
    }
}
```

## Step 4 — Extend `ChunkProcessingResult`

Add the new count field:

```rust
pub my_check_counts: HashMap<usize, usize>,
```

And initialize it to `HashMap::new()` in any place that constructs this struct.

## Step 5 — Extend `CsvAggregator` / `ColumnStats`

Add a field to `ColumnStats`:

```rust
my_check_count: usize,
```

Update `add_chunk_results` to merge the new map, and `generate_report` to print it — follow the existing null/empty/whitespace pattern exactly.

## Step 6 — Update `src/main.rs`

The `add_chunk_results` call in the result aggregation loop (around line 69) must be updated to pass the new count map:

```rust
aggregator.add_chunk_results(
    &result.null_counts,
    &result.empty_counts,
    &result.whitespace_counts,
    &result.my_check_counts,   // add this
    result.rows_processed,
);
```

## Step 7 — Extend `SparkStyleFormatter` in `src/formatter.rs`

The formatter renders the per-column and summary tables. Add your new count/percentage column to the table-building code, following the `whitespace` column as a template.

## Step 8 — Write tests

### Unit test (inline in `src/lib.rs`)

```rust
#[test]
fn test_my_check() {
    let check = MyCheck::new();
    assert!(check.check("value that should match"));
    assert!(!check.check("value that should not match"));
}
```

### Integration test in `tests/integration_tests.rs`

Add an import at the top:

```rust
use true_sight_csv::MyCheck;
```

Write a focused unit test:

```rust
#[test]
fn test_my_check_patterns() {
    let check = MyCheck::new();
    assert!(check.check("..."));
    assert!(!check.check("..."));
}
```

If your check should detect values in `tests/sample-warehouse-data.csv`, add an assertion to `test_process_csv_chunks` by summing the new count field across all chunk results and asserting the expected total — the same pattern used for `total_empty_found` (32) and `total_null_found` (15).

## Checklist

- [ ] Struct + `PatternCheck` impl in `src/lib.rs`
- [ ] `Arc::new(MyCheck::new())` in `process_csv_chunks`
- [ ] New parameter + counter in `process_single_chunk`
- [ ] `process_record` updated (findings vec + lock + update)
- [ ] `ChunkProcessingResult` has new field
- [ ] `ColumnStats` + `CsvAggregator` updated
- [ ] `SparkStyleFormatter` renders the new column
- [ ] Unit test passes (`cargo test test_my_check`)
- [ ] `src/main.rs` `add_chunk_results` call updated with new map argument
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean
