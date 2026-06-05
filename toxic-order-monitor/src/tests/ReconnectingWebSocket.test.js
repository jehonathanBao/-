import { describe, expect, it } from "vitest";
import { reconnectDelay, toWebSocketUrl } from "../hooks/useReconnectingWebSocket.js";

describe("toWebSocketUrl", () => {
  it("keeps absolute websocket URLs", () => {
    expect(toWebSocketUrl("wss://example.com/ws/signals")).toBe("wss://example.com/ws/signals");
  });

  it("builds a same-origin websocket URL for proxied paths", () => {
    expect(toWebSocketUrl("/ws/signals")).toBe("ws://localhost:3000/ws/signals");
  });

  it("uses exponential reconnect backoff with a cap", () => {
    expect(reconnectDelay(0, 1000, 5000)).toBe(1000);
    expect(reconnectDelay(1, 1000, 5000)).toBe(2000);
    expect(reconnectDelay(4, 1000, 5000)).toBe(5000);
  });
});
