# 有毒订单监控用户使用指南

这份指南面向每天打开页面看信号的人。
它不讲部署，只讲这个软件怎么看、信号怎么解读、出现什么提示代表什么。

## 1. 先记住一句话

本系统只做提醒和观察，不自动交易、不下单、不拦截。

页面里所有 `Candidate` 都表示“候选信号”。它的意思是：系统发现了异常迹象，建议你人工复核。
它不是确定结论，也不是买卖指令。

当前系统已经拆成两套评分：

- 短线有毒订单评分：告诉你这里短线危险不危险。
- 现货 + 合约主力结构评分：告诉你这里是不是发生过真正有意义的主力行为。

这两个系统要分开看。短线危险不等于中长线主力介入；极端行情冲击也不一定等于主力建仓。

## 2. 最推荐的看盘顺序

1. 先看 `dataQuality`，低于 70 时先降权，不要急着下结论。
2. 再看短线有毒订单评分，判断当前位置有没有扫盘、插针、假突破风险。
3. 再看现货 + 合约主力结构评分，判断是不是主力建多、建空、吸筹、派发、吸收或压制。
4. 看 `structureBias`，判断结构偏多、偏空，还是还没展开。
5. 看 `regimeType`，判断这次异常到底是什么类型。
6. 最后看 Discord 状态，确认信号有没有通过推送 gate。

一句话：

```text
有毒订单评分负责告诉你：这里短线危险不危险。
现货 + 合约主力结构评分负责告诉你：这里是不是发生过真正有意义的主力行为。
```

## 3. 当前有毒订单判断逻辑

这部分原来展示在监控首页。现在统一放到使用指南里，首页只保留状态摘要，避免占用看盘空间。

### 安全边界

系统只做盘口 / L2 / 成交异常提醒，不执行下单、拦截、封禁或资金操作。
所有结果都是 `Candidate only`，意思是“候选信号，需要人工复核”，不是执法结论，也不是买卖指令。

### 候选生成

候选信号来自公开盘口和成交数据。

触发线索包括：

- L2 撤单 / 挂单异常
- trade imbalance
- depth withdrawal
- spread widening
- VPIN-lite
- 主动扫盘
- 盘口突然变薄
- 成交后价格反向伤害

这些线索会生成现货 TOF 候选和短线有毒订单候选。

### 合约增强

合约侧会把下面这些指标作为增强确认：

- OI
- Funding
- Liquidation pressure
- Aggressive order flow
- CWM 主力合约成交流

系统会把现货候选和合约候选按 `symbol` 合并，但不会因为单独合约爆量就直接确认主力。

### 双评分系统

当前不是一个总分系统，而是两套分开的评分：

- 短线有毒订单评分 `toxicScore`：使用 `1s / 5s / 15s / 60s`，判断短线扫盘、插针、假突破风险。
- 现货 + 合约主力结构评分 `mainForceScore`：使用现货、合约、OI、清算、Funding、Basis、价格响应和跨市场确认，判断是否出现主力结构。

现货 `spotScore` 主要按五项计算：

- CVD
- 成交量异常
- 吸收
- 盘口变化
- 价格响应

合约 `contractScore` 主要按六项计算：

- CWM 主动流
- OI 冲击
- 清算环境
- Funding 拥挤
- Basis
- enabled 交易所确认

### 跨市场确认

`crossConfirmScore` 的核心是判断现货和合约是不是互相确认。

计算口径：

```text
crossConfirmScore =
0.40 * 现货合约方向一致
+ 0.25 * 多窗口一致
+ 0.20 * 价格响应一致
+ 0.15 * 数据源覆盖
```

`SourceCoverage` 只按 enabled source 算。
例如 OKX 已关闭时，它不会参与覆盖率，也不会降低数据质量。

### 主力确认

`mainForceConfirmed = true` 需要同时满足：

```text
mainForceScore >= 75
confidence >= 70
dataQuality >= 70
7 个确认条件里至少命中 3 个
```

确认条件包括：

- `spotScore >= 60`
- `contractScore >= 70`
- `crossConfirmScore >= 60`
- OI 与方向一致
- 价格响应或吸收结构明确
- 清算不是主要驱动
- 至少两个时间窗口方向一致

