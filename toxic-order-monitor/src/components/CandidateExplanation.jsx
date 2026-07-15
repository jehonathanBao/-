export default function CandidateExplanation({ signal, compact = false }) {
  if (!signal) {
    return null;
  }
  const tags = Array.isArray(signal.explainTags) ? signal.explainTags : [];
  const label = signal.directionLabel || signal.side || "方向不可用";
  const directionConfidence = numberOrNull(signal.directionConfidence);
  const mainForceConfirmed = signal.mainForceConfirmed ?? signal.marketStructureScore?.mainForceConfirmed;
  const extremeImpactConfirmed = signal.extremeImpactConfirmed ?? signal.marketStructureScore?.extremeImpactConfirmed;

  return (
    <div className={compact ? "mt-3 space-y-2" : "rounded-xl border border-slate-700/60 bg-slate-950/40 p-4"}>
      <div className="flex flex-wrap items-center gap-2">
        <span className="rounded-full border border-cyan-400/40 px-2.5 py-1 text-xs font-semibold text-cyan-200">
          {label}
        </span>
        {directionConfidence !== null ? (
          <span className="text-xs text-slate-400">置信度 {Math.round(directionConfidence)}</span>
        ) : null}
        <span className="text-xs text-slate-500">{signal.directionSource || "detector"}</span>
      </div>
      <div className="grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
        <span>
          Toxic {formatNumber(signal.toxicScore ?? signal.finalRiskScore ?? signal.score)} / Quality{" "}
          {formatNumber(signal.dataQuality)}
        </span>
        <span>
          Main Force{" "}
          {formatNumber(signal.mainForceScore ?? signal.riskSystems?.mainForceStructure?.mainForceScore)}
        </span>
        <span>
          Bias {formatSignedNumber(signal.structureBias ?? signal.marketStructureScore?.structureBias)}
        </span>
        <span>
          Confirmed {formatBoolean(mainForceConfirmed)} ·{" "}
          {formatNumber(
            signal.mainForceConfirmationCount ?? signal.marketStructureScore?.mainForceConfirmationCount,
          )}
          /
          {formatNumber(
            signal.mainForceConfirmationTotal ?? signal.marketStructureScore?.mainForceConfirmationTotal,
          )}
        </span>
        <span>
          Structure {signal.marketStructureSeverity || signal.marketStructureScore?.severity || "N/A"} · Extreme{" "}
          {formatNumber(signal.extremeImpactScore ?? signal.marketStructureScore?.extremeImpactScore)}
        </span>
        <span>
          Conf {formatNumber(signal.marketStructureConfidence ?? signal.marketStructureScore?.confidence)} / Quality{" "}
          {formatNumber(signal.marketStructureDataQuality ?? signal.marketStructureScore?.dataQuality)}
        </span>
        <span>
          极端行情 {formatBoolean(extremeImpactConfirmed, "是", "否")} ·{" "}
          {regimeTypeLabel(signal.regimeType)}
        </span>
        <span>
          Spot {formatNumber(signal.spotScore ?? signal.marketStructureScore?.spotScore)} / Contract{" "}
          {formatNumber(signal.contractScore ?? signal.marketStructureScore?.contractScore)}
        </span>
        <span>
          Spot CVD {formatNumber(signal.spotCvdScore ?? signal.marketStructureScore?.spotCvdScore)} / Vol{" "}
          {formatNumber(signal.spotVolumeAnomaly ?? signal.marketStructureScore?.spotVolumeAnomaly)} / Abs{" "}
          {formatNumber(signal.spotAbsorption ?? signal.marketStructureScore?.spotAbsorption)}
        </span>
        <span>
          Floor {formatNumber(signal.spotContractFloor ?? signal.marketStructureScore?.spotContractFloor)} / Duration{" "}
          {formatNumber(signal.durationScore ?? signal.marketStructureScore?.durationScore)}
        </span>
        <span>
          CWM Flow {formatNumber(signal.cwmAggressiveFlow ?? signal.marketStructureScore?.cwmAggressiveFlow)} / OI{" "}
          {formatNumber(signal.oiImpulse ?? signal.marketStructureScore?.oiImpulse)} / Liq{" "}
          {formatNumber(signal.liquidationContext ?? signal.marketStructureScore?.liquidationContext)}
        </span>
        <span>Severity {signal.level || signal.risk || "N/A"}</span>
        {signal.finalCandidateType ? <span>Final {signal.finalCandidateType}</span> : null}
        {signal.metricsDirection ? <span>Metrics {signal.metricsDirection}</span> : null}
      </div>
      <p className="text-xs font-semibold text-slate-300">Type: {signal.candidateType || signal.type || "N/A"}</p>
      {signal.perpCandidateType ? (
        <p className="text-xs font-semibold text-indigo-200">Perp: {signal.perpCandidateType}</p>
      ) : null}
      {signal.advancedCandidateType ? (
        <p className="text-xs font-semibold text-fuchsia-200">Advanced: {signal.advancedCandidateType}</p>
      ) : null}
      {tags.length > 0 ? (
        <div className="flex flex-wrap gap-2">
          {tags.slice(0, compact ? 3 : 8).map((tag) => (
            <span className="rounded-full border border-slate-700 px-2 py-0.5 text-[11px] text-slate-400" key={tag}>
              {tag}
            </span>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function formatNumber(value) {
  const number = numberOrNull(value);
  return number === null ? "N/A" : Math.round(number);
}

function formatSignedNumber(value) {
  const number = numberOrNull(value);
  if (number === null) {
    return "N/A";
  }
  return number > 0 ? `+${Math.round(number)}` : `${Math.round(number)}`;
}

function formatBoolean(value, trueLabel = "Yes", falseLabel = "No") {
  if (typeof value !== "boolean") {
    return "N/A";
  }
  return value ? trueLabel : falseLabel;
}

function numberOrNull(value) {
  if (value === null || value === undefined || value === "") {
    return null;
  }
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function regimeTypeLabel(value) {
  return {
    main_force_long_build: "主力建多",
    main_force_short_build: "主力建空",
    contract_flow_shock: "合约冲击",
    spot_accumulation: "现货吸筹",
    spot_distribution: "现货派发",
    contract_short_squeeze: "空头挤压",
    long_liquidation_cascade: "多头踩踏",
    downside_absorption: "下方吸收",
    upside_resistance: "上方压制",
    range_rotation: "高换手震荡",
    unclear: "结构不清晰",
  }[value] || value || "N/A";
}
