# 有毒订单监控用户使用指南

这份指南面向每天打开页面看信号的人。
重点不是部署，而是告诉你：这个软件怎么看、信号怎么解读、出现不同提示代表什么。

## 1. 先记住一句话

本系统只做提醒和观察，不自动交易。

页面里所有 `Candidate` 都表示“候选信号”，意思是系统发现了异常迹象，建议你人工复核。
它不是确定结论，也不是买卖指令。

看到信号时，先看三件事：

1. 风险等级
2. 方向
3. 核心原因

再看分数、数据质量和交易所状态。

## 2. 左侧菜单怎么看

### 监控首页

这是总览页。

你主要看：

- Discord 推送状态
- 当前有毒订单判断逻辑
- 高风险异常数量
- 中风险异常数量
- 最新 High / Critical 候选信号

适合开盘后第一眼确认系统是否正常。

### BTC/ETH 合约监控

这是合约主力成交流监控。

它看的是 BTC / ETH 永续合约里的主动成交冲击，比如：

- 主力拉盘
- 主力砸盘
- 下方吸收
- 上方压制

这个页面更像“大行情雷达”。
如果这里出现 S 级或 Critical，通常说明合约市场短时间出现了明显资金冲击。

### BTC/ETH 现货监控

这是现货主力成交流监控。

它看的是 Binance Spot 和 Coinbase Spot 的 BTC / ETH 现货主动成交。

重点用于判断：

- 现货是否真的有主动买盘
- 现货是否真的有主动卖盘
- 合约异动是否有现货配合
- Binance 和 Coinbase 是否出现价格错位

如果合约和现货同方向一起异常，信号可信度通常更高。

### 异常信号

这里看普通盘口异常和有毒订单候选。

高风险信号会优先展示。
中风险信号通常只用于观察，不会推 Discord。

### 信号历史

这里用来复盘。

你可以回看之前出现过的信号，观察：

- 当时是什么方向
- 短线有毒订单评分是多少
- 中长线主力结构评分是多少
- Discord 有没有推送
- 后续价格是否验证了信号

### 告警规则

这里看系统当前怎么判断异常。

如果你不确定某个信号为什么出现，先看这里的规则说明，再回到信号详情里对照。

### Discord 设置

这里看 Discord 是否配置成功。

常见状态：

- `未配置`：没有 webhook，测试消息不会发送
- `测试消息发送成功`：Discord webhook 可用
- `403`：webhook 权限或地址有问题
- `429`：Discord 限流
- `dry_run`：系统只模拟发送，不会真的发到 Discord

### 两套 Discord 提醒怎么分

当前系统把外部提醒拆成两条独立链路：

- `短线有毒订单 Discord`
- `主力结构 / 巨量成交 Discord`

短线有毒订单 Discord 只负责秒级到分钟级的风险提示。
文案会明确出现：

- `短线有毒订单`
- `短线风险`
- `可能扫盘 / 插针 / 假突破`
- `不代表中长线趋势`

所以看到这类提醒时，优先把它当成“当前位置短线危险”，而不是“中长线已经反转”。

## 3. 风险等级怎么理解

### S 级

最高级别。

表示系统认为这条信号非常异常，通常满足：

- 成交量很大
- 方向集中
- 数据质量足够
- 多交易所确认
- 分数很高

处理建议：

- 立刻打开详情
- 看方向和核心原因
- 对照合约和现货是否同向
- 不要只凭一条信号下结论

S 级满足 gate 时会推 Discord。

### Critical

严重级别。

表示市场出现明显异常冲击，但确定性通常略低于 S 级。

处理建议：

- 尽快查看
- 重点看价格是否已经响应
- 看是否多交易所共振
- 看是否可能是清算推动

Critical 满足 gate 时会推 Discord。

### High

高风险。

表示异常已经比较明显，但可能还需要更多确认。

处理建议：

- 重点观察
- 看是否升级成 Critical / S
- 看同方向是否持续出现

High 不一定推 Discord。
只有分数足够高，并且多交易所确认时才可能推送。

### Medium

中风险。

表示有异常迹象，但证据不够强。

