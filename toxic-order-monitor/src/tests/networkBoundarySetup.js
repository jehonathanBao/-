// Keep browser-like tests hermetic. Individual API tests provide their own
// axios mocks; any request that slips through must fail locally instead of
// opening a real socket.
import axios from "axios";

if (axios?.defaults) {
  axios.defaults.adapter = async () => {
    throw new Error("Network disabled in tests");
  };
}

class BlockedXMLHttpRequest {
  constructor() {
    this.readyState = 0;
    this.status = 0;
    this.responseText = "";
    this.responseURL = "";
    this.timeout = 0;
    this._listeners = new Map();
  }

  open(method, url) {
    this.method = method;
    this.url = url;
    this.readyState = 1;
  }

  setRequestHeader() {}

  addEventListener(type, listener) {
    this._listeners.set(type, listener);
  }

  removeEventListener(type) {
    this._listeners.delete(type);
  }

  send() {
    const error = new Error(`Network disabled in tests: ${this.method || "GET"} ${this.url || ""}`);
    queueMicrotask(() => {
      this.onerror?.(error);
      this._listeners.get("error")?.(error);
      this.onloadend?.();
      this._listeners.get("loadend")?.();
    });
  }

  abort() {}
}

globalThis.XMLHttpRequest = BlockedXMLHttpRequest;
if (typeof window !== "undefined") {
  Object.defineProperty(window, "XMLHttpRequest", {
    configurable: true,
    writable: true,
    value: BlockedXMLHttpRequest,
  });
}
