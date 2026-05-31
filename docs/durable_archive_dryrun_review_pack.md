# Durable Archive Dry-run Review Pack

This document describes the read-only operator review pack built on top of the durable archive dry-run contract.

The review pack is for manual review only.

- readOnly=true
- runtimeModified=false
- analysisOnly=true
- manualReviewRequired=true
- executionEnabled=false
- archiveWriteEnabled=false
- durableStorageEnabled=false
- databaseWriteEnabled=false
- jsonlWriteEnabled=false
- sqliteWriteEnabled=false
- notificationSent=false
- executionTriggered=false

## Purpose

The review pack summarizes the latest durable archive dry-run payload.

It exists to help an operator inspect:

- prepared dry-run records
- validation errors
- validation warnings
- unsafe fields
- field contract
- safety boundary

## API

- `GET /api/archive/dry-run/review-pack/latest`
- `GET /api/archive/dry-run/review-pack/:dryRunId`

The JSON response includes a `markdown` field so the operator can copy either JSON or Markdown without any write step.

## Markdown Contract

The Markdown review pack must align with the JSON contract and include:

- Dry Run ID
- Summary
- Validation Errors
- Validation Warnings
- Unsafe Fields
- Field Contract
- Prepared Records
- Safety Boundary
- Operator Notes

## Safety Boundary

The review pack does not write DB state.

- No DB write
- No JSONL write
- No SQLite write
- No file write
- No runtime mutation
- No apply
- No reload
- No notification sending
- No webhook
- No Telegram
- No order placement
- No wallet/signing
- No live trading

## Notes

The review pack is generated from the current dry-run snapshot and is not durable storage.

It does not make archive MVP ready.
