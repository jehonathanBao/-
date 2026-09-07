# BTC 合约主力视角审计

审计日期：2026-09-07。结论：**REQUEST CHANGES——能识别公开成交压力，但尚不足以可靠识别大资金完整的建仓、吸收和退出过程。**

## 执行摘要

- 本次没有修改业务代码、服务器配置、通知开关，也没有部署、发 Discord 或执行交易。仅增加本审计报告。
- 以服务器运行中的后端 `3fa60d097d2fb218fa4d2184b2cd201eb9c53b88` 为准。服务器工作树及前端为 `2561d2433adc810c7778a02b8ca10a9e70e32b01`；本地与运行后端之间的 `src`、`config`、Cargo 文件没有 Git 差异。
- 实时 BTC summary 显示 `binance_only`，configured / eligible / active 合约来源均只有 Binance；健康状态 healthy。采样时合约数据质量 95、现货 92。这些数字不代表主力识别准确率。
- 最大问题不是阈值不够低，而是：**单交易所模式与行为分类门槛冲突、吸收评级存在断点、统一评级后仍有第二套通知准入条件、公共成交与账户行为的边界不够一致。**
- 采用交易系统审计和项目业务规则规范，逐段核对采集、触发、归因、评级、通知与展示。所有检查由同一审计流程完成，未调用子代理。

## 从大资金交易方式看覆盖能力

| 如果我是大资金 | 程序现在能看到什么 | 不能据此确认什么 |
|---|---|---|
| 集中主动买入或卖出 | 5/15/60 秒成交量、净主动流、价格变化、异常程度 | 这些成交是否来自同一账户，是否全部属于开仓 |
| 持续拆单执行 | 已有 1/5/15/60 分钟持续净流候选检测 | 低参与率执行可能不入选；候选不能发 Discord；不是母单重建 |
| 挂限价单承接卖盘 | 能看到主动卖出、价格下跌有限；全局采集器已有盘口 | 当前行为判定没有使用补单/成交消耗证据；单来源门槛还会阻断吸收分类 |
| 在不同市场对冲 | 当前主要看到 Binance 一侧及其现货/OI 背景 | 全账户净风险、跨平台腿、期权或场外对冲 |
| 在大规模清算期间成交 | 能看到清算样本和异常价格/成交形态 | 全量清算额、剩余成交是否自愿建仓、是否有人故意猎取止损 |
| 与大量其他交易者同时成交 | 能看到总体资金流 | 不能区分一个大账户和多个同向小账户 |

