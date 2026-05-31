use anyhow::anyhow;

use super::secret_scanner::scan_forbidden_secrets;

pub fn assert_read_only_runtime() -> anyhow::Result<()> {
    let findings = scan_forbidden_secrets();
    if findings.is_empty() {
        return Ok(());
    }

    let keys = findings
        .iter()
        .map(|finding| finding.key.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(anyhow!("unsafe runtime configuration: {keys}"))
}