前端会显示已确认、待确认和命中情况。没有达到这些条件时，即使合约成交量很大，也只应理解为“合约冲击”或“观察信号”。

### 极端行情与结构分类

`extremeImpactConfirmed` 只表示冲击很剧烈，不等于主力确认。

`regimeType` 会把结构归类成：

- 主力建多
- 主力建空
- 现货吸筹
- 现货派发
- 空头挤压
- 多头踩踏
- 下方吸收
- 上方压制
- 高换手震荡
- 合约冲击

如果清算量很大、OI 快速下降、价格单边滑动，系统会提高 `extremeImpactScore`，但会降低 `mainForceScore`，避免把清算瀑布误报成主力建仓。

### 方向与置信度

`structureBias` 单独表示方向，范围是 `-100` 到 `+100`：

- 正数：偏多结构
- 负数：偏空结构
- 接近 0：中性、吸收、压制或尚未展开

`mainForceScore` 表示主力结构强度，不直接表示方向。

`confidence` 和 `dataQuality` 也要分开看：

- `dataQuality` 看数据健康，例如交易所在线、延迟、warmup、enabled source 覆盖。
- `confidence` 还要看 sourceCoverage、多窗口一致性和 signalAgreement。

### 交易所确认

当前合约监控只按 enabled 交易所计算。

OKX 关闭时：

- 不参与总成交量
- 不参与数据质量扣分
- 不参与多平台确认
- 不影响 Discord gate

当前口径里，Binance 是主流动性源，Bitfinex 是确认源。
只有 Binance + Bitfinex 同向确认时，才给 S 级空间。

### 短线衰减

短线 `toxicScore` 使用 `Calm / Watch / High / Critical / S` 等级。

它是短生命周期信号：

```text
halfLifeSec 通常 30 - 45 秒
max TTL 约 3 - 5 分钟
```

如果后续没有持续异常，`decayedScore` 会快速下降。

### 推送边界

短线有毒订单 Discord gate 通常要求：

```text
toxicScore >= 85
confidence >= 70
dataQuality >= 70
cooldown >= 60s
```

文案会明确提示“不代表中长线趋势”。

CWM 大行情提醒保留独立 gate、独立冷却和 dry-run 观察。
Medium / Low 只在前端展示，不自动推送 Discord。

## 4. 首页两张评分卡怎么看

### 短线有毒订单评分

这张卡看秒级到分钟级风险。

它关注：

- 主动扫盘
- 插针风险
- 假突破风险
- 盘口突然变薄
- spoofing / 虚假挂单
- 流动性缺口

示例：

```text
短线有毒订单评分
toxicScore：87
短线压力：偏空 -72
类型：主动卖出扫盘
有效期：48 秒
用途：短线风险
```

字段解释：

- `toxicScore`：短线毒性强度，范围 0 到 100，越高说明短线越危险。
- `shortPressure`：短线方向压力，正数偏多，负数偏空。
- `toxicType`：短线异常类型，例如主动扫盘、流动性缺口、spoofing。
- `ttlSec` / 有效期：短线信号会快速衰减，过期后参考价值下降。
- `confidence`：判断可靠度。
- `dataQuality`：数据质量。低于 70 时，不建议当作强信号。

短线等级：

```text
0 - 39     Calm      无明显短线毒性
40 - 59    Watch     有轻微异常
60 - 74    High      明显短线风险
75 - 89    Critical  强短线风险
90 - 100   S         极强短线毒性
```

短线 Discord 推送条件：

```text
toxicScore >= 85
confidence >= 70
dataQuality >= 70
同方向同类型冷却约 60 秒
```

看到短线 Discord 时，应该理解为：

```text
当前位置短线可能扫盘 / 插针 / 假突破。
这不代表中长线趋势已经改变。
```

### 现货 + 合约主力结构评分

这张卡看分钟级到小时级结构。

它关注：

- 现货主动买入或卖出
- 合约主动买入或卖出
- OI 是否同向变化
- 清算是不是主要驱动
- 价格响应是否健康
- 多个时间窗口是否一致
- Binance 与 Bitfinex 等启用交易所是否同向