处理建议：

- 前端观察即可
- 不要当作强信号
- 适合放在信号历史里复盘

Medium 默认不推 Discord。

### Low

低风险。

通常只是轻微信号或调试信息。
一般不需要处理。

## 3.1 短线有毒订单什么时候会推 Discord

短线 Discord 使用独立 gate：

- `toxicScore >= 85`
- `confidence >= 70`
- `dataQuality >= 70`
- 同方向同类型冷却至少 `60` 秒

没达到 gate 的信号，仍然可能显示在页面里，但不会发到短线 Discord 通知链路。

## 3.2 现货 + 合约主力结构什么时候会推 Discord

主力结构 Discord 是另一条独立链路。

它主要看两种情况：

1. `mainForceScore >= 80` 且 `confidence >= 70` 且 `dataQuality >= 70`
2. 或者 `extremeImpactScore >= 85` 且 `dataQuality >= 70`

默认冷却会比短线 toxic 长很多，通常是 `15` 到 `30` 分钟。

这类提醒的关键词会是：

- `主力结构异动`
- `主力建多 / 主力建空`
- `现货吸筹 / 现货派发`
- `下方吸收 / 上方压制`
- `极端行情冲击`

看到这类 Discord 时，重点不是“这里有没有短线扫盘”，而是：

- 现货和合约有没有一起确认
- OI 是不是同向变化
- 是主力建仓，还是只是清算瀑布
- 结构方向是偏多、偏空，还是还没完全展开

## 4. 信号卡片字段怎么看

### Symbol

币种。

常见：

- `BTC`
- `ETH`
- `BTCUSDT`
- `ETHUSDT`

### Type / Candidate Type

信号类型。

它告诉你系统发现的异常属于哪一类。

例如：

- `SpoofingCandidate`：疑似诱导 / spoofing
- `LiquidityThinnessCandidate`：流动性变薄
- `WhaleFlowCandidate`：大额主动流异常
- `SpotAggressiveBuy`：现货主动买入爆发
- `SpotAggressiveSell`：现货主动卖出爆发
- `SpotDownsideAbsorption`：现货下方吸收
- `SpotUpsideSuppression`：现货上方压制
- `Aggressive Buy`：合约主动买入爆发
- `Aggressive Sell`：合约主动卖出爆发

### Direction

方向。

常见含义：

- `Ask/Sell`：偏卖压，可能对价格形成下行压力
- `Bid/Buy`：偏买盘，可能对价格形成上行压力
- `Buy`：主动买入更强
- `Sell`：主动卖出更强
- `Absorption`：卖盘很大但跌不动，疑似下方承接
- `Suppression`：买盘很大但涨不动，疑似上方压制
- `Neutral`：方向不明显

### Final Result / 最终结果

这是最重要的自然语言解释。

它会告诉你系统最终怎么看这条信号。

例如：

```text
卖方挂单诱导，潜在下行压力
```

意思是系统发现盘口上可能有诱导卖压或撤单行为，需要警惕下行风险。

```text
现货主动买入同步放大，疑似现货资金主动推升
```

意思是 Binance / Coinbase 现货主动买盘增强，价格可能被现货资金推升。

### Toxic Score

短线有毒订单评分，范围 0 到 100。

它只回答一个问题：

```text
现在这一小段盘口 / 成交流有没有短线毒性？
```

主要看 1s / 5s / 15s / 60s 的 L2、成交、撤单、价差、VPIN-lite 等短线指标。

简单理解：

- `90-100 / S`：极强短线毒性，可能插针、扫损或假突破
- `75-89 / Critical`：短线强风险，可能马上扫盘
- `60-74 / High`：有明显短线风险
- `40-59 / Watch`：有轻微异常，观察
- `0-39 / Calm`：无明显短线毒性

注意：

- `toxicScore` 不等于看多或看空。
- `shortPressure` 才表示短线方向压力，正数偏多压力，负数偏空压力。

### Toxic Score 会衰减

短线信号不能长期有效。

系统会带上：

