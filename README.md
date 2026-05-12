# true-sight-csv

![banner](image.png)

A high-performance Rust-based CSV analysis tool that reveals hidden patterns and potential data quality issues in your CSV files. Just as the True Sight spell reveals what's hidden from normal view, this tool uncovers the hidden problems in your data that could impact your analysis.

## 🚀 Key Features
   - 🔍 Data Quality Detection: Identifies empty fields, NULL-like values, whitespace-only entries, placeholder values, digits-only fields, and dash-only fields
   - 🏷️ Header Analysis: Detects missing, empty, duplicate, or malformed column headers before processing begins
   - ⚡ High Performance: Parallel processing with chunked file reading for handling large datasets
   - 📊 Detailed Reporting: Comprehensive statistics with percentages and processing metrics in spark-like table format
   - 🔄 Memory Efficient: Processes files in configurable chunks (default: 1M rows) to handle datasets larger than available RAM
   - 📈 Performance Metrics: Real-time processing rates and timing information
   - 🎯 Zero-Lock Parallelism: Uses Rayon's `fold/reduce` pattern — each thread owns its accumulator, no mutexes, results merged once at the end


## 🔍 Current Checks

### Data Checks (per cell)
- Empty fields
- NULL-like values — variations of `NULL`, `N/A`, `NA`, `NONE`, `NaN` (case-insensitive)
- Whitespace-only values
- Placeholder values — `TBD`, `TODO`, `PLACEHOLDER`, `UNKNOWN` (case-insensitive)
- Digits-only values — fields containing only numeric characters
- Dash-only values — fields whose trimmed value is `-` or `--`
- Boolean-like values — fields containing `true`, `false`, `yes`, `no`, `1`, `0`, `on`, `off` (case-insensitive); flags columns stored as free text that should be a proper boolean type

### Header Checks (once per file)
- Empty column names (e.g. trailing comma in header row)
- Duplicate column names
- NULL-like column names
- Numeric column names — flags columns that may be data values mistaken for headers, with a warning when all headers are numeric (strong signal the file is missing a header row entirely)

## Parallelism

Records within each chunk are processed using Rayon's `fold/reduce` pattern rather than shared `Arc<Mutex<T>>` counters:

- `fold` — each Rayon thread gets its own local accumulator (`HashMap` per check type), zero contention during processing
- `reduce` — only N_threads accumulators are merged once at the end, regardless of how many issues are found

This means performance scales linearly with cores and improves as issue density increases. On clean data the difference is modest (~7% faster); on high-issue-rate data the gap widens (~15% faster, ~33% less CPU) because the mutex version pays a lock acquisition cost for every flagged cell while the fold/reduce version pays nothing until the final merge.

## 🛠️ Installation
```bash
# Clone this repository
git clone

# Build the project
cd true-sight-csv
cargo build --release

```

## 📖 Usage
### Basic Usage

Analyze a CSV file with default settings (1,000,000 row chunks, parallel processing enabled)
```
./target/release/true-sight-csv /path/to/your/file.csv
```

Run directly with cargo during development
```
cargo run -- "path/to/your/file.csv"
```

### With Subcommands

#### Custom chunk size
Process with smaller chunks (useful for memory-constrained environments)
```
./target/release/true-sight-csv /path/to/your/file.csv --row-chunk-size 500000
```

#### Disable parallel processing
Run in single-threaded mode
```
./target/release/true-sight-csv /path/to/your/file.csv --disable-parallel
```

#### Combine options
Use custom chunk size with single-threaded processing
```
./target/release/true-sight-csv /path/to/your/file.csv --row-chunk-size 100000 --disable-parallel
```

### Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--row-chunk-size <SIZE>` | Number of rows to process in each chunk | 1,000,000 |
| `--disable-parallel` | Disable parallel processing (use single thread) | Parallel enabled |
| `--help` | Show help information | - |
| `--version` | Show version information | - |

## Testing
```
cargo test
```

### Windows
```powershell
# Navigate to project directory
cd C:\path\to\true-sight-csv

# Build release version (do this once)
cargo build --release

# Basic usage
.\target\release\true-sight-csv.exe "C:\data\your-file.csv"

# With custom chunk size
.\target\release\true-sight-csv.exe "C:\data\your-file.csv" --row-chunk-size 500000

# Disable parallel processing
.\target\release\true-sight-csv.exe "C:\data\your-file.csv" --disable-parallel

# Both options together
.\target\release\true-sight-csv.exe "C:\data\your-file.csv" --row-chunk-size 100000 --disable-parallel

# Example with test data
.\target\release\true-sight-csv.exe "tests\sample-data.csv"
```