示例：

```text
现货 + 合约主力结构评分
mainForceScore：84
structureBias：偏多 +62
extremeImpactScore：58
类型：主力建多
主力确认：是
用途：中长线结构
```

字段解释：

- `mainForceScore`：像不像真实主力行为，范围 0 到 100。
- `structureBias`：结构方向，范围 -100 到 +100。正数偏多，负数偏空。
- `extremeImpactScore`：极端行情冲击强度。高分可能是主力冲击，也可能是清算瀑布。
- `regimeType`：系统判断这里发生了什么。
- `mainForceConfirmed`：是否达到主力确认条件。
- `extremeImpactConfirmed`：是否达到极端行情冲击条件。
- `liquidationDriven`：是否更像清算推动。
- `confidence`：这次结构判断是否可靠。
- `dataQuality`：数据源是否健康。

中长线等级：

```text
0 - 39     Calm       无明显主力结构
40 - 59    Watch      有局部异动，但确认不足
60 - 74    Confirmed  有结构性异动，值得关注
75 - 89    Major      高概率主力行为
90 - 100   Extreme    极强主力行为或极端结构变化
```

主力确认条件：

```text
mainForceScore >= 75
confidence >= 70
dataQuality >= 70
至少 3 个确认条件成立
```

确认条件包括：

- 现货评分够强
- 合约评分够强
- 现货合约同向确认
- OI 与方向一致
- 价格响应或吸收结构明确
- 不是清算主导
- 多个时间窗口方向一致

## 5. 信号卡片字段怎么看

### Symbol

币种或交易对。

常见：

- `BTC`
- `ETH`
- `BTCUSDT`
- `ETHUSDT`

### Direction

方向。

- `Ask/Sell`：偏卖出压力，可能下行。
- `Bid/Buy`：偏买入压力，可能上行。
- `Mixed`：方向混杂，不宜只看单边。
- `Neutral`：方向不明显。

### Toxic Score

短线有毒订单评分。

它回答：

```text
这里短线危险不危险？
```

它不回答：

```text
这里是不是主力建仓？
```

### Main Force Score

现货 + 合约主力结构评分。

它回答：

```text
这里是不是发生过有意义的主力行为？
```

它不回答：

```text
下一秒会不会插针？
```

### Structure Bias

结构方向。

- `+100`：极强偏多结构
- `0`：中性、不明确、吸收或压制尚未展开
- `-100`：极强偏空结构

示例：

```text
mainForceScore = 82
structureBias = +64
regimeType = main_force_long_build
```

含义：高概率主力建多，结构偏多。

另一个例子：

```text
mainForceScore = 78
structureBias = +12
regimeType = downside_absorption
```

含义：有明显下方承接，但方向还没有完全展开。

### Extreme Impact

极端行情冲击评分。

它高的时候，说明行情冲击很强，但不一定是主力行为。

例如：

```text
extremeImpactScore = 91
mainForceScore = 54
regimeType = long_liquidation_cascade
```

含义：这是极端下跌冲击，但暂不确认是主力建空。

### Discord Alert Status

Discord 推送状态。

常见状态：

- `sent`：已推送
- `pending`：推送中
- `failed`：推送失败
- `dry_run`：只是模拟发送，没有真实推送
- `not_configured`：Discord 未配置
- `suppressed`：没有达到推送门槛

### finalRiskScore

这是旧兼容字段。

当前版本里，不要再把 `finalRiskScore` 当作总分。
看盘时请优先看：

- `toxicScore`
- `mainForceScore`
- `structureBias`
- `extremeImpactScore`

## 6. regimeType 怎么解读

### main_force_long_build / 主力建多

现货主动买入增强，合约主动买入增强，OI 上升，价格上涨或回调不破。

理解：

```text
高概率主力偏多结构。
```

### main_force_short_build / 主力建空

现货主动卖出增强，合约主动卖出增强，OI 上升，价格下跌或反弹不过。

理解：

```text
高概率主力偏空结构。
```

### spot_accumulation / 现货吸筹

现货持续主动买入，合约没有明显追多，价格横盘或缓慢抬升，下跌时承接明显。

理解：

