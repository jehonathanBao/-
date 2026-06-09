import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SignalTable from "../components/SignalTable.jsx";
import { mockSignals } from "../data/mockSignals.js";
import { useSignalsStore } from "../store/signalsStore.js";

describe("Signal inbox card display", () => {
  beforeEach(() => {
    resetStore();
  });

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
  });

  it("keeps old signals in raw inbox when new signals arrive", () => {
    useSignalsStore.getState().setSignals([
      {
        ...mockSignals[0],
        id: "sig_new",
        dedupeKey: "binance:BTCUSDT:new-flow",
      },
    ]);

    const ids = useSignalsStore.getState().rawInboxSignals.map((signal) => signal.id);
    expect(ids).toContain("sig_001");
    expect(ids).toContain("sig_new");
  });

  it("merges live snapshots into the persistent inbox instead of replacing it", () => {
    resetStore([]);
    const firstSnapshot = [mockSignals[0], mockSignals[1]];
    const nextSnapshot = [
      {
        ...mockSignals[2],
        id: "sig_live_snapshot_new",
        dedupeKey: "binance:XRPUSDT:layering:new-live-snapshot",
      },
    ];

    useSignalsStore.getState().setSignals(firstSnapshot);
    useSignalsStore.getState().setSignals(nextSnapshot);

    const state = useSignalsStore.getState();
    const ids = state.rawInboxSignals.map((signal) => signal.id);
    expect(ids).toContain("sig_001");
    expect(ids).toContain("sig_002");
    expect(ids).toContain("sig_live_snapshot_new");
    expect(state.rawInboxSignals.find((signal) => signal.id === "sig_001").isLive).toBe(false);
    expect(state.rawInboxSignals.find((signal) => signal.id === "sig_live_snapshot_new").isLive).toBe(true);
  });

  it("keeps cached signals when the latest backend snapshot is empty", () => {
    resetStore([]);
    useSignalsStore.getState().setSignals([mockSignals[0]]);

    useSignalsStore.getState().setSignals([]);

    const state = useSignalsStore.getState();
    expect(state.rawInboxSignals).toHaveLength(1);
    expect(state.rawInboxSignals[0].id).toBe("sig_001");
    expect(state.rawInboxSignals[0].isLive).toBe(false);
  });

  it("persists local review status markers across refresh state loads", () => {
    useSignalsStore.getState().setSignalReviewStatus("sig_001", "acknowledged");

    const state = useSignalsStore.getState();
    expect(state.rawInboxSignals.find((signal) => signal.id === "sig_001").reviewStatus).toBe("acknowledged");
    const persisted = JSON.parse(window.localStorage.getItem("toxic-order-monitor.signal-inbox.v1"));
    expect(persisted.rawInboxSignals.find((signal) => signal.id === "sig_001").reviewStatus).toBe("acknowledged");
  });

  it("does not render duplicate dedupeKey values twice", () => {
    useSignalsStore.getState().setSignals([
      {
        ...mockSignals[0],
        id: "sig_duplicate",
        dedupeKey: mockSignals[0].dedupeKey,
      },
    ]);

    expect(useSignalsStore.getState().rawInboxSignals).toHaveLength(mockSignals.length);
  });

  it("lets filtering affect visible signals without deleting raw inbox", () => {
    useSignalsStore.getState().setRiskFilter("high");
    const rawBefore = useSignalsStore.getState().rawInboxSignals.length;
    const visibleSignals = useSignalsStore
      .getState()
      .rawInboxSignals.filter((signal) => signal.risk === useSignalsStore.getState().activeRiskFilter);

    expect(visibleSignals.length).toBeLessThan(rawBefore);
    expect(useSignalsStore.getState().rawInboxSignals).toHaveLength(rawBefore);
  });

  it("hides technical tags and internal fields from the card", () => {
    renderInbox([mockSignals[0]]);

    expect(screen.queryByText("inferred_from_l2_delta")).not.toBeInTheDocument();
    expect(screen.queryByText("信心 / 风险评分")).not.toBeInTheDocument();
    expect(screen.queryByText("dataQuality")).not.toBeInTheDocument();
    expect(screen.queryByText("dedupeKey")).not.toBeInTheDocument();
    expect(screen.queryByText("未处理")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /更多信息/ })).not.toBeInTheDocument();
  });

  it("shows only the final result description when evidence is missing", () => {
    renderInbox([mockSignals[4]]);

    expect(screen.getByText("无法判断方向")).toBeInTheDocument();
    expect(screen.queryByText("insufficient_trade_confirmation")).not.toBeInTheDocument();
  });

  it("shows stale candidates instead of deleting them from the card list", () => {
    const staleSignal = {
      ...mockSignals[0],
      isLive: false,
      lastSeenAt: Date.now() - 120_000,
    };

    renderInbox([staleSignal]);

    expect(screen.getByTestId("signal-card-sig_001")).toBeInTheDocument();
    expect(screen.queryByText(/stale · last seen/)).not.toBeInTheDocument();
  });

  it("keeps low-score candidates visible even when Discord push is gated", () => {
    const gatedSignal = {
      ...mockSignals[7],
      score: 20,
      dataQuality: 30,
    };

    renderInbox([gatedSignal]);

    expect(screen.getByTestId("signal-card-sig_008")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /推送 sig_008 到 Discord/ })).toHaveTextContent("仅页面展示");
  });

  it("shows a manual Discord push action for high-risk candidates", () => {
    renderInbox([mockSignals[0]]);

    expect(screen.getByRole("button", { name: /推送 sig_001 到 Discord/ })).toHaveTextContent("手动推送");
    expect(screen.getByText("卖方挂单诱导，潜在下行压力")).toBeInTheDocument();
  });

  it("shows the latest visible signal time in the status cards", () => {
    renderInbox([mockSignals[0], mockSignals[1]]);

    expect(screen.getByTestId("signal-inbox-updated-at")).toHaveTextContent("更新时间");
    expect(screen.getByTestId("signal-inbox-updated-at")).toHaveTextContent("12:34:10");
  });

  it("shows Discord auto push status and opens read-only review details", async () => {
    const user = userEvent.setup();
    const onMarkStatus = vi.fn();
    renderInbox([
      {
        ...mockSignals[0],
        toxicHalfLifeSec: 45,
        toxicMaxTtlSec: 300,
        toxicDecayedScore: 91,
        toxicDecayFormula: "decayedScore = previousScore * exp(-elapsedSec / halfLifeSec)",
        toxicReasons: [
          {
            reasonType: "SpoofCancel",
            score: 82,
            weight: 0.15,
            windowSec: 15,
            direction: "bearish",
            description: "fake wall and near-touch cancel count",
          },
        ],
        marketStructureSeverity: "Major",
        marketStructureDataQuality: 86,
        mainForceConfirmed: true,
        mainForceConfirmationCount: 6,
        mainForceConfirmationTotal: 7,
        mainForceConfirmationThreshold: 3,
        extremeImpactConfirmed: true,
        regimeType: "main_force_long_build",
        marketStructureConfidence: 93,
        marketStructureDataQuality: 91,
        structureBias: 72,
        structureRaw: 85,
        spotContractFloor: 75,
        durationScore: 100,
        liquidationPenalty: 0,
        crowdingPenalty: 0,
        spotScore: 75,
        spotCvdScore: 84,
        spotVolumeAnomaly: 72,
        spotAbsorption: 64,
        spotLiquidityShift: 73,
        spotPriceResponse: 85,
        contractScore: 90,
        cwmAggressiveFlow: 94,
        oiImpulse: 88,
        liquidationContext: 93,
        fundingCrowding: 88,
        basisPremium: 74,
        activeExchangeConfirmation: 92,
        crossConfirmScore: 92,
        spotContractDirectionConsistency: 90,
        multiWindowConsistency: 92,
        priceResponseConsistency: 90,
        sourceCoverage: 100,
        signalAgreement: 95,
        oiScore: 88,
        liquidationScore: 93,
        fundingCrowdingScore: 88,
        cwmScore: 94,
        marketStructureReasons: [
          {
            reasonType: "CrossConfirmScore",
            score: 92,
            weight: 0.2,
            timeframe: "15m/1h",
            direction: "bullish",
            description: "weighted cross-confirm composite",
          },
        ],
        alertStatus: "skipped",
        alertReason: "cached_on_boot",
        discordAlert: {
          autoEligible: false,
          autoSent: false,
          lastDecision: "skipped",
          reason: "cached_on_boot",
        },
      },
    ], { onMarkStatus });

    expect(screen.getByText("主力确认 已确认 · 6/7")).toBeInTheDocument();
    expect(screen.getAllByText(/极端行情 是/).length).toBeGreaterThan(0);
    expect(screen.getByText("偏向 +72")).toBeInTheDocument();
    expect(screen.getByText("Discord：未推送，原因：历史缓存不自动推送")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /查看回放 sig_001/ })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: /Review sig_001/ }));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText("Candidate Review")).toBeInTheDocument();
    expect(screen.getByText("Symbol")).toBeInTheDocument();
    expect(screen.getAllByText("BTCUSDT", { exact: false })).not.toHaveLength(0);
    expect(screen.getByText("Toxic Score")).toBeInTheDocument();
    expect(screen.getByText("Toxic Half-Life")).toBeInTheDocument();
    expect(screen.getByText("Decayed Score")).toBeInTheDocument();
    expect(screen.getByText("Decay Formula")).toBeInTheDocument();
    expect(screen.getByText("Toxic Reasons")).toBeInTheDocument();
    expect(screen.getByText("SpoofCancel 82")).toBeInTheDocument();
    expect(screen.getByText("Main Force Score")).toBeInTheDocument();
    expect(screen.getByText("Main Force Confirmed")).toBeInTheDocument();
    expect(screen.getByText("Main Force Confirmation Count")).toBeInTheDocument();
    expect(screen.getByText("6/7 (min 3)")).toBeInTheDocument();
    expect(screen.getByText("Extreme Market Impact")).toBeInTheDocument();
    expect(screen.getByText("Regime Type")).toBeInTheDocument();
    expect(screen.getByText("Market Structure Confidence")).toBeInTheDocument();
    expect(screen.getByText("Signal Agreement")).toBeInTheDocument();
    expect(screen.getByText("主力建多 · main_force_long_build")).toBeInTheDocument();
    expect(screen.getByText("Market Structure Severity")).toBeInTheDocument();
    expect(screen.getByText("Structure Raw")).toBeInTheDocument();
    expect(screen.getByText("Spot/Contract Floor")).toBeInTheDocument();
    expect(screen.getByText("Duration Score")).toBeInTheDocument();
    expect(screen.getByText("Liquidation Penalty")).toBeInTheDocument();
    expect(screen.getByText("Crowding Penalty")).toBeInTheDocument();
    expect(screen.getByText("Spot Score")).toBeInTheDocument();
    expect(screen.getByText("Spot CVD")).toBeInTheDocument();
    expect(screen.getByText("Spot Volume Anomaly")).toBeInTheDocument();
    expect(screen.getByText("Spot Absorption")).toBeInTheDocument();
    expect(screen.getByText("Spot Liquidity Shift")).toBeInTheDocument();
    expect(screen.getByText("Spot Price Response")).toBeInTheDocument();
    expect(screen.getByText("Contract Score")).toBeInTheDocument();
    expect(screen.getByText("CWM Aggressive Flow")).toBeInTheDocument();
    expect(screen.getByText("OI Impulse")).toBeInTheDocument();
    expect(screen.getByText("Liquidation Context")).toBeInTheDocument();
    expect(screen.getAllByText("Funding Crowding").length).toBeGreaterThan(0);
    expect(screen.getByText("Basis Premium")).toBeInTheDocument();
    expect(screen.getByText("Active Exchange Confirmation")).toBeInTheDocument();
    expect(screen.getByText("Cross Confirm")).toBeInTheDocument();
    expect(screen.getByText("Spot/Contract Direction")).toBeInTheDocument();
    expect(screen.getByText("Multi-Window Consistency")).toBeInTheDocument();
    expect(screen.getByText("Price Response Consistency")).toBeInTheDocument();
    expect(screen.getByText("Source Coverage")).toBeInTheDocument();
    expect(screen.getByText("OI Score")).toBeInTheDocument();
    expect(screen.getByText("Liquidation Score")).toBeInTheDocument();
    expect(screen.getByText("Funding Crowding")).toBeInTheDocument();
    expect(screen.getByText("Market Structure Reasons")).toBeInTheDocument();
    expect(screen.getByText("CrossConfirmScore 92")).toBeInTheDocument();
    expect(screen.getByText("Data Quality")).toBeInTheDocument();
    expect(screen.getByText("TOF Score")).toBeInTheDocument();
    expect(screen.getByText("Discord Alert Status")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "important" }));
    expect(onMarkStatus).toHaveBeenCalledWith("sig_001", "important");
  });

  it("shows CWM contribution on the card and review modal", async () => {
    const user = userEvent.setup();
    renderInbox([
      {
        ...mockSignals[0],
        cwmContribution: {
          available: true,
          score: 94,
          weightedContribution: 23.5,
          windowSec: 15,
          mainExchange: "binance",
          exchangeCount: 2,
          discordGateIndependent: true,
        },
      },
    ]);

    expect(screen.getByText(/CWM 94/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Review sig_001/ }));

    expect(screen.getByText("CWM Contribution")).toBeInTheDocument();
    expect(screen.getByText(/Score 94 · main-force component \+23.5 · 15s · binance · active venues 2 · CWM gate independent/)).toBeInTheDocument();
  });

  it("opens redacted replay snapshot when replay data exists", async () => {
    const user = userEvent.setup();
    renderInbox([
      {
        ...mockSignals[0],
        replaySnapshot: {
          signalId: "sig_001",
          symbol: "BTCUSDT",
          eventType: "book_delta",
          rawPayload: "must not render",
          evidence: "must not render",
          markout: "must not render",
          token: "must not render",
          safeSummary: "redacted replay snapshot",
        },
      },
    ]);

    const replayButton = screen.getByRole("button", { name: /查看回放 sig_001/ });
    expect(replayButton).toBeEnabled();
    await user.click(replayButton);

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText("Replay Snapshot")).toBeInTheDocument();
    expect(screen.getByText(/redacted replay snapshot/)).toBeInTheDocument();
    expect(screen.queryByText("must not render")).not.toBeInTheDocument();
    expect(screen.queryByText("rawPayload")).not.toBeInTheDocument();
    expect(screen.queryByText("evidence")).not.toBeInTheDocument();
    expect(screen.queryByText("markout")).not.toBeInTheDocument();
  });

  it("shows medium-risk candidates as display-only for Discord", () => {
    const mediumSignal = {
      ...mockSignals[2],
      score: 95,
      dataQuality: 95,
    };

    renderInbox([mediumSignal]);

    expect(screen.getByTestId("signal-card-sig_003")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /推送 sig_003 到 Discord/ })).toHaveTextContent("仅页面展示");
  });
});

function renderInbox(signals, overrides = {}) {
  return render(
    React.createElement(SignalTable, {
      inboxStats: { total: signals.length, high: 0, medium: signals.length, low: 0 },
      onPush: vi.fn(),
      onSelect: vi.fn(),
      selectedSignal: signals[0],
      signals,
      ...overrides,
    }),
  );
}

function resetStore(initialSignals = mockSignals) {
  useSignalsStore.setState({
    rawInboxSignals: initialSignals,
    signals: initialSignals,
    selectedSignal: initialSignals[0] ?? null,
    activeRiskFilter: "all",
    pushStatus: {},
    storageWarning: null,
    pushLogs: [],
    discordConnected: false,
    lastPushedAt: null,
    clearedAtMs: 0,
    clearedSignalKeys: [],
  });
}
