export function finalResultDescription(signal) {
  const explicit =
    signal?.finalResult ||
    signal?.resultDescription ||
    signal?.finalDescription ||
    signal?.displayResult;
  if (explicit) {
    return explicit;
  }

  const type = String(signal?.type || signal?.eventType || "").toLowerCase();
  const side = String(signal?.side || "").toLowerCase();
  const reason = String(signal?.reason || signal?.summary || "");

  if (type.includes("liquiditypull") || type.includes("thinness") || reason.includes("深度")) {
    if (side.includes("bid") || side.includes("buy") || reason.includes("买")) {
      return "买方流动性移除，潜在上行压力";
    }
    if (side.includes("ask") || side.includes("sell") || reason.includes("卖")) {
      return "卖方流动性移除，潜在下行压力";
    }
    return "流动性移除，方向暂不明确";
  }

  if (type.includes("spoofing") || type.includes("layering")) {
    if (side.includes("ask") || side.includes("sell")) {
      return "卖方挂单诱导，潜在下行压力";
    }
    if (side.includes("bid") || side.includes("buy")) {
      return "买方挂单诱导，潜在上行压力";
    }
    return "疑似盘口诱导，方向暂不明确";
  }

  if (type.includes("whale") || type.includes("sweep") || type.includes("aggressive")) {
    if (side.includes("ask") || side.includes("sell")) {
      return "主动卖出放大，潜在下行压力";
    }
    if (side.includes("bid") || side.includes("buy")) {
      return "主动买入放大，潜在上行压力";
    }
    return "大额主动成交，方向暂不明确";
  }

  if (type.includes("liquidation")) {
    if (side.includes("sell") || reason.includes("下")) {
      return "下跌清算压力，可能加速下行";
    }
    if (side.includes("buy") || reason.includes("上")) {
      return "上涨清算压力，可能加速上行";
    }
    return "清算压力升高，方向暂不明确";
  }

  if (type.includes("notrade") || type.includes("watch") || type.includes("spread")) {
    return "无法判断方向";
  }

  return signal?.impact || "无法判断方向";
}
