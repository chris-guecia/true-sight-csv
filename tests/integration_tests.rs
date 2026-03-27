use std::fs::File;
use std::path::PathBuf;
use true_sight_csv::{
    prepare_csv_reader, process_csv_chunks, CsvChunkIterator, DashOnlyCheck, DigitsOnlyCheck,
    EmptyCheck, NullLikeCheck, PatternCheck, PlaceholderCheck, ProcessingConfig,
    WhiteSpaceOnlyCheck,
};

// Helper function to get the path to a fixture file
fn get_fixture_path(fixture_filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests"); // Assuming fixtures are in a subdirectory of 'tests'
    path.push(fixture_filename);
    path
}

#[test]
fn test_path_printing() {
    let test_path = get_fixture_path("sample-warehouse-data.csv");
    println!("{:?}", test_path); // Use double quotes, not single quotes

    // Or if you want it as a readable string:
    println!("{}", test_path.display());
}

#[test]
fn test_null_like_checks() {
    let null_check = NullLikeCheck::new();

    for &null_value in &NullLikeCheck::NULL_LIKE_VALUES {
        assert!(null_check.check(null_value))
    }
}

#[test]
fn test_empty_checks() {
    let empty_check = EmptyCheck::new();

    assert!(empty_check.check(""));
    assert!(!empty_check.check(" "))
}

#[test]
fn test_white_space_only_check() {
    let white_space_only = WhiteSpaceOnlyCheck::new();

    assert!(white_space_only.check("         "))
}

#[test]
fn test_csv_chunk_iterator() {
    let test_path = get_fixture_path("sample-warehouse-data.csv");

    let (_headers, mut rdr) = prepare_csv_reader(&test_path).unwrap();
    let chunk_size = 3;
    let mut chunk_iterator: CsvChunkIterator<'_, File> =
        CsvChunkIterator::new(rdr.records(), chunk_size);

    let first_chunk = chunk_iterator.next().unwrap().unwrap();

    assert_eq!(first_chunk.len(), 3);

    // Verify the content of the first chunk
    // Record 1: 1001,1/15/2024,SKU123,5,29.99,94105,john.doe@email.com,2024-01-15T08:30:00Z,
    assert_eq!(first_chunk[0].get(0), Some("1001"));
    assert_eq!(first_chunk[0].get(1), Some("1/15/2024"));
    assert_eq!(first_chunk[0].get(2), Some("SKU123"));
    assert_eq!(first_chunk[0].get(3), Some("5"));
    assert_eq!(first_chunk[0].get(6), Some("john.doe@email.com"));

    // Record 2: 1002,1/15/2024,SKU456,2,49.99,60601,,2024-01-15T09:15:00Z,
    assert_eq!(first_chunk[1].get(0), Some("1002"));
    assert_eq!(first_chunk[1].get(2), Some("SKU456"));
    assert_eq!(first_chunk[1].get(6), Some("")); // Empty email field

    // Record 3: 1003,1/16/2024,SKU789,1,99.99,10001,alice.smith@email.com,2024-01-16T14:20:00Z,
    assert_eq!(first_chunk[2].get(0), Some("1003"));
    assert_eq!(first_chunk[2].get(2), Some("SKU789"));
    assert_eq!(first_chunk[2].get(6), Some("alice.smith@email.com"));

    // Test second chunk
    let second_chunk = chunk_iterator.next().unwrap().unwrap();
    assert_eq!(second_chunk.len(), 3);

    // Verify some fields from second chunk
    assert_eq!(second_chunk[0].get(0), Some("1004")); // First record of second chunk
    assert_eq!(second_chunk[1].get(0), Some("1005")); // Second record
    assert_eq!(second_chunk[2].get(0), Some("1005")); // Third record (with blanks)
    assert_eq!(second_chunk[2].get(2), Some("   ")); // Blank product_sku with spaces

    // Test third chunk
    let third_chunk = chunk_iterator.next().unwrap().unwrap();
    assert_eq!(third_chunk.len(), 3);

    // This chunk should contain the problematic records
    assert_eq!(third_chunk[0].get(0), Some("MISSING1005"));
    assert_eq!(third_chunk[1].get(0), Some("NULL")); // First NULL record
    assert_eq!(third_chunk[2].get(0), Some("NULL")); // Second NULL record

    // Test fourth chunk (should have remaining records)
    let fourth_chunk = chunk_iterator.next().unwrap().unwrap();
    assert_eq!(fourth_chunk.len(), 3);

    // Last records with various NULL patterns
    assert_eq!(fourth_chunk[0].get(0), Some("NULL"));
    assert_eq!(fourth_chunk[1].get(0), Some("NULL"));
    assert_eq!(
        fourth_chunk[2].get(0),
        Some("                  NULL                   ")
    );

    // No more chunks should be available
    assert!(chunk_iterator.next().is_none());
}