```text
偏中长线承接，不一定马上拉升。
```

### spot_distribution / 现货派发

现货持续主动卖出，合约仍有追多，价格涨不动，高位成交放大。

理解：

```text
上方可能有人持续卖出。
```

### contract_short_squeeze / 空头挤压

价格快速上涨，主动买入爆发，空头清算增加，OI 下降。

理解：

```text
这是极端行情，但不一定等于主力建多。
```

### long_liquidation_cascade / 多头踩踏

价格快速下跌，主动卖出爆发，多头清算增加，OI 下降。

理解：

```text
这是极端行情，但不一定等于主力建空。
```

### downside_absorption / 下方吸收

主动卖出很大，但价格跌不动，现货买盘承接，低点没有继续下移。

理解：

```text
下方有人接，structureBias 可以轻微偏多。
```

### upside_resistance / 上方压制

主动买入很大，但价格涨不动，现货卖盘压制，高点无法突破。

理解：

```text
上方有人压，structureBias 可以轻微偏空。
```

### range_rotation / 高换手震荡

成交量大，OI 变化不明显，价格区间震荡，现货和合约方向不一致。

理解：

```text
这里更像换手，不要强行看单边。
```

### contract_flow_shock / 合约冲击

合约爆量明显，但现货不确认、OI 不确认、价格响应混乱。

理解：

```text
这是合约侧冲击，不是主力确认。
```

## 7. 合约监控信号怎么解读

合约监控看的是 BTC / ETH 永续合约主动成交流。

它主要识别：

- 主力拉盘
- 主力砸盘
- 下方吸收
- 上方压制
- 清算推动
- 合约冲击

关键字段：

- `窗口`：5s、15s、60s，窗口越长代表持续性越强。
- `成交量`：合约主动成交量。
- `净方向`：主动买入减主动卖出。
- `方向强度`：净方向占总成交量的比例。
- `主导平台`：这轮信号主要来自哪个交易所。
- `价格变化`：成交爆发后价格是否配合。
- `dataQuality`：交易所连接和数据质量。

当前 OKX 关闭时：

- OKX 不参与总成交量。
- OKX 不参与交易所数量。
- OKX 不降低 dataQuality。
- OKX disabled 不是故障。

如果只有 Binance 合约爆量，但现货、OI、价格响应没有确认，系统会尽量标成：

```text
合约冲击
```

而不是：

```text
主力确认
```

## 8. 现货监控信号怎么解读

现货监控看的是 Binance Spot 和 Coinbase Spot 的主动成交流。

它主要帮助判断：

- 现货是否真的有主动买盘
- 现货是否真的有主动卖盘
- 合约异动是否有现货配合
- Binance 和 Coinbase 是否出现明显差异

常见类型：

- `现货主动买入`：现货买盘主动性增强。
- `现货主动卖出`：现货卖盘主动性增强。
- `下方吸收`：卖出很大但价格跌不动。
- `上方压制`：买入很大但价格涨不动。

读法：

```text
合约强 + 现货强 + OI 同向 = 更可信的主力结构。
合约强 + 现货弱 = 更像合约冲击或清算推动。
现货强 + 合约弱 = 更像现货吸筹或派发，需要等合约确认。
```

## 9. Discord 状态怎么理解

当前系统有两条 Discord 链路：

- 短线有毒订单 Discord
- 主力结构 / 极端行情 Discord

### 短线有毒订单 Discord

触发条件：

```text
toxicScore >= 85
confidence >= 70
dataQuality >= 70
冷却约 60 秒
```

文案关键词：

- 短线有毒订单
- 短线风险
- 可能扫盘 / 插针 / 假突破
- 不代表中长线趋势

### 主力结构 / 极端行情 Discord

触发条件：

```text
mainForceScore >= 80
confidence >= 70
dataQuality >= 70
```

或者：

```text
extremeImpactScore >= 85
dataQuality >= 70
```

默认冷却通常是 15 到 30 分钟。

文案关键词：

- 主力结构异动
- 主力建多 / 主力建空
- 现货吸筹 / 现货派发
- 下方吸收 / 上方压制
- 极端行情冲击

如果 Discord 显示未配置、403、429 或 dry_run：

