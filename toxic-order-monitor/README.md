# Toxic Order Monitor

Dark monitoring dashboard for toxic order and orderbook anomaly candidates.

## Discord Proxy

The frontend no longer calls a Discord webhook directly. All push requests go through the backend proxy:

```text
Frontend
   ↓
/api/discord/push
   ↓
Discord Webhook
```

Frontend configuration:

```env
VITE_API_BASE_URL=http://localhost:3000
```

Backend configuration keeps the real webhook server-side only.

## Persistent Signal Inbox

Candidate signals are kept in a frontend inbox so operators can review older candidates after a page refresh.

- Signals are not automatically cleared from the dashboard.
- `stale`, `insufficient_data`, and similar technical tags only describe data freshness or evidence quality; they do not remove a candidate from the inbox.
- Backend responses are treated as live snapshots and are merged into the persistent inbox, not used to replace it.
- If a candidate disappears from the latest snapshot, the card becomes `stale` / non-live but remains visible.
- Discord alert gates only control push eligibility; they do not control frontend candidate visibility.
- Clicking `清除缓存` only clears the current browser's frontend display cache. It does not delete backend data.
- After refresh, any candidates still stored in `localStorage` are restored automatically.
- Cleared candidates with the same `dedupeKey` are not re-added from an old backend response; new candidates with new keys will continue to append.
- If browser storage is unavailable or quota is exceeded, the page keeps the inbox in memory for the current session and shows a lightweight warning state.

中文说明：

- 后端最新响应只作为实时快照合并进持久 Inbox，不会替换已缓存候选。
- 某条信号不在最新快照中，只会变成 `stale`，不会从页面删除。
- Discord 推送门槛只控制是否推送，不控制页面是否展示。
- 点击 `清除缓存` 只清空当前浏览器前端展示缓存，不删除后端、回放或归档数据。
