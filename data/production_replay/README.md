# Production Replay Data

Place real historical L2 / trade replay files here when running local production
validation. Real market data, reports, and human labels are ignored by git by
default.

Supported input formats:

- JSONL: one event object per line.
- CSV: header-based rows with the same field names.

Required fields:

- `event_type`: `trade`, `book_delta`, `snapshot`, or `snapshot_reset`
- `venue`
- `symbol`
- `ts_ms`: UTC timestamp in milliseconds
- `side`: `bid`, `ask`, `buy`, `sell`, or `none`
- `price`
- `qty`
- `qty_before`
- `qty_after`
- `sequence`
- `trade_id`

Optional fields:

- `order_id`

Run the read-only contract check before production replay:

```powershell
cargo run --bin replay_data_contract_check -- --input data/production_replay/Binance_BTCUSDT_2025-01-01.jsonl
```

The checker writes local-only reports under:

```text
data/production_replay/reports/<run_id>/data_contract.json
data/production_replay/reports/<run_id>/data_contract.md
```

For local replay, copy `config/replay.production.example.toml` to the ignored
`config/replay.production.local.toml`, point `[input].path` at your real local
JSONL/CSV file, then run:

```powershell
cargo run --bin replay_production -- --config config/replay.production.local.toml
```

This engine emits toxic-order risk candidates only. It does not emit confirmed
manipulation findings. Public L2 data without native order lifecycle IDs remains
probabilistic and is marked as inferred L2 evidence.