- `未配置`：没有配置 webhook，消息不会发送。
- `403`：Webhook 权限或地址有问题。
- `429`：Discord 限流，系统会避免刷屏。
- `dry_run`：只模拟发送，不会真的发到频道。

## 10. 常见场景怎么判断

### toxicScore 高，mainForceScore 低

含义：

```text
短线危险，但不是中长线主力确认。
```

处理：

- 注意扫盘、插针、假突破。
- 不要把它直接当作趋势反转。

### mainForceScore 高，structureBias 为正

含义：

```text
高概率主力偏多结构。
```

处理：

- 看现货是否跟随。
- 看 OI 是否上升。
- 看价格回调是否守住。

### mainForceScore 高，structureBias 为负

含义：

```text
高概率主力偏空结构。
```

处理：

- 看现货是否卖出。
- 看 OI 是否上升。
- 看反弹是否过不去。

### extremeImpactScore 高，但 mainForceConfirmed 否

含义：

```text
极端行情冲击，可能是清算瀑布，不一定是主力建仓。
```

处理：

- 不要直接追方向。
- 先看 `liquidationDriven` 是否为 true。
- 看 OI 是否快速下降。

### 主动卖出巨大，但价格不跌

含义：

```text
下方吸收。
```

处理：

- 这不是单纯偏空。
- 可能说明下方承接强。
- `structureBias` 可以轻微偏多。

### 主动买入巨大，但价格不涨

含义：

```text
上方压制。
```

处理：

- 这不是单纯偏多。
- 可能说明上方卖盘强。
- `structureBias` 可以轻微偏空。

### 合约爆量，但现货不跟

含义：

```text
合约冲击。
```

处理：

- 不要直接判定主力。
- 等现货、OI、价格响应进一步确认。

## 11. 信号历史和主力事件怎么看

信号历史用于复盘短线和中长线信号。

你可以回看：

- 当时是什么方向
- toxicScore 是多少
- mainForceScore 是多少
- structureBias 是偏多还是偏空
- Discord 有没有推送
- 后续价格有没有验证

主力事件用于回答：

```text
这里是否发生过一段持续的主力行为？
```

事件开始条件：

```text
mainForceScore >= 75
或 extremeImpactScore >= 85
```

事件结束条件：

```text
mainForceScore < 55
且 extremeImpactScore < 60
持续约 15 分钟
```

事件会记录：

- 开始时间
- 结束时间
- 峰值时间
- 峰值主力评分
- 峰值极端冲击
- 峰值结构方向
- 类型
- 主要原因

它的用途是复盘：

```text
14:35 - 15:20 主力建多，峰值评分 88。
```

## 12. 最容易误读的地方

### 误读 1：短线 toxic 高，就等于主力建仓

不对。

短线 toxic 高，只说明当前位置短线危险。
它可能是扫盘、插针、假突破，不一定是主力结构。

### 误读 2：合约爆量，就等于主力

不对。

合约爆量需要现货、OI、价格响应共同确认。
否则更可能是合约冲击或清算推动。

### 误读 3：极端行情高，就一定是主力

不对。

极端行情可能来自清算瀑布。
这时 `extremeImpactScore` 会高，但 `mainForceScore` 会被降权。

### 误读 4：主动卖出就是看空

不一定。

如果主动卖出很大但价格跌不动，可能是下方吸收。

### 误读 5：主动买入就是看多

不一定。

如果主动买入很大但价格涨不动，可能是上方压制。

## 13. 日常使用建议

开盘或打开页面后，建议这样看：

1. 看顶部连接状态和 Discord 状态。
2. 看短线有毒订单评分，确认当前有没有短线危险。
3. 看现货 + 合约主力结构评分，确认是否有主力结构。
4. 看合约监控页面，确认是否有大额主动成交流。
5. 看现货监控页面，确认现货是否配合。
6. 看信号历史，复盘前面的信号是否有效。
7. 如果出现 Discord 提醒，先判断它属于短线 toxic 还是主力结构。

最重要的一点：

```text
不要用一条信号做结论。
要把短线评分、主力结构评分、现货、合约、OI、价格响应放在一起看。
```
