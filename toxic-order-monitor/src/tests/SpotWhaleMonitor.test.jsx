import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import SpotWhaleMonitor from "../components/SpotWhaleMonitor.jsx";
import {
  fetchSpotWhaleHistory,
  fetchSpotWhaleLatest,
  fetchSpotWhaleSummary,
} from "../api/spotWhale.js";

vi.mock("../api/spotWhale.js", () => ({
  fetchSpotWhaleSummary: vi.fn((symbol = "BTC") =>
    Promise.resolve({
      summary: {
        status: "strong",
        healthStatus: "healthy",
        latestDirection: "buy",
        latestSeverity: "critical",
        signalCount: 1,
        enabled: true,
        dryRun: false,
        symbol,
        trend60s: {
          buyVolumeBase: symbol === "ETH" ? 4000 : 820,
          sellVolumeBase: 120,
          totalVolumeBase: symbol === "ETH" ? 4120 : 940,
          netVolumeBase: symbol === "ETH" ? 3880 : 700,
          dominance: 0.74,
          buyRatio: 0.87,
          sellRatio: 0.13,
          updatedAtMs: 1_700_000_000_000,
        },
        exchanges: {
          binance: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_000, latencyMs: 50 },
          coinbase: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_000, latencyMs: 80 },
          bitfinex: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_000, latencyMs: 120 },
        },
      },
      error: null,
    }),
  ),
  fetchSpotWhaleLatest: vi.fn((limit = 50, symbol = "BTC") =>
    Promise.resolve({
      summary: { enabled: true, dryRun: false, symbol, exchanges: {} },
      items: [
        {
          id: `spot-whale-${symbol}`,
          ts: 1_700_000_000_000,
          symbol,
          windowSec: 15,
          signalType: "spot_aggressive_buy",
          direction: "buy",
          severity: "critical",
          score: 88,
          totalVolumeBase: symbol === "ETH" ? 4200 : 820,
          netVolumeBase: symbol === "ETH" ? 3900 : 700,
          totalNotionalUsd: 62_000_000,
          dominance: 0.74,
          priceMovePct: 0.21,
          coinbasePremiumPct: 0.04,
          mainExchange: "binance",
          dataQuality: 91,
          discordEligible: true,
          discordSent: false,
          discordReason: "critical_or_s_gate",
          exchanges: [
            { exchange: "binance", buyVolumeBase: 520, sellVolumeBase: 80, dominance: 0.73 },
            { exchange: "coinbase", buyVolumeBase: 300, sellVolumeBase: 40, dominance: 0.76 },
            { exchange: "bitfinex", buyVolumeBase: 40, sellVolumeBase: 0, dominance: 1 },
          ],
          finalResult: "Binance / Coinbase / Bitfinex 现货主动买入同步放大",
          isPermanent: true,
        },
      ],
      limit,
      offset: 0,
      total: 1,
      hasMore: false,
      error: null,
    }),
  ),
  fetchSpotWhaleHistory: vi.fn(() =>
    Promise.resolve({ summary: {}, items: [], limit: 50, offset: 0, total: 0, hasMore: false, error: null }),
  ),
}));

