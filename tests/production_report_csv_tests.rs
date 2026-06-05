use btc_toxic_flow_monitor_rs::replay::production_report::sanitize_csv_cell;

#[test]
fn csv_formula_prefixes_are_escaped() {
    assert_eq!(sanitize_csv_cell("=1+1"), "'=1+1");
    assert_eq!(sanitize_csv_cell("+SUM(A1:A2)"), "'+SUM(A1:A2)");
    assert_eq!(sanitize_csv_cell("-10+20"), "'-10+20");
    assert_eq!(sanitize_csv_cell("@cmd"), "'@cmd");
    assert_eq!(sanitize_csv_cell("  =1+1"), "'  =1+1");
}

#[test]
fn normal_csv_values_are_unchanged_before_quote_escaping() {
    assert_eq!(sanitize_csv_cell("normal text"), "normal text");
    assert_eq!(sanitize_csv_cell("\"quoted\""), "\"quoted\"");
    assert_eq!(sanitize_csv_cell(""), "");
    assert_eq!(sanitize_csv_cell("   "), "   ");
}