### Linux/macOS
```bash
# Navigate to project directory
cd /path/to/true-sight-csv

# Build release version (do this once)
cargo build --release

# Basic usage
./target/release/true-sight-csv "/home/user/data/your-file.csv"

# With custom chunk size
./target/release/true-sight-csv "/home/user/data/your-file.csv" --row-chunk-size 500000

# Disable parallel processing
./target/release/true-sight-csv "/home/user/data/your-file.csv" --disable-parallel

# Both options together
./target/release/true-sight-csv "/home/user/data/your-file.csv" --row-chunk-size 100000 --disable-parallel

# Example with test data
./target/release/true-sight-csv "tests/sample-data.csv"
```

### Development with Cargo

```bash
# Basic usage during development
cargo run -- "path/to/your/file.csv"

# With options during development
cargo run -- "path/to/your/file.csv" --row-chunk-size 250000
cargo run -- "path/to/your/file.csv" --disable-parallel
cargo run -- "path/to/your/file.csv" --row-chunk-size 100000
```

## 🛠️ Development Setup

### Pre-commit Hooks

This project uses [pre-commit](https://pre-commit.com/) to maintain code quality and consistency. Pre-commit hooks run automatically before each commit to:

- Format code with `cargo fmt`
- Lint code with `cargo clippy`
- Run tests with `cargo test`
- Check for security vulnerabilities with `cargo audit`
- Validate file formatting and catch common issues

#### Installation

1. **Install pre-commit**:
   ```bash
   # Using pip
   pip install pre-commit

   # Using Chocolatey (Windows)
   choco install pre-commit

   # Using Homebrew (macOS)
   brew install pre-commit
   ```

2. **Install the hooks**:
   ```bash
   cd /path/to/true-sight-csv
   pre-commit install
   ```

3. **Install cargo-audit** (optional but recommended):
   ```bash
   cargo install cargo-audit
   ```

#### What Gets Checked

| Check | Purpose | Auto-fixes |
|-------|---------|------------|
| `cargo fmt` | Code formatting | ✅ Yes |
| `cargo clippy` | Linting and best practices | ❌ Manual |
| `cargo check` | Compilation errors | ❌ Manual |
| `cargo test` | Test suite | ❌ Manual |
| `cargo audit` | Security vulnerabilities | ❌ Manual |
| File formatting | Whitespace, line endings, etc. | ✅ Yes |

#### Working with Pre-commit

**Normal workflow:**
```bash
# Make your changes
git add .
git commit -m "your message"
# Hooks run automatically, commit proceeds if all pass
```

**When hooks fail:**
```bash
# Fix any issues reported by clippy/tests
# Re-stage auto-fixed files
git add .
# Try committing again
git commit -m "your message"
```

**Emergency bypass** (use sparingly):
```bash
git commit -m "emergency fix" --no-verify
```

**Run hooks manually:**
```bash
# Run all hooks on all files
pre-commit run --all-files

# Run specific hook
pre-commit run clippy --all-files
```

#### Troubleshooting

**Slow first run**: `cargo audit` downloads a security database on first run. Subsequent runs are much faster.

**Permission issues**: On Windows, run PowerShell as Administrator if you encounter permission errors.

**Skip cargo-audit**: If cargo-audit is too slow or not needed, comment out those lines in `.pre-commit-config.yaml`.

---


## 📋 Sample Output
```
Provided full path to file: TrueSightCsvArgs { file_full_path: "\\true-sight-csv\\tests\\sample-warehouse-data.csv" }
Valid CSV path: "\\true-sight-csv\\tests\\sample-warehouse-data.csv"
Found headers: ["customer_id", "order_date", "product_sku", "quantity", "unit_price", "shipping_zip", "email", "last_updated_timestamp", ""]
Chunk read with 12 records

Found headers: ["customer_id", "order_date", "product_sku", "quantity", "unit_price", "shipping_zip", "email", "last_updated_timestamp", ""]
Chunk read with 12 records
=== PROCESSING SUMMARY ===
+-------------------+-------+----------------+
|      Metric       | Count | % of All Cells |
+-------------------+-------+----------------+
| Total Rows        |    12 | -              |
| Total Chunks      |     1 | -              |
| Total Cells       |   108 |       100.000% |
| NULL-like Values  |    15 |        13.889% |
| Empty Values      |    32 |        29.630% |
| Whitespace Values |     4 |         3.704% |
+-------------------+-------+----------------+
Dataset: 12 rows × 9 columns = 108 total cells

=== DATA QUALITY SUMMARY BY COLUMN ===
+--------+------------------------+------------+------------------+-------------+-------------------+------------------+------------------------+
| Column |      Column Name       | NULL Count | NULL % of Column | Empty Count | Empty % of Column | Whitespace Count | Whitespace % of Column |
+--------+------------------------+------------+------------------+-------------+-------------------+------------------+------------------------+
|      0 | customer_id            |          5 |            41.7% |           0 |              0.0% |                0 |                   0.0% |
|      1 | order_date             |          2 |            16.7% |           1 |              8.3% |                0 |                   0.0% |
|      2 | product_sku            |          2 |            16.7% |           3 |             25.0% |                2 |                  16.7% |
|      3 | quantity               |          2 |            16.7% |           3 |             25.0% |                0 |                   0.0% |
|      4 | unit_price             |          1 |             8.3% |           3 |             25.0% |                0 |                   0.0% |
|      5 | shipping_zip           |          1 |             8.3% |           3 |             25.0% |                0 |                   0.0% |
|      6 | email                  |          0 |             0.0% |           7 |             58.3% |                0 |                   0.0% |
|      7 | last_updated_timestamp |          1 |             8.3% |           3 |             25.0% |                1 |                   8.3% |
|      8 |                        |          1 |             8.3% |           9 |             75.0% |                1 |                   8.3% |
+--------+------------------------+------------+------------------+-------------+-------------------+------------------+------------------------+

=== NULL-LIKE VALUES ===
+--------+------------------------+-----------------+--------------------+------------------+
| Column |      Column Name       | NULL-like Count | % of All NULL-like | % of Column Rows |
+--------+------------------------+-----------------+--------------------+------------------+
|      0 | customer_id            |               5 |              33.3% |          41.667% |
|      1 | order_date             |               2 |              13.3% |          16.667% |
|      2 | product_sku            |               2 |              13.3% |          16.667% |
|      3 | quantity               |               2 |              13.3% |          16.667% |
|      4 | unit_price             |               1 |               6.7% |           8.333% |
|      5 | shipping_zip           |               1 |               6.7% |           8.333% |
|      6 | email                  |               0 | -                  |           0.000% |
|      7 | last_updated_timestamp |               1 |               6.7% |           8.333% |
|      8 |                        |               1 |               6.7% |           8.333% |
+--------+------------------------+-----------------+--------------------+------------------+
Total null-like values: 15 (13.889% of all cells in dataset)

=== EMPTY VALUES ===
+--------+------------------------+-------------+----------------+------------------+
| Column |      Column Name       | Empty Count | % of All Empty | % of Column Rows |
+--------+------------------------+-------------+----------------+------------------+
|      0 | customer_id            |           0 | -              |           0.000% |
|      1 | order_date             |           1 |           3.1% |           8.333% |
|      2 | product_sku            |           3 |           9.4% |          25.000% |
|      3 | quantity               |           3 |           9.4% |          25.000% |
|      4 | unit_price             |           3 |           9.4% |          25.000% |
|      5 | shipping_zip           |           3 |           9.4% |          25.000% |
|      6 | email                  |           7 |          21.9% |          58.333% |
|      7 | last_updated_timestamp |           3 |           9.4% |          25.000% |
|      8 |                        |           9 |          28.1% |          75.000% |
+--------+------------------------+-------------+----------------+------------------+
Total empty values: 32 (29.630% of all cells in dataset)

=== WHITESPACE VALUES ===
+--------+------------------------+------------------+---------------------+------------------+
| Column |      Column Name       | Whitespace Count | % of All Whitespace | % of Column Rows |
+--------+------------------------+------------------+---------------------+------------------+
|      0 | customer_id            |                0 | -                   |           0.000% |
|      1 | order_date             |                0 | -                   |           0.000% |
|      2 | product_sku            |                2 |               50.0% |          16.667% |
|      3 | quantity               |                0 | -                   |           0.000% |
|      4 | unit_price             |                0 | -                   |           0.000% |
|      5 | shipping_zip           |                0 | -                   |           0.000% |
|      6 | email                  |                0 | -                   |           0.000% |
|      7 | last_updated_timestamp |                1 |               25.0% |           8.333% |
|      8 |                        |                1 |               25.0% |           8.333% |
+--------+------------------------+------------------+---------------------+------------------+
Total whitespace values: 4 (3.704% of all cells in dataset)


=== PROCESSING COMPLETE ===
Total rows processed: 12
Total chunks processed: 1
Processing time: 13.1522ms

=== CSV QUALITY REPORT ===
Total rows processed: 12
Total columns: 9
Total data quality issues found:
  - NULL-like values: 15
  - Empty values: 32
  - Whitespace-only values: 4
  - Total issues: 51
Processing rate: 12 rows/second
Overall data quality: 52.78% clean cells
```


## Example CLI input with Output on larger dataset ~1.2M row CSV at 529 MB (554,823,961 bytes) with a Processing time: ~4s (wall clock, release build)
data found here -> https://www.kaggle.com/datasets/asaniczka/tmdb-movies-dataset-2023-930k-movies?resource=download
```
Provided full path to file: TrueSightCsvArgs { file_full_path: "Path_to_\\TMDB_movie_dataset_v11.csv", row_chunk_size: 1000000, disable_parallel: false }
Valid CSV path: "Path_to_\\TMDB_movie_dataset_v11.csv"
Using chunk size: 1000000
Parallel execution: true
Found headers: ["id", "title", "vote_average", "vote_count", "status", "release_date", "revenue", "runtime", "adult", "backdrop_path", "budget", "homepage", "imdb_id", "original_language", "original_title", "overview", "popularity", "poster_path", "tagline", "genres", "production_companies", "production_countries", "spoken_languages", "keywords"]

=== HEADER ANALYSIS ===
No header issues detected.
Chunk read with 1000000 records
Chunk read with 225501 records
=== PROCESSING SUMMARY ===
+--------------------------+----------+----------------+
|          Metric          |  Count   | % of All Cells |
+--------------------------+----------+----------------+
| Total Rows               |  1225501 | -              |
| Total Chunks             |        2 | -              |
| Total Cells              | 29412024 |       100.000% |
| NULL-like Values         |     1200 |         0.004% |
| Empty Values             |  7746445 |        26.338% |
| Whitespace Values        |     1191 |         0.004% |
| Placeholder Values       |       62 |         0.000% |
| Digits-Only Values       |  6130046 |        20.842% |
| Dash-Only Values         |       28 |         0.000% |
| Boolean-Like Values      |  4958155 |        16.858% |
| Special-Char-Only Values |      364 |         0.001% |
+--------------------------+----------+----------------+
Dataset: 1225501 rows × 24 columns = 29412024 total cells

=== DATA QUALITY SUMMARY BY COLUMN ===
+--------+----------------------+------------+------------------+-------------+-------------------+------------------+------------------------+-------------------+-------------------------+-----------------+-----------------------+-------------------+-------------------------+--------------------+--------------------------+-------------------+-------------------------+
| Column |     Column Name      | NULL Count | NULL % of Column | Empty Count | Empty % of Column | Whitespace Count | Whitespace % of Column | Placeholder Count | Placeholder % of Column | Dash-Only Count | Dash-Only % of Column | Digits-Only Count | Digits-Only % of Column | Boolean-Like Count | Boolean-Like % of Column | SpecialChar Count | SpecialChar % of Column |
+--------+----------------------+------------+------------------+-------------+-------------------+------------------+------------------------+-------------------+-------------------------+-----------------+-----------------------+-------------------+-------------------------+--------------------+--------------------------+-------------------+-------------------------+
|      0 | id                   |          0 |             0.0% |           0 |              0.0% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |           1225501 |                  100.0% |                  0 |                     0.0% |                 0 |                    0.0% |
|      1 | title                |          3 |             0.0% |          13 |              0.0% |                3 |                   0.0% |                29 |                    0.0% |               7 |                  0.0% |              1179 |                    0.1% |                 67 |                     0.0% |                96 |                    0.0% |
|      2 | vote_average         |          0 |             0.0% |           0 |              0.0% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 0 |                    0.0% |                  0 |                     0.0% |                 0 |                    0.0% |
|      3 | vote_count           |          0 |             0.0% |           0 |              0.0% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |           1225501 |                  100.0% |            1000790 |                    81.7% |                 0 |                    0.0% |
|      4 | status               |          0 |             0.0% |           0 |              0.0% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 0 |                    0.0% |                  0 |                     0.0% |                 0 |                    0.0% |
|      5 | release_date         |          0 |             0.0% |      225713 |             18.4% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 0 |                    0.0% |                  0 |                     0.0% |                 0 |                    0.0% |
|      6 | revenue              |          0 |             0.0% |           0 |              0.0% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |           1225500 |                  100.0% |            1204105 |                    98.3% |                 0 |                    0.0% |
|      7 | runtime              |          0 |             0.0% |           0 |              0.0% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |           1225500 |                  100.0% |             363682 |                    29.7% |                 0 |                    0.0% |
|      8 | adult                |          0 |             0.0% |           0 |              0.0% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 0 |                    0.0% |            1225501 |                   100.0% |                 0 |                    0.0% |
|      9 | backdrop_path        |        486 |             0.0% |      906894 |             74.0% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 0 |                    0.0% |                  0 |                     0.0% |                 0 |                    0.0% |
|     10 | budget               |          0 |             0.0% |           0 |              0.0% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |           1225501 |                  100.0% |            1160872 |                    94.7% |                 0 |                    0.0% |
|     11 | homepage             |          0 |             0.0% |     1096986 |             89.5% |               20 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 0 |                    0.0% |                  0 |                     0.0% |                 0 |                    0.0% |
|     12 | imdb_id              |        450 |             0.0% |      601811 |             49.1% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 0 |                    0.0% |                  0 |                     0.0% |                 0 |                    0.0% |
|     13 | original_language    |          0 |             0.0% |           0 |              0.0% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 0 |                    0.0% |               3051 |                     0.2% |                 0 |                    0.0% |
|     14 | original_title       |          3 |             0.0% |          13 |              0.0% |                0 |                   0.0% |                24 |                    0.0% |               7 |                  0.0% |              1143 |                    0.1% |                 55 |                     0.0% |               103 |                    0.0% |
|     15 | overview             |          0 |             0.0% |      260087 |             21.2% |             1168 |                   0.1% |                 4 |                    0.0% |               0 |                  0.0% |                 8 |                    0.0% |                  4 |                     0.0% |               127 |                    0.0% |
|     16 | popularity           |          0 |             0.0% |           0 |              0.0% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 0 |                    0.0% |                  0 |                     0.0% |                 0 |                    0.0% |
|     17 | poster_path          |        249 |             0.0% |      403416 |             32.9% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 0 |                    0.0% |                  0 |                     0.0% |                 0 |                    0.0% |
|     18 | tagline              |          9 |             0.0% |     1053523 |             86.0% |                0 |                   0.0% |                 3 |                    0.0% |              14 |                  0.0% |               180 |                    0.0% |                 25 |                     0.0% |                36 |                    0.0% |
|     19 | genres               |          0 |             0.0% |      508428 |             41.5% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 0 |                    0.0% |                  0 |                     0.0% |                 0 |                    0.0% |
|     20 | production_companies |          0 |             0.0% |      683192 |             55.7% |                0 |                   0.0% |                 2 |                    0.0% |               0 |                  0.0% |                30 |                    0.0% |                  3 |                     0.0% |                 2 |                    0.0% |
|     21 | production_countries |          0 |             0.0% |      561763 |             45.8% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 0 |                    0.0% |                  0 |                     0.0% |                 0 |                    0.0% |
|     22 | spoken_languages     |          0 |             0.0% |      540087 |             44.1% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 0 |                    0.0% |                  0 |                     0.0% |                 0 |                    0.0% |
|     23 | keywords             |          0 |             0.0% |      904519 |             73.8% |                0 |                   0.0% |                 0 |                    0.0% |               0 |                  0.0% |                 3 |                    0.0% |                  0 |                     0.0% |                 0 |                    0.0% |
+--------+----------------------+------------+------------------+-------------+-------------------+------------------+------------------------+-------------------+-------------------------+-----------------+-----------------------+-------------------+-------------------------+--------------------+--------------------------+-------------------+-------------------------+

=== NULL-LIKE VALUES ===
+--------+----------------------+-----------------+--------------------+------------------+
| Column |     Column Name      | NULL-like Count | % of All NULL-like | % of Column Rows |
+--------+----------------------+-----------------+--------------------+------------------+
|      0 | id                   |               0 | -                  |           0.000% |
|      1 | title                |               3 |               0.2% |           0.000% |
|      2 | vote_average         |               0 | -                  |           0.000% |
|      3 | vote_count           |               0 | -                  |           0.000% |
|      4 | status               |               0 | -                  |           0.000% |
|      5 | release_date         |               0 | -                  |           0.000% |
|      6 | revenue              |               0 | -                  |           0.000% |
|      7 | runtime              |               0 | -                  |           0.000% |
|      8 | adult                |               0 | -                  |           0.000% |
|      9 | backdrop_path        |             486 |              40.5% |           0.040% |
|     10 | budget               |               0 | -                  |           0.000% |
|     11 | homepage             |               0 | -                  |           0.000% |
|     12 | imdb_id              |             450 |              37.5% |           0.037% |
|     13 | original_language    |               0 | -                  |           0.000% |
|     14 | original_title       |               3 |               0.2% |           0.000% |
|     15 | overview             |               0 | -                  |           0.000% |
|     16 | popularity           |               0 | -                  |           0.000% |
|     17 | poster_path          |             249 |              20.8% |           0.020% |
|     18 | tagline              |               9 |               0.8% |           0.001% |
|     19 | genres               |               0 | -                  |           0.000% |
|     20 | production_companies |               0 | -                  |           0.000% |
|     21 | production_countries |               0 | -                  |           0.000% |
|     22 | spoken_languages     |               0 | -                  |           0.000% |
|     23 | keywords             |               0 | -                  |           0.000% |
+--------+----------------------+-----------------+--------------------+------------------+
Total null-like values: 1200 (0.004% of all cells in dataset)

=== EMPTY VALUES ===
+--------+----------------------+-------------+----------------+------------------+
| Column |     Column Name      | Empty Count | % of All Empty | % of Column Rows |
+--------+----------------------+-------------+----------------+------------------+
|      0 | id                   |           0 | -              |           0.000% |
|      1 | title                |          13 |           0.0% |           0.001% |
|      2 | vote_average         |           0 | -              |           0.000% |
|      3 | vote_count           |           0 | -              |           0.000% |
|      4 | status               |           0 | -              |           0.000% |
|      5 | release_date         |      225713 |           2.9% |          18.418% |
|      6 | revenue              |           0 | -              |           0.000% |
|      7 | runtime              |           0 | -              |           0.000% |
|      8 | adult                |           0 | -              |           0.000% |
|      9 | backdrop_path        |      906894 |          11.7% |          74.002% |
|     10 | budget               |           0 | -              |           0.000% |
|     11 | homepage             |     1096986 |          14.2% |          89.513% |
|     12 | imdb_id              |      601811 |           7.8% |          49.107% |
|     13 | original_language    |           0 | -              |           0.000% |
|     14 | original_title       |          13 |           0.0% |           0.001% |
|     15 | overview             |      260087 |           3.4% |          21.223% |
|     16 | popularity           |           0 | -              |           0.000% |
|     17 | poster_path          |      403416 |           5.2% |          32.918% |
|     18 | tagline              |     1053523 |          13.6% |          85.967% |
|     19 | genres               |      508428 |           6.6% |          41.487% |
|     20 | production_companies |      683192 |           8.8% |          55.748% |
|     21 | production_countries |      561763 |           7.3% |          45.839% |
|     22 | spoken_languages     |      540087 |           7.0% |          44.071% |
|     23 | keywords             |      904519 |          11.7% |          73.808% |
+--------+----------------------+-------------+----------------+------------------+
Total empty values: 7746445 (26.338% of all cells in dataset)

=== WHITESPACE VALUES ===
+--------+----------------------+------------------+---------------------+------------------+
| Column |     Column Name      | Whitespace Count | % of All Whitespace | % of Column Rows |
+--------+----------------------+------------------+---------------------+------------------+
|      0 | id                   |                0 | -                   |           0.000% |
|      1 | title                |                3 |                0.3% |           0.000% |
|      2 | vote_average         |                0 | -                   |           0.000% |
|      3 | vote_count           |                0 | -                   |           0.000% |
|      4 | status               |                0 | -                   |           0.000% |
|      5 | release_date         |                0 | -                   |           0.000% |
|      6 | revenue              |                0 | -                   |           0.000% |
|      7 | runtime              |                0 | -                   |           0.000% |
|      8 | adult                |                0 | -                   |           0.000% |
|      9 | backdrop_path        |                0 | -                   |           0.000% |
|     10 | budget               |                0 | -                   |           0.000% |
|     11 | homepage             |               20 |                1.7% |           0.002% |
|     12 | imdb_id              |                0 | -                   |           0.000% |
|     13 | original_language    |                0 | -                   |           0.000% |
|     14 | original_title       |                0 | -                   |           0.000% |
|     15 | overview             |             1168 |               98.1% |           0.095% |
|     16 | popularity           |                0 | -                   |           0.000% |
|     17 | poster_path          |                0 | -                   |           0.000% |
|     18 | tagline              |                0 | -                   |           0.000% |
|     19 | genres               |                0 | -                   |           0.000% |
|     20 | production_companies |                0 | -                   |           0.000% |
|     21 | production_countries |                0 | -                   |           0.000% |
|     22 | spoken_languages     |                0 | -                   |           0.000% |
|     23 | keywords             |                0 | -                   |           0.000% |
+--------+----------------------+------------------+---------------------+------------------+
Total whitespace values: 1191 (0.004% of all cells in dataset)

=== PLACEHOLDER VALUES ===
+--------+----------------------+-------------------+----------------------+------------------+
| Column |     Column Name      | Placeholder Count | % of All Placeholder | % of Column Rows |
+--------+----------------------+-------------------+----------------------+------------------+
|      0 | id                   |                 0 | -                    |           0.000% |
|      1 | title                |                29 |                46.8% |           0.002% |
|      2 | vote_average         |                 0 | -                    |           0.000% |
|      3 | vote_count           |                 0 | -                    |           0.000% |
|      4 | status               |                 0 | -                    |           0.000% |
|      5 | release_date         |                 0 | -                    |           0.000% |
|      6 | revenue              |                 0 | -                    |           0.000% |
|      7 | runtime              |                 0 | -                    |           0.000% |
|      8 | adult                |                 0 | -                    |           0.000% |
|      9 | backdrop_path        |                 0 | -                    |           0.000% |
|     10 | budget               |                 0 | -                    |           0.000% |
|     11 | homepage             |                 0 | -                    |           0.000% |
|     12 | imdb_id              |                 0 | -                    |           0.000% |
|     13 | original_language    |                 0 | -                    |           0.000% |
|     14 | original_title       |                24 |                38.7% |           0.002% |
|     15 | overview             |                 4 |                 6.5% |           0.000% |
|     16 | popularity           |                 0 | -                    |           0.000% |
|     17 | poster_path          |                 0 | -                    |           0.000% |
|     18 | tagline              |                 3 |                 4.8% |           0.000% |
|     19 | genres               |                 0 | -                    |           0.000% |
|     20 | production_companies |                 2 |                 3.2% |           0.000% |
|     21 | production_countries |                 0 | -                    |           0.000% |
|     22 | spoken_languages     |                 0 | -                    |           0.000% |
|     23 | keywords             |                 0 | -                    |           0.000% |
+--------+----------------------+-------------------+----------------------+------------------+
Total placeholder values: 62 (0.000% of all cells in dataset)

=== DIGITS-ONLY VALUES ===
+--------+----------------------+-------------------+----------------------+------------------+
| Column |     Column Name      | Digits-Only Count | % of All Digits-Only | % of Column Rows |
+--------+----------------------+-------------------+----------------------+------------------+
|      0 | id                   |           1225501 |                20.0% |         100.000% |
|      1 | title                |              1179 |                 0.0% |           0.096% |
|      2 | vote_average         |                 0 | -                    |           0.000% |
|      3 | vote_count           |           1225501 |                20.0% |         100.000% |
|      4 | status               |                 0 | -                    |           0.000% |
|      5 | release_date         |                 0 | -                    |           0.000% |
|      6 | revenue              |           1225500 |                20.0% |         100.000% |
|      7 | runtime              |           1225500 |                20.0% |         100.000% |
|      8 | adult                |                 0 | -                    |           0.000% |
|      9 | backdrop_path        |                 0 | -                    |           0.000% |
|     10 | budget               |           1225501 |                20.0% |         100.000% |
|     11 | homepage             |                 0 | -                    |           0.000% |
|     12 | imdb_id              |                 0 | -                    |           0.000% |
|     13 | original_language    |                 0 | -                    |           0.000% |
|     14 | original_title       |              1143 |                 0.0% |           0.093% |
|     15 | overview             |                 8 |                 0.0% |           0.001% |
|     16 | popularity           |                 0 | -                    |           0.000% |
|     17 | poster_path          |                 0 | -                    |           0.000% |
|     18 | tagline              |               180 |                 0.0% |           0.015% |
|     19 | genres               |                 0 | -                    |           0.000% |
|     20 | production_companies |                30 |                 0.0% |           0.002% |
|     21 | production_countries |                 0 | -                    |           0.000% |
|     22 | spoken_languages     |                 0 | -                    |           0.000% |
|     23 | keywords             |                 3 |                 0.0% |           0.000% |
+--------+----------------------+-------------------+----------------------+------------------+
Total digits-only values: 6130046 (20.842% of all cells in dataset)

=== DASH-ONLY VALUES ===
+--------+----------------------+-----------------+--------------------+------------------+
| Column |     Column Name      | Dash-Only Count | % of All Dash-Only | % of Column Rows |
+--------+----------------------+-----------------+--------------------+------------------+
|      0 | id                   |               0 | -                  |           0.000% |
|      1 | title                |               7 |              25.0% |           0.001% |
|      2 | vote_average         |               0 | -                  |           0.000% |
|      3 | vote_count           |               0 | -                  |           0.000% |
|      4 | status               |               0 | -                  |           0.000% |
|      5 | release_date         |               0 | -                  |           0.000% |
|      6 | revenue              |               0 | -                  |           0.000% |
|      7 | runtime              |               0 | -                  |           0.000% |
|      8 | adult                |               0 | -                  |           0.000% |
|      9 | backdrop_path        |               0 | -                  |           0.000% |
|     10 | budget               |               0 | -                  |           0.000% |
|     11 | homepage             |               0 | -                  |           0.000% |
|     12 | imdb_id              |               0 | -                  |           0.000% |
|     13 | original_language    |               0 | -                  |           0.000% |
|     14 | original_title       |               7 |              25.0% |           0.001% |
|     15 | overview             |               0 | -                  |           0.000% |
|     16 | popularity           |               0 | -                  |           0.000% |
|     17 | poster_path          |               0 | -                  |           0.000% |
|     18 | tagline              |              14 |              50.0% |           0.001% |
|     19 | genres               |               0 | -                  |           0.000% |
|     20 | production_companies |               0 | -                  |           0.000% |
|     21 | production_countries |               0 | -                  |           0.000% |
|     22 | spoken_languages     |               0 | -                  |           0.000% |
|     23 | keywords             |               0 | -                  |           0.000% |
+--------+----------------------+-----------------+--------------------+------------------+
Total dash-only values: 28 (0.000% of all cells in dataset)

=== BOOLEAN-LIKE VALUES ===
+--------+----------------------+--------------------+-----------------------+------------------+
| Column |     Column Name      | Boolean-Like Count | % of All Boolean-Like | % of Column Rows |
+--------+----------------------+--------------------+-----------------------+------------------+
|      0 | id                   |                  0 | -                     |           0.000% |
|      1 | title                |                 67 |                  0.0% |           0.005% |
|      2 | vote_average         |                  0 | -                     |           0.000% |
|      3 | vote_count           |            1000790 |                 20.2% |          81.664% |
|      4 | status               |                  0 | -                     |           0.000% |
|      5 | release_date         |                  0 | -                     |           0.000% |
|      6 | revenue              |            1204105 |                 24.3% |          98.254% |
|      7 | runtime              |             363682 |                  7.3% |          29.676% |
|      8 | adult                |            1225501 |                 24.7% |         100.000% |
|      9 | backdrop_path        |                  0 | -                     |           0.000% |
|     10 | budget               |            1160872 |                 23.4% |          94.726% |
|     11 | homepage             |                  0 | -                     |           0.000% |
|     12 | imdb_id              |                  0 | -                     |           0.000% |
|     13 | original_language    |               3051 |                  0.1% |           0.249% |
|     14 | original_title       |                 55 |                  0.0% |           0.004% |
|     15 | overview             |                  4 |                  0.0% |           0.000% |
|     16 | popularity           |                  0 | -                     |           0.000% |
|     17 | poster_path          |                  0 | -                     |           0.000% |
|     18 | tagline              |                 25 |                  0.0% |           0.002% |
|     19 | genres               |                  0 | -                     |           0.000% |
|     20 | production_companies |                  3 |                  0.0% |           0.000% |
|     21 | production_countries |                  0 | -                     |           0.000% |
|     22 | spoken_languages     |                  0 | -                     |           0.000% |
|     23 | keywords             |                  0 | -                     |           0.000% |
+--------+----------------------+--------------------+-----------------------+------------------+
Total boolean-like values: 4958155 (16.858% of all cells in dataset)

=== SPECIAL-CHAR-ONLY VALUES ===
+--------+----------------------+-------------------------+---------------------------+------------------+
| Column |     Column Name      | Special-Char-Only Count | % of All Special-Char-... | % of Column Rows |
+--------+----------------------+-------------------------+---------------------------+------------------+
|      0 | id                   |                       0 | -                         |           0.000% |
|      1 | title                |                      96 |                     26.4% |           0.008% |
|      2 | vote_average         |                       0 | -                         |           0.000% |
|      3 | vote_count           |                       0 | -                         |           0.000% |
|      4 | status               |                       0 | -                         |           0.000% |
|      5 | release_date         |                       0 | -                         |           0.000% |
|      6 | revenue              |                       0 | -                         |           0.000% |
|      7 | runtime              |                       0 | -                         |           0.000% |
|      8 | adult                |                       0 | -                         |           0.000% |
|      9 | backdrop_path        |                       0 | -                         |           0.000% |
|     10 | budget               |                       0 | -                         |           0.000% |
|     11 | homepage             |                       0 | -                         |           0.000% |
|     12 | imdb_id              |                       0 | -                         |           0.000% |
|     13 | original_language    |                       0 | -                         |           0.000% |
|     14 | original_title       |                     103 |                     28.3% |           0.008% |
|     15 | overview             |                     127 |                     34.9% |           0.010% |
|     16 | popularity           |                       0 | -                         |           0.000% |
|     17 | poster_path          |                       0 | -                         |           0.000% |
|     18 | tagline              |                      36 |                      9.9% |           0.003% |
|     19 | genres               |                       0 | -                         |           0.000% |
|     20 | production_companies |                       2 |                      0.5% |           0.000% |
|     21 | production_countries |                       0 | -                         |           0.000% |
|     22 | spoken_languages     |                       0 | -                         |           0.000% |
|     23 | keywords             |                       0 | -                         |           0.000% |
+--------+----------------------+-------------------------+---------------------------+------------------+
Total special-char-only values: 364 (0.001% of all cells in dataset)


=== PROCESSING COMPLETE ===
Total rows processed: 1225501
Total chunks processed: 2
Processing time: 3.5289402s

=== CSV QUALITY REPORT ===
Total rows processed: 1225501
Total columns: 24
Total data quality issues found:
  - NULL-like values: 1200
  - Empty values: 7746445
  - Whitespace-only values: 1191
  - Total issues: 7748836
Processing rate: 347272 rows/second
Overall data quality: 73.65% clean cells
```