- `halfLifeSec`：半衰期，通常 30 到 45 秒
- `maxTtlSec / ttlSec`：最长有效期，High 约 3 分钟，Critical 约 4 分钟，S 约 5 分钟
- `decayedScore`：按时间衰减后的短线评分

衰减公式：

```text
decayedScore = previousScore * exp(-elapsedSec / halfLifeSec)
```

如果后续没有继续异常，90 分的短线毒单几十秒后就会明显变弱。

普通有毒订单 Discord gate 主要看它：

```text
toxicScore >= 85
confidence >= 70
dataQuality >= 70
severity = High / Critical / S
cooldown >= 60s
```

Discord 文案会明确写“短线有毒订单”，并提示“不代表中长线趋势”。这类提醒只代表秒级到分钟级的扫盘、插针、假突破或快速反杀风险，不等于中长线主力行为。

### Market Structure Score / Main Force Score

现货 + 合约主力结构评分，范围 0 到 100。

它回答的是另一个问题：

```text
5m / 15m / 1h / 4h 里是否出现真实主力行为、吸筹、派发、压制或清算推动？
```

它会综合现货、合约、CWM 主力合约监控等中长线结构信息。

核心计算分两步。

第一步先看现货、合约和跨市场确认：

```text
structureRaw =
  0.40 * spotScore
+ 0.40 * contractScore
+ 0.20 * crossConfirmScore
```

第二步再计算真正的主力行为评分：

```text
mainForceScore =
  0.65 * structureRaw
+ 0.25 * min(spotScore, contractScore)
+ 0.10 * durationScore
- liquidationPenalty
- crowdingPenalty
```

这里最关键的是 `min(spotScore, contractScore)`。

它的意思是：如果只有合约爆量，但现货没有确认，主力结构分不会轻易打到很高。反过来，只有现货异动、合约不配合，也一样会被压住。

核心字段分三类：

- `mainForceScore`：是不是像真实主力行为，分数越高越像建多、建空、吸筹、派发、压制或吸收。
- `mainForceConfirmed`：是否达到“真实主力行为确认”门槛。不是所有高分都算确认，系统会额外检查确认条件数量。
- `extremeImpactScore`：是不是极端行情冲击，分数高可能是主力冲击，也可能只是清算瀑布。
- `structureBias`：中长线结构方向，范围 `-100` 到 `+100`。正数偏多，负数偏空，接近 `0` 更像吸收、压制或方向还没完全展开。

分项字段：

