---
name: frontend-risk-console
description: Build or review risk console UI using React, TypeScript, Tailwind, Radix, Axios, and TanStack Query patterns while preserving sensitive-data masking and confirmation gates.
---

# Frontend Risk Console

Use this skill when creating or reviewing React/TypeScript risk UI, API hooks, dialogs, tables, filters, and mutations.

## Architecture Pattern

```text
Component -> domain API hook -> Axios client -> backend API
Mutation -> onSettled cache invalidation -> UI refresh
Filters -> URL search params -> shareable view
```

## UI Rules

- Use one canonical risk badge and color map for all risk levels.
- Mask sensitive values by default: phone, address, ID number, full email, token, webhook.
- Require confirmation dialogs for delete, release, blacklist, threshold change, and rule update.
- Do not hardcode API tokens in frontend code.
- Keep table pagination mandatory; never fetch all orders for the dashboard.
- Keep logic and UI separable: hooks for data, components for rendering, dialogs for actions.
- Disable mutation buttons while pending to prevent double-click duplicates.

## Toxic Signal Inbox Rules

- Fetch persisted candidates from `/api/toxicity/signal-inbox/recent`.
- Keep `rawInboxSignals` as the source of truth for high, critical, medium, and low display state.
- High and critical candidates are the default primary list and should sort above medium/low by risk, then time.
- Medium candidates are display-only, default collapsed, and must not enable Discord push.
- Signal cards should lead with final result: direction plus core reason. Keep technical tags, evidence, stale state, and markout folded.
- Clear-cache actions only clear the frontend inbox view and must not delete backend data.

## Testing Rules

- Test network failure, 401/403/404/500, empty response, malformed response, boundary pagination, and double-click.
- Query by role, label, visible text, or stable `data-testid`; avoid CSS implementation details.
- Test that sensitive fields are masked in detail views.
- Test that action dialogs require explicit confirmation.
- Test backend inbox mapping, empty inbox behavior, persistent merge, clear-cache cleared keys, high/critical gate, and medium suppression.

## Verification Commands

Use these when a React frontend is present:

```powershell
npm run lint
npm test
npx playwright test
```

For the current static dashboard, at minimum run:

```powershell
node --check web\app.js
cargo test -j 1 dashboard_static
```
