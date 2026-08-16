//! Bulk mapping CSV parsing (pure, unit-tested, no I/O).
//!
//! Input format (see `docs/mapping.md`):
//!
//! ```csv
//! icode,working_code,drug_name_hosxp,drug_name_invs
//! 041234,WA001,Amoxicillin 500 mg,Amoxicillin (แคปซูล)
//! ```
//!
//! The header row is optional (detected by name); the two name columns are
//! optional.  Lines that cannot be parsed are reported as errors, never
//! silently dropped.

use csv::StringRecord;

/// One parsed row from the import text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkRow {
    pub icode: String,
    pub working_code: String,
    pub drug_name_hosxp: String,
    pub drug_name_invs: String,
}

/// Recognised header names (case-insensitive) for header-row detection.
const HEADER_NAMES: &[&str] = &[
    "icode",
    "working_code",
    "workingcode",
    "drug_name_hosxp",
    "drug_name_invs",
    "code",
    "name",
];

fn is_header(record: &StringRecord) -> bool {
    // A header row is recognized only when BOTH leading fields look like
    // column names.  Requiring both makes it impossible for a real data row
    // to be eaten: a working_code or drug name that happens to equal one of
    // these words (e.g. a drug literally named "Code") can no longer
    // disguise a data line as a header.
    if record.len() < 2 {
        return false;
    }
    let is_name = |f: &str| {
        let f = f.trim().to_lowercase();
        HEADER_NAMES.contains(&f.as_str())
    };
    is_name(&record[0]) && is_name(&record[1])
}

/// Parse the CSV text into rows.  Returns `(rows, errors)` where each error
/// is a human-readable line reference; a header row is consumed, blank lines
/// are ignored.
#[must_use]
pub fn parse_bulk_csv(text: &str) -> (Vec<BulkRow>, Vec<String>) {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .has_headers(false) // header row (if any) is detected explicitly below
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());

    let mut rows = Vec::new();
    let mut errors = Vec::new();
    let mut record = csv::StringRecord::new();

    loop {
        // Position before the read = the physical line this record starts on.
        let line = reader.position().line();
        match reader.read_record(&mut record) {
            Ok(true) => {}
            Ok(false) => break,
            Err(e) => {
                errors.push(format!("cannot read CSV: {e}"));
                break;
            }
        }
        if record.iter().all(|f| f.is_empty()) {
            continue;
        }
        if is_header(&record) {
            continue;
        }
        if record.len() < 2 {
            errors.push(format!(
                "บรรทัด {line}: ต้องมีอย่างน้อย 2 คอลัมน์ (icode, working_code)"
            ));
            continue;
        }
        rows.push(BulkRow {
            icode: record[0].trim().to_owned(),
            working_code: record[1].trim().to_owned(),
            drug_name_hosxp: record.get(2).unwrap_or("").trim().to_owned(),
            drug_name_invs: record.get(3).unwrap_or("").trim().to_owned(),
        });
    }
    (rows, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rows_with_optional_header_and_names() {
        let text = "\
icode,working_code,drug_name_hosxp,drug_name_invs
041234,WA001,Amoxicillin 500 mg,Amoxicillin (แคปซูล)
041235,WA002,
";
        let (rows, errors) = parse_bulk_csv(text);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].icode, "041234");
        assert_eq!(rows[0].working_code, "WA001");
        assert_eq!(rows[0].drug_name_hosxp, "Amoxicillin 500 mg");
        assert_eq!(rows[0].drug_name_invs, "Amoxicillin (แคปซูล)");
        assert_eq!(rows[1].drug_name_hosxp, "");
    }

    #[test]
    fn headerless_two_column_input_works() {
        let (rows, errors) = parse_bulk_csv("041234,WA001\n041235,WA002\n");
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            BulkRow {
                icode: "041234".to_owned(),
                working_code: "WA001".to_owned(),
                drug_name_hosxp: String::new(),
                drug_name_invs: String::new(),
            }
        );
    }

    #[test]
    fn short_lines_are_errors_and_blank_lines_are_ignored() {
        let (rows, errors) = parse_bulk_csv("\n\n041234,WA001\n041235\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("บรรทัด 4"), "{errors:?}");
    }

    #[test]
    fn names_containing_commas_survive_quoting() {
        let (rows, errors) =
            parse_bulk_csv("\"041234\",\"WA001\",\"Paracetamol, พาราเซตามอล\",\"\"\n");
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(rows[0].drug_name_hosxp, "Paracetamol, พาราเซตามอล");
    }

    #[test]
    fn header_detection_requires_both_leading_fields() {
        // A drug named "Code" (or a working_code literally "code") must not
        // make a data row look like a header.
        let (rows, errors) = parse_bulk_csv("041234,code,Amoxicillin\ncode,WA001\n");
        assert_eq!(rows.len(), 2, "both data rows survive");
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(rows[0].working_code, "code");
        assert_eq!(rows[1].icode, "code");
    }

    #[test]
    fn header_with_both_column_names_is_still_consumed() {
        let (rows, errors) = parse_bulk_csv("icode,working_code\n041234,WA001\n");
        assert_eq!(rows.len(), 1);
        assert!(errors.is_empty(), "{errors:?}");
    }
}