- `spotScore`：现货侧主动力量。
- `spotCvdScore`：现货主动净流 / CVD 变化，关注真实主动买盘或卖盘是否持续进入。
- `spotVolumeAnomaly`：现货成交量异常，目标口径是 5m / 15m 相对最近 7 天同窗口 P95 / P99 / P99.5；当前实时路径使用 trade rate、VPIN bucket 和窗口成交量作为可替换 proxy。
- `spotAbsorption`：现货吸收 / 派发 / 压制。主动卖出很多但跌不动偏下方吸收；主动买入很多但涨不动偏上方压制或派发。
- `spotLiquidityShift`：现货盘口结构变化，例如关键价位买卖盘堆积、深度变厚或变薄、支撑阻力迁移。
- `spotPriceResponse`：现货价格响应。主动买入能推升、主动卖出能压低属于健康响应；主动流很强但价格不动则更像吸收或压制。
- `contractScore`：合约侧 OI / Funding / Liquidation / Aggressive Flow / CWM 综合行为分。
- `cwmAggressiveFlow`：主力合约成交流，来自 BTC/ETH 合约监控。High 约 60-74，Critical 约 75-89，S 约 90-100。
- `oiImpulse`：OI 冲击。OI 上升且方向与主动流一致，更像新资金开仓；OI 下降则更可能是平仓、止损或强平。
- `liquidationContext`：清算环境。它会抬高 `extremeImpactScore`，但不会自动抬高 `mainForceScore`。
- `fundingCrowding`：资金费率拥挤程度。它不是简单看多/看空加分，而是风险修正项；极端多头费率下继续主动买入，可能代表过热，不一定适合追多。
- `basisPremium`：基差溢价 / 贴水上下文，权重较低。合约长期溢价代表杠杆多头更激进，长期贴水代表杠杆空头更激进；当前若没有显式基差数据，会使用合约价格响应和方向强度作为 proxy。
- `activeExchangeConfirmation`：启用交易所确认。只统计 enabled 交易所；OKX disabled 时不参与总量、不参与 exchange count、不降低 dataQuality，也不拖累 multi-exchange confirmed。当前两平台模式里，Binance 是主流动性源，Bitfinex 是确认源；Binance 单平台极端异常可作为 High/Critical 证据，Bitfinex 单平台最高只当 High 证据，只有 Binance + Bitfinex 同向确认时才给 S 级空间。
- `crossConfirmScore`：现货、合约、CWM 是否同向确认。它按 `0.40*SpotContractDirectionConsistency + 0.25*MultiWindowConsistency + 0.20*PriceResponseConsistency + 0.15*SourceCoverage` 计算。
- `spotContractDirectionConsistency`：现货和合约方向是否一致。现货主动买入、合约主动买入、OI 上升且价格上涨或回调不破，会提高偏多确认；偏空同理。
- `multiWindowConsistency`：多窗口一致性。5m 更像启动，15m 是确认，1h 是结构，4h 是背景；多个窗口互相支持时更可靠。
- `priceResponseConsistency`：价格响应一致性。买入流推涨、卖出流压跌是趋势推动；卖出不跌是吸收，买入不涨是压制，两者会被单独解释。
- `sourceCoverage`：来源覆盖率，按 `healthyEnabledSources / enabledSources` 计算。OKX disabled 不进入分母；例如 Binance 和 Bitfinex 都 enabled 且健康，OKX disabled 时是 `2 / 2 = 100%`。
- `signalAgreement`：现货、合约、OI、价格响应之间到底是不是在“说同一件事”。数据很好但方向打架时，`signalAgreement` 会低，`confidence` 也会跟着下降。
- `structureRaw`：现货、合约、跨市场确认的基础结构分。
- `spotContractFloor`：`min(spotScore, contractScore)`，用于强制检查现货和合约是否都配合。
- `durationScore`：信号持续性，结合 CWM 等级、窗口和跨市场确认。
- `liquidationPenalty`：疑似清算瀑布时扣分。
- `crowdingPenalty`：资金费率拥挤但跨市场确认不足时扣分。
- `oiScore`：持仓变化是否支持新开仓或去杠杆判断。
- `liquidationScore`：清算驱动或极端冲击程度。
- `fundingCrowdingScore`：资金费率拥挤和潜在挤压背景。
- `cwmScore`：BTC/ETH 合约巨量成交监控贡献。

中长线等级：

- `0-39`：`Calm`，无明显主力结构
- `40-59`：`Watch`，有局部异动，但确认不足
- `60-74`：`Confirmed`，有结构性异动，值得关注
- `75-89`：`Major`，高概率主力行为
- `90-100`：`Extreme`，极强主力行为或极端结构变化

主力确认条件：

`mainForceConfirmed = true` 需要同时满足：

- `mainForceScore >= 75`
- `confidence >= 70`
- `dataQuality >= 70`
- 至少满足 3 个确认条件

确认条件共有 7 个：

- `spotScore >= 60`
- `contractScore >= 70`
- `crossConfirmScore >= 60`
- OI 与方向一致
- 价格响应或吸收/压制结构明确
- 清算不是主要驱动
- 至少两个时间窗口方向一致

注意：

- Main Force Score 不会和 Toxic Score 强行融合成一个总分。
- Extreme Impact 很高不一定代表真实主力建仓，可能只是清算瀑布。
- Spot Score 不是单纯大单提醒，它按 `0.30*CVD + 0.25*VolumeAnomaly + 0.20*Absorption + 0.15*LiquidityShift + 0.10*PriceResponse` 计算。
- Contract Score 也不是单纯合约爆量，它按 `0.30*CWM Flow + 0.20*OI + 0.15*Liquidation + 0.15*Funding + 0.10*Basis + 0.10*Active Exchange` 计算。
- Cross Confirm Score 不是简单“同向就满分”，它还看多窗口、价格响应和 enabled source 覆盖。
- Funding 高不是自动加分，它主要提示拥挤；Basis 只做低权重背景；Active Exchange 会按当前启用交易所重新计算，已关闭的 OKX 不会拖低质量，也不会假装确认。
- CWM 分高不等于 Main Force Score 一定高，因为主力结构还要看现货、OI、清算、funding、价格响应和跨市场确认。
- CWM 大行情提醒有独立 Discord gate 和冷却。
- 普通毒单推送不要只看 Main Force Score。

