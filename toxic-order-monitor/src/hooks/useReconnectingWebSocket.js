import { useCallback, useEffect, useRef, useState } from "react";

export function useReconnectingWebSocket(
  path,
  { enabled = true, retryMs = 1000, maxRetryMs = 15000, onMessage } = {},
) {
  const [status, setStatus] = useState("idle");
  const socketRef = useRef(null);
  const retryRef = useRef(null);
  const stoppedRef = useRef(false);
  const attemptRef = useRef(0);
  const onMessageRef = useRef(onMessage);

  onMessageRef.current = onMessage;

  const connect = useCallback(() => {
    if (!enabled || stoppedRef.current || typeof WebSocket === "undefined") {
      return;
    }
    const socket = new WebSocket(toWebSocketUrl(path));
    socketRef.current = socket;
    setStatus("connecting");

    socket.onopen = () => {
      attemptRef.current = 0;
      setStatus("open");
    };
    socket.onmessage = (event) => {
      onMessageRef.current?.(event);
    };
    socket.onerror = () => {
      socket.close();
    };
    socket.onclose = () => {
      if (stoppedRef.current) {
        setStatus("closed");
        return;
      }
      setStatus("reconnecting");
      const delay = reconnectDelay(attemptRef.current, retryMs, maxRetryMs);
      attemptRef.current += 1;
      retryRef.current = window.setTimeout(connect, delay);
    };
  }, [enabled, maxRetryMs, path, retryMs]);

  useEffect(() => {
    stoppedRef.current = false;
    connect();
    return () => {
      stoppedRef.current = true;
      if (retryRef.current) {
        window.clearTimeout(retryRef.current);
      }
      socketRef.current?.close();
    };
  }, [connect]);

  return { status, socket: socketRef.current };
}

export function reconnectDelay(attempt, retryMs = 1000, maxRetryMs = 15000) {
  const base = Number.isFinite(Number(retryMs)) ? Number(retryMs) : 1000;
  const max = Number.isFinite(Number(maxRetryMs)) ? Number(maxRetryMs) : 15000;
  const exponent = Math.max(0, Number(attempt) || 0);
  return Math.min(max, base * 2 ** exponent);
}

export function toWebSocketUrl(path) {
  if (/^wss?:\/\//i.test(path)) {
    return path;
  }
  const protocol = window.location.protocol === "https:" ? "wss" : "ws";
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  return `${protocol}://${window.location.host}${normalizedPath}`;
}
