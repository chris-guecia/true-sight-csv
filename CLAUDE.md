# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build --release          # Optimized production build
cargo build                    # Debug build
cargo test                     # Run all tests
cargo test <test_name>         # Run a single test by name
cargo fmt                      # Format code
cargo clippy --all-targets --all-features -- -D warnings  # Lint (warnings as errors)
cargo check --all-targets      # Check compilation without building
cargo audit                    # Security vulnerability scan
```

Pre-commit hooks run `fmt`, `clippy`, `check`, `test`, and `audit` automatically on commit. To run manually:
```bash
pre-commit run --all-files
```

## Architecture

`true-sight-csv` is a CLI tool for detecting data quality issues (empty fields, NULL-like values, whitespace-only entries) in CSV files, with parallel processing support via Rayon.

**Data flow:**
1. `src/args.rs` — Parses and validates CLI args (`TrueSightCsvArgs`): file path, `--row-chunk-size` (default 1,000,000), `--disable-parallel`
2. `src/lib.rs` — Core library:
   - `prepare_csv_reader()` opens the CSV and extracts headers
   - `CsvChunkIterator` reads records in configurable chunks to bound memory usage
   - `PatternCheck` trait with three implementations: `NullLikeCheck`, `EmptyCheck`, `WhiteSpaceOnlyCheck`
   - `process_csv_chunks()` orchestrates chunk iteration → `process_single_chunk()` (parallel via `rayon::par_iter` or serial) → `process_record()` applies all pattern checks per field
   - Results accumulate in `CsvAggregator` as `ChunkProcessingResult` values (HashMaps of column→count)
3. `src/formatter.rs` — `SparkStyleFormatter` renders ASCII summary and per-column tables using `prettytable`
4. `src/main.rs` — Wires everything together

**Parallelism:** Records within each chunk are processed in parallel using `rayon`. Thread-safe aggregation uses `Arc<Mutex<HashMap>>`. Pass `--disable-parallel` to force serial processing.

**Tests:** Unit tests live inline in `src/args.rs`, `src/lib.rs`, and `src/formatter.rs`. Integration tests are in `tests/integration_tests.rs` and use `tests/sample-warehouse-data.csv` (12 rows × 9 columns with intentional data quality issues).

## Key Conventions

- Pre-commit hooks must pass before pushing.