### Structure Bias / 结构偏向

`structureBias` 不等于 `mainForceScore`。

它单独表示方向，口径是：

```text
structureBias = weighted_average(
  spotDirection,
  contractDirection,
  oiDirection,
  priceResponseDirection,
  liquidationDirection
)
```

简单理解：

- `+100`：极强偏多结构
- `0`：中性、吸收、压制，或者方向还没展开
- `-100`：极强偏空结构

例子：

```text
mainForceScore = 82
structureBias = +64
regimeType = main_force_long_build
```

意思是：

- 这段结构很像主力建多
- 而且方向已经比较明确地偏多

另一个例子：

```text
mainForceScore = 78
structureBias = +12
regimeType = downside_absorption
```

意思是：

- 下面有明显承接
- 但方向还没有完全展开成趋势

### Confidence 和 Data Quality 的区别

这两个现在要分开看。

`dataQuality` 回答的是：

```text
数据本身有没有问题？
```

它更偏向运行状态，例如：

- 交易所是否在线
- WebSocket 是否延迟或刚重连
- OKX 是否 disabled
- 数据是不是还在 warmup
- 价格 / 合约元数据是否异常

当前 OKX disabled 时，OKX 不扣分。

两平台模式下，可以这样理解：

- Binance online + Bitfinex online：通常 `90-100`
- Binance online + Bitfinex offline：通常 `70-80`
- Binance offline + Bitfinex online：通常 `50-65`
- 全部 offline：通常 `0-30`

`confidence` 回答的是：

```text
这个判断到底稳不稳？
```

它不是简单复制 `dataQuality`，而是按下面的结构算：

```text
confidence =
  0.35 * dataQuality
+ 0.25 * sourceCoverage
+ 0.20 * multiWindowConsistency
+ 0.20 * signalAgreement
```

所以会出现一种很常见的情况：

- 数据本身没问题，`dataQuality` 很高
- 但现货、合约、OI、价格响应互相打架
- 这时 `confidence` 仍然应该偏低

一个常见例子：

```text
极端行情：是
主力确认：否
类型：多头踩踏 / 多头清算瀑布
```

这通常表示：

- 合约成交突然爆发
- 价格快速下跌
- 多头清算很多
- OI 快速下降
- 但现货没有给出足够强的主动卖出确认

也就是说，市场冲击很剧烈，但它更像清算推动，而不是“主力主动建空”。

### Regime Type 怎么看

`regimeType` 不是另一个分数，它是在告诉你：这段结构更像什么。

常见分类：

- `main_force_long_build / 主力建多`：现货主动买入增强，合约主动买入增强，OI 上升，价格上涨或回调不破，且清算不是主要驱动。
- `main_force_short_build / 主力建空`：现货主动卖出增强，合约主动卖出增强，OI 上升，价格下跌或反弹不过，且清算不是主要驱动。
- `spot_accumulation / 现货吸筹`：现货持续主动买入，合约没有明显追多，价格横盘或缓慢抬升，下跌时承接明显。
- `spot_distribution / 现货派发`：现货持续主动卖出，合约仍有追多，价格涨不动，高位成交放大。
- `contract_short_squeeze / 空头挤压`：价格快速上涨，主动买入爆发，空头清算增加，OI 下降。它属于极端行情，但不一定等于主力建多。
- `long_liquidation_cascade / 多头踩踏`：价格快速下跌，主动卖出爆发，多头清算增加，OI 下降。它属于极端行情，但不一定等于主力建空。
- `downside_absorption / 下方吸收`：主动卖出很多，但价格跌不动，现货买盘承接，低点没有继续下移。
- `upside_resistance / 上方压制`：主动买入很多，但价格涨不动，现货卖盘压制，高点无法突破。
- `range_rotation / 高换手震荡`：成交量大，但 OI 变化不明显，价格仍在区间内震荡，现货与合约方向不一致。