#[test]
fn test_csv_chunk_iterator_comprehensive() {
    let test_path = get_fixture_path("sample-warehouse-data.csv");
    let (_headers, mut rdr) = prepare_csv_reader(&test_path).unwrap();
    let chunk_size = 4; // Different chunk size

    let chunk_iterator: CsvChunkIterator<'_, File> =
        CsvChunkIterator::new(rdr.records(), chunk_size);

    // Collect all chunks to verify total structure
    let all_chunks: Result<Vec<_>, _> = chunk_iterator.collect();
    let all_chunks = all_chunks.unwrap();

    // Your CSV has 12 data rows, so with chunk_size=4: 4+4+4 = 3 chunks
    assert_eq!(all_chunks.len(), 3);
    assert_eq!(all_chunks[0].len(), 4);
    assert_eq!(all_chunks[1].len(), 4);
    assert_eq!(all_chunks[2].len(), 4);

    // Verify first record of each chunk
    assert_eq!(all_chunks[0][0].get(0), Some("1001"));
    assert_eq!(all_chunks[1][0].get(0), Some("1005")); // First record of second chunk
    assert_eq!(all_chunks[2][0].get(0), Some("NULL")); // First record of third chunk
}

#[test]
fn test_csv_chunk_iterator_edge_cases() {
    let test_path = get_fixture_path("sample-warehouse-data.csv");

    // Test with chunk size larger than total records
    let (_headers, mut rdr) = prepare_csv_reader(&test_path).unwrap();
    let mut chunk_iterator: CsvChunkIterator<'_, File> = CsvChunkIterator::new(rdr.records(), 20);

    let only_chunk = chunk_iterator.next().unwrap().unwrap();
    assert_eq!(only_chunk.len(), 12); // All records in one chunk
    assert!(chunk_iterator.next().is_none());

    // Test with chunk size of 1
    let (_headers, mut rdr) = prepare_csv_reader(&test_path).unwrap();
    let chunk_iterator: CsvChunkIterator<'_, File> = CsvChunkIterator::new(rdr.records(), 1);

    let all_single_chunks: Result<Vec<_>, _> = chunk_iterator.collect();
    let all_single_chunks = all_single_chunks.unwrap();

    assert_eq!(all_single_chunks.len(), 12); // 12 chunks of 1 record each
    for chunk in &all_single_chunks {
        assert_eq!(chunk.len(), 1);
    }
}

