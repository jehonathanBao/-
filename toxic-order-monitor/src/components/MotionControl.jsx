import { useEffect, useState } from "react";
import { AdjustmentsHorizontalIcon } from "@heroicons/react/24/outline";

const PREFERENCE_KEY = "whale-desk.motion";

export default function MotionControl() {
  const [enabled, setEnabled] = useState(() => {
    try { return window.localStorage.getItem(PREFERENCE_KEY) !== "off"; }
    catch { return true; }
  });
  const [reduced, setReduced] = useState(() => window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false);
  const [visible, setVisible] = useState(() => document.visibilityState !== "hidden");

  useEffect(() => {
    const media = window.matchMedia?.("(prefers-reduced-motion: reduce)");
    const onChange = (event) => setReduced(event.matches);
    const onVisibility = () => setVisible(document.visibilityState !== "hidden");
    media?.addEventListener?.("change", onChange);
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      media?.removeEventListener?.("change", onChange);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, []);

  useEffect(() => {
    document.documentElement.dataset.motion = enabled && !reduced && visible ? "on" : "off";
    return () => { delete document.documentElement.dataset.motion; };
  }, [enabled, reduced, visible]);

  function toggle() {
    const next = !enabled;
    setEnabled(next);
    try { window.localStorage.setItem(PREFERENCE_KEY, next ? "on" : "off"); }
    catch { /* The in-memory preference still works when storage is blocked. */ }
  }

  return (
    <button
      className="terminal-motion-control"
      type="button"
      aria-label={`动态效果：${reduced ? "跟随系统关闭" : enabled ? "开启" : "关闭"}`}
      aria-pressed={enabled && !reduced}
      disabled={reduced}
      onClick={toggle}
      title="只控制界面动效，不影响实时数据与通知"
    >
      <AdjustmentsHorizontalIcon aria-hidden="true" />
      <span>动态效果</span>
      <span className="terminal-toggle" aria-hidden="true"><i /></span>
    </button>
  );
}