Binance 官方接口把同价、同主动方向的成交按约 100ms 聚合；返回成交编号、价格、数量和主动方向等，不提供公开账户身份。普通盘口也是价位级信息，不能从中直接恢复隐藏母单或账户持仓。[官方成交接口](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/ws-streams/market#aggregate-trade-streams)、[官方盘口接口](https://developers.binance.info/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/ws-streams/public)

## 风险地图

| 环节 | 核心文件 | 结论 |
|---|---|---|
| 数据采集 | `src/connectors/binance.rs`、`src/normalizers/trade.rs` | 成交和盘口已分别接入；不等于盘口进入主力分类 |
| 快速/持续检测 | `detector.rs`、`sustained_flow.rs` | 有实际检测，但漏检边界和通知边界不同 |
| 行为归因 | `classification.rs`、`behavior_assessment.rs`、`trajectory.rs` | 有候选/缺证据机制，仍存在来源门槛与语义冲突 |
| 统一等级 | `impact_grade.rs` | 已统一主评级，低波动吸收分支需要修正 |
| 通知 | `discord_notifier.rs`、`src/app.rs` | 候选硬屏蔽及旧分数/严重度构成额外准入 |
| 评价 | `outcome_calibration.rs` 等 | 价格后验评价不能替代识别主力行为的召回率 |

下文 P1 表示会明显影响用户目标的正确性问题；P2 表示能力或证据表达应改进。没有发现需要在本次只读审计中立刻执行停机的 P0 事项；这不等于完成全项目安全审计。

## P1-1：Binance 单来源运行，吸收/强主力分类却仍要求多交易所

- 证据位置：[classification.rs:62](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/classification.rs:62)、[config.rs:1511](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/config.rs:1511)、[detector.rs:1157](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/detector.rs:1157)。
- 服务器配置未设置两项 `require_multi_exchange`，对应环境覆盖也未设置，因此两者使用默认值 true；检测器在来源少于两个时，`multi_exchange_confirmed` 必为 false。
- 结果：经典“主动卖出很多但跌不动”的低效率成交，即使满足成交额、方向占比、质量条件，也不能进入 V2 的下方吸收分支；强主力推升/砸盘分支同样受阻。注意：这不表示统一 A/S 评级必须两家交易所；评级层已经支持 Binance 单来源，问题在另一层。
- **实盘样本**：2026-09-07 北京时间 19:03:59.999 的 BTC 持续流记录，`signalType=downside_absorption`、`direction=absorption`，却同时有 `structureInterpretation=active_sell_pressure`、`displaySignalType=主动卖压`，价格变化 -0.0386%。它说明不同层在表达不同解释，不证明某账户真实吸筹。样本来自只读 SQLite 查询。
- 修改：按已配置来源模式设置证据要求。单 Binance 可以产生“单市场吸收候选”，多交易所作为增强证据；缺失已配置来源仍应降级，不能把缺源伪装成单源正常。明确分离主动成交方向、被动承接方向、行为假设，消除旧/新字段冲突。
- 不建议只把两个开关改成 false 就上线“确认主力吸筹”：应先接入已有盘口的消耗、补单、价位保持与数据连续性证据；没有这些证据只能输出候选。
- 必须测试：单 Binance 卖流受阻能产生吸收候选；同样卖流造成持续下跌不能判为吸收；买卖对称；已配置第二平台断流时不能默默当作完整证据。
- 置信度：高，配置、代码和服务器记录交叉支持。

## P1-2：吸收评级的 0.10% 分界造成不连续降级

- 证据位置：[impact_grade.rs:455](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/impact_grade.rs:455)、[impact_grade.rs:508](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/impact_grade.rs:508)、[impact_grade.rs:548](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/impact_grade.rs:548)。
- 当前吸收豁免为 `flow_anomaly_score >= 80 && price_move_pct < 0.10`。不满足后，重新要求 A 的价格变化至少 0.5%，B 至少 0.15%。
- 固定其他证据：BTC 总量 8000、净量 6000、名义额 5 亿美元、质量 90、历史分位 99.95、robust z=6、基线 20000 个、有效来源 Binance；不提供 S 所需硬证据。按当前函数条件代入：

| 绝对价格变化 | 吸收豁免 | 当前分支给出的等级 |
|---|---|---|
| 0.05% | 是 | A |
| 0.10% | 否 | C |
| 0.12% | 否 | C |
| 0.20% | 否 | B |
| 0.50% | 否 | A |

- 这是**当前分支的离线条件代入**，不是重放真实历史事件，也不是执行 Rust 单元测试所得。它证明算法存在该非单调边界，不代表真实行情必然沿此顺序更新。
- 另一个不足：吸收豁免仅依赖异常分数与低涨跌幅，没有直接的被动成交/补单证据；不能把“高换手且价格不动”自动当成真实主力吸收。
- 修改：趋势型和吸收型提供不同证据，但映射到同一套 S/A/B/C；吸收证据使用按市场波动归一化的价格效率、持续性和盘口承接，而不是单个固定涨跌幅断点。事件重要性不应因为轻微越过该边界而凭空从 A 降至 C；行为假设改变则应明确显示“失效/转换”。
- 必须测试：0.099/0.100/0.101% 边界；同等证据下的等级连续性；被动承接成立与普通高换手横盘的负例；不能通过只增加重复成交或重叠窗口升到 A/S。
- 置信度：高，分支逻辑确定；是否造成过线上等级下降尚未做历史逐事件统计。

## P1-3：统一评级尚未贯通通知决策

- 证据位置：[discord_notifier.rs:238](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/discord_notifier.rs:238)、[impact_grade.rs:224](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/impact_grade.rs:224)、[app.rs:1999](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/app.rs:1999)。
- V3 通知在检查统一评级后，仍要求旧 `score >= 80`、旧 severity 至少 High。评级映射并没有同步改写这两个字段。因此“统一等级够了”并不等于通知会通过。
- 所有 `sustained_flow` 候选还在函数开头被拒绝。这是当前明确的安全/产品策略，不是 Discord 故障。上述 19:03 样本确实记录 `sustained_candidate_display_only`、未发送；该样本本身只是 C，不是已证实的 A/S 漏发案例。
- 对用户目标的影响：大资金以慢速方式交易，程序即使识别出持续流，也无法通过该路径通知用户；高速事件还可能被另一套旧评分挡住。
- 修改：形成一个通知决策结果，明确“统一等级、行为候选状态、数据质量、冷却/去重、运行安全开关”各自作用。旧评分若仍保留，必须明确其目的及用户可见拒绝原因，不能隐含作为第二套等级门槛。
- 慢流先做影子记录和对照回放；达到认可证据条件后才允许升级到可通知事件。保留候选禁发、重启不补发、干跑、冷却和去重保护，启用新通知需另行批准。
- 必须测试：相同 canonical A/S 在旧分数不同的情况下给出明确且一致的策略结果；合格慢流升级后只发一次；未确认候选、历史补算、断流、重复更新不能发。
- 置信度：高，代码路径明确；没有量化过去 A/S 被旧分数拦截的次数。

## P2-1：慢速拆单检测已有，但高门槛净流不等于拆单识别

- 证据位置：[sustained_flow.rs:132](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/sustained_flow.rs:132)、[sustained_flow.rs:187](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/sustained_flow.rs:187)、[sustained_flow.rs:284](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/sustained_flow.rs:284)。
- 入选需历史至少 60 个有效桶、覆盖率至少 90%、至少 80% 桶同向、净流占总成交至少 15%、异常分位至少 95。历史范围约 3 小时；一次选择最长满足条件的窗口。
- 因此正常大资金执行若占市场成交比例较低，或被其他主体反向成交抵消，即使长时间累计成交很大也可能不入选。现有检测已比只看大单好，但不能声称实现了所有拆单/母单识别。
- 事件 ID 依赖窗口长度和时间取整，跨边界或从短窗切到长窗时会换身份；它不是持续跟踪同一真实主体的证明。
- 修改：保留现有高异常检测；增加低强度但持续的净流累积证据、交易节奏、价格效率，采用按时段/波动状态的基线；使用带失效规则的行为过程跟踪，按独立原始时间桶计算新增证据和唯一成交量。窗口是证据，不能相加冒充资金规模。
- 测试：低参与率持续执行、正常平衡成交、同方向人群共振、窗口重叠、跨时间边界、来源断流与重启。没有账户标签时只能评估行为候选，不得宣称账户级召回。
- 置信度：高，漏检条件由代码直接确定；真实漏检比例未知。

## P2-2：盘口已采集，但主力归因仍缺少被动执行证据

- 证据位置：[binance.rs:23](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/connectors/binance.rs:23)、[classification.rs:85](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/classification.rs:85)、[behavior_assessment.rs:421](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/behavior_assessment.rs:421)。
- 全局连接器已有 `depth20@100ms`。但当前主力吸收分类主要使用主动方向、成交额、价格效率和质量，行为证据列表没有价位消耗/补单的直接证据。不能简单说“项目没盘口模块”，问题是核心判断尚未融合这些证据。
- 修改：复用现有盘口能力，形成同时间、同价位的“成交消耗后仍有承接”“重复补量”“价格未穿透”等证据；区分成交消耗、撤单、移出前 20 档、快照丢失。需要完整价位生命周期时，再评估增量盘口及序号连续性维护。
- 不能承诺仅凭公开盘口识别某账户冰山单、确定撤单意图，或把撤单直接标为诱骗。
- 测试：真实成交消耗与纯撤单的差异；盘口断流/乱序不能产生补单证据；跨快照档位变化不得误记为撤单；正常做市补量负例。
- 置信度：高，所审行为路径的输入可直接核对；未穷尽项目所有非 BTC 主力模块。

## P2-3：OI、清算和轨迹标签仍容易被读成账户意图

- 证据位置：[classification.rs:660](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/classification.rs:660)、[behavior_assessment.rs:296](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/behavior_assessment.rs:296)、[trajectory.rs:102](D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/trajectory.rs:102)、[ContractWhaleMonitor.jsx:5828](D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/components/ContractWhaleMonitor.jsx:5828)。
- OI 增加与主动买入同向，可支持“偏多新增仓位活动”假设，但不证明这批主动买单全部是某个主力开多；每笔合约交易都有对手方，还可能有其他账户同时开平仓。
- 服务器已明确标识清算流为 sampled，不是此前的“全市场全量”口径；这一修复应保留。Binance 官方流只推送每个交易对约 1000ms 内的最新清算订单，低样本占比不能证明实际清算占比低。[官方清算流说明](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/ws-streams/market#liquidation-order-streams)
- 轨迹层仍将 `liquidation_suspected` 映射为 `stop_hunt`，部分路径生成 `stop_hunting` / `liquidity_manipulation`；虽然说明文字已有“不能确认意图”的限制，前端短标签仍是“扫损 / 清算猎取”“流动性操控”。此外，推断轨迹时把 stop_hunt 计入卖方压力，未在该动作类型内区分空头挤压和多头清算。
- 修改：区分观测事实、行为候选和意图不可归因；将这些短标签改为“清算/扫损形态”“混合方向流”；分别保留多头清算和空头回补的方向。清算证据须带健康状态、采样语义和同窗口同来源分母。
- 测试：纯空头清算不能变成卖方分布/主动猎损；普通多空换手不称操控；有清算正样本、健康但未见样本、断流未知三种情况分别处理；现货和合约反向应允许对冲候选而非直接认定看空。
- 置信度：高，规则和展示标签可见；未断言已有真实事件造成交易损失。

## 优化任务卡与顺序

### 第一阶段：修正解释和等级一致性

范围：P1-1、P1-2、P1-3 的决策统一，以及 P2-3 的过强意图标签。复用现有评级和数据，不引入新评级系统。

验收：同一事件页面/历史/通知使用同一个 S/A/B/C；主动成交方向与被动承接方向分别表达；所有拒绝原因可追踪；0.10% 边界不会形成无依据的等级断崖；安全通知开关保持原状。

### 第二阶段：增加被动执行和低强度持续执行证据

范围：复用盘口，增加行为过程跟踪；把 1/5/15/60 分钟作为独立证据窗口，避免重复累计。跨交易所数据先作为可选增强，不因没有第二平台而永远禁止单市场候选。

验收：能在低价格冲击情况下发现“疑似持续承接”，也能拒绝平衡换手、普通做市和断流伪迹；输出来源、方向、可观测唯一成交量和缺失证据，不输出虚构主力账户规模。

### 第三阶段：用回放决定是否开放新通知

建立明确标注的合成执行场景和人工复核的历史案例，做时间外验证。先在影子模式比较旧/新结果，再申请开放满足条件的新通知路径。

验收指标：行为候选精度、场景召回、首次发现延迟、方向误判率、重复告警率、缺失数据误报率。5 分钟后涨跌只评价价格后续反应，不能当作“主力身份识别正确”的真值。

## 必须补充的验收场景

- [ ] 集中主动买卖均能识别；不把卖出平多与开空确定等同。
- [ ] 单 Binance 限价承接产生吸收候选，跌穿时失效。
- [ ] 低参与率持续拆单的检测延迟可度量；正常成交负例不过度触发。
- [ ] 被动补量、普通做市、纯撤单、盘口丢帧可区分。
- [ ] 评级 0.10% 附近边界、买卖镜像、跨波动状态均覆盖。
- [ ] 同一原始成交进入多个窗口只贡献一次唯一成交量。
- [ ] 空头清算与多头清算保持方向；采样缺失不是零。
- [ ] 现货/合约反向及跨平台反向，不直接升级为单向主力确认。
- [ ] 新通知保持干跑、候选禁发、去重、冷却、无重启历史补发。

## 本次验证记录

| 检查 | 结果 | 范围 |
|---|---|---|
| 本地和服务器版本、运行镜像来源、健康检查 | 已核对 | 后端 3fa60d0，前端/工作树 2561d24 |
| 运行后端与本地源码差异 | `git diff --name-only ... -- src config Cargo.toml Cargo.lock` 无输出 | 未把前端版本误当后端版本 |
| BTC summary | healthy、Binance 单来源 | 瞬时状态，不代表历史持续可用性或识别精度 |
| rating-health 中清算采集健康 | BTC/ETH healthy，明确 sampled | 未使用它推导全市场清算总额 |
| 服务器配置和环境白名单 | 两项多来源分类覆盖未设置，使用 true 默认值 | 没有输出任何密钥或完整环境变量 |
| SQLite `mode=ro` + `query_only` | 读到 19:03:59.999 方向解释冲突样本 | 仅小范围抽样，不是全历史统计 |
| 评级条件代入 | A → C → B → A 的边界可成立 | 分支条件推演，非 Rust 测试、非真实收益回放 |
| 完整 Cargo 测试、行为回放 | 本次未运行 | 未修改业务代码；不引用旧测试数量作为本次验证 |

少量初始本地查询因 PowerShell 路径写法、旧文件名或 JSON 嵌套字段假设不符失败，随后更正。分类字段实际为扁平序列化，最终服务器样本使用正确字段读取；早期空字段结果没有作为缺失证据结论。初始旧版 Binance 文档地址只返回导航页，最终使用能读取正文的官方新目录。

## 假设、残余风险与最终门槛

- 没有获得任何真实主力账户/母单标签，因此不能给出可信的“能抓住百分之多少主力”的数字。
- 本次没有核验用户早前提到的某日 10% 拉升、历史清算纪录；不能把本次发现直接宣称为当日评级的唯一原因。
- 所审目标是公共行情的只读识别与通知，不是交易策略盈利保证，也没有审计全部自动执行模块。
- Capital Risk：本次无资金操作；对整个项目资金安全未作结论。
- Correctness：REQUEST CHANGES，上述来源门槛和评级/解释问题需修复。
- Security：本次遵守只读和凭据隔离边界；非完整安全审计。
- Architecture：可在现有数据、事件、统一评级结构上增量修改，无需再起一套主力系统。
- Tests：NEEDS VERIFICATION，新增行为能力应通过以上回放场景后才验收。
- 最终建议：保留当前系统作为成交压力监控；先完成第一阶段，再把“发现主力完整交易行为”作为有证据约束的升级目标。