看卡片或详情时，一个顺手的顺序是：

1. 先看 `主力确认` 是不是已确认。
2. 再看 `极端行情` 是不是“是”。
3. 最后看 `regimeType`，判断它更像主力建仓、吸收压制，还是清算瀑布。

### 兼容字段 finalRiskScore

部分旧接口或旧卡片里还可能看到 `finalRiskScore`。

在新版里，它只作为兼容字段使用，普通毒单场景下等同于短线 `toxicScore`，不要再把它理解成“现货 + 合约 + 主力合约”的融合总分。

Discord 通常要求：

```text
toxicScore >= 85
confidence >= 70
```

### Data Quality

数据质量，范围 0 到 100。

它衡量这条信号的数据是否完整、及时、可信。

常见理解：

- `90+`：数据非常好
- `70-89`：可用于推送判断
- `50-69`：只能观察
- `<50`：谨慎参考

Discord 通常要求：

```text
dataQuality >= 70
```

如果分数很高但数据质量低，系统也不会推 Discord。

### Dominance

方向占比。

例如：

```text
Buy 70% / Sell 30%
```

表示主动买入明显强于主动卖出。

Dominance 越高，方向越集中。
如果成交量很大但 Dominance 很低，说明买卖双方混杂，方向不一定清晰。

### Total Volume

窗口内总成交量。

合约页面一般显示 BTC / ETH 数量和美元名义金额。
现货页面显示现货 BTC / ETH 数量和美元名义金额。

成交量越大，越可能影响短线价格。

### Notional

美元名义金额。

例如：

```text
$120M
```

表示这个窗口内成交的美元规模约 1.2 亿。

只看币数量不够，因为 BTC / ETH 价格会变。
名义金额可以更直观地判断冲击规模。

### Price Move

价格变化。

它告诉你成交爆发后价格有没有跟着动。

关键判断：

- 主动买入大，价格上涨：拉盘更可信
- 主动卖出大，价格下跌：砸盘更可信
- 主动卖出大，价格没跌：可能是下方吸收
- 主动买入大，价格没涨：可能是上方压制

### Exchange Breakdown

交易所拆分。

你要看异常来自一个交易所，还是多个交易所一起出现。

多交易所同向出现，可信度更高。
单交易所异常，可能是局部噪音、数据问题或单所大单。

## 5. 合约监控信号怎么解读

### 主力拉盘 / Aggressive Buy

含义：

合约主动买入成交突然放大，价格也同步上涨。

通常代表：

- 多头主动进攻
- 短线向上冲击增强
- 可能有主力资金介入

你要确认：

- 是否 Binance 参与
- 是否多个交易所同向
- 价格是否真的上涨
- 是否现货也同步买入

### 主力砸盘 / Aggressive Sell

含义：

合约主动卖出成交突然放大，价格也同步下跌。

通常代表：

- 空头主动进攻
- 短线下行压力增强
- 可能引发连锁止损或清算

你要确认：

- 是否成交量足够大
- 是否价格同步下跌
- 是否现货也有主动卖出
- 是否可能是清算驱动

### 下方吸收 / Absorption

含义：

主动卖出很大，但价格没有明显跌下去。

这通常说明下方有人承接。

可能代表：

- 空头打不动
- 下方买盘强
- 后续可能反弹

注意：

吸收信号不是立刻看多。
它只是说明卖压被接住了，需要继续观察后续是否反转。

### 上方压制 / Suppression

含义：

主动买入很大，但价格没有明显涨上去。

这通常说明上方有人持续卖出压制。

可能代表：

- 多头打不动
- 上方卖盘强
- 后续可能回落

注意：

压制信号不是立刻看空。
它只是说明买盘被压住了，需要观察是否继续失速。

## 6. 现货监控信号怎么解读

### 现货主动买入爆发

