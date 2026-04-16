# 订单工作台主表技术方案决策

- 日期：2026-04-15
- 范围：订单审核、订单复核、订单查询，以及后续同步到工作区模版的主数据表入口

## 1. 结论

订单工作台主表采用以下技术路线：

1. 核心标准采用 `AG Grid Community 兼容优先`。
2. `Enterprise 只作为可选增强`，不把核心业务流程绑定到必须付费的 Enterprise-only 能力。
3. 页面层统一通过 `OrderWorkbenchMainGridEntry` 接入主表，避免订单审核、订单复核、订单查询各自直接接不同实现。
4. 短期不重写三套业务逻辑，先统一页面入口层；中期再逐步收敛内部实现和列能力。

## 2. 为什么这样选

当前仓库三页主表底层其实都基于 AG Grid，但接入方式不统一：

- 订单审核：页面直接接 `OrderReviewMainGrid`
- 订单复核：页面直接接 `OrderExceptionsMainGrid`
- 订单查询：页面直接接 `ErpDataGridNext`

这说明问题不在“是否使用 AG Grid”，而在“页面入口层已经分叉”。继续沿着分叉路径迭代，会让工作区模版、交互习惯和维护方式越来越不一致。

同时，仓库虽然已经安装了 `ag-grid-enterprise`，但当前没有把授权和核心能力治理做成稳定前提。结合现阶段“尚未正式有用户、预算需要保守”的约束，把关键业务体验建立在 Enterprise-only 能力上，会放大后续的成本和迁移风险。

因此，本次选择的目标不是否定 Enterprise，而是先把主链路建立在可持续、可复制、可模板化的基础上：

- Community 能满足的能力，优先走 Community 兼容实现
- Enterprise 能明显增益的能力，作为增强项按需打开
- 页面层和模版层先统一入口，降低后续维护与培训成本

## 3. 页面层统一方案

新增共享入口组件：

- `apps/erp-web/src/pages/orders/shared/OrderWorkbenchMainGridEntry.tsx`

分发规则：

- `variant: 'review'` -> `OrderReviewMainGrid`
- `variant: 'recheck'` -> `OrderExceptionsMainGrid`
- `variant: 'query'` -> `OrderQueryMainGrid`

订单查询新增轻量包装组件：

- `apps/erp-web/src/pages/orders/shared/OrderQueryMainGrid.tsx`

这样三页页面层的主表入口将统一成一个组件，后续工作区模版、诊断开关、滚动条、底栏、列行为约束，都可以先落在统一入口层或其邻近层，不需要继续在页面里分散复制。

## 4. 对 Enterprise 的使用边界

当前建议边界如下：

1. 不把“单列筛选、选择列、列宽、排序、工作台主流程交互”做成 Enterprise 授权前提。
2. 分组聚合、透视分析、复杂侧边栏等明显偏增强分析场景的能力，可以保留 Enterprise 评估空间。
3. 一旦未来正式采购 Enterprise，优先把增强能力接在统一入口层附近，而不是再让每个页面各自演化。

## 5. 本次落地内容

本次改动应同步完成以下事项：

1. 三页页面层改为统一接入 `OrderWorkbenchMainGridEntry`
2. 导出 `OrderReviewMainGridProps` 与 `OrderExceptionsMainGridProps`，供共享入口复用
3. 恢复订单审核页底部滚动条回归点
4. 保持订单复核、订单查询现有业务逻辑不重写
5. 决策文档独立落盘，避免继续堆叠到历史乱码文档中

## 6. 后续演进建议

下一阶段建议按以下顺序推进：

1. 继续把工作区模版中的主表约束收拢到统一入口层附近
2. 把订单查询、订单审核、订单复核的列配置描述逐步标准化
3. 再评估是否将审核/复核的部分列筛选能力改写成完全 Community 兼容实现
4. 等真实用户场景稳定后，再决定是否采购并深度启用 Enterprise 增强能力
