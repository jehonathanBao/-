use std::env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretFinding {
    pub key: String,
    pub reason: String,
}

const FORBIDDEN_KEYS: [&str; 18] = [
    "PRIVATE_KEY",
    "WALLET_KEY",
    "MNEMONIC",
    "EXCHANGE_SECRET",
    "EXCHANGE_API_KEY",
    "EXCHANGE_API_SECRET",
    "CCXT_API_KEY",
    "CCXT_SECRET",
    "BINANCE_API_KEY",
    "BINANCE_API_SECRET",
    "BINANCE_SECRET",
    "BYBIT_API_KEY",
    "BYBIT_API_SECRET",
    "BYBIT_SECRET",
    "OKX_API_KEY",
    "OKX_API_SECRET",
    "OKX_PASSPHRASE",
    "OKX_SECRET",
];

const FORBIDDEN_TRUE_FLAGS: [&str; 2] = ["ORDER_EXECUTION_ENABLED", "LIVE_TRADING_ENABLED"];

pub fn scan_forbidden_secrets() -> Vec<SecretFinding> {
    let mut findings = Vec::new();

    for key in FORBIDDEN_KEYS {
        if env_key_has_value(key) {
            findings.push(SecretFinding {
                key: key.to_string(),
                reason: "No API key boundary violated; private keys, exchange API keys, seed phrases, and exchange secrets are not allowed".to_string(),
            });
        }
    }

    for (key, value) in env::vars() {
        if is_forbidden_key_pattern(&key) && !value.trim().is_empty() {
            findings.push(SecretFinding {
                key,
                reason: "No API key boundary violated; private keys, exchange API keys, seed phrases, and exchange secrets are not allowed".to_string(),
            });
        }
    }

    findings.sort_by(|left, right| left.key.cmp(&right.key));
    findings.dedup_by(|left, right| left.key == right.key);

    for key in FORBIDDEN_TRUE_FLAGS {
        if env::var(key)
            .ok()
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            findings.push(SecretFinding {
                key: key.to_string(),
                reason: "order execution and live trading must stay disabled".to_string(),
            });
        }
    }

    findings
}

fn env_key_has_value(key: &str) -> bool {
    env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_some()
}

fn is_forbidden_key_pattern(key: &str) -> bool {
    let normalized = key.to_ascii_uppercase();
    if matches!(
        normalized.as_str(),
        "API_HOST" | "API_PORT" | "ENABLE_BINANCE"
    ) {
        return false;
    }
    normalized.ends_with("_API_KEY")
        || normalized.ends_with("_API_SECRET")
        || normalized.ends_with("_SECRET_KEY")
        || normalized.ends_with("_ACCESS_KEY")
}