含义：

Binance / Coinbase 现货主动买入同步放大。

通常代表：

- 现货真实买盘增强
- 合约上涨如果有现货配合，可信度更高
- 可能不是单纯合约拉盘

你要看：

- Binance 和 Coinbase 是否都在线
- 是否两个交易所都有买入
- 现货价格是否同步上涨
- 合约页面是否同方向

### 现货主动卖出爆发

含义：

Binance / Coinbase 现货主动卖出同步放大。

通常代表：

- 现货真实卖盘增强
- 合约下跌如果有现货配合，可信度更高
- 可能有真实抛压

### 现货下方吸收

含义：

现货主动卖出很大，但价格跌不动。

通常代表：

- 下方真实买盘承接
- 卖方力量被吸收
- 可能形成短线支撑

### 现货上方压制

含义：

现货主动买入很大，但价格涨不动。

通常代表：

- 上方真实卖盘压制
- 买方力量被吸收
- 可能形成短线阻力

### 现货交易所错位

含义：

Binance 和 Coinbase 的价格出现异常偏离。

可能代表：

- 某个市场先动
- 跨交易所资金不同步
- 短时价差异常

这种信号要结合交易所健康状态看。
如果某个交易所数据延迟或断开，错位信号可信度会下降。

## 7. 有毒订单 / 盘口异常信号怎么解读

### SpoofingCandidate

疑似诱导挂单或撤单。

常见表现：

- 某一侧挂单突然堆积
- 随后快速撤掉
- 试图影响其他人的判断

如果方向是 `Ask/Sell`，通常代表上方卖压或诱导卖压。
如果方向是 `Bid/Buy`，通常代表下方买盘或诱导买盘。

### LiquidityThinnessCandidate

流动性变薄。

含义：

盘口深度变浅，价格更容易被大单打穿。

常见风险：

- 滑点变大
- 小成交也能推动价格
- 假突破或假跌破概率上升

### WhaleFlowCandidate

大额主动流异常。

含义：

短时间内出现明显大额主动买入或卖出。

这类信号要和合约、现货主力监控一起看。
如果多个页面同方向，可信度更高。

## 8. Discord 状态怎么理解

### 已发送 / sent

说明这条信号已经成功推送到 Discord。

通常是：

- S
- Critical
- 或符合更严格 gate 的 High

### 未配置 / webhook_missing

说明没有配置 Discord webhook。

页面能显示信号，但不会发到 Discord。

### dry_run

说明当前是模拟发送模式。

系统会判断“本来应该推”，但不会真实发送。

### data_quality_display_only

说明信号数据质量不够。

即使风险分高，也只在前端显示，不推 Discord。

### medium_or_low_display_only

说明这是 Medium / Low。

只展示，不推送。

### high_without_discord_confirmation

说明是 High，但不满足更严格推送条件。

通常是：

- 分数还不够高
- 或没有多交易所确认

### cooldown

说明同方向、同类型、同币种短时间内已经推过。

系统为了防止刷屏，会跳过重复推送。

如果信号升级到更高等级，或者方向反转，可能会再次推送。

### duplicate

说明这条信号已经处理过。

系统不会重复推同一条。

### webhook_invalid / 403

说明 webhook 地址或权限有问题。

需要检查 Discord webhook 是否还有效。

### 429

说明 Discord 限流。

通常等一段时间会恢复。

## 9. 页面状态提示怎么看

### connected

数据源连接正常。

### disconnected

数据源断开。

如果只有一个交易所断开，系统可能降级但不完全停止。

### reconnecting

正在自动重连。

通常不用手动处理，先观察一会儿。

### degraded

降级状态。

代表至少有一个数据源异常，或者数据质量不完整。

看到 degraded 时，信号仍可参考，但要降低置信度。

### healthy

健康状态。

说明主要数据源近期都有数据。

### calm

平静。

说明当前没有明显异常。

### watch

观察。

说明出现 Medium 或轻度异常，需要留意但不强。

### active

活跃。

说明出现 High 级别异常。

### strong

强异动。

说明出现 Critical 或 S 级别异常。