describe("SpotWhaleMonitor", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders a locked BTC spot monitor with exchange health and signals", async () => {
    render(<SpotWhaleMonitor lockedSymbol="BTC" />);

    expect(await screen.findByText("BTC 现货监控")).toBeInTheDocument();
    expect(screen.getByText("BTC SPOT WHALE FLOW")).toBeInTheDocument();
    expect(screen.getByText("币种：BTC（当前页面固定）")).toBeInTheDocument();
    expect(screen.queryByLabelText("币种")).not.toBeInTheDocument();
    expect(screen.queryByText("SOL")).not.toBeInTheDocument();
    expect(screen.getAllByText("Binance").length).toBeGreaterThan(0);
    expect(screen.getByText("Coinbase")).toBeInTheDocument();
    expect(screen.getByText("Bitfinex")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "实时流" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "历史查询" })).toBeInTheDocument();
    expect(await screen.findByText("现货主动买入")).toBeInTheDocument();
    expect(
      screen.getAllByText((_, element) => {
        const text = element?.textContent || "";
        return text.includes("BTC") && text.includes("$75,610");
      }).length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText("$75,610").length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText("符合 gate")).toBeInTheDocument();
    expect(screen.getByText(/\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}/)).toBeInTheDocument();
  });

  it("uses the shared workspace panel, fields, and interactive row primitives", async () => {
    const user = userEvent.setup();
    render(<SpotWhaleMonitor lockedSymbol="BTC" />);

    const heading = await screen.findByRole("heading", { name: "BTC 现货监控" });
    expect(heading.closest("section")).toHaveClass("workspace-monitor-panel", "console-panel");
    expect(await screen.findByTestId("spot-whale-row-spot-whale-BTC")).toHaveClass("console-row");

    await user.click(screen.getByTestId("spot-whale-row-spot-whale-BTC"));
    expect(screen.getByText("Spot Candidate Review").closest(".workspace-dialog")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "关闭" }));

    await user.click(screen.getByRole("button", { name: "历史查询" }));

    for (const label of ["等级", "类型", "Discord", "净方向", "开始时间", "结束时间"]) {
      expect(screen.getByLabelText(label)).toHaveClass("console-field");
    }

  });

  it("filters visible spot signals by absolute net direction threshold", async () => {
    const user = userEvent.setup();
    render(<SpotWhaleMonitor />);

    expect(await screen.findByTestId("spot-whale-row-spot-whale-BTC")).toBeInTheDocument();

    fetchSpotWhaleHistory
      .mockResolvedValueOnce({
        summary: { enabled: true, dryRun: false, symbol: "BTC", exchanges: {} },
        items: [],
        limit: 50,
        offset: 0,
        total: 0,
        hasMore: false,
        error: null,
      })
      .mockResolvedValueOnce({
        summary: { enabled: true, dryRun: false, symbol: "BTC", exchanges: {} },
        items: [
          {
            id: "spot-whale-negative-BTC",
            ts: 1_700_000_000_001,
            symbol: "BTC",
            windowSec: 15,
            signalType: "spot_aggressive_sell",
            direction: "sell",
            severity: "critical",
            score: 86,
            totalVolumeBase: 680,
            netVolumeBase: -520,
            totalNotionalUsd: 44_000_000,
            dominance: 0.76,
            priceMovePct: -0.18,
            coinbasePremiumPct: -0.02,
            mainExchange: "coinbase",
            dataQuality: 90,
            discordEligible: true,
            discordSent: false,
            discordReason: "critical_or_s_gate",
            exchanges: [],
            finalResult: "现货主动卖出同步放大",
          },
        ],
        limit: 50,
        offset: 0,
        total: 1,
        hasMore: false,
        error: null,
      });

    await user.click(screen.getByRole("button", { name: "历史查询" }));
    await user.selectOptions(screen.getByLabelText("净方向"), "abs500");
    expect(fetchSpotWhaleHistory).toHaveBeenCalledWith(
      expect.objectContaining({ limit: 50, offset: 0, net_direction: "abs500", symbol: "BTC" }),
    );
    expect(await screen.findByTestId("spot-whale-row-spot-whale-negative-BTC")).toBeInTheDocument();
    expect(screen.getByText("-520 BTC")).toBeInTheDocument();
  });

  it("supports abs50 and abs100 net-direction options in the spot filter", async () => {
    const user = userEvent.setup();
    fetchSpotWhaleHistory
      .mockResolvedValueOnce({
        summary: { enabled: true, dryRun: false, symbol: "BTC", exchanges: {} },
        items: [],
        limit: 50,
        offset: 0,
        total: 0,
        hasMore: false,
        error: null,
      })
      .mockResolvedValueOnce({
        summary: { enabled: true, dryRun: false, symbol: "BTC", exchanges: {} },
        items: [
          {
            id: "spot-whale-negative-100-BTC",
            ts: 1_700_000_000_002,
            symbol: "BTC",
            windowSec: 15,
            signalType: "spot_aggressive_sell",
            direction: "sell",
            severity: "medium",
            score: 80,
            totalVolumeBase: 150,
            netVolumeBase: -100,
            totalNotionalUsd: 6_000_000,
            dominance: 0.67,
            priceMovePct: -0.05,
            coinbasePremiumPct: 0,
            mainExchange: "binance",
            dataQuality: 88,
            discordEligible: false,
            discordSent: false,
            exchanges: [],
            finalResult: "spot sell pressure",
          },
        ],
        limit: 50,
        offset: 0,
        total: 1,
        hasMore: false,
        error: null,
      });

    render(<SpotWhaleMonitor />);

    await user.click(screen.getByRole("button", { name: "历史查询" }));
    await screen.findByLabelText("净方向");
    await user.selectOptions(screen.getByLabelText("净方向"), "abs100");

    expect(fetchSpotWhaleHistory).toHaveBeenCalledWith(
      expect.objectContaining({ limit: 50, offset: 0, net_direction: "abs100", symbol: "BTC" }),
    );
    expect(await screen.findByTestId("spot-whale-row-spot-whale-negative-100-BTC")).toBeInTheDocument();
    expect(screen.getByText("-100 BTC")).toBeInTheDocument();
  });

  it("scopes summary and latest to locked ETH", async () => {
    render(<SpotWhaleMonitor lockedSymbol="ETH" />);

    expect(await screen.findByText("ETH 现货监控")).toBeInTheDocument();
    expect(screen.getByText("ETH SPOT WHALE FLOW")).toBeInTheDocument();
    expect(screen.getByText("币种：ETH（当前页面固定）")).toBeInTheDocument();
    expect(screen.queryByLabelText("币种")).not.toBeInTheDocument();
    expect(screen.queryByText("SOL")).not.toBeInTheDocument();
    await waitFor(() => {
      expect(fetchSpotWhaleSummary).toHaveBeenCalledWith("ETH");
      expect(fetchSpotWhaleLatest).toHaveBeenCalledWith(50, "ETH");
    });
  });

  it("uses ETH-specific net-direction thresholds on the ETH spot page", async () => {
    const user = userEvent.setup();
    fetchSpotWhaleHistory.mockResolvedValue({
      summary: { enabled: true, dryRun: false, symbol: "ETH", exchanges: {} },
      items: [],
      limit: 50,
      offset: 0,
      total: 0,
      hasMore: false,
      error: null,
    });

    render(<SpotWhaleMonitor lockedSymbol="ETH" />);

    await user.click(screen.getByRole("button", { name: "历史查询" }));
    const netDirection = await screen.findByLabelText("净方向");
    expect(netDirection).toHaveDisplayValue("全部");
    expect(screen.getByRole("option", { name: "大于 1000（正负）" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "大于 2000（正负）" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "大于 5000（正负）" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "大于 10000（正负）" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "大于 50（正负）" })).not.toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "大于 500（正负）" })).not.toBeInTheDocument();

    await user.selectOptions(netDirection, "abs5000");
    expect(fetchSpotWhaleHistory).toHaveBeenCalledWith(
      expect.objectContaining({
        limit: 50,
        offset: 0,
        net_direction: "abs5000",
        symbol: "ETH",
      }),
    );
  });

  it("shows stale spot exchanges instead of reporting them as online", async () => {
    const staleSummary = {
      status: "calm",
      healthStatus: "unhealthy",
      latestDirection: "neutral",
      latestSeverity: "calm",
      signalCount: 0,
      enabled: true,
      dryRun: false,
      symbol: "BTC",
      trend60s: {
        buyVolumeBase: 0,
        sellVolumeBase: 0,
        totalVolumeBase: 0,
        netVolumeBase: 0,
        dominance: 0,
        buyRatio: 0,
        sellRatio: 0,
        updatedAtMs: null,
      },
        exchanges: {
          binance: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_000, latencyMs: 50 },
          coinbase: { connected: false, status: "stale", lastTradeAt: 1_699_999_000_000, latencyMs: 1_000_000 },
          bitfinex: { connected: false, status: "stale", lastTradeAt: 1_699_999_000_000, latencyMs: 1_000_000 },
        },
    };
    fetchSpotWhaleSummary.mockResolvedValueOnce({ summary: staleSummary, error: null });
    fetchSpotWhaleLatest.mockResolvedValueOnce({ summary: staleSummary, items: [], error: null });

    render(<SpotWhaleMonitor />);

    expect((await screen.findAllByText("无近期成交")).length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText("健康状态")).toBeInTheDocument();
    expect(screen.getByText("异常")).toBeInTheDocument();
    expect(screen.getByText(/60s 总成交 0 BTC/)).toBeInTheDocument();
  });

  it("warns when the latest spot signal is an old snapshot", async () => {
    const staleSummary = {
      enabled: true,
      dryRun: true,
      symbol: "BTC",
      status: "calm",
      healthStatus: "healthy",
      latestIsStale: true,
      latestAgeSec: 720,
      latestStaleReason: "older_than_latest_ttl",
      exchanges: {},
      trend60s: {},
    };
    fetchSpotWhaleSummary.mockResolvedValueOnce({ summary: staleSummary, error: null });
    fetchSpotWhaleLatest.mockResolvedValueOnce({
      summary: staleSummary,
      items: [],
      limit: 50,
      offset: 0,
      total: 0,
      hasMore: false,
      error: null,
    });

    render(<SpotWhaleMonitor lockedSymbol="BTC" />);

    expect(await screen.findByText(/BTC latest 为旧快照/)).toBeInTheDocument();
  });

  it("supports permanent-only history paging", async () => {
    const user = userEvent.setup();
    fetchSpotWhaleHistory
      .mockResolvedValueOnce({
        summary: { enabled: true, dryRun: false, symbol: "BTC", exchanges: {} },
        items: [],
        limit: 50,
        offset: 0,
        total: 51,
        hasMore: true,
        error: null,
      })
      .mockResolvedValueOnce({
        summary: { enabled: true, dryRun: false, symbol: "BTC", exchanges: {} },
        items: [
          {
            id: "spot-whale-history-page-1",
            ts: 1_700_000_000_000,
            symbol: "BTC",
            windowSec: 15,
            signalType: "spot_aggressive_buy",
            direction: "buy",
            severity: "critical",
            score: 90,
            totalVolumeBase: 500,
            netVolumeBase: 65,
            totalNotionalUsd: 35_000_000,
            dominance: 0.7,
            mainExchange: "binance",
            dataQuality: 90,
            discordEligible: true,
            discordSent: true,
            discordReason: "critical_or_s_gate",
            exchanges: [],
            finalResult: "permanent page 1",
            isPermanent: true,
          },
        ],
        limit: 50,
        offset: 0,
        total: 51,
        hasMore: true,
        error: null,
      })
      .mockResolvedValueOnce({
        summary: { enabled: true, dryRun: false, symbol: "BTC", exchanges: {} },
        items: [
          {
            id: "spot-whale-history-page-2",
            ts: 1_699_999_000_000,
            symbol: "BTC",
            windowSec: 15,
            signalType: "spot_aggressive_sell",
            direction: "sell",
            severity: "high",
            score: 86,
            totalVolumeBase: 300,
            netVolumeBase: -80,
            totalNotionalUsd: 20_000_000,
            dominance: 0.68,
            mainExchange: "coinbase",
            dataQuality: 88,
            discordEligible: true,
            discordSent: false,
            discordReason: "high_score_multi_exchange",
            exchanges: [],
            finalResult: "permanent page 2",
            isPermanent: true,
          },
        ],
        limit: 50,
        offset: 50,
        total: 51,
        hasMore: false,
        error: null,
      });

    render(<SpotWhaleMonitor />);

    await user.click(screen.getByRole("button", { name: "历史查询" }));
    await user.click(screen.getByLabelText("只看永久信号"));

    await waitFor(() => {
      expect(fetchSpotWhaleHistory).toHaveBeenLastCalledWith(
        expect.objectContaining({
          symbol: "BTC",
          limit: 50,
          offset: 0,
          permanent_only: true,
        }),
      );
    });

    expect(await screen.findByText("共 51 条 · 第 1 / 2 页")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "下一页" }));

    await waitFor(() => {
      expect(fetchSpotWhaleHistory).toHaveBeenLastCalledWith(
        expect.objectContaining({
          symbol: "BTC",
          limit: 50,
          offset: 50,
          permanent_only: true,
        }),
      );
    });

    expect(await screen.findByTestId("spot-whale-row-spot-whale-history-page-2")).toBeInTheDocument();
  });
});