#[test]
fn test_placeholder_check_patterns() {
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

#[test]
fn test_placeholder_check_detected_in_csv() {
    let test_path = get_fixture_path("placeholder-test-data.csv");

    let (_found_headers, mut rdr) = prepare_csv_reader(&test_path).unwrap();

    let chunk_size = 10;
    let config = ProcessingConfig {
        chunk_size,
        enable_parallel: false,
    };
    let chunk_iterator = CsvChunkIterator::new(rdr.records(), chunk_size);

    let results = process_csv_chunks(chunk_iterator, config).unwrap();

    // placeholder-test-data.csv has 6 data rows:
    //   Alice: status=active,            notes=Great customer  -> 0 placeholders
    //   Bob:   status=TBD (col 1),       notes=TODO (col 2)    -> 2 placeholders
    //   Carol: status=UNKNOWN (col 1),   notes=PLACEHOLDER (col 2) -> 2 placeholders
    //   Dave:  status=todo (col 1),      notes=tbd (col 2)     -> 2 placeholders
    //   Eve:   status=active,            notes=unknown (col 2) -> 1 placeholder
    //   Frank: status=placeholder (col 1), notes=active        -> 1 placeholder
    // Total = 8 placeholder hits

    let total_placeholder_found: usize = results
        .iter()
        .map(|r| r.placeholder_counts.values().sum::<usize>())
        .sum();

    assert_eq!(
        total_placeholder_found, 8,
        "Expected 8 placeholder values in placeholder-test-data.csv, found {}",
        total_placeholder_found
    );

    // col 1 (status): TBD, UNKNOWN, todo, placeholder = 4
    let col1_placeholder: usize = results
        .iter()
        .map(|r| r.placeholder_counts.get(&1).copied().unwrap_or(0))
        .sum();
    assert_eq!(
        col1_placeholder, 4,
        "Expected 4 placeholder values in 'status' column"
    );

    // col 2 (notes): TODO, PLACEHOLDER, tbd, unknown = 4
    let col2_placeholder: usize = results
        .iter()
        .map(|r| r.placeholder_counts.get(&2).copied().unwrap_or(0))
        .sum();
    assert_eq!(
        col2_placeholder, 4,
        "Expected 4 placeholder values in 'notes' column"
    );
}

#[test]
fn test_process_csv_chunks() {
    let test_path = get_fixture_path("sample-warehouse-data.csv");

    // Get both headers and reader
    let (found_headers, mut rdr) = prepare_csv_reader(&test_path).unwrap();
    println!("Found headers: {:?}", found_headers);

    // Define chunk size
    let chunk_size = 3;

    let config = ProcessingConfig {
        chunk_size,
        enable_parallel: false,
    };
    let chunk_iterator = CsvChunkIterator::new(rdr.records(), chunk_size);

    let results = process_csv_chunks(chunk_iterator, config).unwrap();

    assert_eq!(results.len(), 4); // 12 rows / 3 = 4 chunks

    // Check first chunk
    assert_eq!(results[0].chunk_number, 1);
    assert_eq!(results[0].rows_processed, 3);

    // Check that we found some empty values (from your CSV)
    let total_empty_found: usize = results
        .iter()
        .map(|r| r.empty_counts.values().sum::<usize>())
        .sum();
    assert!(
        total_empty_found == 32,
        "Should find 32 empty values in test CSV"
    );

    // Check that we found some NULL-like values
    let total_null_found: usize = results
        .iter()
        .map(|r| r.null_counts.values().sum::<usize>())
        .sum();
    assert!(
        total_null_found == 15,
        "Should find 15 NULL-like values in test CSV"
    );
}

#[test]
fn test_digits_only_check_basic() {
    let check = DigitsOnlyCheck::new();

    // Positive cases: fields that are entirely digits
    assert!(check.check("12345"), "Pure digit string should match");
    assert!(check.check("0"), "Single zero should match");
    assert!(check.check("9999"), "All nines should match");

    // Negative cases
    assert!(!check.check(""), "Empty string should not match");
    assert!(!check.check("12.34"), "Float with decimal should not match");
    assert!(!check.check("12 34"), "Digits with space should not match");
    assert!(!check.check("abc"), "Letters only should not match");
    assert!(!check.check("1a2"), "Mixed alphanum should not match");
    assert!(!check.check("-5"), "Negative number should not match");
}

#[test]
fn test_digits_only_check_trait_metadata() {
    let check = DigitsOnlyCheck::new();
    assert_eq!(check.name(), "DigitsOnlyCheck");
    assert_eq!(
        check.show_check_pattern(),
        "Fields containing only digit characters (0-9)"
    );
}

#[test]
fn test_process_csv_chunks_digits_only_counts() {
    let test_path = get_fixture_path("sample-warehouse-data.csv");

    let (_found_headers, mut rdr) = prepare_csv_reader(&test_path).unwrap();
    let chunk_size = 3;

    let config = ProcessingConfig {
        chunk_size,
        enable_parallel: false,
    };
    let chunk_iterator = CsvChunkIterator::new(rdr.records(), chunk_size);

    let results = process_csv_chunks(chunk_iterator, config).unwrap();

    // The sample CSV has digits-only values in customer_id (col 0), quantity (col 3),
    // and shipping_zip (col 5) for the first several rows.
    // Rows 1-6 contribute 3 each (col0, col3, col5), row 7 contributes 1 (col5 only).
    // Total expected: 3*6 + 1 = 19 digits-only field occurrences.
    let total_digits_only_found: usize = results
        .iter()
        .map(|r| r.digits_only_counts.values().sum::<usize>())
        .sum();
    assert_eq!(
        total_digits_only_found, 19,
        "Should find 19 digits-only field occurrences in test CSV"
    );

    // The first chunk (rows 1-3) should have digits-only hits in col 0, col 3, col 5
    assert!(
        results[0].digits_only_counts.contains_key(&0),
        "First chunk should have digits-only values in column 0 (customer_id)"
    );
    assert!(
        results[0].digits_only_counts.contains_key(&3),
        "First chunk should have digits-only values in column 3 (quantity)"
    );
    assert!(
        results[0].digits_only_counts.contains_key(&5),
        "First chunk should have digits-only values in column 5 (shipping_zip)"
    );
}

#[test]
fn test_dash_only_check_basic() {
    let check = DashOnlyCheck::new();

    // Positive cases
    assert!(check.check("-"), "Single dash should match");
    assert!(check.check("--"), "Double dash should match");
    assert!(
        check.check("  -  "),
        "Single dash with whitespace should match"
    );
    assert!(
        check.check("  --  "),
        "Double dash with whitespace should match"
    );

    // Negative cases
    assert!(!check.check(""), "Empty string should not match");
    assert!(!check.check("---"), "Triple dash should not match");
    assert!(!check.check("-a"), "Dash with letter should not match");
    assert!(!check.check("a-b"), "Dash between letters should not match");
    assert!(!check.check("some value"), "Regular value should not match");
    assert!(!check.check("N/A"), "N/A should not match");
}

#[test]
fn test_dash_only_check_trait_metadata() {
    let check = DashOnlyCheck::new();
    assert_eq!(check.name(), "DASH_ONLY");
    assert_eq!(
        check.show_check_pattern(),
        "Fields whose trimmed value is '-' or '--'"
    );
}