## 10. 推荐看盘顺序

### 第一步：看监控首页

确认：

- Discord 是否正常
- 高风险数量是否突然增加
- 当前展示是否有 High / Critical

### 第二步：看合约监控

确认：

- 合约主动买卖方向
- 是否出现 S / Critical
- Buy / Sell 比例是否明显倾斜
- 是否多个交易所同向

### 第三步：看现货监控

确认：

- 现货是否同方向配合
- Binance / Coinbase 是否都在线
- 现货是否出现主动买入或主动卖出爆发

### 第四步：回到异常信号

看具体盘口异常：

- spoofing
- liquidity thinness
- whale flow

### 第五步：打开详情 / Review

重点看：

- 方向
- Toxic Score：短线有毒订单评分
- Main Force Score：现货 + 合约主力结构评分
- 数据质量
- 最终结果
- Discord 状态
- 交易所拆分
- 价格变化

## 11. 怎么判断信号强不强

一个强信号通常同时满足：

- 等级是 S 或 Critical
- toxicScore 大于等于 85
- confidence 大于等于 70
- dataQuality 大于等于 70
- 方向明确
- 多交易所确认
- 合约和现货同方向
- 价格有响应
- 不是刚启动后的 warmup 信号

一个弱信号通常有这些特征：

- 只有 Medium
- 只有单交易所异常
- dataQuality 低
- dominance 不高
- 成交量大但价格不动
- Discord reason 是 display only

## 12. 几个典型场景

### 场景一：合约 S 级主力拉盘，现货也主动买入

解读：

这是比较强的上行动能信号。
合约和现货同时确认，说明不是单纯合约假拉的概率降低。

你应该：

- 打开合约详情
- 打开现货详情
- 看 price move 是否同步
- 看 Discord 是否已推送

### 场景二：合约主动买入很大，但现货没动

解读：

可能是合约端短线冲击，也可能是假突破。

你应该：

- 降低置信度
- 看是否出现上方压制
- 等现货确认

### 场景三：主动卖出很大，但价格跌不动

解读：

可能是下方吸收。

你应该：

- 看是否多次出现 Absorption
- 看后续是否从卖压转为买入
- 不要只因为卖出量大就判断继续下跌

### 场景四：页面显示 Discord 未配置

解读：

前端可以正常看信号，但不会发到 Discord。

你应该：

- 检查 Discord 设置页
- 检查 webhook 是否配置
- 发送测试消息

### 场景五：信号很多但 Discord 没响

解读：

大概率是 Medium / High 未过 gate，或 cooldown 生效。

你应该：

- 看每条信号的 Discord 状态
- 看 reason 是不是 `medium_or_low_display_only`、`cooldown` 或 `data_quality_display_only`

## 13. 不要误解的地方

### 信号不是买卖建议

系统只提示异常。
它不判断你该开仓、平仓、止盈或止损。

### S 级不是百分百正确

S 级只是说明异常强，不代表方向一定延续。
强异动也可能是清算、诱导或短时冲击。

### Medium 不是没用

Medium 不推 Discord，但适合观察。
连续多个 Medium 同方向出现，可能会升级。

### Discord 没推不代表页面没信号

前端会显示更多候选。
Discord 只推高置信度信号，避免刷屏。

### 单交易所异常要谨慎

如果只有 Binance 或 Coinbase 一个交易所异常，可信度比多交易所同向低。

## 14. 每天使用建议

开盘或开始盯盘时：

1. 看 `监控首页` 是否正常。
2. 看 Discord 状态是否正常。
3. 看 `BTC/ETH 合约监控` 是否 healthy。
4. 看 `BTC/ETH 现货监控` 是否 healthy。
5. 如果出现 S / Critical，先看详情，不要只看标题。
6. 如果 Discord 没响但页面有信号，看 Discord reason。
7. 复盘时去 `信号历史` 看当时的分数、方向和后续走势。

最简单的判断口诀：

```text
先看等级，再看方向；
先看原因，再看分数；
先看数据质量，再看是否多交易所确认；
合约现货同向，信号更强；
只前端显示不推送，通常说明证据还不够强。
```
