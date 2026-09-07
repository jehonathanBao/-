import "@testing-library/jest-dom/vitest";
import { act, cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";
import Sidebar from "../components/Sidebar.jsx";
import Header from "../components/Header.jsx";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  window.localStorage.clear();
  delete document.documentElement.dataset.motion;
});

function renderSidebar() {
  return render(<MemoryRouter initialEntries={["/contract-whale/eth"]}><Sidebar /></MemoryRouter>);
}

describe("financial terminal navigation and motion", () => {
  it("groups every existing destination and keeps the active asset accessible", () => {
    renderSidebar();
    const nav = screen.getByRole("navigation", { name: "主导航" });
    for (const label of ["总览", "合约市场", "现货市场", "信号中心", "工作区"]) {
      expect(within(nav).getByText(label)).toBeInTheDocument();
    }
    expect(within(nav).getAllByRole("link")).toHaveLength(14);
    expect(screen.getByRole("link", { name: "ETH 合约监控" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: "跳到主要内容" })).toHaveAttribute("href", "#workspace-main");
  });

  it("links the existing settings control to its actual page", () => {
    render(<Header highUnhandledCount={0} discordConnected={false} />);
    expect(screen.getByRole("link", { name: "系统设置" })).toHaveAttribute("href", "/settings");
  });

  it("lets the operator turn off dynamic effects without changing data refresh", async () => {
    const user = userEvent.setup();
    renderSidebar();
    const control = screen.getByRole("button", { name: /动态效果/ });
    expect(control).toHaveAttribute("aria-pressed", "true");
    await user.click(control);
    expect(control).toHaveAttribute("aria-pressed", "false");
    expect(document.documentElement).toHaveAttribute("data-motion", "off");
  });

  it("honors reduced motion and stops effects in a background tab", () => {
    let onMediaChange;
    vi.stubGlobal("matchMedia", vi.fn(() => ({
      matches: true,
      addEventListener: (_, callback) => { onMediaChange = callback; },
      removeEventListener: vi.fn(),
    })));
    renderSidebar();
    expect(document.documentElement).toHaveAttribute("data-motion", "off");
    expect(screen.getByRole("button", { name: /动态效果/ })).toBeDisabled();
    act(() => onMediaChange({ matches: false }));
    expect(document.documentElement).toHaveAttribute("data-motion", "on");
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
    act(() => document.dispatchEvent(new Event("visibilitychange")));
    expect(document.documentElement).toHaveAttribute("data-motion", "off");
  });
});
