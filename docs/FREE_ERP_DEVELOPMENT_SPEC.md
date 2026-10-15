# 易格免费 ERP 具体实现和性能验收规格

> 更新日期：2026-10-04
> 用途：供 Grok 领取任务、接续现有代码和交付验收证据。
> 范围：[100项功能清单](FREE_ERP_CHECKLIST.md)的配套实施规格，细化40个关键实施单元、28个页面宿主、14条业务验收剧本和22组性能任务；功能优先级、免费与收费边界沿用原清单。
> 状态：开发规格。本文的函数名、接口和性能数值分别注明现有或规划；文档检查通过不代表功能、压测或上线通过。

同一个企业可以采购转售，也可以自产销售。免费版保留一个业务登录账号，允许同账号多端在线；席位数与会话数分别控制。当前源码默认有效会话上限为8，这是一项可配置安全限制，不能解释为8个免费用户。制造与贸易共用商品、单位、仓库、库存、成本和往来事实，全部新页面使用共享 Rust egui 与自研 DataGrid。

| 阅读目的 | 入口 |
| --- | --- |
| 核对所有功能、免费范围和后续收费候选 | [原100项功能卡片](FREE_ERP_CHECKLIST.md#十八-逐项功能和文件落点) |
| 查具体函数、字段、事务和失败后的事实 | [40个实施单元](#二-具体实施单元) |
| 查采购和制造动作怎样进入同一事务 | [业务提交顺序](#三-关键动作的事务顺序) |
| 构造性能负载和判断是否达标 | [22组性能任务](#四-性能验收任务) |
| 接续命令原结果、丢失响应、状态权威与具体负载 | [8个实施任务、15个样本和8组性能要求](#十六-命令原结果和丢失响应逐动作规格) |
| 实现制造整单、真实任务执行和多端单据冲突 | [20个细化任务、16组样本和8组专属性能要求](#十七-制造整单任务执行和多端单据开发细目) |
| 逐功能定位页面、代码、验收和性能；补真实账号接线 | [100行交付矩阵、10账号任务、12样本及6组负载](#十八-逐功能交付定位和真实账号专项) |
| 确定当前该交付什么以及如何结项 | [批次和交付记录](#五-开发批次和交付记录) |
| 确定菜单入口、页面布局和具体 UI 文件 | [28个页面宿主](#六-正式页面的入口和实现位置) |
| 把验收方向转成可逐步执行的业务样本 | [14条验收剧本](#八-可直接执行的业务验收剧本) |
| 拆分耗时、控制资源和处理超限 | [性能埋点和超限处理](#九-性能埋点和资源超限的实施规则) |
| 对照库存API、字段精度、报表宿主和当前真实测试 | [库存读取具体接线](#库存读取与聚合的实际接线) |
| 接续后台库存CSV、权限、快照、回执与文件交付 | [库存后台CSV实际接线](#库存后台-csv-的实际接线) |
| 接续旧组合来源、原父项数量、不可变组件和历史拒绝 | [组合根请求和冻结组件](#组合根请求和冻结组件) |
| 接续正式订单分配、发货、释放和并发验收 | [正式订单来源分配与履约](#正式订单来源分配与履约) |
| 补批次入库、调整、制造领退料和真实流水追溯 | [批次库存逐动作实施规格](#十一-批次库存逐动作实施规格) |
| 实现完工待检、质量放行、来源核对和原命令重试 | [生产收货放行逐动作规格](#十二-生产收货放行逐动作规格) |
| 实现三种检验来源、完整冻结标准及质量审核证明 | [质量来源与冻结标准逐动作规格](#十三-质量来源与冻结标准逐动作规格) |
| 接续实测、复检、真实审核与授权原历史 | [实测读数与审核规格](#十四-实测读数复检和审核逐动作规格) |
| 检查贸易发货、制造领料和直接完工的完整质量闸门 | [消费者质量闸门与精确贸易库存](#十五-消费者质量闸门与精确贸易库存) |

## 一 位置和契约的读法

仓库根为 `G:/erp`。`UI` 表示 `apps/erp-ui/src/`，`API` 表示 `crates/erp-api/src/`，`DOMAIN` 表示 `crates/erp-domain/src/`，`REPO` 表示 `crates/erp-repo/src/`，`WORKER` 表示 `crates/erp-worker/src/`。标为“现”的位置须复用和核对；标为“规”的位置是建议职责落点，文件或函数可能尚未建立，不表示已经实现。同一功能有现有公开接口时沿用其兼容契约，不能另建一条只能演示的写入链。

现 `REPO/shared_contract.rs` 中的契约方法作为接续入口。新增 SQL 和事务宜放到对应叶模块；保留原入口委托，按受影响功能逐次迁移。不要为执行本文先拆完整个大文件。制造 BOM 不复用套装 BOM 的业务身份，业务算法和 UI 组件互不反向依赖。

### 公共输入和结果

以下是规划契约的语义，实际类型可复用已有模型。更新 OpenAPI 源文件 `openapi/src/` 后，使用已有打包入口生成契约；不能只修改生成文件。

| 字段或类型 | 具体规则 | 实施位置 |
| --- | --- | --- |
| `CommandContext` | 企业、认证用户、会话、授权仓库来自已验证身份；不接受客户端自报身份替代授权。业务填报的员工ID单独保存 | 规 `API/command_context.rs`；复用现 `API/auth_layers.rs` 和仓库数据范围设施 |
| `request_id` | 一次业务意图一个UUID；网络重试沿用原ID。同ID同规范化内容重放原回执，同ID不同内容409 | 现 `REPO/idempotency.rs`、`REPO/shared_contract.rs` 的命令设施；规对应域服务 |
| `expected_version` | 修订、下达、确认、冲正等锁内复核版本。版本冲突无部分业务效果，UI保留草稿 | 规各域输入类型；现有版本接口保持兼容 |
| 数量与单位 | HTTP使用十进制字符串，至多6位有效小数；领域使用现 `DOMAIN/quantity.rs::ScaledQty`。基本单位与业务单位、换算版本同时保存 | 现数量类型、单位表及精确库存；禁止浮点计算数量 |
| 金额、税和费率 | 人民币金额以分为定点整数；数量×价格使用宽中间值及明确舍入。人工工时与费率另定精度，保留原费率版本 | 现 `DOMAIN/trade.rs`、成本模块；规相应DTO及校验 |
| `CommandReceipt` | 命令、对象、动作、提交版本、原效果ID及原结果快照。受理202与提交完成分别表示；不能在重试时返回后来的余额冒充原结果 | 现 `REPO/inventory_movement_commands.rs` 及原库存命令回查API；其他域仍须逐入口迁移和验收 |
| 查询分页 | 企业和数据范围先约束完整查询集，再筛选、排序和分页；排序追加稳定ID。汇总使用同一查询身份和数据截止点 | 现 `REPO` 各查询；规统一查询DTO及摘要 |
| 错误结果 | 401身份失效、403范围拒绝、404按公开契约隐藏不存在或不可见对象、409版本/余量/同ID冲突、422输入不合法、429资源限流 | 现API错误设施。精确库存的 `inventory_reconciliation_required` 应保留可辨识错误码 |

查不到物料、仓库、原来源或余额时，按明确业务错误拒绝；不要创建零余额掩盖历史缺失。暂估成本、待检库存、未决命令与过期MRP均保留明确状态。

### 核心表单的具体字段

这些是待补完整接口的语义字段，沿已有OpenAPI扩展；已有字段拼写不同应映射兼容，不重复另建正式对象。请求中的对象身份均须重新查询验证归属，客户端不能决定来源余量、认证提交人或权威成本。

| 表单与输入类型 | 必须落地的字段 | 默认与输入限制 |
| --- | --- | --- |
| 采购/销售单 `TradeOrderInput` | `request_id`、`expected_version`（修订）、`partner_id`、`business_date`、`currency`、`lines[{line_id,sku_id,unit_id,quantity,unit_price,tax_rate,need_date}]` | 新建生成稳定行ID；必填数量>0，价格/税精度按货币规则；确认后的修改受已执行量限制 |
| 工单 `WorkOrderInput` | 产品、计划精确量、来源计划/销售行、授权仓库、计划起止、BOM/路线选择政策、原命令和版本 | DRAFT不扣库存；下达由服务器确定有效版本并冻结，缺少有效资料拒绝 |
| 材料批量 `MaterialBatchInput` | 原命令、工单版本、每行稳定ID、仓库、SKU、可选批次、动作、数量、原领料ID、原因 | ISSUE/SUPPLEMENT创建新来源；RETURN/CONSUME必须引用原来源；默认整单原子，无隐含部分成功 |
| 报工 `OperationReportInput` | 工单、冻结工序、作业员工、开始/结束或明确工时、合格/不良/返工数量、原因、原命令/版本 | 工时及数量不得为负；认证提交人来自身份；冲正引用原报工，累计受前序和本序余量约束 |
| 完工 `ProductionReceiptInput` | 原有效报工、产品、精确入库量、批次、仓库/库位、检验策略、原命令/版本 | 默认按检验策略进入待检或直接合格；客户端不能自报质量放行或原成本 |
| 检验 `InspectionInput` | 原对象/批次、冻结标准版本、项目读数、合格/不良量、判定、原命令/版本 | 项目类型与上下限由标准约束；放行引用有效检验余量；需复检时保留前次记录 |
| 收付核销 `SettlementInput` | 收/付款原记录、伙伴、`allocations[{document_id,amount_minor}]`、业务日期、期间、原命令 | 金额为币种最小单位整数；核销合计≤现金未分配且每行≤原单未结，不跨币种隐式换算 |
| 正式列表 `QueryInput` | 稳定查询身份、状态/日期/仓库等过滤、排序字段/方向、页码或游标、limit、需要的汇总 | 服务端白名单排序、授权先行；支持范围/页长与真实API一致，不能只在客户端筛当前页 |

以下材料请求是规划批量契约，不是现有单行 `/materials` 已接受的负载。样例业务前提为同一工单、SKU和批次原领料1.000000，本次退0.200000及耗0.600000，提交后该原来源持料0.200000；两个动作不能指向其他领料。

```json
{
  "request_id": "fdbf0196-7c6e-42b7-bc24-36607b1db0cf",
  "expected_version": 7,
  "lines": [
    {
      "line_id": "ed48223a-527b-4c52-98c0-8c8d5ef3d4b9",
      "warehouse_id": "bd55f738-6d41-4c78-9b30-f67e3a6c2e94",
      "sku_id": "c99a1b87-a397-4e28-b1af-20e381691ea3",
      "batch_id": "6dd9ad69-c120-4d70-a677-3412d3a4b5a4",
      "kind": "RETURN",
      "quantity": "0.200000",
      "original_issue_id": "00ebbf71-fb16-40f7-8673-4500cfa62495",
      "reason": "退回余料"
    },
    {
      "line_id": "c1da167a-9dc8-4392-8748-36e558bbfc92",
      "warehouse_id": "bd55f738-6d41-4c78-9b30-f67e3a6c2e94",
      "sku_id": "c99a1b87-a397-4e28-b1af-20e381691ea3",
      "batch_id": "6dd9ad69-c120-4d70-a677-3412d3a4b5a4",
      "kind": "CONSUME",
      "quantity": "0.600000",
      "original_issue_id": "00ebbf71-fb16-40f7-8673-4500cfa62495",
      "reason": "确认实际耗用"
    }
  ]
}
```

建议批量响应逐行返回原动作/效果ID、原结果快照和提交版本，并明确整单已提交。单行当前命名可能不同，必须同步OpenAPI与两端DTO；不要让当前客户端向单行接口发送上述批量请求。

以下为规划核销请求。前提是该收款600元、未分配600元，该原应收1000元、未结1000元；确认后收款未分配0、原单未结400元，核销重试返回原结果。`amount_minor=60000` 表示人民币600元。

```json
{
  "request_id": "09acac86-1944-499b-baf5-e1a89c5feb34",
  "payment_id": "69b8226e-28cb-4110-a4f4-0a10b0632c3c",
  "business_date": "2026-10-03",
  "period": "2026-10",
  "allocations": [
    {
      "document_id": "55f766e7-b4ef-4d78-92cf-6a457b1b1452",
      "amount_minor": 60000
    }
  ]
}
```

## 二 具体实施单元

每个实施单元都要交付真实 API、权威数据、共享页面或运维动作及独立验收；仅保存契约记录不算整项完成。下文性能编号沿用原清单 `ERP-P01` 至 `ERP-P22`。测试名是建议新增名称，只有接入已登记目标并实际执行后才能记录通过。

### ERP-IMPL-01 企业注册和初始化

- 功能与字段：A01/A05。验证渠道和凭据、企业名称、贸易/制造开关、时区、币种、首账号、初始化步骤与错误。验证凭据一次消费，支持中断后的进度回查。
- 现有接续：`REPO/tenants.rs`、`REPO/auth.rs`；`REPO/shared_contract.rs` 的 `open_onboarding`、`record_onboarding_step`、`complete_onboarding`。
- 补齐位置：规 `API/onboarding/service.rs::{verify_identity,create_enterprise,get_progress}`、`REPO/onboarding.rs`、`UI/pages/account/onboarding.rs`；新增路由在现 `API/router.rs` 接入。
- 事务与拒绝：锁验证凭据和原请求；企业、首账号、免费套餐及必要默认配置同事务建立，较长初始化以持久化步骤继续。失败不留可登录但缺必需配置的企业。
- 验收与性能：`onboarding_retry_creates_one_enterprise`，20路同请求只生成一个企业；配置写入注入失败后回查或恢复。真实登录P01，同步普通初始化P04；后台初始化分别记录受理和完成时间。

### ERP-IMPL-02 免费席位准入

- 功能与字段：A02。套餐版本、有效席位上限1、用户可登录状态、升级/降级生效时间；员工档案不进入登录席位计数。
- 现有接续：`API/rbac_user_handlers.rs::{create_user,update_user}`、`REPO/access_plan.rs::admit_new_login_seat`、`REPO/rbac.rs`。
- 补齐位置：在所有实际启用入口委托同一准入事务；规 `UI/pages/account/plan.rs` 展示当前权益和有效账号。邀请、SSO、恢复禁用账号也必须检查。
- 事务与拒绝：锁企业权益与用户启用事实，事务内重新计数；不能先读席位再单独启用用户。降级选定保留账号并撤销其余资格，历史业务保留。
- 验收与性能：`free_seat_all_activation_paths_share_limit`；已存在1席位时各启用入口和旧客户端都不能突破。P06，20并发×100轮，有效席位始终≤1、判定P95≤1秒。

### ERP-IMPL-03 多端登录和会话撤销

- 功能与字段：A03/A07。真实密码、会话ID、客户端类型、到期、最近活动、凭据代次、当前会话和撤销原因。
- 现有接续：`API/auth_handlers.rs::{login,list_own_sessions,revoke_own_session,update_current_user_password}`，`API/auth_layers.rs::auth_middleware`，`REPO/access_plan.rs::{open_session,active_session,bump_credentials_epoch_in_tx}`。
- 补齐位置：规 `UI/pages/account/sessions.rs`；现PUT改密已将正式密码与代次更新放在同一事务；核对禁用、找回、真实续期及必要安全记录接线；桌面/Web网络适配分别接真实身份。
- 事务与拒绝：开会话锁有效配额，过期会话不计数。撤销提交后新受保护请求立即拒绝；已受理写命令仍有原回执，不能因退出而重发。
- 验收与性能：`session_limit_revoke_and_password_epoch`；真实账号跨桌面/Web登录，达到当前配置8会话后新登录明确拒绝。P01/P06/P07，拒绝与成功样本分组。

### ERP-IMPL-04 企业和仓库数据范围

- 功能与字段：A04/O05。企业、用户、权限、授权仓库、对象来源、下载对象及审计身份。
- 现有接续：`API/auth_layers.rs`、RBAC和各域handler；`REPO/shared_contract.rs` 的 `check_browser_request`、`authorize_download_object` 只作为需核对的契约入口。
- 补齐位置：规 `API/command_context.rs`；查询、详情、写入、批量操作、导出、附件及关联查询都调用同一范围校验，SQL保留企业和仓库谓词。
- 事务与拒绝：URL企业与会话企业一致；原来源、批次、文件和目标仓库逐项归属验证。批量传入一个越权ID时按整单原子策略拒绝，不能先处理合法前几行。
- 验收与性能：`tenant_and_warehouse_scope_all_surfaces`；至少两个非空企业及一个限制仓库账号。跨企业/无权仓库读取、写入、原命令回查与下载均拒绝；权限耗时包含在P02/P03/P04内。

### ERP-IMPL-05 共享主数据和单位

- 功能与字段：D01/D02/D03/D04/D05。SKU、编码、条码、基本/业务单位及换算版本、客户供应商、结算条件、仓库库位、员工班组和启停状态。
- 现有接续：`REPO/products.rs`、`skus.rs`、`product_units.rs`、`product_unit_conversions.rs`；`REPO/shared_contract.rs` 的 `save_shared_item`、`bind_item_identity`、`save_unit_conversion`。
- 补齐位置：规 `UI/pages/master/{items,partners,warehouses,employees}.rs`；实际主数据服务写正式对象，契约表仅保留必要映射与版本，不创建第二套SKU身份。
- 事务与拒绝：企业内编码按既定唯一范围检查；被历史业务引用的单位、换算和身份不能破坏性改写。使用者登录和作业员工分开建模。
- 验收与性能：`master_data_units_and_retired_references`；1箱=12件、1.25kg等精确换算可复算，重复编码、错企业、被引用后改基本单位拒绝。10万物料选项P09，列表/详情P02/P03，普通保存P04。

### ERP-IMPL-06 主数据导入

- 功能与字段：D06/U07。文件、模板版本、列映射、预检错误、行号、确认版本、处理结果、成功/失败数和产物摘要。
- 现有接续：`REPO/shared_contract.rs::{precheck_shared_import,confirm_shared_import,download_import_result}`；现有任务宿主与文件服务。
- 补齐位置：规 `WORKER/jobs/import_jobs.rs::{precheck,execute_import}`、`REPO/import_jobs.rs`、`UI/pages/tools/import.rs`；每行调用正式主数据校验和保存服务。
- 事务与拒绝：先预检再确认，确认绑定同一文件摘要与映射版本。全量原子或分批策略在模板中明确；部分成功必须保留逐行结果和可恢复身份。
- 验收与性能：`import_file_identity_and_row_results`；错单位、重复编码、跨企业引用、重复确认及Worker中断。P15，1万行×20列≤10MiB，预检≤30秒、确认导入≤60秒、新增峰值≤128MiB；上传另计。

### ERP-IMPL-07 期初启用和调整

- 功能与字段：D07/F01/I01。启用截止点、数量金额、批次、应收应付、原文件/手工来源、预检差额、确认和调整原因。
- 现有接续：`REPO/shared_contract.rs::{confirm_opening,resume_opening,rebuild_opening,correct_opening}`；正式库存、成本和往来服务。
- 补齐位置：规 `API/opening/service.rs::confirm_opening`、`UI/pages/master/opening.rs`；新增迁移保留期初来源身份和与权威分录的关联。
- 事务与拒绝：确认通过正式库存/价值/往来分录建立期初；重建从原来源复算。已启用后的修正追加可审计调整，不直接覆盖余额，不借期初掩盖历史不一致。
- 验收与性能：`opening_replay_and_adjustment_conserve_facts`；重复确认无新增、失败无部分期初、原数据与调整差额可独立复算。20/100行P04，较大导入按P15，重建按P14。

### ERP-IMPL-08 附件和扩展字段

- 功能与字段：D08。原对象、文件身份、大小/类型/摘要、上传与审核状态、字段类型/值、下载权限、配额和删除状态。
- 现有接续：`REPO/shared_contract.rs::{save_shared_attachment,cite_shared_attachment,save_attachment_quota,save_custom_field}`；现文件下载设施。
- 补齐位置：规 `API/attachments/service.rs`、`UI/pages/tools/attachments.rs`；附件元数据引用实际对象存储，不把可访问URL等同已验证文件。
- 事务与拒绝：上传成功后再建立可用元数据；失败和孤立对象有可回查清理策略。下载每次核对企业与对象范围，短时授权不泄露凭据。
- 验收与性能：`attachment_scope_checksum_and_quota`；跨企业URL、超限、错误摘要、删除后的下载和重复关联。元数据查询P03、任务受理P04，文件大小及网络耗时单列，不套用500ms承诺完整下载。

### ERP-IMPL-09 精确库存查询和汇总

- 功能与字段：I01/D02。实存、预占、冻结、在途和可用精确量；仓库/SKU/批次、来源、数据截止及是否需要对账。
- 现有接续：`REPO/inventory_reads.rs::{list_inventory,list_inventory_summary,list_inventory_costs,list_ledgers}`、`REPO/inventory.rs::get_balance`、`REPO/inventory_precision.rs::{get_precise_balance,get_precise_balance_with_tx}`、`REPO/inventory_grid_report.rs::aggregate`、`REPO/atp_inventory.rs::quote_fefo_exact`、`API/inventory_print_report_handlers/inventory_core.rs::get_inventory_atp`、`API/inventory_views.rs`。
- 补齐位置：原四个公开列表、聚合、单键读取、旧整数写入回执及后台库存CSV已接精确量；原库存报表增加授权仓库和毛发货口径，实际位置见本节。规 `UI/pages/inventory/balances.rs`，继续接正式页面、共享贸易来源分类、净销量和平均库存法周转。原整数响应为兼容投影，新业务使用完整精度字段。
- 事务与拒绝：可用量先计算精确的实存−预占−冻结，再转换展示；筛选、总计、排序和ATP均使用同一精度。未知或未对账键不能显示虚假的确定可用量。
- 验收与性能：现 `crates/erp-api/tests/free_erp_acceptance/inventory_reads.rs`；实存1.9、预占0.9、冻结0.9，可用0.1，整量兼容可用0；筛选≥1不命中。跨仓汇总、缺成本、宽总量、越权和未知历史分别验。100万流水/10万物料按P02/P03；四个读取接口目前上限2000，但实际500/2000行、并发负载与正式双端尚未验收。

#### 库存读取与聚合的实际接线

以下公共前缀为 `/v1/tenants/{tenant_id}`。这些文件和路由已存在，功能证据覆盖保护库及synthetic JWT；正式库存页仍是规划。`inventory_balances` 的四个 `*_micros` 和原来源流水为权威，不从旧整数推断小数可用量。

| 功能 / 真实路径 | 实现在什么位置 | 输入、输出和执行规则 | Grok后续验收方向 |
| --- | --- | --- | --- |
| 仓库/SKU明细，GET `/inventory` | 现 `API/inventory_print_report_handlers/inventory_core.rs::list_inventory` → `REPO/inventory_reads.rs::list_inventory`；DTO在 `API/inventory_views.rs` | 仓库、SKU、编码、条码、标签、可用量范围及安全库存；默认50、最大2000，取limit+1判断末页。按更新时间/仓库ID/SKUID降序游标；返回 `data/next_cursor` | 构造100/200/500/2000行，独立核对页数与稳定身份；并发修改时属于实时翻页，不能当成固定导出快照 |
| 跨授权仓库SKU汇总，GET `/inventory/summary` | 现 `API/inventory_print_report_handlers/inventory_core.rs::list_inventory_summary` → `REPO/inventory_reads.rs::list_inventory_summary` | 先按SKU合并授权仓的精确四桶，再筛可用量，再计数/分页；`items/total`，每页最大2000 | 1.9+0.2实存为2.1，预占/冻结各0.9，可用0.3；不能分别取整后再合并或只合并当前页 |
| 参考库存价值，GET `/inventory/costs` | 现 `API/inventory_print_report_handlers/inventory_core.rs::list_inventory_costs` → `REPO/inventory_reads.rs::list_inventory_costs` | 同汇总分页；`estimated_value` 为元的精确文本，最多8位小数，`valuation_source=sku_master_estimate`；原 `unit_cost_cent/total_cost_cent` 可空 | 2.1件×7分=0.147元；普通SKU或KIT组件成本不明时三个价值字段为null。实际账面价值继续由ERP-IMPL-15的成本分录接线 |
| 原库存流水，GET `/inventory/ledgers` | 现 `API/inventory_print_report_handlers/inventory_core.rs::list_inventory_ledgers` → `REPO/inventory_reads.rs::list_ledgers` | 授权仓/SKU/原因/时间，`items/total`，最大2000；新增四个 `quantities.delta_*`，并返回精度来源 | 原整数1仍表示1基本单位；四个精确字段齐全时 `source=canonical_micros`，均缺失时按原整单位读为 `legacy_whole_units`，部分缺失409。当前保护库新插入旧形态流水会被触发器补精度，历史全空分支尚未单独验收 |
| 全范围仓库/分类/类型聚合，POST `/reports/inventory/aggregate` | 现 `API/router.rs`、`API/inventory_print_report_handlers/inventory_aggregate.rs::post_inventory_aggregate` → `REPO/inventory_grid_report.rs::aggregate` | `schema/warehouse_id/search/by_category/measure`；授权和字面搜索后完整聚合，返回所有叶桶、总计及快照时间。schema2数量scale6、价值scale8，原始宽整数以字符串传输 | 数量与价值分组count/sum/min/max各自独立复算；成本不明422，未知库存409。schema1保持原整单位/分单位，遇不能精确表示的小数409要求schema2 |
| 桌面/Web报表、框选与文件 | 现 `UI/remote_view/inventory_report.rs`、`UI/selection_summary.rs`、`UI/export.rs`；现 `crates/erp-grid-core/src/query.rs`、`crates/erp-ui-egui/src/report_chart.rs` | 同一宿主请求schema2并核对measure/scale/source；DataGrid小数列精度明确，i128计算/比较/求和；CSV/XLSX使用完整叶桶和一次总计，长数按文本保真；图表仅几何映射使用浮点 | .133+.014=.147元，折叠不丢导出分组、无父子重复；真实双端文件保存与内容复读另验。小数编辑、公式和其他库存页面须按对应契约继续实现 |

| 字段、错误或边界 | 当前具体约定 | 必须保留的验收断言 |
| --- | --- | --- |
| `quantities.on_hand/reserved/frozen/in_transit/available` | 基本单位十进制字符串，最多6位小数；单键存i64微单位，跨仓汇总和计算中间值用SQL NUMERIC/i128 | 1.9−0.9−0.9=0.1；两个最大单键合计可为18446744073709.551614，不能缩成i64微单位或浮点 |
| 原同名整数和状态 | deprecated整单位投影，`available`由精确差值向下投影；明细与原库存调整回执返回 `quantity_state=precise` 和完整quantities。单键及预占回执接续见下节 | 新页面不能用三个旧整数重新算可用；其余共享贸易、批次/库位/WMS写入口仍须分别验证 |
| `available_min/max` | 三个库存列表接字符串，基本单位、含边界、空值不限，最长50字节、最多6位小数、i128量程 | `0.1..0.1`命中0.1；倒置、NaN、科学计数、超精度/量程400，保留用户输入，不查询后静默改条件 |
| `inventory.safety_stock` | 现 `API/inventory_print_report_handlers/inventory_core.rs::parse_inventory_safety_setting`；旧JSON整数1是1基本单位，新增小数使用字符串，可直接给值或包在value内；缺设置默认0。按available严格小于阈值 | 阈值字符串0.100001包含0.1；负数、7位小数、JSON小数number、非法文本422 `inventory_safety_stock_invalid`，不变成0 |
| 同一数据截点 | 现 `REPO/inventory_reads.rs::begin_read`；只读Repeatable Read事务内做库存完整范围预检、计数和当前页。聚合as_of取建立快照的第一条SELECT时钟 | 其他事务后提交的数据不进入本结果；安全库存设置当前在库存事务前读取，配置版本尚未与库存快照绑定，不承诺历史政策回放 |
| 未知历史 | 现 `REPO/inventory_reads.rs::require_reconciled`；检查授权非数量范围内余额及库存/制造/共享来源的键，数量筛选和分页前执行 | 未激活、字段不全、桶不合法、已有来源却缺余额409 `inventory_quantity_reconciliation_required`；数值过滤不能掩盖未知键，读取不激活、不建0余额 |
| 范围授权 | 四GET与聚合继续使用真实企业/仓库范围；显式无权仓库403，空授权仓集合不等于全部仓 | 未授权仓的未知库存既不能泄漏，也不能使授权仓查询失效；当前无显式范围的旧全仓策略仍需正式身份政策验收 |
| 缺成本与溢出 | SKU参考成本缺失不补0；KIT要求存在合法组件及成本。成本聚合缺失422 `inventory_cost_missing`；i128数量/价值超界422 `inventory_read_value_overflow` | 缺普通SKU/KIT组件分别验证；3个最大库存键×最大单价不绕回、截断或返回虚假成功 |
| 读取资源上限 | 四GET每页最多2000；聚合最多4096桶/64种类型/8MiB保留文本。库存事务内每条SQL超时10秒 | 超限不截断；10秒是单SQL护栏，尚不是含池等待、预检、编码的整请求时限，也不是P95成绩 |

13项实际HTTP用例位置为现 `crates/erp-api/tests/free_erp_acceptance/inventory_reads.rs`。最新完整登记目标68项通过；另一个psql连接生成事实，再由现 `qa/acceptance/free-erp-inventory-read-verify.py` 独立复算1332项、13个新样本和80次HTTP结果，原有38个样本保持不变。测试覆盖精确明细、跨仓汇总、两页游标、范围与安全库存、未知/孤立历史、宽总量、价值溢出、schema兼容、缺KIT成本、原整数流水及限制仓库。登记回归23.81秒是debug测试时间，不填入性能指标。

**性能实施任务仍未运行：** 沿ERP-P02/P03，在10万物料/100万流水、20并发下，对明细、SKU汇总、价值、流水分别按100/200/500/2000行测，每个动作×页大小至少1000请求；正常命中、稀疏范围、深页/游标、跨仓各分组。100/200行目标P95≤500ms/P99≤1秒；500行目标P95≤1秒/P99≤2秒；2000行为容量组，候选P95≤2秒/P99≤4秒、完整JSON≤8MiB，未达标不得在正式页开放此档。聚合按数量/价值及1/100/1000/4096桶分组，各至少1000次，目标P95≤500ms/P99≤1秒；先验证完整组和精确总计。鉴权、池等待、完整范围来源预检、SQL、编码全计，记录查询计划、扫描/返回行数、P50/P95/P99/最大、错误率及峰值内存；重点检查多来源UNION预检成本。拒绝样本单列，不稀释成功延迟。

**继续交给Grok的具体缺口：** 后台库存CSV已沿原任务接口迁到精确授权快照，实际函数和交付边界见 [ERP-IMPL-38库存后台CSV](#库存后台-csv-的实际接线)。单键读取、旧写回执及原报表的本轮接线见下节；共享贸易销售/退货来源、净销量、平均库存法周转及真实双端仍待完成。正式库存页接对象选择器、精确字段、500/2000能力标记、对账/成本未知及真实文件保存；小数编辑、公式计算列、历史全空流水、已映射渠道Outbox、整条迁移/恢复和Release负载逐项验收。共享库根持续并行修改，实际后端目标源码认证以本轮证据为准；不得据此勾选整个I01/I05/R01或发布。

#### 单键余额 写回执和库存覆盖报表的实际接线

| 功能和真实入口 | 实现在什么位置 | 字段和执行规则 | Grok接续验收 |
| --- | --- | --- | --- |
| 原单键余额 | 现 `REPO/inventory.rs::get_balance`、`REPO/shared_contract.rs::read_balance` → `REPO/inventory_precision.rs::get_precise_balance` | 同一只读快照检查仓库/SKU归属、历史及四个精确桶。无库存且无来源为None；存在未激活或孤立历史时拒绝，不猜测、回填或激活。`InventoryBalance.quantities`为完整小数，可用量先减预占/冻结再投影 | 1.9/0.9/0.9/2.3返回可用0.1、整量0；未知库存7不得返回确定7。另一连接核对原余额、流水、激活、来源映射不变 |
| 旧整数库存写回执 | 现 `REPO/inventory.rs::apply_with_tx` → `REPO/inventory_precision.rs::whole_projection`；现 `API/inventory_views.rs::inventory_balance_response` | 老整数1仍是1基本单位。使用写事务内实际余额，不提交后重新查询；API返回`quantities`及`quantity_state=precise`，保留原整数展示字段 | `POST /v1/tenants/{tenant_id}/inventory/adjustments`，在1.9上增加1后实存2.9、可用1.1、兼容可用1，数据库读回2900000；三入口的原命令与日志接线见ERP-IMPL-10；真实连接中断和双端恢复仍须单独验收 |
| 原订单预占及终态重放 | 现 `REPO/inventory.rs::{reserve_for_order_item,update_reservation}` | 新建、预占重放、释放/消费及其终态重放均返回精确量；终态重放仍须核对原仓库/SKU/数量。这是原入口的当前余额回执，不能当作冻结的原动作响应 | 先预占1、再释放或消费并各重放；重放无新流水。改变仓库/SKU/原量须冲突；KIT、批次及实际HTTP预占路径另验 |
| 原库存报表，GET `/reports/inventory` | 现 `API/inventory_print_report_handlers/reports_exports.rs::get_inventory_report` → `REPO/reports.rs::inventory_report` → `REPO/inventory_turnover.rs::read_report` | 必须有reports.read和inventory.read；显式无权仓库403，默认范围只包括授权仓。只读REPEATABLE READ内完成来源预检、计数、毛发货和分页；未知键在分页前409。默认50、最大200，稳定编码/SKU/仓库排序 | 限制仓库只返回1个授权键，不读另仓数量和发货；深空页仍拒绝授权内未知历史，权限外未知历史不污染本查询。正式页面与公网账号仍待验 |
| 精确阈值、缺货 | 现 `API/lib.rs::InventoryReportQuery`；现 `REPO/inventory_turnover.rs::read_report` | `warning_quantity`是最多6位小数的非负字符串；原`warning_threshold`是整基本单位，默认5；两者不能同传。warning使用精确available≤阈值，out_of_stock使用available≤0。非法/负数/倒置日期/不足1ms区间400 | 可用0.1、阈值0.099999为false，0.1及0.100001为true；兼容available=0时缺货仍为false。错误输入保持原事实不变 |
| 发货量与覆盖天数 | 现 `REPO/inventory_turnover.rs::read_report`；源 `inventory_ledgers` | 日期仅筛选已入账`SALES_SHIP`效果；按实际仓库/SKU归集，不统计订单需求。返回`schema=2/snapshot_at/sales_basis=gross_posted_sales_ship/turnover_basis=closing_stock_cover`；`sold_quantity`为完整小数，`sold_qty`为整量兼容投影 | 已入账0.05、取消订单99、另仓发货0.09：本仓仅0.05。当前实存1.85、观察区间2日，覆盖天数74。这是当前库存按毛发货速率的估算，不能冒称平均存货法周转，也不净扣退货 |
| 未分类来源 | 现 `REPO/inventory_turnover.rs::read_report`；源OpenAPI `InventoryTurnoverItem` | 观察期内共享库存引用尚未分出销售/非销售，或SALES_SHIP方向不合法，`sales_state=source_classification_required`，销量及覆盖字段均为null。库存量仍可读；未知不能填零 | 提供实际共享销售交付/退货的来源链并逐条独立复算后再开放指标；不能仅按SKU同名、旧单状态或任意source_line文本猜测销售来源 |

保护库回归位置为现 `crates/erp-api/tests/free_erp_acceptance/inventory_consumers.rs`；完整登记目标101通过/0失败/0忽略，新增15项，独立事实1975项通过、1325个既有QA企业保持不变。终态回归实际复现并拒绝数量/仓库变更；SKU匹配还应在真实接口样本核对。原失败、一次未完成运行、完整回归、独立事实及900输入源码变化见 [余额与报表修复证据](../qa/acceptance/free-erp-inventory-consumers-repair.json)。共享仓储根并行变化，整体源码认证PARTIAL；严格后端Clippy仍FAIL。合成JWT不替代真实密码、多端会话和席位验收。40.13秒是debug功能测试段，不能填入ERP-P性能表。

**单独性能任务仍待运行：** 沿ERP-P02/P03/P14，在10万SKU/100万库存流水、20并发下，单键余额、报表100/200行、深页、1/10/100个授权仓库及无发货/密集发货各组至少1000请求，记录鉴权、池等待、来源预检、COUNT、发货聚合、分页/编码及事务时长。单键和100/200行目标P95≤500ms/P99≤1秒；完整库存报告目标P95≤2秒，响应与正确性分别记录。原生和Web单独验收查询、精确/未知列、重登与真实保存，不以API日志代替客户端。每条SQL10秒是护栏，不是整请求时限或性能成绩；重任务及平均库存计算按P14接入有界任务。

#### ATP公开接口的实际契约

ATP接线沿用 `GET /v1/tenants/{tenant_id}/inventory/atp`，下表描述现有代码与OpenAPI中的契约。四个公开库存列表、单键余额、回执与库存报表的接线见上表，后台导出见ERP-IMPL-38；共享来源分类、完整周转与正式页面仍按ERP-IMPL-09/15/38继续接线。

| 位置或字段 | 具体功能 | 拒绝或验收要求 |
| --- | --- | --- |
| 查询 `warehouse_id`、`sku_id`、`request_qty` | 前两项为UUID；需求量为基本单位十进制文本，省略为0，最多6位小数，上限9223372036854.775807；旧文本1仍为1基本单位 | 非法、负数、7位小数、科学计数或溢出返回400，不改库存 |
| 响应 `quantities` | `on_hand/reserved/frozen/available/request_qty/atp_qty/shortage_qty`全部为权威小数字符串；可用量先减预占与冻结，承诺量受可用批次限制 | 1.9/0.9/0.9，需求1时可用0.1、承诺0.1、缺货0.9；需求0.075时承诺0.075、缺货0 |
| 原同名整数响应 | 仅作旧整单位调用方的兼容投影；`available/atp_qty`向下取整，`shortage_qty=整数request_qty−整数atp_qty`；OpenAPI标为deprecated | 前例为可用0、承诺0、缺货1；新业务计算、排序、分配和展示使用 `quantities`，不得重新按三个整数桶相减 |
| `allocations[].quantity` | 每批分配量为精确文本；按到期时间、稳定批次ID排序，无到期批次最后；过期批次不参与推荐 | 过期1.6、有效0.25且预占0.2、无效期0.05，需求0.075，推荐0.05+0.025；原 `allocated_qty`仅整单位兼容投影 |
| `REPO/inventory_precision.rs::get_precise_balance_with_tx` | 仓库/SKU归属、SKU余额及批次同一只读Repeatable Read事务；读取不激活历史、不建零余额、不建立预占或命令 | 未激活、缺精确批次、批次总实存不等SKU实存、批次预占大于SKU预占、已有来源却缺余额返回409 `inventory_reconciliation_required`；外企业仓库/SKU返回403 |
| `REPO/atp_inventory.rs::MAX_ATP_BATCHES` | 交互报价每仓/SKU支持最多1000个已存批次（含过期）；SQL最多取1001条，检测超预算；批次数量和合计使用宽中间值 | 1001个正确批次返回422 `inventory_atp_batch_limit`，不截断报价、不伪称库存不足，不产生库存效果 |
| 性能任务及后续UI | 规划 `UI/pages/inventory/balances.rs` 的ATP动作解析权威字符串；非批次、10/100/1000批次四组分别测，至少1000次请求/组、20并发，含鉴权、等待池、SQL和编码 | 目标P95≤500ms/P99≤1秒；记录四组吞吐、错误率及峰值内存。该指标尚未Release压测或正式双端验收；超预算拒绝用例不代替性能通过 |

完整冻结原因、质量隔离、库位分配和批次成功出入库仍属于后续逐入口门禁；本表的只读报价不替代实际销售/制造提交时的库存锁内复核。验收源码为 `crates/erp-api/tests/free_erp_acceptance/inventory_precision.rs`；保护库测试保留原失败HTTP反例。

### ERP-IMPL-10 所有库存写入口使用一个服务

- 功能与字段：I01/I02/O01/O02。四库存桶精确增量、来源类型/ID、原效果、命令、仓库/SKU、原提交快照和兼容投影。
- 现有接续：`REPO/inventory.rs::{apply_with_tx,apply_adjustment_with_tx}` → `REPO/inventory_precision.rs::apply_precise_with_tx`；`REPO/audit_logs.rs::insert_with_tx`、`REPO/operation_logs.rs::insert_with_tx`；`REPO/inventory_movement_commands.rs::{execute,find,verified_snapshot}`；`API/inventory_print_report_handlers/inventory_commands.rs::{execute_inventory_command,get_inventory_command}`；`migrations/0283_inventory_precision.sql`、`migrations/0284_inventory_movement_commands.sql`。
- 补齐位置：按原清单第二十五节14项入口接线采购、销售、WMS、调拨、盘点、冻结、ATP、制造和Worker；历史整数及精确来源归入同一权威账本。
- 事务与拒绝：同事务完成准入、余额、来源流水、必要Outbox和回执；稳定顺序锁库存键。历史未证明一致时返回需要对账，不能按旧整数覆盖已证明的小数来源。
- 验收与性能：`all_stock_writers_share_exact_ledger`；逐入口核对另一连接读回，0.5库存20路各扣0.4仅1笔成功；精确历史激活和激活期间共享写入分别测。P04/P05，20并发×100轮；迁移触发器拒绝不代替旧入口的成功接线。

#### 库存移动和日志的事务接线

| 实际功能与入口 | 实现在什么位置 | 提交和拒绝规则 | Grok接续验收 |
| --- | --- | --- | --- |
| 原库存调整，POST `/v1/tenants/{tenant_id}/inventory/adjustments`；原入库/出库，POST `/v1/tenants/{tenant_id}/inventory/inbounds`、`/v1/tenants/{tenant_id}/inventory/outbounds` | 现 `API/inventory_print_report_handlers/inventory_core.rs::apply_inventory_movement`；原调整handler及 `API/inventory_print_report_handlers/stock_movement.rs::{inbound_inventory,outbound_inventory}` | 移动helper在鉴权、仓库范围和功能准入后开事务；库存、来源流水、该服务产生的Outbox、操作日志和审计完成后才提交。任何提交前存储错误不能留下部分业务效果 | 对每个入口分别令operation_logs、audit_logs真实INSERT失败，6组均返回500；另一连接核对余额、流水、激活、Outbox和两个日志前后完全相同。不能只断言HTTP失败 |
| 库存/批次写入由调用者控制事务 | 现 `REPO/inventory.rs::apply_adjustment_with_tx`；原 `apply_adjustment`保留独立事务包装 | 公共事务方法不自行commit，不从Pool另开库存事务；批次写入同一executor。直接仓储调用与其他WMS服务还须各自传入事务和必要审计 | 实际带批次、批次后段失败、过期/外企业/错SKU批次逐项验；本批无批次夹具不证明这些入口或身份检查已完成 |
| 必需的两个日志与精确结果 | 现 `REPO/operation_logs.rs::insert_with_tx`；现 `REPO/audit_logs.rs::insert_with_tx` | 原Pool版insert委托相同规范化实现；API传同一事务连接。metadata/after_json包含仓库、SKU、batch_id、原整数增量、原因、引用和事务内quantities，操作者来自认证身份 | 三入口各1次成功：实存2.9加1为3.9/可用2.1，出1为1.9/可用0.1；独立读回2条含期初流水、1条操作和1条审计，两个日志的物理键/操作者/精确结果与已提交余额一致 |
| 原命令与未决结果 | 现三个无批次/库位维度的原移动handler → `API/inventory_print_report_handlers/inventory_commands.rs::execute_inventory_command` → `REPO/inventory_movement_commands.rs::execute`；不带身份的兼容路径仍保留 | 提供合法request_id或Idempotency-Key时，原动作绑定完整负载、认证操作者与不可变原结果，使用共享命令唯一空间；旧HTTP回包缓存不再处理这三个入口。兼容无ID请求仍不具备原命令恢复 | 下方逐字段、逐函数及HTTP断言定义本批范围；真实commit断线、重登/撤销会话、完整批次/库位和双端页面另验，不能用reference_id或当前余额替代原回执 |

本批实际测试位置为现 `crates/erp-api/tests/free_erp_acceptance/inventory_mutation_atomicity.rs`。故障仅由本测试拥有的独立schema/view及请求连接search_path产生，不改公共表/触发器，不阻断其他企业。原失败0通过/2失败保留；修后3项定向通过，包含9次实际HTTP（6个日志拒绝、3个成功）。完整登记目标104通过/0失败/0忽略，独立psql复算1734项通过、1430个既有QA企业不变；已知timestamp按同一时刻比较，首次时区文本误差报告和原始事实保留。实际901输入中6项并行变化，整体源码认证PARTIAL；严格Clippy仍FAIL。完整目标、独立事实、源码与门禁统一见 [库存移动事务修复证据](../qa/acceptance/free-erp-inventory-mutations-repair.json)。合成JWT、未映射渠道和无批次夹具的边界单列，不能替代真实平台同步、密码/席位/会话或双端验收。

**性能接续仍待测：** 沿ERP-P04/P05，在10万SKU/100万流水背景中，对这三个入口各测单键、不同键及受限仓库，20并发×100轮；记录准入、池等待、库存键锁等待、库存/流水、Outbox、operation_logs、audit_logs及commit分段。成功与拒绝分别统计，P95/P99按原性能表，至少独立复算每轮前后库存和两类日志。增加两个日志到同一事务后，锁持有、连接占用和日志索引成本必须纳入结果；2.07秒为debug夹具回归，不能当作负载达标。另测带批次/真实映射、日志存储故障、连接断开和提交未决恢复，禁止全量抑制数据库错误。

#### 原库存命令、原结果快照与回查接线

本段落实I01/I02/O01/O02的一个具体接续批次。实际范围是上述三个原库存POST入口的**无批次、无库位、无质量维度、基本单位整数增量**命令分支，以及一个持久化GET回查入口。整数请求`1`仍是1基本单位，存储效果为1000000微量；原有小数余额不会被截断。正式小数录单、带批次/WMS单据及各类共享业务命令继续按各自实施单元验收。

| 具体功能和数据 | 实现在什么位置 | 必须满足的行为和验收方向 |
| --- | --- | --- |
| 入库/出库`quantity`，调整`quantity_delta`，`warehouse_id`、`sku_id`、可选`reason`/`reference_id`/`request_id` | 现 `API/lib.rs` 的 `InventoryMovementRequest`、`InventoryAdjustmentRequest`；现 `API/inventory_print_report_handlers/stock_movement.rs`、`API/inventory_print_report_handlers/inventory_core.rs` 的三个handler | 未知字段422，非法UUID或非法数量400；命令调整非零，入出库正数量。先鉴权及仓库准入；客户端不能填认证操作者。超过精确i64微量范围400，无回执或库存部分效果 |
| 请求体/请求头身份解析 | 现 `API/inventory_print_report_handlers/stock_movement.rs::shared_request_id`；现 `REPO/inventory_movement_commands.rs::normalize_inventory_request_id` | 两处先trim；规范化后1—128个可见ASCII字节，不含内部空白。空、非法编码、超长或重复请求头400；头与体不一致409；不得把非法身份丢弃后降级成无ID写入。两处都省略/null才走原兼容路径 |
| 规范化完整负载与指纹 | 现 `REPO/inventory_movement_commands.rs::{InventoryMovementCommandInput,normalize,payload_hash,snapshot_payload_hash}` | 当前`inventory-movement-v2`绑定企业、认证用户、原ID、动作、仓库、SKU、原整数增量、原因/引用及可选库位/来源/UUID批次；不把session_id算作业务语义，原sid仍保存为提交出处。历史v1用原版本和完整原输入核对，不重写摘要。原因trim后≤512字节、引用≤128字节，无控制字符；空值规范化为None。同原ID改任一业务语义409；reference_id不替代request_id |
| 原命令唯一空间与竞争准入 | 现 `REPO/inventory_movement_commands.rs::execute`；现 `migrations/0284_inventory_movement_commands.sql`，沿用`command_receipts`的企业+原ID主键 | 先原命令事务锁，再占共享命令身份，再锁物理库存；不能新建另一套身份表造成同ID跨动作重复写入。另一个旧业务已经占用原ID时拒绝冲突，不猜原动作。20路请求头/请求体混用同ID，只能有1个首提交与19个原结果重放 |
| 库存、台账、Outbox、操作、审计与原结果共同提交 | 现 `REPO/inventory_movement_commands.rs::execute` → `REPO/inventory.rs::apply_adjustment_with_tx`，两个日志的`insert_with_tx` | 入库、出库、调整分别沿原reason/action。台账ref_type固定`inventory_movement_command`，ref_id是原ID；日志含完整业务指纹、物理键、原因、引用与事务内精确量，v2另含submitter_session_id锚定原提交端。原结果保存version、规范化输入/原sid、余额及台账/两个日志ID。任一提交前失败时连RUNNING身份占位也回滚 |
| 不可变原结果与证据复核 | 现 `REPO/inventory_movement_commands.rs::{verified_snapshot,valid_balance}`；现迁移中的`guard_inventory_movement_command_receipt` | 原结果的5个精确量、可用公式和5个整数投影一致；企业/操作者/动作/键/指纹与回执一致；原台账和两个日志均存在且数量绑定。成功回执禁止UPDATE改变或DELETE。旧记录不补造快照，未完成或损坏原证据不能重执行 |
| 原命令持久化回查 | GET `/v1/tenants/{tenant_id}/inventory/commands/{request_id}`；现 `API/inventory_print_report_handlers/inventory_commands.rs::get_inventory_command` → `REPO/inventory_movement_commands.rs::find`；现 `API/router.rs`、`openapi/src/` | 要求inventory.read、原认证用户和当前仓库范围；另一操作者/缺失原ID404，缺读权限或当前仓库授权被撤销403；证据不完整409，返回`inventory_command_reconciliation_required`。原结果保留原updated_at与原effect/log/audit ID，当前库存另查。只读回查不扣库、不写新日志、不返回后来的余额 |
| 回包缓存与身份冲突隔离 | 现 `API/middleware/idempotency.rs::match_idempotency_tenant` | 三个原移动入口交给事务命令设施，旧响应缓存不得提前返回历史回包、缓存一次临时500、忽略认证用户或因原ID的传输方式不同而拒绝合法重放；其他入口继续按原独立契约验收 |
| 客户端未知结果与正式页面接线 | 接续现 `UI/action_store.rs`、`UI/http.rs`、`UI/remote_actions.rs`；规正式库存动作页面及该动作的恢复适配 | 开始提交前保存原意图、原ID、企业/账号与规范化内容；超时、断线或commit结果不明保留未知状态。按原ID回查，查到SUCCEEDED应用原结果；查不到仍只能沿原ID核对/重试，不生成新ID。改变数量是新意图；历史无ID动作不自动补发。这一行仍为待开发/待真实双端验收 |

**逐场景验收清单：** 现 `crates/erp-api/tests/free_erp_acceptance/inventory_commands.rs` 包含最初两项实际反例；现 `crates/erp-api/tests/free_erp_acceptance/inventory_command_edges.rs` 包含三动作原结果重放、全负载/操作者/动作冲突、20路混用身份的并发、不同ID争同库存、非法请求头/体、回包丢弃后的原结果查回、当前权限复核、未支持维度/非法数量、6种真实日志写失败、成功快照更新/删除拒绝、未完成/损坏/旧回执拒绝再执行。每场景用另一连接核对真实余额、流水、原回执、两个日志、Outbox及激活；不以HTTP状态或回执存在单独算通过。

**现有证据状态：** 最初0通过/2失败，修后2项定向通过；扩展第一轮11通过/1失败，发现旧回包缓存抢先处理请求头，原失败及QA事实保留。移除这三个缓存入口后12项定向通过，完整登记目标116通过/0失败/0忽略，debug测试段60.59秒。独立psql复算2820项通过，1596个既有QA企业不变，31个新命令样本、28个成功原命令及190次扩展HTTP与持久化原结果核对；另有原两项回归的5次实际HTTP断言。6个真实日志写失败在证明回滚后沿原ID成功提交。三动作各20路同ID混用请求头/体仅1次效果；20个不同出库ID争4.9实存/0.9预占/0.9冻结，3笔成功、17笔明确冲突，终值1.9且可用0.1。

906个输入中，完整运行期间onboarding、共享测试、实际共享仓储根和两份OpenAPI源共5项并行变化，整体当前源码认证PARTIAL；本批受影响源未在该窗口变化。7个叶文件固定入口格式检查通过，旧缓存其他路径的3项库级回归通过，最终OpenAPI打包/v2门禁通过。库级首轮过滤未命中，修正过滤后的首次尝试触发80GiB缓存准入拒绝；后来缓存状态变化后仍沿统一入口完成检查，本任务未清缓存或终止别的构建。严格Clippy实际执行仍FAIL：仓储lib 234/lib test 236、四适配器合计27；本批叶文件无诊断不能代替包级通过。原输入、各次失败、独立事实、schema目录回查和门禁统一见 [原库存命令修复证据](../qa/acceptance/free-erp-inventory-commands-repair.json)。弃用响应体只验证服务器原结果持久化，不证明真实网络在commit边界中断、真实密码登录、撤销会话或正式客户端恢复。

最后交付身份核对实际为FAIL：本批15个代码、迁移及依赖输入与修复证据一致，但两份共享OpenAPI源和生成包在最后门禁之后再次变化。保留原门禁PASS，当前契约认证记PARTIAL；变更后的门禁未重跑，不把原结果绑定到新文件。实际SHA、差异和文档核验快照见 [交付核对证据](../qa/acceptance/free-erp-inventory-commands-delivery-qualification.json)。该核对不增加运行或性能成绩。

**明确后续范围：** 上述116项是前一批无维度命令的历史证据。库位/批次码/质量入库由下段接续，不能把两个批次的结果混算。UUID型batch_id、库位出库、预占/释放/WMS/调拨/盘点/制造/Worker接线、精确小数输入和历史未知命令对账仍按各实施单元及原14项库存入口清单完成；不带原ID提供库位维度仍拒绝。两个批次均不代表所有库存写入口完成。

**性能指标和测量位置：** 同ERP-P04/P05，单行原命令首提交/合法重放/GET原回查分别测，不把快重放混入首提交分布。首发基线中的10万SKU/100万台账背景、同ID与不同ID20路竞争各≥100轮；报告成功、明确拒绝、超时/未决、重复/部分效果及全部耗时原样本。单行首提交目标P95≤1秒/P99≤2秒；20/100行后续单据提交分别P95≤1/2秒、P99≤2/4秒；同键完成或明确冲突P99≤3秒，重复效果、超扣、丢更新和部分日志为0；原ID回查按P02目标P95≤500ms/P99≤1秒并单独留样本。代码位置沿`execute`/`find`记录池等待、原命令锁、身份冲突、库存锁、精确效果/Outbox、两个日志、快照写入、提交和HTTP结束。当前每SQL10秒、每锁5秒是防止无界等待的保护设置，不能当作总请求SLA或已达性能门禁；Release与稳态/故障原样本仍待运行。

#### 库位入库与权威库存的共同提交

本段接续I01/I02/O01/O02。原POST `/inventory/inbounds`在有原request_id或Idempotency-Key并完整提供location_id、batch_code、quality_status时，也使用同一原命令事务和GET回查。具体范围为基本单位整数入库、真实warehouse_locations库位及批次码；不是UUID批次管理、完整WMS收货/上架或质量释放。旧分支把原ID直接作为指纹，实际复现数量1改成2仍返回200重放、qty_micros却改成2000000；另一个首次成功只有共享台账、没有权威库存余额。原0通过/2失败和旧样本保持不变。

| 功能 字段及状态 | 实现位置 | 具体验收方向 |
| --- | --- | --- |
| 完整库位负载 | 现 `REPO/inventory_location_commands.rs` 的 `InventoryInboundLocation::normalize`；现 `REPO/inventory_movement_commands.rs` 的 `InventoryMovementCommandInput` | location_id非空UUID，batch_code trim后1—128个UTF-8字节且无控制字符；质量仅GOOD/PENDING/FAILED/HOLD。完整指纹追加库位/批次/质量并继续绑定原用户、数量、原因和引用；改变任一语义409。省略location字段时跳过序列化，原v1无维度指纹及旧快照保持可读 |
| 原handler接线 | 现 `API/inventory_print_report_handlers/stock_movement.rs::shared_stock_ledger_inbound` →现 `API/inventory_print_report_handlers/inventory_commands.rs::execute_inventory_command` | 结构化入库不再把原ID冒充指纹；走相同写权限、当前仓库授权、写功能开关与共享身份占位。三维必须完整；库位缺失、禁用或不属于本企业/仓库400。原UUID batch_id仍拒绝 |
| 库位锁与权威库存 | 现 `REPO/inventory_location_commands.rs::apply_location_inbound` →现 `REPO/inventory_precision.rs::apply_precise_with_tx` | 新命令锁有效库位行和原共享库位键，再锁权威库存，保留既有库位→物理键顺序。先产生精确库存效果，再追加ON_HAND共享投影；inventory_ledgers的ref_type为shared_stock_ledger，ref_id为本次共享ledger_id。两者和两类日志、Outbox、原结果共一次commit；任一步失败连RUNNING身份及激活一起回滚 |
| 质量与可用量 | 现同函数中的 `InventoryInboundLocation::frozen_delta` | GOOD只增加实存；PENDING/FAILED/HOLD同时增加等量实存和冻结，当前可用不增加。测试各入2，非GOOD实存2/冻结2/可用0，随后普通出库1拒绝且无效果。质量释放、批次兑现及库位特定出库另行实施，不直接改旧流水 |
| 两个原结果与回查 | 现 `InventoryCommandSnapshot.location_effect`、`InventoryLocationEffect`、`verify_location_effect`；现 `API/inventory_print_report_handlers/inventory_commands.rs::inventory_command_response` | 返回原仓库record和原location/location_record，保留原主台账、共享台账、两个日志ID及时间；核对真实共享来源的维度、原量和时间。兼容ledger_id/qty_micros/bucket保留，effect_id仍为主库存台账；数量计算使用record.quantities及location_record.on_hand文本。期初4.9入1、再入2后，原回查仍返回仓库5.9/库位1，当前查询为7.9/3 |
| 投影不可变与查询索引 | 现 `migrations/0285_inventory_location_projection_guard.sql` | 已引用权威库存效果的共享投影拒绝有变化的UPDATE与DELETE，修正用新来源追加；未关联旧行保留对账范围。完整企业/仓库/库位/SKU/批次/质量/bucket索引INCLUDE(delta_micros)服务精确求和；索引存在不等于百万流水负载达标 |

**实际验收：** 现 `crates/erp-api/tests/free_erp_acceptance/inventory_location_commands.rs`共9项，覆盖原反例、八个负载字段及操作者/无维度入口冲突、原双快照、四质量状态、20路头/体混用同ID、6种实际日志故障、非法/禁用库位与撤权、已关联投影不可变。首次扩展8通过/1失败是保护库缺少既有0030迁移的trace_id列，错误发生在预期日志故障之前；原失败保留。补齐该可空列后，测试必须确认指定日志view的实际check-option错误，不能把任何500当作该日志回滚证明。

修后9项定向通过，debug段7.47秒；完整登记目标125通过/0失败/0忽略，debug段63.82秒。独立psql复算2432项通过、1761个既有QA企业的全部选定事实不变；17个新库位样本、18个成功原命令、69次真实HTTP、6次实际日志故障和2个实际Outbox生产事件逐项核对。Outbox采用synthetic映射谓词夹具，只证明真实本地生产/回滚，不证明平台发送或Worker派发；合成JWT不证明真实密码、席位和会话。

完整运行盘点909输入，共享仓储根、onboarding、另一共享测试、两份OpenAPI源及生成包共6项并行变化，整体当前源码认证PARTIAL，本批受影响源在该运行窗口稳定。六叶格式首检发现测试日志两参数需要换行，原FAIL保留；仅作这一处空白修正，独立核对与实际被测源码精确比较，修后固定入口检查PASS，未据此重跑业务。严格Clippy仍FAIL（仓储lib 234/lib test 236、适配器合计27）；未观察到本批诊断不算包级通过。最后OpenAPI窗口内打包及v2门禁通过，其后共享输入变化，当前契约认证PARTIAL。实际源/产物SHA、原日志、数据库事实、schema/触发器/索引回查及各门禁见 [库位入库修复证据](../qa/acceptance/free-erp-inventory-location-commands-repair.json)。仅在保护库补齐选定真实库位schema、trace列和新投影守卫/索引，不证明整条迁移或恢复通过。

**性能任务仍待运行：** ERP-P04/P05沿原命令任务增加GOOD/PENDING/FAILED/HOLD、1/10/100库位、同库位与跨库位、同ID/不同ID各组，在10万SKU/100万流水背景下各≥1000请求、竞争各≥100轮。首提交P95≤1秒/P99≤2秒；竞争完成或明确冲突P99≤3秒；原回查P95≤500ms/P99≤1秒。分别记录库位行锁、共享键锁、池等待、历史激活/精确库存、库位SUM与投影INSERT、Outbox、日志、双快照、commit及HTTP全时长；错误率、重复效果、非良品误出和部分提交为0。查询计划、索引命中、峰值内存和连接占用留原样本；当前debug正确性测试与一次20路竞争不填入Release门槛。正式双端检查点、commit断线、真实映射发送、完整小数录单及其他库存入口仍待开发/验收。

#### 库位出库与本库位可用量

接续上段I01/I02/O01/O02，原POST `/inventory/outbounds`已接location_id/batch_code/quality_status分支与原命令GET。输入quantity仍为正整数基本单位，仓储保存负quantity_delta，兼容qty_micros为负；原整数1保持1基本单位。仓库/库位两个原结果与精确小数余额保持可读。完整小数录单、UUID batch_id和来源预占消费另按对应实施单元接续。

| 具体功能与字段 | 实现在什么位置 | 验收方向及失败后的事实 |
| --- | --- | --- |
| 完整出库意图与原身份 | 现 `API/inventory_print_report_handlers/stock_movement.rs::outbound_inventory`、`inventory_movement_location`；现 `REPO/inventory_movement_commands.rs::InventoryMovementCommandInput::normalize` | request_id或一个Idempotency-Key，完整仓库/SKU/库位/批次码/质量、原正整数quantity、原因、引用；头体不同409、多头400、缺身份或部分库位字段400。原batch_id在该分支仍400；原操作者、方向、物理键及全部语义参与已有v1指纹；同原ID改字段或换普通入口不允许新效果 |
| GOOD与质量拒绝 | 现 `InventoryMovementCommandInput::normalize`；现 `API/inventory_print_report_handlers/inventory_commands.rs::inventory_command_error` | 有效质量仍为GOOD/PENDING/FAILED/HOLD，通用库位出库只允许GOOD。非GOOD返回409 `inventory_location_quality_not_issuable`；即使同仓GOOD还有3件，也不能用这些数量兑现待检/不良/隔离批次；未知质量400 |
| 统一锁和本库位准入 | 现 `REPO/inventory_location_commands.rs::{apply_location_outbound,apply_location_movement}`；现 `REPO/inventory_precision.rs::require_owned_key` | 锁有效库位行、原ON_HAND库位键，再复用权威物理键锁；物理锁在查询各桶之前取得，与共享投影INSERT守卫共用同一键。查询完整企业/仓库/库位/SKU/批次/质量的ON_HAND、RESERVED、FROZEN、UNAVAILABLE；非负i64各桶以i128减法计算可用，越界、负桶及已过占409对账，不靠入库自动掩盖历史差异 |
| 本库位不足与精确量 | 现同函数的四桶SUM与 `InventoryCommandError::InsufficientLocationStock` | 不足返回409 `inventory_location_insufficient_available`，available_micros/requested_micros均为精确文本。指定库位实物1、预占0.3、冻结0.2，另一库位有5，出1仍拒绝，回包可用500000/请求1000000；拒绝后原身份占位、全部选定库存和日志不变 |
| 双层库存与一次提交 | 现 `REPO/inventory_movement_commands.rs::execute` →现 `REPO/inventory_location_commands.rs::apply_location_outbound` →现 `REPO/inventory_precision.rs::apply_precise_with_tx` | 本库位准入后仍在相同锁内复核权威实物/可用；原精确库存服务写负实物，现共享投影ON_HAND同量为负并引用主台账来源，预占/冻结/在途不因普通出库清零。必要Outbox、两个日志和原双快照同commit；任一步存储失败原样回滚。本批复用0283/0284/0285守卫与索引，没有新建平行出库余额 |
| 原小数和零库位结果 | 现 `verify_location_effect`、`verified_snapshot`；现 `API/inventory_print_report_handlers/inventory_commands.rs::inventory_command_response` | 真实期初4.9出整数1为3.9；之后入2当前5.9，原POST重试与GET仍完整返回当次仓库3.9/库位3.9及原ID/时间。库位出完后原0合法且可回查；原共享效果的负量、维度、时间以及两个日志都核对。当前余额另查，原回执不覆盖当前查询 |
| 当前授权及库位状态 | 现 `inventory_command_warehouse_access`及同库位事务方法 | 当前仓库授权撤销，原POST重试及GET均403；已禁用库位的新出库400，已经提交的原动作仍可按当前仓库授权回查/重放且不再扣量。另一个已授权用户用原ID提交409、回查404；上述身份规则不证明真实密码及单席位准入 |
| 历史来源与生产者边界 | 现精确库存激活/来源核对及现 `migrations/0285_inventory_location_projection_guard.sql` | 未关联旧共享库位5件不能出售或静默激活，409对账且不新增余额/台账/原命令；已关联投影仍不可改删。synthetic关联期初/预占及映射只用于夹具，不补造历史命令。实际Outbox生产/回滚单独读回，平台发送与Worker派发继续另验 |

**实际证据：** 新验收文件为现 `crates/erp-api/tests/free_erp_acceptance/inventory_location_outbound.rs`；复用已有库位测试的真实HTTP及另一连接事实读取助手。原两个充足库存案例均返回400“不支持”，0通过/2失败和其私有QA样本保留。修后11个定向场景通过，debug段9.57秒，独立3155项通过、1914个既有QA企业选定事实不变；完整登记目标136通过/0失败/0忽略，debug段78.22秒，独立3167项通过、1926个既有QA企业选定事实不变。两组各12个新出库样本、18个入库原命令、11个出库原命令及98次真实HTTP逐项复算；3笔实际关联期初/预占/冻结夹具、1个未关联历史拒绝样本、2次真实指定日志view拒绝和2个真实Outbox生产事件分别核对。

20路混用头/体同ID只1个出库效果、19次原结果重放；20个不同ID竞争本库位3件只3笔成功/17笔拒绝，另一库位100件保持，权威实存100/本库位0。两个日志的实际check-option错误必须命中指定table，失败前后另一连接核对全部选定事实相同；确认回滚后同原ID成功且只有1个新Outbox事件。合成JWT和本地生产者均不证明真实登录或外部发送。

定向快照覆盖等待期间的异步格式化，5个本批叶文件在前后快照变化，该区间认证PARTIAL；没有据此声称被测字节稳定。随后完整窗口盘点910输入，本批8个源码的前/后/交付SHA一致，共享仓储根、onboarding、另一共享测试及两份OpenAPI源共5项变化，整体认证PARTIAL。七叶固定入口格式检查PASS；严格Clippy实际仍FAIL（仓储lib234/lib test236、四适配器27），无本批诊断不代表包或API lint通过。OpenAPI窗口内打包/v2门禁通过，其后变化，当前契约认证PARTIAL。原日志、源盘点、指纹/精确量复算及失败范围见 [库位出库修复证据](../qa/acceptance/free-erp-inventory-location-outbound-repair.json)；本批没有应用新数据库DDL，也未验完整迁移/恢复。

**性能埋点与指标：** 沿ERP-P04/P05增加指定库位全可用、部分预占/冻结、不足但其他库位充足、空库位、非GOOD拒绝及同ID重放；1/10/100库位、10万SKU/100万流水背景，各首提交/重放/GET组≥1000请求，20路争用各≥100轮。首提交P95≤1秒/P99≤2秒，竞争成功或明确冲突P99≤3秒，原GET P95≤500ms/P99≤1秒；重复效果、串库位借货、非良品误出、超占和部分提交为0。沿上述函数分别计时准入、池等待、原命令/库位行/库位键/物理键锁等待、四桶SUM、权威精确效果、共享INSERT、Outbox、日志、双快照、commit及HTTP结束。保存实际查询计划、索引命中、连接占用、峰值内存及全部结果；七叶格式和一次debug20路正确性不计作Release性能通过。

**继续实施：** 完整小数输入、UUID批次、正式来源预占/释放/消费联动、销售交付与调拨在途、WMS/质量放行、其他库存写入；真实双端检查点与commit断线恢复、账号/席位/会话、完整迁移/恢复、公网和Release负载仍有独立门禁。I01/I02/O01/O02及免费首发继续保留未完成。

### ERP-IMPL-11 来源预占与释放

- 功能与字段：I02/M08。来源单/行、仓库/SKU、原预占量、当前余量、已消费/释放量及状态。
- 现有接续：`REPO/inventory_location_reservations.rs::{apply_location_reservation,verify_reservation_effect}`、`REPO/inventory_location_commands.rs::lock_location_quantities`、`REPO/inventory_movement_commands.rs::{execute,find}`；完整订单/工单继续接续 `REPO/fulfillment/store.rs::release_order_reservations`、`REPO/shared_contract/material_moves.rs::post_manufacturing_reservation`、`REPO/shared_contract.rs::reserve_manufacturing_stock`。
- 补齐位置：正式 `inventory_reservations` 与精确服务共用事务；订单取消、工单取消、领料和交付分别消费对应来源；规来源预占查询与UI明细。
- 事务与拒绝：锁原来源并复核剩余量；释放只扣预占，消费按对应业务同时处理实存；不能先直接改预占后再调用服务。已消费后同命令仍重放原结果。
- 验收与性能：`reservation_original_quantity_and_source_release`；X占0.5、Y占0.2，消费/释放X不动Y；改变原数量冲突；后行失败全回滚。P05/P12，原量与当前量分别读回。

#### 来源预占与部分释放

**当前接线（2026-10-03）：** `POST /inventory/reservations`、`POST /inventory/releases` 的原命令分支共同调用精确库存内核，来源记录使用现有 `inventory_reservations`，按库位建立来源与不可变事件。`GET /inventory/commands/{request_id}` 回查对应原命令；当前来源余量查询、销售订单及工单状态流转、来源消费、完整小数输入和双端页面另行接线。以下是本批具体实现和后续交付要求，验收状态以 [来源预占/释放修复证据](../qa/acceptance/free-erp-inventory-location-reservations-repair.json) 为准。

| 功能及实现位置 | 具体字段和行为 | 拒绝与验收方向 |
|---|---|---|
| 原身份：现 `crates/erp-api/src/inventory_print_report_handlers/stock_movement.rs::execute_source_inventory_command`、`shared_request_id` | 头/体单一请求编号、企业、实际用户、命令类型、仓库/SKU、正整数基础数量、三项库位维度、来源别名和可选订单UUID上下文；预占转正reserved delta、释放转负delta | 头/体不一致409；多头、空编号、无完整维度/来源400；未知字段422；同ID改变来源、数量、路由、上下文、SKU或库位拒绝，无新增效果 |
| 来源身份：现 `crates/erp-repo/src/inventory_location_reservations.rs::InventoryReservationSource` | `source_line`修剪后1—128 UTF-8字节，无控制字符；`order_item_id`/`reference_id`至少一个，两者同时提供必须一致；订单UUID只保留上下文 | 该接口未验证或驱动真实销售/生产单据；不得把引用字符串当作销售/工单联动已验。已提交来源不能用新ID再次加量，增量需明确新来源版本 |
| 正式来源表：现 `migrations/0286_inventory_location_reservations.sql` | `source_type=LOCATION_COMMAND`、`source_id`、`location_id`、`batch_code`、`quality_status=GOOD`、`source_context`、原reserve编号；`original_qty_micros`固定，`qty_micros`当前余量，`released_micros`累计释放；状态RESERVED/RELEASED | 企业/来源唯一，物理维度不可改；原量=当前余量+已释放，终态不重新启用；历史来源新增列为NULL，不伪造库位、原actor或回执。新来源的order_id/order_item_id不冒充正式销售关系 |
| 原事件：现同迁移 `inventory_location_reservation_events`；现 `crates/erp-repo/src/inventory_location_reservations.rs::{InventoryReservationEffect,verify_reservation_effect}` | 每个请求1条事件，记录来源ID、关联共享流水ID、signed micro delta、原提交时来源余量/累计释放/本库位总预占及时间；JSON精确量用十进制字符串 | 正式来源当前量可变化，原事件/原回执不可改删；另一连接按事件复算来源当前余量；重放及GET仍返回原提交结果 |
| 库位准入：现 `crates/erp-repo/src/inventory_location_commands.rs::lock_location_quantities`；现 `crates/erp-repo/src/inventory_location_reservations.rs::apply_location_reservation` | 按原命令→来源→库位ON_HAND键→仓库/SKU物理键串行，再锁正式来源行；指定完整库位键的四桶可用量=ON_HAND−RESERVED−FROZEN−UNAVAILABLE | 只GOOD可新增预占；精确量不足409，不能借他库位库存；释放必须匹配原来源上下文/物理键，数量≤来源余量；禁用库位禁止新reserve，允许释放已存在匹配来源 |
| 一次库存效果：现 `crates/erp-repo/src/inventory_location_reservations.rs::apply_location_reservation`、`crates/erp-repo/src/inventory_precision.rs::apply_precise_with_tx` | 预占/释放仅改变reserved，实存/冻结/在途delta均0；共享RESERVED流水引用同事务的权威流水；只有真实可用量变化且有有效渠道映射时产生Outbox | 来源、事件、权威余额/流水、共享投影、Outbox、operation/audit日志、SUCCEEDED快照共用一次commit；两个日志分别真实失败时全部回滚，确认后同原ID可成功 |
| 回执与HTTP缓存：现 `crates/erp-repo/src/inventory_movement_commands.rs::{execute,find}`；现 `crates/erp-api/src/middleware/idempotency.rs::match_idempotency_tenant` | 在原入/出/调整身份域增加RESERVE/RELEASE；可选字段缺省不进入旧v1哈希；新命令走事务回执，头/体两种重试都复核权限并返回原结果 | 不能让旧HTTP响应缓存短路权限/规范化回执；同ID混用20次头/体只有1效果；不同用户不能重放或读取他人原命令 |
| 当前权限：现 `crates/erp-api/src/inventory_print_report_handlers/inventory_commands.rs::{inventory_command_warehouse_access,inventory_command_response,inventory_command_error}` | 每次POST/GET复核当前仓库授权；响应包含原warehouse record、reservation_record、两个日志ID、权威effect_id、共享ledger_id、RESERVED桶 | 当前撤权403；他人原GET404；来源冲突409；来源余量不足409并返回十进制字符串 `remaining_micros/requested_micros`；越企业SKU403，授权SKU错误来源409 |
| 客户端与正式业务：规 `UI/pages/inventory/reservations.rs`；正式订单/工单、领料/交付和取消的原服务 | 主表按来源及稳定reservation_id，底部自研DataGrid显示不可变事件；分开展示原预占/命令结果与当前余量；保留待提交命令完整上下文，断线优先原GET | 真实登录、查询、编辑、保存、断线恢复和失败分别验桌面/Web。当前无对应正式页面或来源消费API；只读夹具不算完成；旧UUID来源的两POST日志共同提交见下节，其他旧消费者继续逐入口验收 |

请求示例（正整数2表示2件，内部2000000 micro，不能把quantity=2000000当2件）：

```json
{"request_id":"reserve-so-001-line-01-v1","warehouse_id":"11111111-1111-4111-8111-111111111111","sku_id":"22222222-2222-4222-8222-222222222222","location_id":"33333333-3333-4333-8333-333333333333","batch_code":"BATCH-001","quality_status":"GOOD","reference_id":"SO-001-LINE-01-V1","quantity":2}
```

释放使用相同来源/维度及上下文、独立原编号 `release-so-001-line-01-01`、quantity=1。原来源量2/当前余量1/已释放1；其他来源不变。再次同ID仍返回这次原结果，之后另一命令释放剩余1不替换前次结果。任一新ID释放已归零来源409。销售消费将来必须同时扣对应实存和来源预占且净可用量不重复变化；禁止先释放再另行出库制造中间可用库存/重复Outbox。

**验收执行位置：** 现 `crates/erp-api/tests/free_erp_acceptance/inventory_location_reservations.rs` 和 `inventory_location_reservation_edges.rs`，统一入口 `Scope api / IntegrationTest free_erp_acceptance / IntegrationFilter inventory_location_reservation`。9个来源用例覆盖权威入库后的预占、无ID结构化释放拒绝、两来源部分释放、后续变化下原结果回查、全部身份/来源键、20次头/体同ID、20个来源争用/20个不同ID释放、2个日志×2种命令的实际PostgreSQL拒绝、当前撤权/他actor/禁用库位，以及来源/事件改删和无事件释放的6项数据库门禁。另用保护身份、只读另一连接，记录全企业旧事实/原指纹、余额与流水、来源与事件、HTTP及Outbox。完整目标与独立结果在上述证据保存；debug正确性和文档检查不计入性能或线上验收。

**本批实际验证：** 两个原反例0通过/2失败（debug0.79秒）；首轮专项7通过/2失败（9.27秒），已保留原日志及样本，随后修复请求头绕入HTTP缓存，并使用企业内真实第二SKU验证来源键冲突。修后9专项通过/0失败/0忽略（11.08秒），另一连接3132项核对PASS、2101个既有QA企业选定事实不变。完整登记目标145通过/0失败/0忽略（90.53秒），独立3142项核对PASS、2111个既有QA企业选定事实不变。两组各10个新来源样本、10个原入库/14个原预占/6个原释放，共20个来源事件、120次HTTP；20次同ID混用头/体为1初次/19重放；20来源抢5件只5成功/15拒绝，20个新ID释放一个1件来源只1成功/19拒绝；两日志×预占/释放的4个实际check-option错误全部回滚，实际4个Outbox生产者事件正确。6次来源/事件改删/无事件释放均实际拒绝，另有缺movement_kind约束探针拒绝。

0286仅应用至保护的私有QA库；最初共享流水复合外键DDL失败并回滚，修正后应用v2，再补NULL上下文约束。独立核对2091个既有企业事实，6个新列仅增加NULL、没有伪造来源/事件，实际schema函数/约束/索引另读回留证。未运行完整部署迁移或恢复。

专项与完整窗口各914个源码/迁移/契约输入，13个本批关键源码及迁移前后SHA相同；共享仓储根、onboarding、共享测试及3份OpenAPI输入在测试窗口变更，整体认证PARTIAL。九叶固定入口格式检查PASS；严格Clippy仍FAIL，仓储lib235/lib test237、四适配器27项，未观测本批诊断不代表依赖或未运行到的API lint通过。OpenAPI打包/v2命令通过，输入窗口两源变化，契约认证PARTIAL。合成JWT、私有PostgreSQL及一次debug并发不能证明真实账号、性能、双端或上线。

**性能负载与埋点（目标，尚未验）：** 按ERP-P04/P05/P12，在10万SKU/100万流水、1万/10万有效来源及每来源1/10/100事件背景下，预占首提交/释放首提交/同ID重放/原GET各≥1000请求，20路同ID、20路不同来源抢有限量、20路不同ID释放同一来源各≥100轮。首提交P95≤1秒/P99≤2秒；争用成功或明确冲突P99≤3秒；原GET/重放P95≤500ms/P99≤1秒；超占、越源释放、重复效果、局部提交、错误原结果均0。索引为 `inventory_location_reservation_source_once`、`inventory_location_reservation_events_source` 及库位完整键；保存EXPLAIN ANALYZE实际扫描行数。沿上述函数记录池等待、命令/来源/库位/物理锁等待、四桶SUM、source row读取/写入、权威/共享流水、事件INSERT、Outbox、两日志、快照、commit和HTTP完整耗时；记录连接峰值、超时、错误率和内存。至少连续3轮使用统一入口登记的Release性能目标运行，不得把这批debug时长换算成P95或吞吐量，也不得借用BOM负载证明库存性能。

**继续完成：** 正式来源当前查询、完整小数请求、原领料/订单消费及取消、批次/质量转换、其他旧消费者的库存/日志共同提交、真实单账号多端检查点、完整迁移/恢复及官方公网负载。旧UUID来源两个HTTP写入口的接续见下节；I01/I02/T04/M08/M09/O01/O02与免费发布整项继续未完成。

#### 旧UUID来源预占与日志共同提交

**接口范围：** 原 `POST /inventory/reservations`、`POST /inventory/releases` 无request_id、无库位/批次/质量字段时，以企业内UUID `order_item_id`为旧来源身份。库存、来源及日志现在由同一仓储事务负责；本分支返回当前余额，没有持久化原命令回执，也不支持部分释放。正式双端新增写操作继续使用上节完整原命令协议。本节不把旧UUID引用视为已经验证的销售订单/工单关系，不为历史数据补造原actor、库位或快照。

| 功能及实际实现位置 | 输入、效果及提交边界 | 验收方向和失败后事实 |
|---|---|---|
| 请求字段：现 `crates/erp-api/src/lib.rs` 的 `InventoryReservationRequest`、`InventoryReleaseRequest`；现 `crates/erp-api/src/inventory_print_report_handlers/stock_movement.rs::legacy_reservation_source_id` | warehouse_id/SKU为UUID，quantity为正整数基本单位，转换micro不得溢出；order_item_id/reference_id至少一个，修剪后解析为非零UUID，同时提供须为同一规范UUID；预占另有可选非零order_id | 0、负量、i64溢出、零/非法来源、零订单和冲突别名400；大写UUID与小写同源合法。不能把quantity=1000000当1件 |
| 当前授权与actor：现同处理文件 `reserve_inventory`、`release_inventory`；现 `crates/erp-api/src/repo_ports.rs` 的 `InventoryRepo`、`InventoryRepoDb` | 两POST保留真实ctx.user_id和当前仓库权限检查，将actor传入仓储；重试不绕过当前授权 | 首次成功后撤销仓库授权，再预占或释放403，来源/余额/日志/Outbox均不增加；合成JWT验证不等于真实密码/席位/会话通过 |
| 外层事务：现 `crates/erp-repo/src/inventory_reservation_audit.rs::{reserve_for_order_item_with_audit,release_reservation_with_audit,begin_audited_reservation}` | 建立一次transaction，tenant+根来源advisory锁；仅此封装设置statement_timeout=10秒、lock_timeout=5秒。精确库存、来源行、激活记录、必要Outbox和两个必需日志均成功后才commit | 任一必需日志的实际数据库INSERT拒绝，整笔回滚；事务超时覆盖数据库等待，不覆盖取得连接前的池等待，不能用10秒超时声明端到端P99达标 |
| 内核与锁顺序：现 `crates/erp-repo/src/inventory.rs::{reserve_for_order_item_with_tx,update_reservation_with_tx,lock_reservation_keys,validate_reservation_input}` | 组合展开后按SKU/子来源排序；先对父SKU及全部子SKU排序/去重，复核企业归属并取得权威物理锁，再锁来源/余额；由精确InventoryService应用旧整数的完整micro效果 | 与原精确入库混合20路并发无死锁、无局部写。2.9实存−0.9预占−0.9冻结=1.1可用，只能再占1；20不同来源仅1成功，其余409 |
| 原来源匹配：现同库存内核；现 `crates/erp-api/src/inventory_views.rs::inventory_error_response` | 已存在来源的仓库、SKU、原数量及reserve的order_id必须一致；终态仍核对原键/数量，释放完整原量，qty/qty_micros保留原量 | 改变已占来源的订单上下文409 `reservation_mismatch`，终态来源不能重新reserve；本分支qty不是当前余量，不能套用LOCATION_COMMAND的余量列语义 |
| 必需日志：现 `crates/erp-repo/src/inventory_reservation_audit.rs::write_required_logs` | applied=true才写INVENTORY_RESERVED/INVENTORY_RELEASED及INVENTORY.RESERVE/RELEASE；绑定USER/真实actor、根ORDER_ITEM来源、请求仓库/SKU/原量/订单；result保留实际返回SKU/仓库及精确quantities | 同来源成功重试0新增日志，20reserve+20release总计各1次效果/1对日志。组合商品只1对根日志，result明确最后实际子SKU，不伪造父SKU余额 |
| 组合与故障：现 `crates/erp-api/tests/free_erp_acceptance/inventory_legacy_reservation_atomicity.rs`、`inventory_legacy_reservation_edges.rs` | 两个子SKU均有独立精确余额及渠道映射，分别在operation_logs/audit_logs INSERT处制造实际check-option拒绝；正常重试后两来源、两库存效果、1对根日志、实际Outbox | 故障前后两子项的全部来源、余额/流水、激活、日志和Outbox相等；不得只检查最后子SKU。本行是上一批共同提交验收；父请求原量、冻结组件及变更BOM接续按下节新批证据判断 |
| 证据读回：现 `qa/acceptance/free-erp-inventory-legacy-reservations-stream-readback.sql`、`free-erp-inventory-legacy-reservations-verify.py` | 保护QA身份下repeatable-read/read-only，逐企业JSONL输出完整事实；另一进程按精确流水复算余额、原量、日志、实际HTTP/Outbox及旧企业完整行集合SHA | nullable排序列可能并列，只对顶层记录集合规范排序，嵌套业务数组/全部值保留。业务occurred_at与transaction now()不代表锁后顺序；混合并发以原快照/日志精确前后数量组成完整状态链复核 |

**Grok继续接线：** 在既有库存仓储及正式业务调用方逐个补齐原请求编号与可回查结果，不把这两个旧兼容封装当作全部写入口已经统一。首次父SKU/请求量、可选order上下文、实际组件版本及子来源已按下节接线并完成私有QA；首请求与真实父订单行数量的关系、正式消费者、取消/消费竞争、当前来源查询及客户端恢复仍需分别完成。没有证据的历史根身份须另设核对流程，不能批量猜测回填。

**本批实际验证：** 原7反例全失败，登记目标3通过/7失败（debug2.54秒）；首专项16通过/1失败（9.92秒），修正原订单上下文的400/409映射后17通过/0失败/0忽略（9.74秒），独立3137项PASS/2311既有QA企业选定完整事实不变。首完整目标158/1（103.49秒）揭示成本命令锁漏调用，恢复现 `crates/erp-repo/src/shared_contract/item_cost.rs::lock_cost_command` 在关期、记账、重建、未知成本四入口的命令→商品/期间顺序；原成本历史用例1通过（0.15秒），另连接16项精确原成本/分录顺序核对PASS。随后完整159通过/0失败/0忽略（99.46秒），独立3342项PASS/2516既有QA企业选定完整事实不变。两个成功窗口各14本批新企业、110次合成JWT/Axum真实路由请求、24来源、54权威流水、34对必需日志、10原入库回执及10实际Outbox。6次指定日志view拒绝均全回滚（包含两组件的两次故障）；20路同来源各1reserve/1release效果、20来源抢1.1只1成功，10原入库+10预占本次全部成功。原失败样本/日志未修改，详见 [本批修复证据](../qa/acceptance/free-erp-inventory-legacy-reservations-repair.json)。

917项实际输入全部记录；完整窗口11本批Rust输入稳定，但6个其他依赖/契约输入变化，整体PARTIAL。八叶局部固定格式分别通过；严格Clippy仍FAIL（仓储236/238、四适配器27），依赖或未到达API lint不能算通过；OpenAPI仅其独立采集区间打包/v2门禁PASS。旧整库读回发生断连并留下0字节失败文件，保留后改保护身份的只读逐企业JSONL，另进程验证旧行集合；没有数据库重启、清理或历史来源回填。首核对错用锁前时间排序，失败报告/脚本保存后改完整精确前后状态链，全部数量/日志断言保留。本批不新增DDL，私有QA/debug未验真实登录、双端、恢复、公网或性能。

**性能指标与实际位置（ERP-P04/P05/P12，目标）：** 在10万SKU/100万流水、1万及10万活动旧来源背景下，单SKU预占首提交、全量释放首提交、同来源重试各≥1000次；1/10/100组件的组合来源分别测量。20路同来源、20路不同来源抢1/5/100件、10路精确入库+10路预占各≥100轮、至少3轮Release。单SKU首提交P95≤1秒/P99≤2秒，重试P95≤500ms/P99≤1秒，争用成功/明确拒绝P99≤3秒；100组件共同提交P95≤2秒/P99≤4秒；超占、重复日志、局部提交、误扣其他来源和死锁均0。分别沿上述仓储函数记录连接池等待、根来源锁、物理锁、组件查询/展开、来源/余额锁、精确效果、Outbox、两日志、commit及完整HTTP耗时。复核现有(tenant_id,order_item_id)唯一索引与组件查询索引的EXPLAIN ANALYZE/扫描行数，记录池/连接峰值、失败类别、峰值内存；锁超时和业务拒绝纳入结果分布。此批debug正确性、源码存在及BOM的Release结果不代替该库存负载；必须经统一构建入口登记对应Release性能目标后运行。

### 组合根请求和冻结组件

**功能归属与边界：** I02及T04/M08/M09共用库存底座的旧UUID分支。现有两个POST地址不变，quantity仍为正整数基本单位；本节补首次接受的父SKU/请求原量、可选订单上下文、实际组件与子来源冻结。原命令编号、部分释放、当前来源查询、真实订单关系及正式UI仍按各自实施单元交付。套装sku_boms与order_item_components不是制造BOM，不能将本批改名为制造工单验收。

| 具体功能 | 实际实现位置和提交规则 | 验收方向 |
|---|---|---|
| 冻结首次根请求 | 现 `REPO/inventory_order_reservations.rs::resolve_order_reservation_items`；现 `REPO/inventory.rs::reserve_for_order_item_with_tx` 委托解析，在同一事务创建根、组件、子来源及精确库存 | 同根量从1改2、父SKU换成另一套装、仓库或reserve订单上下文改变均拒绝；子组件相同不代表另一个父项可借用原来源 |
| 冻结实际组件 | 现 `REPO/inventory.rs::{expand_order_item,ReservationExpansion,ReservationItem,component_order_item_id}`；优先读原order组件实际总量/版本，否则首次读SKU套装组件；现 `REPO/inventory_order_reservations.rs::FrozenReservationItems` 返回原清单 | 根请求只首次展开；子ID按根UUID与SKU原始字节计算UUIDv5；BOM新增第三SKU不能追加预占，删除原组件不能遗留旧预占 |
| 按原清单释放及仓储消费 | 现 `REPO/inventory.rs::update_reservation_with_tx` 在物理键锁之前取得原清单；原HTTP释放仍由现 `REPO/inventory_reservation_audit.rs::release_reservation_with_audit` 包装 | 删除/替换BOM或重分类后，释放原子SKU；公开仓储consume及重放只有一次原组件效果。正式销售的私有消费函数不因本测试而通过 |
| 保全历史直接来源 | 无根快照时，现根解析只承认既有自然根ID的直接来源行，核对仓库、SKU、原量及reserve订单上下文 | 原NORMAL改KIT后仍能释放原直接来源；关联保持NULL，不补造父量/原actor/快照；另一新根的已关联子项不能冒充历史直接来源 |
| 拒绝无证据历史组合 | 现 `REPO/inventory_order_reservations.rs::refuse_unverified_legacy_children`：只扫描企业内未关联旧来源，500行稳定ID游标；逐行用真实SKU重算子ID，不依赖现BOM | 原BOM删除后，旧子来源仍被识别；预占/释放409 `reservation_source_reconciliation_required`，库存、日志及来源不改写；缺历史根信息保持待核对 |
| 共同提交及并发 | 原HTTP外层根锁→新自然根锁→父/子SKU去重排序物理锁→来源/余额锁→效果/Outbox/两日志→commit；公开仓储方法从新自然根锁开始。复用现 `REPO/inventory.rs::lock_reservation_keys` | 后一组件不足，以及两个日志×预占/释放的4次实际INSERT故障全组回滚；20同根各动作只有1效果，20不同根抢每子SKU1.1件只有1个完整根成功 |
| 错误与公开契约 | 现 `API/inventory_views.rs::inventory_error_response`；现 `openapi/src/components.yaml`、`openapi/src/paths.yaml` 由现 `scripts/openapi_bundle.mjs` 生成公开契约 | 409保留根冲突/非活动/待核对码；已展开组件超过1000返回422 `reservation_components_limit`且0部分效果；旧释放无原来源404；成功仍为当前record |
| 数据库不可变及归属 | 现 `migrations/0287_inventory_order_reservation_sources.sql`：根及清单禁止UPDATE/DELETE，来源关联禁止更换/清空；延迟FK和完整性触发器在commit核对子来源原量、订单及物理键 | 10个实际探针覆盖根/组件改删、清空关联、改子SKU/订单、额外子行、不完整根及零根UUID，全部拒绝；失败后完整事实一致 |

**具体字段与数量：** 新根表inventory_order_reservation_sources保存source_id、tenant_id、root_order_item_id、warehouse_id、首次sku_id、可选order_id、original_qty、component_count、definition_kind及created_at。definition_kind为DIRECT、ORDER_COMPONENTS或SKU_BOM。新清单表inventory_order_reservation_components保存企业/根、真实子order_item_id、sku_id、实际总qty及可空bom_version。inventory_reservations.reservation_source_id为新可空关联，旧数据保留NULL。

请求1仍是1基本单位，子qty_micros及original_qty_micros为1,000,000。根原量与每子SKU实际总量分别存储；原order组件量已经是总量，不能再乘父请求量。根及子整数转换micro都检查范围，迁移允许原量1至9,223,372,036,854。旧来源终态qty保留原量，不能作为当前可释放余量；清单保存实际旧套装版本整数，不虚构制造版本身份。冻结首次请求量只证明后续重放不能改量，尚不证明首请求量等于真实销售订单行的权威数量。

**索引与资源位置：** 根使用企业+根UUID唯一索引，清单使用企业+根+SKU主键及企业+子UUID唯一键；既有物理来源仍使用企业+子UUID唯一键。inventory_unmapped_order_reservations是关联NULL且有子UUID的部分索引，支持500行旧源游标；inventory_order_reservation_children是关联非NULL的企业+根反向索引，避免同根来源校验扫描企业全部来源。首次组件查询/原清单查询最多读1001行，用以辨认1000上限。历史探查分页只控制内存，总扫描时间仍计入端到端；延迟完整性检查随组件量重复执行，须测100/1000组件commit成本，不能以有索引推断线性耗时或P95达标。

| 性能场景和目标，均待Release验收 | 固定负载 | 实施计时和失败后事实 |
|---|---|---|
| ERP-P04/P12：单SKU首次P95≤1秒/P99≤2秒；原根重放P95≤500ms/P99≤1秒 | 10万SKU/100万流水；分别1万及10万已关联、未关联旧来源；每种动作≥1000次，3轮Release | 沿根解析/展开及库存with_tx记录池等待、两个根锁、物理锁、旧源扫描页/行、组件读取、库存/日志及commit。新根扫描全部未关联来源的耗时不能剔除 |
| ERP-P04/P12：100组件共同提交P95≤2秒/P99≤4秒 | 1/10/100组件各≥1000次；1000作为资源上限单独测量并报告分位数，不沿用100组件成绩；1001实际拒绝 | 任意末行失败0部分根/清单/库存/日志/Outbox；记录延迟触发器时间、扫描行数、SQL次数、峰值连接/内存及事务持续时间 |
| ERP-P05：争用成功/明确拒绝P99≤3秒；超占、重复效果、死锁均0 | 20同根、20不同根争1/5/100件，以及原精确入库和预占混合，各≥100轮；正式取消与消费20路另验 | 成功和冲突都计入分布，超时单列；原API statement_timeout=10秒/lock_timeout=5秒不是端到端承诺，也不包含池等待 |
| 性能脚本及执行入口 | 规 `crates/erp-api/tests/free_erp_inventory_reservation_performance.rs`；现 `scripts/Invoke-ErpBuild.ps1` 待登记该目标及Release/RunPerformance准入 | 当前仅BOM性能目标已登记，不能用它冒充本库存负载，也不使用原始cargo绕过入口；独立连接复算流水、来源、根清单、日志和生产者事件，记录EXPLAIN ANALYZE及资源环境 |

**具体样本：** 每原子SKU实存2.9、预占0.9、冻结0.9，可用1.1。父A根R请求1，两子各占1后余量均0.1。BOM将第二子替换第三子，版本改8，第三子另有5件；重放R不得占第三子或新增根日志。量改2或父换B均409。释放R后原两子预占各回0.9，第三子仍未预占；BOM全删也按原清单释放。无根快照的历史两子保持原预占并返回待核对，不能把当前BOM为空解释成直接来源。

**本批实际证据：** 原五反例0通过/5失败（debug1.67秒），原源码/10次实际路由/失败事实保留，独立8166项PASS/2703个既有QA企业选定完整事实不变。修后15专项通过/0失败/0忽略（10.86秒），独立9448项PASS/2708个既有企业事实不变；完整登记174通过/0失败/0忽略（120.74秒），独立9689项PASS/2726个既有企业事实不变。两个成功窗口各18本批新企业、114次合成JWT/Axum实际HTTP、35精确余额、80权威流水、14根/27冻结组件/30物理来源、21对日志/39个真实Outbox。独立核对只对顶层记录集合规范排序，保留所有列及嵌套数组；原量/子ID/完整库存/当次日志及生产者效果均可复算。

4次指定日志check-option故障及后序SKU不足均全组回滚；20同根三组含改量冲突，20不同根仅1成功；10次实际数据库改删/不完整关联均拒绝。公开仓储消费原两子只产生2条SALES_SHIP，重放0新增，不代表真实销售/工单消费或取消竞争通过。初次独立核对漏算10条释放时可用增加的生产者事件，失败报告/脚本保留后，按实际生产者和精确流水证明39条并补逐类别断言，没有修改库存或删除回滚断言。

0287只应用保护QA，首DDL成功后补根子来源反向索引，原DDL/源码与追加索引脚本分别留证。迁移独立8899项PASS：2708原企业选定完整事实仅新增关联NULL，759旧来源仍未关联，0历史根/组件回填。现 [只读采集](../qa/acceptance/free-erp-inventory-kit-sources-stream-readback.sql)、[独立核对](../qa/acceptance/free-erp-inventory-kit-sources-verify.py) 与 [修复记录](../qa/acceptance/free-erp-inventory-kit-sources-repair-v2.json) 保存原失败、迁移、成功区间及SHA。首文档核对发现常量声明未纳入位置解析，以及销售退货GET新增后仍按旧路径数检查；原失败与核对器版本已保留，修订继续精确比较7路径/10操作和具名常量。修订证据重新枚举当前完整输入，不用旧文件名单掩盖新增或删除。

921实际输入完整盘点，两个成功窗口8本批Rust/迁移输入稳定；专项6项、完整2项其他共享源码/契约变化，整体PARTIAL。五叶固定入口格式PASS；严格Clippy仍FAIL（仓储236/238、适配器共27），API lint未到；未观察到本批诊断不算依赖链通过。OpenAPI独立区间6个源/工具稳定，打包/v2门禁PASS，生成文件变化为预期重建。构建既有增量缓存“拒绝访问”提示留在原日志，命令/测试仍结束成功，未清理或绕开缓存。私有QA/debug不代表官方托管、真实登录/多端、完整升级恢复或Release负载。

**Grok剩余具体接线：** 现 `REPO/orders/store.rs::insert_bundle_components` 与真实订单行/父量、可选order上下文共同验证，并拒绝数量乘法溢出；现 `REPO/fulfillment/store.rs::consume_reservations` 等正式消费者接同一原清单/事务，单验取消、消费和部分执行竞争。规 `REPO/inventory_source_queries.rs`、`API/inventory_source_handlers.rs` 提供授权下当前根/子来源/事件查询；规 `UI/pages/inventory/reservations.rs` 用DataGrid分别显示父请求原量和实际子SKU量、当前来源和原命令回查，保存冲突草稿。历史待核对须真实来源证据、当前认证复核人及独立审计，不能直接改NULL关联或生成猜测根；核对流程未实现。以上接线、Release及正式双端仍是I02/T04/M08/M09结项条件，本批不勾选整项完成。

### 正式订单来源分配与履约

**功能归属：** I02、T04及ERP-IMPL-11/20。当前接续的是原OMS订单和发货单：orders/order_items/order_index、shipments/shipment_items、inventory_reservations及order_cogs。普通贸易的sales-orders交付、生产工单领料仍有各自接线任务，不能用OMS专项结果替代。以下“现”表示当前源码位置，测试状态在本节末单独记录。

#### 实际接口和输入

完整前缀为 `/v1/tenants/{tenant_id}/oms`；企业、认证使用者、权限及会话来自服务端认证上下文。订单原量和SKU读取正式order_items，客户端不能通过分配请求自报父量或实际组件。原整数1继续表示1基本单位。

| 具体动作 | 已有接口和API位置 | 实际输入及成功含义 |
|---|---|---|
| 分配库存 | POST `/orders/{order_id}/allocate`；现 `API/orders/fulfillment.rs::handle_oms_allocation` | 当前接收可选carrier_code；按正式订单行分配。返回ALLOCATED及真实shipments，或SHORTAGE及缺量；SHORTAGE不得留下部分预占或半张发货单 |
| 查看订单发货单 | GET `/orders/{order_id}/shipments`；现 `API/orders/fulfillment.rs::list_oms_order_shipments` | 按本企业/订单返回发货单及明细；现列表上限200属于公开读取契约。内部重放另读4097识别4096资源上限，不以公开首200条冒充完整来源集 |
| 确认发货 | POST `/shipments/{shipment_id}/ship`；现 `API/orders/fulfillment.rs::ship_oms_shipment` | 可选carrier_code、tracking_no、shipped_at、weight_gram；消费该单真实仓库、订单、子SKU和原量。只有实际来源消费与成本/发货状态一同提交才算成功 |
| 释放订单预占 | POST `/orders/{order_id}/release`；现 `API/orders/fulfillment.rs::handle_oms_release` | 按原订单已应用的物理来源释放；当前released为本次实际释放的来源行数。同终态再次释放为0，不代表原命令结果快照 |
| 补发 | POST `/orders/{order_id}/resend`；现 `API/orders/fulfillment.rs::handle_oms_resend` | 当前每次请求生成新补发命名空间；沿原组件意图建立新库存来源。持久化request_id、原补发回执、单号不碰撞及重复请求准入另补，不能报告网络重试安全 |

当前发货权限检查含orders.write和shipping.write，分配还检查inventory.write；库存和发货feature flag沿用原设施。**待补**：在正式写事务内再次验证当前授权仓库，并把读列表、详情、默认分仓和重放全部限制在该范围；不能先扣无权仓库存，再在响应处过滤。现界面 `UI/remote_view/orders.rs` 继续作为原订单宿主；规 `UI/pages/trade/deliveries.rs` 承载贸易交付，所有主表、组件明细和来源选择均接DataGrid。菜单/页面注册改现 `UI/page_registry.rs` 与 `UI/erp_app.rs`，原生和Web各自实际登录、提交及回查。

#### 函数接线和数据库位置

| 必须完成的行为 | 现有实现位置 | 数据及失败后的事实 |
|---|---|---|
| 同订单串行准入 | 现 `REPO/fulfillment/store.rs::{allocate_order_internal,lock_fulfillment_order}` | 先开事务并取企业+订单自然锁，再读原订单和现发货单；重放读取复用当前事务连接，不能在20个事务占满池后另借连接造成池等待环 |
| 从真实父行建立计划 | 现 `REPO/fulfillment/store.rs::{fetch_order_header,fetch_order_items,build_reservation_plan}` | 真实父SKU、line_id、qty及订单来自数据库；父行最多1000，取1001识别超限；SKU缺失、无效量、重复根明确拒绝，不能静默跳过无映射行 |
| 保留原组件定义 | 现 `REPO/inventory_order_reservations.rs::{frozen_order_expansion,resolve_prepared_order_reservation_items}`；现 `REPO/inventory.rs::reserve_frozen_order_item_with_tx` | 已有根校验父SKU/原量/订单后读原清单；新根首次展开并冻结。组件量是实际总量，不再次乘父量；无证据历史组合仍要求核对 |
| 跨组统一物理锁 | 现 `REPO/inventory_order_consumers.rs::{lock_order_roots,lock_order_keys}` | 全订单根UUID排序，再把所有候选仓库×父/子SKU去重排序，取得统一精确库存锁；最多4096个物理键。不得先锁第一组再以不同顺序补锁第二组 |
| 整单不足回滚 | 现 `REPO/fulfillment/store.rs::reserve_item_group` 及allocate_order_internal整单保存点 | 第二组不足时回滚本次所有根、组件、来源、精确预占、流水和库存Outbox；缺货状态按原订单更新。候选仓库试探失败同样回滚本次试探，已有历史效果保持原状 |
| 原分配重放复核 | 现 `REPO/fulfillment/store.rs::{existing_order_shipments,validate_existing_allocation}` | 原明细必须完整匹配计划、源仓库/SKU/订单/原量和活动状态；父量改变或已释放不能靠旧发货单直接返回成功；不同发货单不得重复领取同一CONSUMED来源 |
| 发货与释放共用原来源 | 现 `REPO/fulfillment/store.rs::{consume_reservations,release_order_reservations}` →现 `REPO/inventory_order_consumers.rs::apply_order_terminal_items` | 逐行复核源上下文、旧整数/精确量、根关联及组件映射。消费同时减on_hand和reserved，释放只减reserved；冻结/在途不变，库位命令来源不能冒充无维度订单来源 |
| 批次与成本 | 现 `REPO/inventory.rs::consume_batches_by_fefo` 委托现 `REPO/inventory_batch_consumption.rs::consume`；现 `REPO/fulfillment/costs.rs::insert_shipment_costs` 接原ship_oms_shipment | 开启批次的SKU按精确微单位扣减，实际企业/SKU/仓库及效期复核；成本必须已知且非负、乘法检查范围、全部成本行必须插入。任一成本/批次失败，原发货核心事务回滚；历史成本来源、批次消费追溯及其他写入口继续独立验收 |
| 错误返回 | 现 `API/inventory_views.rs::inventory_error_response` 被原OMS处理器复用 | 409 reservation_mismatch / reservation_not_active / reservation_source_reconciliation_required、404 reservation_not_found、422 reservation_components_limit；实际数据库/事务故障不伪装成功 |

根与组件表及索引沿现 `migrations/0287_inventory_order_reservation_sources.sql`，物理来源保持原inventory_reservations唯一身份。正式订单表和order_index沿现 `migrations/0001_init.sql`；shipments/明细沿现 `migrations/0007_shipping_schema.sql`、`migrations/0022_oms_fulfillment.sql` 及 `migrations/0039_oms_resend_cancel.sql`，成本沿现 `migrations/0045_cost_ledgers.sql`。这些迁移名称须核对现文件再接续；保护QA的兼容准备脚本只建立缺失空表、可空列及必要索引，不回填历史订单状态、原actor或根来源。完整注册升级、未完订单迁移和恢复仍另验。

#### 可执行样本和断言

| 验收场景 | 固定输入或操作 | 独立连接必须看到的结果 |
|---|---|---|
| 真实父量冲突 | 正式订单父量1首次分配后改2，再分配原订单 | 409且0新增来源/库存/发货单；冻结原量仍1，不把相同子SKU解释为另一个请求 |
| 删除BOM后发货/释放 | 父A原有B/C各1，分配后删除BOM或重分类，再发货或取消 | 只消费/释放原B/C；父A及新组件无库存效果，根和清单不可变 |
| 错仓和错订单 | 分配后把测试发货单改为另一实际仓库或另一订单；该仓本身有货 | 409且库存、来源、成本、发货状态、日志和Outbox均无新增；别仓有货不能证明来源正确 |
| 发货明细错上下文 | 测试发货单头保持原订单，单行改为另一订单或改变数量 | 409整单回滚；不能只校验头部而忽略真实明细order_id |
| 同发货单重放/新单借用 | 原单已SHIPPED；原单重试，再创建另一新单指向相同来源 | 原单重放返回已提交结果且0新效果；新单409，不能新增成本或第二条发货成功事件 |
| 后序SKU缺货 | A可用1.1、B可用0；订单每项1，A先完成预占后B不足 | SHORTAGE，A的精确预占仍0.9；本次根、组件、来源、库存流水、Outbox及发货单均0新增 |
| 20路同订单分配 | Barrier同时发20个原接口请求，全部请求结束后才判断 | 20个结果同一shipment_id，只有1个根/来源与1条预占效果；不得首个失败就中止其余请求后声称完成20路 |
| 20路发货/释放竞争 | 10个发货、10个释放同时提交同一已分配订单 | 只能消费或释放一次；若消费胜，实存1.9/预占0.9，若释放胜，实存2.9/预占0.9；失败分支0成本和0库存效果 |
| 必需日志及事件故障 | 分配/整量释放/发货各operation/audit及发货SHIPMENT_SHIPPED共7个实际INSERT故障 | 原OMS核心事实整单回滚；撤掉故障后重试成功、再次重试0新记录。取消申请、waybill及外层WMS/售后各另验，不混作本项已通过 |
| 状态更新没有效果 | 分配后测试单status_version=100，再正式发货 | guarded UPDATE无返回行时409；原来源仍RESERVED，0发货流水/COGS/成功事件，不以执行SQL成功当状态提交成功 |
| 原发货时间 | 请求时间2026-10-03T15:00:00.123456789Z，随后原单重试 | 首次回执、事件及重试均引用PG保存的同一微秒时间；updated_at按实际提交行返回，不能复用请求发货时间或重建纳秒 |
| 等待期间撤权 | 固定订单锁使HTTP在认证之后等待，另连接提交范围替换，再释放锁 | 403且完整事实不变；不能只用中间件较早读取的范围 |
| 授权事务先入场 | 固定来源根锁，确认发货已取得actor共享锁，再调用实际replace_user_warehouses | 范围写事务在真实user行锁等待；发货先提交，范围变更后原单重放403且0新效果；两个任务全部结束再断言 |

保护库测试现位于 `crates/erp-api/tests/free_erp_acceptance/fulfillment_sources.rs` 和 `crates/erp-api/tests/free_erp_acceptance/fulfillment_atomicity.rs`，加入已登记free_erp_acceptance；只读完整事实采集现 `qa/acceptance/free-erp-fulfillment-readback.sql`，独立核对现 `qa/acceptance/free-erp-fulfillment-verify.py` 及本批 `qa/acceptance/free-erp-fulfillment-atomic-verify-v4.py`。核对范围含32张选定表的全部列，另复算精确库存流水、根/组件/源身份、COGS、真实操作者及事件和落库时间；精确时间比较保留纳秒，不能先转PG微秒把原差异抹掉。嵌套JSON数组保持原内容，不能删除失败字段或合并不同来源使结果通过。

#### 当前账号、仓库和必需记录

正式OMS的提交身份现由 `API/auth_context.rs::AuthContext::fulfillment_actor` 从认证扩展和RequestContext构造，8个HTTP调用处显式 `.with_actor(...)`：原提交出库、分配、补发、发货、释放、退回审单及售后换货/补发。请求JSON不得提供或覆盖actor、认证企业或admin。现 `REPO/fulfillment/context.rs` 的 `FulfillmentUser` 保留authentication_tenant、user_id、is_admin、request_id、trace_id、IP/UA；request/trace只是追踪信息，仍不能冒充FUL-03持久化命令身份。

| 具体功能 | 当前实现位置与事务顺序 | 必须验收的事实 |
|---|---|---|
| 当前使用者状态 | 现 `REPO/fulfillment/context.rs::FulfillmentActor::authority`；取得订单自然锁后按认证企业/user_id执行users FOR SHARE，必须ACTIVE | nil、未知、不同企业非管理员、停用账号均拒绝；失败0来源/库存/成本/日志/事件效果。多设备可持有兼容的共享锁，不因同账号变成单会话 |
| 当前仓库范围 | 同一authority查询user_warehouse_access，按warehouse_id排序FOR SHARE，上限10000/read10001；现 `REPO/data_scopes.rs::replace_user_warehouses` 先锁真实user再整体替换 | 沿用已有“无配置范围=全部仓库”政策；撤销最后授权必须改成明确限制集或停用账号，不能把空集描述成拒绝全部。事务等待期间已提交的范围变更也必须生效；先取得授权共享锁时，范围替换须等待该业务事务结束 |
| 分配和缓存重放 | 现 `REPO/fulfillment/store.rs::allocate_order_internal` 用当前范围筛候选仓，复核冻结源仓及原发货单仓 | 无授权候选返回403；有权仓缺货按真实缺货处理，不跨到无权仓。原成功分配再撤权后，重放也拒绝且0新日志 |
| 发货和整量释放 | 现store的ship_oms_shipment/release_order_reservations核对实际单仓/全部物理源仓；原发货API另要求inventory.write | 不以请求仓或当前余额代替来源仓；已SHIPPED重放和已RELEASED释放仍受当前范围约束。旧JWT缺库存写权限不能提交发货 |
| 共同记录 | 现context的record/operation及store的record_allocation；复用operation/audit insert_with_tx和Outbox enqueue_with_tx | 分配：一条订单操作/审计及每张新发货单创建操作；释放：仅本次真实释放>0写操作/审计；发货：库存、来源、COGS、状态、SHIPMENT_SHIPPED和两日志一次提交。每个必需INSERT失败均整笔回滚 |
| 真实操作者与重放 | USER操作绑定认证user_id；审计保留IP/UA，主记录metadata保留request/trace/认证企业。无用户的内部调用明确SYSTEM/internal.fulfillment | 不从订单或库存来源补造用户。缓存分配、已发货和已释放重放不新增“创建/成功”日志；校验独立连接的全部列，不只计数 |

403错误为fulfillment_access_denied，10001条范围为422 fulfillment_scope_limit；数据库/必需记录失败沿internal_error返回500并撤销业务事务，不能先成功再吞错。旧无授权范围政策、管理员认证企业及跨企业访问语义必须在实际部署配置中单独复核。事务当前重查的是账号状态与仓库范围；功能权限沿原JWT/处理器检查，session_id/credentials_epoch仍在原认证中间件处理，不能声称角色撤销或会话撤销已与库存事务原子绑定。内部worker没有真实使用者，其无用户执行策略与业务授权仍需另验。

原OMS核心事务的原子记录不等于整个“提交出库/售后/退回审单”外层工作流原子化：这些调用后续WMS单据、售后关联、外层专用审计仍有独立事务；发货后的waybill使用标记仍是外层best-effort，需另立实际故障样本。现原命令没有原不可变回执/GET；补发仍生成新命名空间，断线后不能盲目重发。上述缺口保留FUL-01/03及对应工作流门禁，不以本节接线宣布全部授权、业务闭环或免费上线完成。

#### 真实成本与精确批次扣减

以下为2026-10-04正式OMS发货的接续实现，不等于贸易sales-orders交付、制造计价或全部批次写入已完成。原数量为整数件，1仍转换为1000000微单位；共享批次中的0.5、0.8等可参与同一次1件发货，不能把旧整数1改成1微单位。

| 具体功能和实施位置 | 输入、锁及提交规则 | 验收结果和错误 |
|---|---|---|
| 已知成本，现 `REPO/fulfillment/costs.rs::FulfillmentCostError` 和 `insert_shipment_costs` | 原来源消费取得根/物理锁后，按实际发货明细的企业/SKU读取cost_cent，SKU排序FOR SHARE；明确0合法，NULL/缺行和负数拒绝 | 409 fulfillment_cost_unavailable/fulfillment_cost_invalid；0库存、来源、批次、成本、状态、日志及事件效果。原请求的发货单保留CREATED，补齐真实成本后才可重试 |
| 检查金额和原成本行，同一costs叶模块 | 正整数qty×非负unit_cost_cent使用checked_mul；现order_cogs继续保存整数件和分。已SHIPPED在原头部识别重放；新提交发现原SHIPMENT_ITEM成本行则拒绝，INSERT不使用ON CONFLICT DO NOTHING | 溢出422 fulfillment_cost_overflow；冲突409 fulfillment_cost_conflict；必须写入全部成本行。合法零成本与成本目录之后变NULL的历史重放分别验收；不能由当前目录重算原已提交成本 |
| 精确批次，现 `REPO/inventory_batch_consumption.rs::consume` | 沿物理库存锁，当前batch_enabled读取FOR SHARE；按稳定batch_id锁实际inventory_batch_balances，最多读1001行。再按企业/SKU/批次ID读取并共享锁定真实sku_batches | 1001行422 inventory_batch_limit且不截断；归属错误、未知精度和数量不一致409 inventory_reconciliation_required。不能由旧整数余额补造历史精度，也不能只按batch_id跨企业JOIN |
| 完整数量核对和FEFO，同一consume | 批次总实物须等于同事务SKU扣减后的实物+本次量；批次预占≤扣减后的SKU预占。每批精确实物/预占及旧整数投影有效；取得元数据锁后用PG clock_timestamp作为决策时间，效期升序、无效期末尾、同效期稳定ID | 只扣未预占且决策时未过期的精确量；不足409 inventory_insufficient。SKU 2.9/原预占0.9，发1后实物1.9；早批0.6/预占0.1扣0.5，晚批1.3/预占0.5扣0.5，过期批1.0/预占0.3不变 |
| 精确落库和整单回滚，现consume及原store核心事务 | 已验证的FEFO分配用有界VALUES一次更新微单位，原值条件及实际影响行数共同复核；现0283批次守卫生成整数投影。批次后成本、订单状态、必需记录和成功事件仍同一事务 | 后序SKU批次不足时，前序SKU、全部成本、发货状态、日志和事件撤销；同发货单重放不重复扣批次。只执行SQL不算更新完成，行数不匹配拒绝 |
| 接口错误和公开契约 | 现 `API/inventory_views.rs::inventory_error_response`；现 `openapi/src/paths.yaml` 原ship路由同时要求shipping.write/inventory.write，声明409/422/500，沿现bundle入口生成契约 | 桌面/Web保留草稿并呈现真实冲突；不得遇409/422后换请求身份盲发，不能把500显示为提交成功。正式客户端和公网仍另验 |

实际HTTP用例位于现 `crates/erp-api/tests/free_erp_acceptance/fulfillment_cost_batches.rs`，14个固定场景加入原free_erp_acceptance。原代码2通过/12失败（6.81秒），独立136090项核对确认NULL→0、负成本、溢出饱和值、冲突成本行被静默跳过、未知精度批次被整数路径发出，以及合法精确批次不能完成；4119个旧QA租户的32张完整选定表不变。证据见 [成本批次原反例](../qa/acceptance/free-erp-fulfillment-cost-batch-original-independent.json)。修后定向14通过（6.74秒），独立136767项PASS、4137个旧租户不变；完整登记216通过/0失败/0忽略（154.99秒），独立137328项PASS、4154个旧租户完整事实不变，见 [当前完整回归和独立事实](../qa/acceptance/free-erp-fulfillment-cost-batch-full-independent.json)。六叶固定入口格式检查通过。928个输入前后盘点，本批9项稳定；完整运行窗口另有manufacturing、shared_contract测试及三份OpenAPI输入变动，资格PARTIAL。严格Clippy及正式客户端、全批次写路径、Release和公网仍另验，不以debug用例时间当性能达标。原核对器的修复前版本单独保留；最终溢出拒绝样本的预期初始实物为2.9+2=4.9，核对器修正这个算术常量，未改业务原始日志或事实。

严格API范围Clippy再次被仓储库232项与四适配器共27项诊断阻断，API lint未到；本批9项输入未观察到诊断，不作为依赖链通过。当前公开契约源/生成的原ship操作、两权限、8种状态及8个具名错误静态核对PASS，见 [接口契约记录](../qa/acceptance/free-erp-fulfillment-cost-batch-openapi.json)；Clippy和静态契约都不能替代真实客户端或Release。仍需接续：现inventory批次入库/调整及制造material_moves等其他写路径，原批次来源迁移/核对政策，每次实际消费的不可变批次追溯，成本方法/期间及退款原成本，批次启用/效期更新竞争、末时效期政策和真实客户端操作。当前SKU预占没有对应的批次来源身份，本路径仅消费明确未预占批次量；不能从批次已预占量中猜测本订单份额。FUL-04整项保留未完成，缺成本拒绝属于当前政策，不宣称已有待核算/暂估工作流。

P04/P05/P12沿下表，额外分组为batch_enabled关闭/开启、1/10/100/1000批次、无/临期/过期批次、成本正常/零/缺失/溢出/冲突、同SKU/不同SKU各20路；每成功动作≥1000样本，竞争≥100轮，三轮Release。分别记录物理锁、SKU成本/批次配置锁、批次行与元数据锁、完整精度/归属预检、VALUES更新、COGS插入、两类日志/事件及commit，记录扫描/返回行数、SQL次数、内存和连接占用；拒绝与合法重放单列。1001批次必须明确超限且0部分效果，不能混进成功延迟。当前新查询和锁的成本尚未量化，不宣称已达P95。

#### 性能任务和超限处理

所有下列数值为**发布前目标，未运行Release则标NOT_RUN**。10万SKU、100万流水及1万/10万历史未关联来源分别测试；包含实际订单读取、权限、池等待、旧源探查、锁、库存、成本、日志/事件和提交。

| 负载与ERP-P | 目标及重复次数 | 原始埋点和资源门槛 |
|---|---|---|
| P04：单SKU新分配/发货/释放 | 每动作≥1000次、3轮Release；P95≤1秒/P99≤2秒 | 按动作统计成功、冲突、失败和超时；记录order/root/physical锁等待、SQL次数、扫描行数、commit时长 |
| P03/P04：原分配及同发货单重放 | 每种≥1000次；P95≤500ms/P99≤1秒 | 完整来源复核计入时间；不以只读首张发货单计时替代重放验证，不剔除池等待 |
| P04/P12：20/100行、1/10/100组件 | 各≥1000次；20行P95≤1秒/P99≤2秒，100行P95≤2秒/P99≤4秒 | 保留整单事务与末行失败回滚；1000行/1000组件资源边界另组测量，不套用100行成绩 |
| P05：20同订单、20不同订单、20消费/取消 | 每种100轮，逐轮全部2000结果；成功/明确冲突P99≤3秒 | 另一连接复算0超占、0重复成本、0重复终态效果；记录实际并发、池等待和死锁，超时单列 |
| 资源上限 | 父行≤1000、单根组件≤1000、物理键和整批来源≤4096；取上限+1识别超限 | 超限明确拒绝且0部分效果；现4096超限沿组件limit错误，独立资源错误码和公开OpenAPI说明仍待补，不截断成功 |
| 测量入口 | 规 `crates/erp-api/tests/free_erp_fulfillment_performance.rs`；现统一构建入口待登记 | 当前入口只登记BOM Release性能目标，先登记新目标及RunPerformance准入；不绕入口执行原始cargo。记录硬件、PG/池配置、源码/产物SHA、峰值内存及EXPLAIN ANALYZE |

当前事务内statement_timeout=10秒、lock_timeout=5秒是数据库护栏，不包括连接池和整个用户等待，不能作为P95成绩。规划埋点放对应现函数及 `qa/performance/free-erp-fulfillment.json` 原始样本；单账号多端按同一业务意图恢复原回执，不把重复设备提交当新单。取消、补发的持久化身份和必需日志共同提交是性能验收前的正确性门禁。

#### Grok可继续领取的具体任务

| 任务和先后依赖 | 必须修改的位置 | 完成时交付及验收 |
|---|---|---|
| FUL-01 当前授权与提交身份，局部接线已完成 | 现 `API/auth_context.rs`、8个API调用处、`REPO/fulfillment/context.rs` 和 `REPO/data_scopes.rs`；会话/RBAC提交绑定规 `REPO/fulfillment/commands.rs` | 保留已接账号/仓库和两种真实锁竞争；继续补当前角色撤销、真实session/credentials_epoch、管理员及跨企业样本、内部worker策略。真实单账号多端权限恢复另验，不能凭合成JWT结项 |
| FUL-02 原OMS核心日志/发货事件已接共同提交；外层仍待验 | 现 `REPO/fulfillment/store.rs::record_allocation`、`REPO/fulfillment/context.rs::FulfillmentActor::record` 及operation/audit/Outbox with_tx；原API已移走4个必需记录块 | 保留7个真实INSERT故障、真实actor、重放0新记录及落库时间断言。继续补waybill、取消申请、submit后的WMS、售后关联和退回审单外层原子化/补偿，分别留实际故障，不复建提交后吞错 |
| FUL-03 原命令与补发回执，依赖FUL-02 | 规 `REPO/fulfillment/commands.rs`；复用现command_receipts身份设施，改现原OMS处理器及客户端恢复 | 同request_id/同规范负载返回不可变原结果；换仓、换数量、换actor或补发来源409。commit后响应断开由GET回查，不另造resend命名空间；20路原请求一张真实补发单 |
| FUL-04 成本与批次事实，原OMS消费局部已修 | 现 `REPO/fulfillment/costs.rs`、`REPO/inventory_batch_consumption.rs`，接原ship核心事务及consume_batches_by_fefo；其他批次写入和成本原来源继续接线 | 保留14成本/批次真实HTTP场景、合法零成本/历史重放及末行整单回滚；接续全部批次写路径、原批次追溯、价值计价/期间/退货、配置与元数据真实竞争、迁移政策及Release负载。不能以原OMS局部修复结项全部成本/批次 |
| FUL-05 当前来源页与原结果，依赖FUL-01/03 | 规 `REPO/inventory_source_queries.rs`、`API/inventory_source_handlers.rs`、`UI/pages/inventory/reservations.rs`；现页面注册/原订单宿主 | DataGrid显示父原量、原组件/版本、各物理来源当前状态及可执行动作；原回执与当前余量分别取权威接口。桌面/Web实际查询、保存、冲突、重登和未知结果恢复，不以夹具替代 |
| FUL-06 规模及资源验收，依赖以上正确性任务 | 规 `crates/erp-api/tests/free_erp_fulfillment_performance.rs`、`qa/performance/free-erp-fulfillment.json`；现统一构建入口新增登记 | 按本节固定数据、1000请求/100轮和Release执行；在组装全部计划/候选键前检查总行数与内存预算，不能把组装后的上限检查当作峰值内存达标。补批量元数据读取，测现组件weight/cost读取的SQL次数、旧源扫描和commit开销。4096/4097边界真实拒绝，原始样本、源码/产物SHA、内存及独立读回齐全 |

以上任务均归原免费基础闭环，不因补发恢复、数据安全、库存正确性或基础审计要求购买收费功能。收费席位和高级能力仍按原清单V项确定。

**本批实际状态：** 原五反例实际0通过/5失败（debug1.95秒），原日志/源码及5个测试企业保留；独立93802项PASS，2931个既有QA企业选定完整事实不变。修后首轮5通过/0失败/0忽略（2.58秒），包含20次分配全部真实返回同一发货单；独立93998项PASS，2936个既有企业事实不变。扩展10专项通过/0失败/0忽略（4.33秒），独立94211项PASS/2941个既有企业事实不变；首次完整184通过/0失败/0忽略（129.46秒），独立94531项PASS/2951个既有企业事实不变。涉及文件的3个旧Clippy诊断修正后，完整184再次通过/0失败/0忽略（121.16秒）；独立101411项PASS/3166个既有企业事实不变。后两个完整区间的10个OMS测试企业各60个合成JWT/Axum实际HTTP、13余额/30权威流水、9根/11组件/11物理源及3成本行均独立复算；所有32张选定表保留完整列。原失败、保护库兼容准备、成功区间、文档和源码输入绑定见 [本批独立读回](../qa/acceptance/free-erp-fulfillment-full-final-independent.json)。

923个实际输入前后盘点，本批8个Rust输入稳定；原反例/专项/完整区间各有其他共享源码或契约变动，最终完整区间为manufacturing.rs和shared_contract.rs，整体资格PARTIAL。6叶固定入口格式检查通过；严格Clippy从仓储236/238降到233/235，四适配器仍共27，API lint未到，最终未再观察到本批文件诊断，不能将依赖链称通过。既有增量缓存拒绝访问提示保留，未清理或绕入口。保护QA准备独立93817项PASS/2931个旧企业事实保持，只新增缺表/可空列，不回填历史。新增位置核对为23函数/5个真实登记操作，另沿原100卡、40单元、28页面和22指标检查。

**接续授权/必需记录批次：** 13个原反例0通过/13失败（6.57秒），两个原落库时间/零行状态反例0通过/2失败（0.77秒），分别保留原源码和实际错误。修后26履约回归通过（13.27秒）、首完整200通过（137.06秒），补足真实撤权等待/范围替换等待、actor、时间/版本后，最终202通过/0失败/0忽略（140.51秒）。另一连接完整事实117185项PASS，3653个既有企业选定事实不变；28个OMS企业、31余额/74库存流水、26根/28组件、9成本行独立复算，7个必需INSERT故障均回滚。另43项双向核对确认8张SHIPPED各1成功事件和操作/审计，19张未发货单无成功事件。事实及原始样本见 [最终独立读回](../qa/acceptance/free-erp-fulfillment-atomic-final-facts-v3.json)、[双向关联](../qa/acceptance/free-erp-fulfillment-atomic-final-linkage.json)。最终925个源码输入窗口内，本批14项涉及输入稳定，另有shared_contract测试变更；严格API范围Clippy被仓储232项及四适配器共27项诊断阻断，API lint未到，本批文件无观察到诊断仍不等于整链通过，整体资格PARTIAL。10叶固定入口格式检查及21个实际声明/8个认证接线核对通过。v2的旧企业总数报告变量被故障循环覆盖，原件counter-invalid保留；v3只修报告变量并重读同一不可变事实，没有改变原失败或业务验收。

收尾去掉3个已借用RequestContext的冗余取引用后，当前源码完整202再次通过/0失败/0忽略（137.10秒）；独立124641项PASS，3886个旧企业事实不变；同样28个OMS样本及8个成功事件、43项双向关联再次核对。当前925输入窗口内本批14项稳定，其他API manufacturing/shared_contract测试/两份OpenAPI输入有变更；证据见 [当前完整事实](../qa/acceptance/free-erp-fulfillment-atomic-current-facts-v4.json) 和 [当前双向关系](../qa/acceptance/free-erp-fulfillment-atomic-current-linkage.json)。原轮与收尾各源码窗口分别留存，不以这些共享输入变更宣布整仓或上线资格通过。

这些debug功能时长不填性能报告。当前账号/仓库和原OMS核心日志/成功事件的共同提交已按本节实际样本验；真实密码/席位、会话/角色撤销、多端正式UI、内部worker、外层waybill/WMS/售后/取消申请、原命令/补发回执、完整成本/精确批次、迁移恢复、Release及公网托管仍有未通过项，详见FUL-01至06；I02/T04和免费发布整项不结项。

### ERP-IMPL-12 盘点和差异

- 功能与字段：I03。盘点范围与截止、账面快照、实盘精确量、差异、复核、原因和调整流水。
- 现有接续：`REPO/stock_counts.rs`；`REPO/shared_contract.rs::{open_count_sheet,confirm_count_adjustment}`。
- 补齐位置：规 `UI/pages/inventory/counts.rs`，在原盘点确认服务接精确库存和成本；导入盘点明细使用同一来源身份。
- 事务与拒绝：盘点期间新移动按明确冻结或动态校正策略处理；不能用过期账面快照覆盖当前余额。确认按差额追加来源分录，批次和成本同步。
- 验收与性能：`stock_count_concurrent_movement_and_adjustment`；盘点时发生入出库、重复确认、负差超可用、被预占库存和末行失败。列表P02、20/100行确认P04、同键竞争P05。

### ERP-IMPL-13 调拨与在途

- 功能与字段：I03。源/目标仓库、SKU/批次、发出/到达/损失/剩余量、成本来源、日期和状态。
- 现有接续：`REPO/stock_transfers.rs`、`REPO/shared_contract.rs::{issue_transfer,receive_transfer,lose_transfer}`。
- 补齐位置：规 `UI/pages/inventory/transfers.rs`；原调拨服务接精确实存、在途和成本，不另建独立调拨余额。
- 事务与拒绝：发出扣源并记在途；到达扣对应在途、加目标；损失保留去向和价值。源/目标按稳定顺序锁，分批到达只消耗本调拨余量。
- 验收与性能：`transfer_partial_receipt_loss_and_replay`；发10、到6、损1，在途3；另测0.25、双向并发、跨范围目标、超到货和重复命令。P04/P05，系统总数量变化仅由明确损失解释。

### ERP-IMPL-14 批次序列和效期

- 功能与字段：I04/I08。批次/序列身份、SKU、仓库/库位、生产/到期日期、质量状态、精确量和来源。
- 现有接续：`REPO/sku_batches.rs`、WMS；`REPO/shared_contract.rs::{save_shared_batch,freeze_shared_batch,assign_shared_serial,trace_batch_use}`。
- 补齐位置：批次和库位余额升级精确契约；规 `UI/pages/inventory/batches.rs`，推荐算法读取授权且可用的批次，返回推荐理由与缺量。
- 事务与拒绝：SKU总账与批次/库位效果同事务一致；序列模式只接受该SKU允许的离散数量。过期/隔离批次禁止普通出库；推荐不自动提交扣减。
- 验收与性能：`batch_quantity_expiry_and_serial_exclusivity`；0.25领、0.125退原批次，错SKU批次、过期批次、重复序列和他仓库原来源拒绝。P02/P04/P05，批次汇总与SKU精确余额相等。

批次写入不能只验FEFO发货。[逐动作实施规格](#十一-批次库存逐动作实施规格) 将入库、调整、领退料、原命令、不可变流水和历史迁移拆为8个任务、16个具体样本及8组性能负载；第十一节C记录已执行的内核修复，其余接续项和性能负载仍待完成。

### ERP-IMPL-15 库存价值和关期

- 功能与字段：I05/F04。原收货/出库效果、精确数量、金额分、期间、暂估/确定、调整及关期版本。
- 现有接续：`REPO/shared_contract/item_cost.rs::{post_cost_receipt,post_cost_issue,post_cost_return,close_item_cost_period,rebuild_item_cost}`、`DOMAIN/trade.rs::cumulative_return_cost`。
- 补齐位置：规 `UI/pages/finance/inventory_costs.rs`；全部实物来源接价值分录，晚到费用有引用原来源的调整接口。
- 事务与拒绝：原数量/价值与累积退回量锁内计算，末笔取得剩余尾差；零已知成本合法，未知成本单独标记。关期与记账共用期间锁，不能写入已关期间。
- 验收与性能：`inventory_value_return_residual_and_period_race`；3微单位成本2分分三次退回为1/0/1分；关期竞争、重建和失败回滚。P04/P05/P14，余额与原分录独立复算。

### ERP-IMPL-16 采购录单与修订

- 功能与字段：T01。供应商、日期、交期、货币、20/100行SKU/单位/数量/价格/税、版本、已收/退/剩余量。
- 现有接续：`API/procurement_handlers.rs`、`REPO/procurement/store.rs::{create_order,confirm_order}`；`REPO/shared_contract.rs::{save_trade_purchase,save_purchase_revision}`。
- 补齐位置：规 `API/procurement/service.rs::save_purchase`、`UI/pages/trade/purchase_orders.rs`；列表、详情和写入收敛到正式采购单据事实。
- 事务与拒绝：修订不能将有效订单量改到已执行量以下；确认冻结相关规则，取消保留收退货历史。规范化全部明细再计算命令摘要。
- 验收与性能：`purchase_revision_preserves_received_quantity`；订单10已收6后改单5拒绝，20/100行末行错误整单无效果，重试命令无新单。P02/P03/P04/P05。

### ERP-IMPL-17 采购到货与放行

- 功能与字段：T02/Q01。采购原行、到货量、批次、实物仓库/库位、待检量、放行量、金额和应付来源。
- 现有接续：`API/procurement_handlers.rs::receive_purchase_order`、`REPO/procurement/store.rs::receive_order`；`REPO/shared_contract.rs::{arrive_purchase_receipt,release_purchase_receipt}`。
- 补齐位置：规 `API/procurement/service.rs::receive_purchase`、`UI/pages/trade/purchase_receipts.rs`；接统一精确库存、质检、价值和应付。
- 事务与拒绝：锁原采购行剩余量；到货与检验放行分开。待检10可用0，放行6可用6；到货、库存来源和应付确认按选定业务时点原子提交。
- 验收与性能：`purchase_receipt_partial_quality_and_replay`；10分收6+4，第三笔不能超收；放行重试、错误批次、后行价值失败。P04/P05，库存/质量/应付都从原行读回。

### ERP-IMPL-18 采购退货

- 功能与字段：T02。原到货/放行、仓库/SKU/批次、可退量、退货原因、原成本与应付冲减。
- 现有接续：`API/procurement_handlers.rs::confirm_purchase_return`、`REPO/procurement/store.rs::{create_return,confirm_return}`；`REPO/shared_contract.rs::return_purchase_receipt`。
- 补齐位置：规 `API/procurement/service.rs::return_purchase`、`UI/pages/trade/purchase_returns.rs`；正式退货按原来源处理实物和价值。
- 事务与拒绝：本批可退和本仓可出都满足才提交；不能用其他收货或其他SKU余量。退货与应付冲减不冒充实际退款到账。
- 验收与性能：`purchase_return_original_source_and_payable`；收10退2后原可退8，同命令只退一次；改SKU、批次、企业、已耗尽来源及末行失败拒绝。P04/P05，原成本与账款差额可复算。

### ERP-IMPL-19 销售录单与修订

- 功能与字段：T03。客户、订单/交付日期、仓库政策、精确数量、价格税额、稳定行ID、版本及已交/退摘要。
- 现有接续：`API/sales_orders.rs::{create_sales_order,update_sales_order,list_sales_orders,get_sales_order,confirm_scaled_trade_sale_route}`、`REPO/sales_orders.rs`，清单第三十一节11个HTTP操作，含原退货GET及确认POST。
- 补齐位置：规 `API/sales_orders/service.rs::save_sale`、`UI/pages/trade/sales_orders.rs`；无电商店铺也可建普通贸易销售单。
- 事务与拒绝：正式列表/详情和保存使用同一订单事实；完整规范化字段绑定原命令；修订不抹历史交退量，版本锁内复核。
- 验收与性能：`sale_precise_lines_version_and_unknown_response`；保存1.25、两端同版本竞争、末行错误、断响应重查和重新登录读回。P02/P03/P04/P05，页面两端分别验收。

### ERP-IMPL-20 销售交付

- 功能与字段：T04。原销售行、待交量、来源预占、仓库/批次、实物效果、成本、应收、物流任务和原回执。
- 现有接续：`API/sales_orders.rs::create_sales_delivery`、`REPO/shared_contract.rs::{plan_sales_delivery,advance_sales_delivery}`、精确库存和履约服务。
- 补齐位置：规 `API/sales_orders/service.rs::deliver_sale`、`UI/pages/trade/deliveries.rs`；库存、价值、往来和Outbox接同一提交。
- 事务与拒绝：锁原待交量与本来源预占；按实际执行时点扣库存。外部物流在提交后异步执行，失败不重做出库，回查沿用原效果。
- 验收与性能：`sale_delivery_replay_and_last_line_rollback`；10交6再交4，不能交5；物流失败、20路重复与末行价值失败。P04/P05，实物扣减、成本和应收只有一次。

### ERP-IMPL-21 销售退货

- 功能与字段：T05。URL订单、原交付行、SKU/仓库/批次、净可退量、收回质量、原成本、应收冲减和退款关联。
- 现有接续：`API/sales_orders.rs::{create_sales_return,read_sales_return}`、`REPO/shared_contract.rs::{return_sales_delivery,read_sales_return}`。原GET为 `/sales-orders/{sales_order_id}/returns/{return_id}`，返回交付ID、合格/不良微量、成本、应收冲减及过账金额。
- 补齐位置：规 `REPO/sales_returns.rs::return_delivery`、`UI/pages/trade/sales_returns.rs`；真实原来源驱动库存和价值反向分录。
- 事务与拒绝：锁原有效出库与累计退量；质量决定回到可用或待检，价值按原成本恢复。应收冲减与现金退款分别记录；GET及POST均核对URL订单和当前业务/仓库授权。现GET忽略URL订单的接线差距仍待修复，不能将参数成本当权威事实或把当前过账金额冒充原命令结果。
- 验收与性能：`sale_return_source_quality_and_value`；交6退2剩可退4；错订单、错SKU、超退、不同批次和并发20路。P04/P05，库存、质量、价值和账款同时读回。

### ERP-IMPL-22 多级 BOM 结构与展开

- 功能与字段：M01。BOM版本、基本产量、稳定节点/父节点、物料、数量单位、损耗、供给类型和有效条件。
- 现有接续：`REPO/shared_contract/bom_structure.rs::{save_bom_structure,explode_bom_structure,read_bom_explode_page}`、`DOMAIN/manufacturing.rs`、`API/manufacturing.rs::save_bom_structure_route`。
- 补齐位置：规 `UI/pages/manufacturing/bom.rs` 结构树、主子表和展开结果；规划读取接口按原清单第二十九节接入，结果有界分页。
- 事务与拒绝：保存和展开均绑定版本；算法检测环、深度和资源预算。共用子树的路径需求保留、物料汇总合并，不能缓存第一次数量后复用成错误总量。
- 验收与性能：`bom_shared_subtree_cycle_and_precision`；独立小样本复算多层需求与损耗；P10，1万物料/5万边/32层，30次完整展开每次≤5秒，新增峰值≤256MiB。

### ERP-IMPL-23 BOM 发布和资料冻结

- 功能与字段：M02。草稿、审批/发布状态、生效区间、版本、原版本、发布者和工单冻结引用。
- 现有接续：`API/manufacturing.rs::publish_bom_version_route`；`REPO/shared_contract.rs::{publish_bom_version,copy_bom_version,retire_bom_version,snapshot_work_order_bom}`。
- 补齐位置：BOM正式页的复制、校验、发布、停用与版本查看；规 `API/manufacturing/bom.rs` 编排有效区间与对象版本锁。
- 事务与拒绝：同产品/适用范围的有效版本不出现未定义重叠；已下达工单保留原冻结资料。发布新版本不追溯重写已有材料需求。
- 验收与性能：`bom_publish_race_preserves_released_snapshot`；两个版本并发发布、区间重叠、停用被引用版本、发布后老工单执行。普通发布P04/P05，校验展开按P10单列。

### ERP-IMPL-24 工艺 工作中心和日历

- 功能与字段：M03/M04。工序和前置、中心、准备/单件工时、必检、费率版本、工作日与每日可用能力。
- 现有接续：`API/manufacturing.rs::{save_operation_route_handler,save_work_center_handler}`；现制造领域和共享契约的路线、快照及日历方法。
- 补齐位置：规 `UI/pages/manufacturing/{routings,work_centers}.rs`、`REPO/manufacturing/routing.rs`；简化路线与工序路线核对后收敛权威定义。
- 事务与拒绝：路线无环、中心属于本企业；工单下达冻结路线及采用费率，日历按业务时区生效。负工时、未知中心及相冲突版本拒绝。
- 验收与性能：`routing_calendar_and_rate_snapshot`；前序约束、休息日倒推、空日历、日历跨月和新费率不改旧工单。列表/详情P02/P03、保存P04；负荷计算超同步预算时任务化。

### ERP-IMPL-25 需求和生产计划

- 功能与字段：M05。销售原行/备货来源、产品、精确需求量、需要日、已执行/取消/剩余量和冻结版本。
- 现有接续：`API/manufacturing.rs::save_production_plan_handler`、`REPO/shared_contract.rs::{save_production_plan,freeze_production_plan}`。
- 补齐位置：规 `UI/pages/manufacturing/production_plans.rs`、`API/manufacturing/plans.rs::freeze_plan`；读取正式销售余量，不能接受客户端自报已执行量。
- 事务与拒绝：冻结计划锁需求来源和版本；取消未执行余量保留已有采购/工单。贸易转制造启用按产品供给政策决定，不要求全部商品有BOM。
- 验收与性能：`production_plan_sales_remainder_and_freeze`；同销售行不能重复计划全量，部分取消、新订单版本和未知来源。P02/P03/P04/P05，计划需求可独立从来源复算。

### ERP-IMPL-26 正式后台 MRP

- 功能与字段：M06。需求/计划身份、BOM与规则版本、精确库存/占用/在制/到货快照、90天窗口、日历、任务阶段和解释。
- 现有接续：`DOMAIN/manufacturing.rs::plan_bucket_mrp`、`API/manufacturing.rs::{save_bucket_mrp_handler,plan_mrp_run}`；原清单第三十一节逐函数流程。
- 补齐位置：规 `REPO/manufacturing/mrp.rs::{load_input_snapshot,commit_results}`、`WORKER/jobs/mrp_jobs.rs::execute_run`、正式MRP页面。
- 事务与拒绝：服务器冻结真实来源；净算只处理冻结输入，无外部I/O；结果全部持久化再标完成。晚到供给不抵早缺料，待检/冻结及他需求占用按明确政策排除。
- 验收与性能：`mrp_server_snapshot_dates_and_restart`；原P/S/A/B独立样本、共用下级批量取整、晚供应、取消和Worker重启。P11/P18，受理P95≤500ms、完整执行每次≤30秒、新增峰值≤512MiB；排队另列。

### ERP-IMPL-27 MRP 建议回查和转单

- 功能与字段：M07。运行、稳定建议行、采购/自制类型、建议/已转/剩余量、需要/下达日、解释、输入版本和目标单。
- 现有接续：`REPO/shared_contract.rs::{convert_mrp_suggestion,lookup_plan_conversion,read_bucket_suggestion,read_mrp_snapshot,cancel_mrp_run,expire_mrp_run}`。
- 补齐位置：规 `API/manufacturing/mrp.rs::{convert_suggestion,get_explanation}`、`UI/pages/manufacturing/mrp_results.rs`；直接调用正式采购/工单事务服务。
- 事务与拒绝：锁运行状态与建议余量；目标单、转换关联及原回执同事务创建，不能只保存假目标UUID。过期结果默认拒绝转换并允许关联重算。
- 验收与性能：`mrp_partial_conversion_creates_real_document`；建议60转20后剩40，再转41拒绝；同命令重放，目标单写入失败全回滚。P02/P03/P04/P05，正式列表能查目标单。

### ERP-IMPL-28 工单录入 下达和取消

- 功能与字段：M08。产品/数量、来源、计划日期、仓库、工单版本、冻结BOM/路线、需求/预占、状态和取消原因。
- 现有接续：`API/manufacturing.rs::{save_work_order_route,transition_work_order_route}`、`REPO/shared_contract.rs::{save_work_order,release_work_order,transition_saved_work_order}`。
- 补齐位置：规 `API/manufacturing/work_orders.rs::release_work_order`、`UI/pages/manufacturing/work_orders.rs`；下达原子冻结和精确来源预占，不能分开提交。
- 事务与拒绝：先锁原命令和工单版本，确定有效BOM/路线，按稳定键锁材料库存与来源。缺料则按本版全量下达策略整单拒绝；部分下达如后续加入，必须有独立数量和状态契约。
- 验收与性能：`work_order_release_snapshot_reservation_atomic`；末材料不足时无任何预占/冻结/状态变化；两端同版本只有一方成功；已领工单取消先处理余料。P04/P05/P12。

### ERP-IMPL-29 批量领补退料和耗用

- 功能与字段：M09。稳定材料行、工单、仓库/SKU/批次、动作、精确量、原领料来源、当前持料和原回执快照。
- 现有接续：`API/manufacturing.rs::save_material_route`、`REPO/shared_contract/material_moves.rs::post_material_move`、精确库存服务。当前正式请求为单行，批量是待补契约。
- 补齐位置：规 `API/manufacturing/materials.rs::post_material_batch`、原材料仓储的同事务批量入口、`UI/pages/manufacturing/materials.rs`。沿用单行动作规则并完整规范化全部行。
- 事务与拒绝：退/耗只能引用同企业、工单、仓库、SKU、批次的原领/补料；累计退+耗≤来源原量。批量锁顺序固定，末行失败整单回滚。领料的仓库转出与持料耗用不能重复扣同一物理库存。
- 验收与性能：`material_batch_source_and_last_line_failure`；原领1退0.2耗0.6剩0.2；错SKU/另笔领料、改末行同命令、20路超耗、小数批次与未知响应。P12/P05，100行P95≤2秒/P99≤4秒；同源20并发100轮。

### ERP-IMPL-30 工序报工与冲正

- 功能与字段：M10。工单、冻结工序、前序有效产出、作业员工、认证提交人、起止/工时、合格/不良/返工量和原报工。
- 现有接续：`REPO/shared_contract.rs::{save_operation_report,count_operation_reports}`；`API/manufacturing.rs::save_operation_report_route`；现制造领域工序报工校验。
- 补齐位置：规 `API/manufacturing/reporting.rs::{post_reports,reverse_report}`、`UI/pages/manufacturing/reporting.rs`；正式读取接口与批量回执一起交付。
- 事务与拒绝：锁工单/工序累计来源与版本，复核前序余量、产出和工时；冲正追加反向记录并核对已入库/后序引用，不能删除原报工。
- 验收与性能：`operation_report_predecessor_and_reversal`；前序10后序不能报11，重复报工无新增，负工时拒绝，有效下游引用时冲正按策略拒绝。P12/P05，报工不冒充实物入库。

### ERP-IMPL-31 完工和半成品入库

- 功能与字段：M11。原有效报工、合格/不良余量、成品或半成品、批次、仓库/库位、检验状态、成本来源和入库效果。
- 现有接续：`REPO/shared_contract.rs::{save_production_receipt,release_production_receipt,reissue_semi_finished}`。
- 放行实际落点：现 `REPO/shared_contract/production_receipt_release.rs::{release_production_receipt_with_actor,source_proof,inspection_proof}`；原内部入口委托同一方法，HTTP绑定认证身份与路径工单。完整任务、异常样本及专属性能见[第十二节](#十二-生产收货放行逐动作规格)，当前方法只支持整笔待检放行。
- 补齐位置：规 `API/manufacturing/receipts.rs::receive_output`、`REPO/manufacturing/receipts.rs`、`UI/pages/manufacturing/receipts.rs`；接原报工、精确库存、质量与价值。
- 事务与拒绝：锁原报工净可入库量；实际接收入库一次增加实存，需待检时同量冻结；后续放行只释放冻结，不能第二次增加实存。半成品再投入引用实际批次。
- 验收与性能：`production_receipt_report_remainder_and_quality`；有效合格10分入6+4，第三笔拒绝；待检10可用0、放行6可用6；原成本暂估状态保留。P12/P05。

### ERP-IMPL-32 检验 放行和不良处置

- 功能与字段：Q01/Q02。标准及冻结版本、检验对象/来源、项目读数、判定、待检/合格/不良量、处置和责任人。
- 现有接续：`REPO/shared_contract.rs::{save_inspection_template,freeze_inspection_template,save_frozen_inspection_reading,release_after_quality,save_cited_quality_disposition}`。
- 补齐位置：规 `API/quality/service.rs::{record_inspection,release_quantity,dispose_defect}`、`UI/pages/quality/{inspections,dispositions}.rs`；正式库存桶、批次及价值接线。
- 事务与拒绝：冻结原标准；放行累计≤合格余量；报废同时处理隔离实物与价值，返工保留原不良来源。改质量字符串不能直接生成销售可用库存。
- 验收与性能：`quality_release_and_disposition_conserve_stock`；待检10放行6剩4，重复放行无变化，处置量不能超不良，跨批次/过期标准拒绝。P04/P05，检验详情P03。

### ERP-IMPL-33 双向实际批次追溯

- 功能与字段：Q03。实际投入批次/序列、投入动作、工单、产出批次/序列、数量、原效果和追溯方向。
- 现有接续：`REPO/shared_contract.rs::{save_genealogy_edge,read_genealogy_keyset,save_trace_export_scope}`。
- 补齐位置：规 `REPO/manufacturing/genealogy.rs::{append_actual_edges,trace_forward,trace_backward}`、`UI/pages/quality/traceability.rs`；来源于实际耗料与产出，不从计划BOM虚构。
- 事务与拒绝：追溯边与对应有效业务来源绑定，撤销保留原边及反向/无效事实。双方向查询有授权、稳定游标、已访问集和明确超预算策略。
- 验收与性能：`genealogy_actual_sources_both_directions`；原料A影响哪些成品、成品反查原料均可独立核对；共用原料、多工单、冲正和超大结果。P13，100万边典型≤1000边P95≤2秒、首100项≤500ms。

### ERP-IMPL-34 工单成本 在制和异常

- 功能与字段：M12/M13/F03。实际耗料、实际工时×冻结费率、制造费用、完工/在制/报废分配、暂估、确认版本、返工追加成本。
- 现有接续：`REPO/shared_contract.rs::{save_cost_run,confirm_work_order_cost,enqueue_cost_batch,complete_cost_batch,save_work_order_exception,close_work_order_after_exceptions}`；`REPO/shared_contract/item_cost.rs`。
- 补齐位置：规 `REPO/manufacturing/costing.rs::{load_actual_sources,confirm_cost}`、`WORKER/jobs/costing_jobs.rs`、`UI/pages/manufacturing/{costs,exceptions}.rs`。
- 事务与拒绝：计算冻结来源截止和算法版本；确认锁工单、来源及期间，结转只能一次。部分完工保留在制；返工新增投料工时另记，未处置余料/质量/价值不允许关闭。
- 验收与性能：`production_cost_wip_late_fee_and_scrap`；材料300元+人工100元+费用100元、完工10件共500元/每件50元；另测部分完工、暂估、晚费用、尾差与关期竞争。P04/P05/P14，批算≤60秒。

### ERP-IMPL-35 应收应付 收付和核销

- 功能与字段：F01/F02/F03/F04。伙伴、原业务单、债权债务、现金账户、收付款、预收预付、核销原来源/金额、费用、期间和反向记录。
- 现有接续：`REPO/finance_ar.rs`、`finance_costing.rs`；`REPO/shared_contract.rs::{confirm_document_balance,save_cash_entry,save_partner_settlement,read_rebuilt_balances,read_rebuilt_cash_open}`。
- 补齐位置：规 `API/finance/service.rs::{post_payment,allocate_payment,reverse_allocation}`、`UI/pages/finance/{balances,payments,settlements}.rs`；联动正式采购、销售和费用事实。
- 事务与拒绝：核销同时锁现金未分配余额及原单未结量，累计不能超二者；同原命令只生效一次。退款、账款冲减与反核销分别可追原事实。
- 验收与性能：`settlement_concurrent_remainder_and_reverse`；应收1000元收600核销600后未结400，另一笔核销401拒绝；预收、部分退货及关期。P02/P03/P04/P05/P14，另一连接复算余额和原金额。

### ERP-IMPL-36 经营 制造报表和异常中心

- 功能与字段：R01/R02/R04。查询身份/截止、销售毛利、库存收发存、往来账龄、工单在制和成本、异常原因及来源钻取。
- 现有接续：`REPO/reports.rs`；`REPO/shared_contract.rs::{save_operating_report,save_manufacturing_report,save_task_exception,act_on_task_exception}`。
- 补齐位置：规 `API/reports/manufacturing.rs`、`UI/pages/reports/`；原查询事实聚合，缓存/投影保留截止和算法，可按原账本重建。
- 事务与拒绝：汇总作用于完整授权查询集，不能合计当前页冒充总额；暂估/未知成本不显示确定利润。异常重试引用原命令，不能重复业务效果。
- 验收与性能：`report_whole_scope_cutoff_and_drilldown`；跨分页总数金额、期末截止、未知成本、不同仓库权限和钻取来源。P14/P02，固定5种报表P95≤2秒，重任务受理≤500ms/完成≤60秒。

### ERP-IMPL-37 正式共享工作台和双端状态

- 功能与字段：U01至U06/U08。页面实例ID、查询代次、稳定RowId、列/视图、选择、草稿、对象版本、保存命令、布局和面板状态。
- 现有接续：`UI/page_registry.rs`、`erp_app.rs`、`workspace_shell.rs`、`workbench/`、`components.rs`，`crates/erp-grid-core/src/remote.rs` 与公共DataGrid。
- 补齐位置：规 `UI/pages/` 真实业务页面；页面注册表、正式宿主tick、网络认证与文件适配接线；通用模块不依赖业务目录，演示夹具不当正式页面。
- 事务与拒绝：绘制线程无I/O；新条件立即取消旧任务，IME组合不查询，提交后250ms防抖。保存超时保留原命令回查；关闭脏页和批量关闭逐页保护草稿。
- 验收与性能：`native_web_query_generation_draft_and_save`；两端分别真实登录、查询、编辑、保存、重登读回，包含冲突/断网/取消。P08/P09/P19/P21；框选汇总与勾选集合分开，金额定点精度。

### ERP-IMPL-38 文件导出 下载和真实打印

- 功能与字段：U07/R03/T07/I06。完整查询范围、输出列/格式、数据截止、文件摘要、下载授权、模板/设备、任务及设备结果。
- 现有接续：`API/inventory_print_report_handlers/reports_exports.rs::{create_export_job,list_export_jobs,get_export_job,download_export_job}`、`API/inventory_export_access.rs::{create,authorize,visibility}`、`API/export_file_stream.rs::export_file_body_checked`；`REPO/inventory_export_jobs.rs::{create_job,authorize_job,parse_spec}`、`REPO/inventory_export_cursor.rs::{open,fetch_next,finish}`、`WORKER/jobs/inventory_export.rs::execute_inventory_job`；打印沿用 `WORKER/printing.rs`、`crates/erp-print-agent/src/main.rs`。
- 补齐位置：正式页面复用导出/打印设施，补有权限文件服务、模板和设备回执；任务宿主统一规 `UI/pages/system/tasks.rs`（PAGE-28）；不得把渲染文本成功回写为真实出纸。
- 事务与拒绝：服务端基于完整授权查询集生成有界输出；任务可回查和重试，下载重新验权。打印渲染、设备提交、出纸和未知结果分开，缺纸/断连不能假完成。
- 验收与性能：`export_full_scope_and_printer_real_outcome`；独立解析CSV/XLSX核行数金额，过期下载/跨企业拒绝；真实打印机在线、缺纸、离线、重复打印。P16/P17，导出10万×20列CSV≤30秒/XLSX≤60秒，新增峰值≤256MiB。

#### 库存后台 CSV 的实际接线

这是现有库存导出任务的schema2升级。原Worker已移除旧整数的offset导出实现；新的CSV与四库存GET共用数量计算、仓库谓词和对账预检。没有为这项修复扩展业务页面。官方托管、正式双端文件保存、XLSX、任务中心和规模性能仍按本节及ERP-IMPL-39/40接续。

| 操作 | 真实路由和代码 | 输入/结果与验收断言 |
| --- | --- | --- |
| 建立库存任务 | POST `/exports/jobs`，`API/router.rs` → `reports_exports.rs::create_export_job` → `inventory_export_access.rs::create` → `REPO/inventory_export_jobs.rs::create_job` | type为INVENTORY_EXPORT；params只收request_id和query。企业来自认证身份；必须具有inventory.read和exports.write，数据库中的账号ACTIVE及当前权限再验。任务与EXPORT.JOB.CREATE审计一事务；job_id等于原request_id |
| 列出可见任务 | GET `/exports/jobs?type=INVENTORY_EXPORT`，`reports_exports.rs::list_export_jobs` → `REPO/export_jobs.rs::list_jobs_visible_inventory` | 默认50，最多200；本企业、原操作者、当前权限和持有仓库范围共同过滤计数与分页。移除全部原限制授权不能变成旧无授权表记录的全范围；其他导出类型保留其原契约 |
| 查原任务/回执 | GET `/exports/jobs/{job_id}`，`reports_exports.rs::get_export_job` → `inventory_export_access.rs::authorize` | params返回服务器固定范围和artifact；原ID异操作者、企业、类型或规范化查询409。相同身份重试回原job、审计和已发布快照，不新建第二个任务 |
| 执行导出 | 现 `WORKER/jobs/export_jobs.rs::process_job` 委托 `WORKER/jobs/inventory_export.rs::execute_inventory_job` | 针对原job取数据库事务级advisory lock，校验RUNNING/attempt；执行前、每页和发布前重验库存/导出写权限。领取仍用现有Worker队列；专项测试只领取自身样本，未验全局队列和崩溃重领 |
| 取授权数据 | `REPO/inventory_export_cursor.rs::{open,fetch_next,finish}`，复用 `REPO/inventory_reads.rs::{require_reconciled,source_filters,quantity_filters,exact_quantities}` | 单一REPEATABLE READ READ ONLY快照；先验整个非数量筛选范围，再加精确数量条件。计数和所有fetch同快照；按warehouse_id、sku_id稳定排序；显式空仓库集合不转成全企业 |
| 发布文件 | `WORKER/jobs/inventory_export.rs::{generate,publish,verify_file}` | 原job独有临时文件create_new；flush/sync_all/关闭后先持久化行数、字节、时间和SHA256，再同目录hard_link发布，禁止覆盖。完成更新受RUNNING/attempt和原回执围栏保护；有效已发布文件重试复核SHA256后沿用原快照 |
| 下载文件 | GET `/exports/jobs/{job_id}/file`，`inventory_export_access.rs::{authorize,check_path}` → `API/export_file_stream.rs::export_file_body_checked` | 只允许原操作者，重验inventory.read、exports.read和当前仓库范围；DONE且回执有效，拒绝链接/重解析路径。流式校验字节和SHA256，给X-Export-Schema=2、X-Export-Sha256及Content-Length。必须读完并核摘要，200响应头不代表文件交付完成 |

请求示例（UUID替换为本企业实际仓库及本动作固定身份）：

```json
{
  "type": "INVENTORY_EXPORT",
  "params": {
    "request_id": "b59c6af0-4d62-4e6a-9a9a-58de0b95aee9",
    "query": {
      "warehouse_id": "34c628ff-161f-4b7b-849b-660cbe135b49",
      "available_min": "0.1",
      "available_max": "100"
    }
  }
}
```

| params/query字段 | 类型和校验 | 实现位置/行为 |
| --- | --- | --- |
| request_id | 非nil UUID，必填；客户端保存后重复使用 | `InventoryExportRequest::normalize`；原动作重试、断响应回查均沿用此ID |
| query | 可省略，默认全部授权库存；不接受null或未知键 | `InventoryExportQuery`，serde deny_unknown_fields；不接受actor_user_id、warehouse_ids、artifact、分页offset/limit及任意SQL排序 |
| warehouse_id、sku_id | 可空非nil UUID；仓库须属于本企业当前范围 | 请求仓库的范围拒绝发生在建任务前；SKU精确筛选，未匹配返回0行带表头文件 |
| sku_code、barcode | 可空字符串，去首尾空白，空转null；≤256 UTF-8字节、无控制字符 | 精确匹配；不改已有编码或条码 |
| tag | 同上长度/字符限制 | 字面大小写不敏感包含查询；%和_转义，不解释为通配符 |
| available_min、available_max | 可空十进制字符串、非负、最多6位小数；包含两端，min≤max | parse_qty后规范化为基本单位字符串；第7位小数、溢出、倒置400，不浮点截断 |
| 服务器范围 | schema=2、actor_user_id、排序唯一warehouse_ids≤10000、restricted_at_admission | 接单时固定具体仓库成员，后授予仓库不扩展该任务；执行数据快照在Worker第一次SELECT建立，不假称接单瞬间库存 |
| artifact | 未准备为null；准备/完成后含schema=2、quantity_scale=6、snapshot_at、rows、bytes、sha256 | receipt先于最终文件持久化；不是客户端可填的“已完成”标志；旧无操作者任务必须新建原请求，不追认归属 |

CSV列固定为 `sku_id,sku_code,warehouse_id,on_hand,reserved,available,frozen,in_transit,quantity_scale,snapshot_at`。前六列名保留，但数量使用真实基本单位十进制文本：旧整数1仍是1基本单位，微量字段1000000也表示1，不能改变单位含义；本版本明确增加其余四列。available严格为on_hand-reserved-frozen；在途不计入可用量。SKU文本做电子表格公式/长数字防护；数量列来自定点转换。数据1.9/0.9/0.9/2.3必须输出可用0.1，不能给1。数量未知、缺余额或缺授权主数据必须失败，不激活库存、不补0、不输出部分成功文件。此CSV不提供账面成本、单位换算名称或截止会计余额。

| 资源/时长 | 已实现硬边界 | 性能验收仍需证明 |
| --- | --- | --- |
| 请求和范围 | route请求体≤8MiB；具体授权仓库≤10000；筛选文本≤256字节 | 创建/同ID重试各1000次，ERP-P02目标；记录鉴权、数据库池等待、审计/提交、序列化；12路功能竞争不充当P95样本 |
| 读取/数量 | 总匹配≤100万行；fetch每次1—1000；SKU编码输出≤1024 UTF-8字节；SQL单语句10秒，游标空闲30秒 | 1千/1万/10万/100万行分别记录预检/计数/游标/编码时长、实际FETCH页数及新增峰值；1002行只是分页/流式功能证据 |
| 生成/文件 | CSV≤1GiB，编码批次≤8MiB；生成及旧文件校验阶段最多300秒，不含排队/领取 | ERP-P16的10万行×20列≤50MiB，CSV完整生成每次≤30秒、新增峰值≤256MiB，30次全部满足；300秒护栏不能当成达标时长。库存CSV为10列，另报同数据量实际大小和耗时 |
| 下载/完整性 | 每次读取≤64KiB，增量SHA256；字节数必须匹配持久化回执 | 首字节、完整下载、客户端摘要核对分别计时；中途截短、增长、同长度改写或摘要错误不能得到完整成功文件；浏览器本地保存独立验收 |
| 公平使用/恢复 | 单job拒绝重复执行，过期attempt不覆盖新状态；已发布文件有回执时沿用原快照。当前未发布尝试清理临时文件，重试可重新读取数据；服务配额仍待ERP-IMPL-39 | 必须补验发布前准备副本的保留/恢复策略，不能将已发布文件重试的证据扩到所有断点；单企业/全局并发、有限排队、存储/留存配额、真正进程崩溃后的RUNNING重领、对象存储发布及ERP-P18/P20均未完成；已传输字节不能撤回 |

错误方向：缺库存/导出权限、非原操作者、当前仓库撤权或旧无归属任务403；非法参数400；原请求身份冲突或DONE缺回执409；本企业不存在job或文件/文件长度不匹配404；授权/受理存储不可用503。超过行/字节/文本/超时或库存需对账由Worker以FAILED及原错误留证，进度不等于完成。发布后的文件损坏可在200响应头之后终止响应流；客户端必须丢弃未验证的临时下载，不能先显示“导出成功”。已开始传输时的撤权竞争、故障注入和对象存储另列待验，不以开始/结束的权限检查假称全程锁定授权。

专项目标已登记在 `crates/erp-api/tests/free_erp_acceptance/inventory_exports.rs`：精确文件/跨操作者/原权限缺失、12路幂等和异内容冲突、伪造字段/越仓、角色撤销、全部仓库撤销、并发写下同快照、已发布回执恢复、同长度文件损坏、过期attempt、未知库存在数量过滤前拒绝、新授权不扩范围、超长SKU、同job双执行、非法fetch配置、旧无归属任务、精确空范围、1002行跨FETCH及64KiB下载。首轮14项、新增后完整登记86项和修正循环后的18项专项分别通过，另8项文件流/原产品导出守卫单测通过；不是同一产物的重复计数。1002行CSV为150386字节、下载3块。独立psql/Python原批14694项、最新14773项通过，最新1288旧企业样本未变、1035个精确库存键与原流水一致，1未知键保持未激活。严格Clippy仍有仓储234项及适配器27项诊断、未到达Worker；本批新仓储叶模块已无诊断，不把未到达的包写PASS。输入及产物认证以 [库存导出修复证据](../qa/acceptance/free-erp-inventory-export-repair.json) 为准，不能据此勾选完整R03/U07或ERP-P16。

Grok下一批：先在正式库存宿主接这四条原任务路由，复用现 `UI/remote_view/exports.rs` 的任务列表/生命周期及其 `create.rs` 中原产品导出准入；不要另造第二个任务窗口。新增库存适配须保存原request_id，分别适配桌面文件落地与浏览器下载；只在完整读完且SHA256匹配后提交文件并提示结果。随后登记受保护的Release库存导出负载到现有 `free_erp_bom_performance`，按统一入口的 `-Scope api -IntegrationTest free_erp_bom_performance -Release -RunPerformance` 验证，保留源码输入、实际环境、独立文件/库存复算和进程内存。不能另建target、用debug功能计时代替P16，或仅增加验收报告把待测标为完成。

### ERP-IMPL-39 命令 后台任务和公平使用

- 功能与字段：A10/O01至O04/O08。命令身份/摘要/结果、持久化任务、租约、尝试次数、下一重试、取消阶段、队列/并发配额和资源计量。
- 现有接续：`REPO/idempotency.rs`、Outbox/Inbox、现Worker宿主；`REPO/shared_contract.rs::{open_command,advance_command,lookup_lost_command,enqueue_durable_task,claim_durable_task,fail_durable_task,request_task_cancel}`。
- 补齐位置：规 `WORKER/jobs/admission.rs`，对应域任务执行器及正式任务中心；原命令接口能查未决状态，业务提交与回执/必要审计和Outbox原子。
- 事务与拒绝：领取任务有租约和唯一执行身份；Worker崩溃不重复已提交效果。单企业MRP/导出各最多1，全局重任务先最多2；队列上限与429重试策略发布前固定。
- 验收与性能：`durable_task_lease_unknown_and_tenant_fairness`；提交后断响应、领取/计算/提交阶段故障、租约重领、死信、取消。P11/P18/P19/P22，另一9家查询/写入仍满足P02/P04。

### ERP-IMPL-40 官方托管 备份迁移和发布

- 功能与字段：A07/A08/A09/O06/O07/O09/O10/O11。公网来源/TLS、会话与下载安全、监控告警、数据库和附件恢复截点、迁移版本、桌面/Web产物、许可证及容量成本。
- 现有接续：`docs/OPERATIONS.md`、`scripts/backup_postgres.sh`、已有CI、发布设施；共享契约中的恢复/发布记录只作为需核对的记录设施。
- 补齐位置：规 `scripts/Restore-ErpBackup.ps1`、`infra/observability/` 和正式托管配置；登录、业务、文件、Worker及恢复演练以真实进程/存储/公网验证。
- 事务与拒绝：迁移先在新库和保护旧库副本各完整运行；不可逆DDL不能假称二进制回退可还原数据。恢复保留原命令与效果身份，截点内账实一致，不猜测未决写入是否成功。
- 验收与性能：`hosted_release_restore_and_dual_client`；3次独立恢复、真实TLS/CORS/会话、错误文件授权、双端登录保存、故障告警和许可证核查。P20/P21/P22；RPO≤15分钟/RTO≤2小时，预算B和发布容量须实际填写。

F1增强、收费V项及行业X项继续按原100项卡片实施，复用以上共享基础。复杂委外、替代料、联副产品、有限产能排程、设备采集和行业追溯要各自固定业务样本、数据规模及专属指标，不能凭通用API性能或菜单存在声明通过。

## 三 关键动作的事务顺序

### 写入服务的公共提交顺序

| 顺序 | 必须完成的事情 | 失败后的事实 |
| --- | --- | --- |
| 1 | 校验真实身份、企业、动作权限、数据范围与全部输入；把单位/数量规范化后生成完整命令摘要 | 不建立业务对象、不扣库存；错误能定位具体行/字段 |
| 2 | 开原事务，锁原命令，若已完成则校验摘要并重放原结果；同ID异内容拒绝 | 原回执保留；不得重放成另一笔业务 |
| 3 | 锁单据/来源行并复核版本、状态与净剩余；相关库存按稳定键顺序锁定，锁策略与其他入口一致 | 没有部分明细、预占、库存、质量或价值效果 |
| 4 | 固定本动作所需快照，执行实物、来源余量、价值/往来和状态变更；调用已有事务服务 | 后行失败回滚前行；不得内部提前提交 |
| 5 | 在同一事务保存原结果快照、必要审计与Outbox；成本暂估或异步阶段明确记录 | 不出现成功业务却没有可查询回执的窗口 |
| 6 | 提交后才返回“已完成”；外部打印/物流/通知通过持久化任务执行 | 响应丢失由原命令回查，不能换ID重做业务 |

函数需要参加上层事务时必须接收现有事务引用。领域算法返回计划/校验结果，HTTP handler做身份与输入转换，不在handler散落多个独立提交。耗时网络和大计算不持有业务行锁；确需分批提交时，明确暂存状态、最终生效点和整单回执。

### 工单下达

规划服务入口为 `API/manufacturing/work_orders.rs::release_work_order(context, work_order_id, expected_version, request_id)`，委托现 `REPO/shared_contract.rs::release_work_order` 的真实事务接线。输入不接受客户端自报BOM展开、已领量或可用库存。

1. 锁原命令和工单；校验DRAFT/允许下达的真实状态、版本、产品和仓库范围。
2. 读取并固定本工单适用BOM与路线版本，按工单量/基本产量计算材料需求，保存可追溯快照。
3. 按仓库/SKU/来源稳定顺序锁精确库存，排除冻结、待检和他来源占用；验证所有材料满足本版下达政策。
4. 在正式来源预占中建立本工单预占，按同一精确库存事务更新reserved；不减少on_hand。
5. 保存工单下达状态/版本、原回执、审计和Outbox后一次提交。任一材料不足时以上写入全部回滚。

验收读回包括工单状态/版本、BOM与路线快照、每个来源预占、库存reserved、流水和唯一回执。只记录 `RELEASED` 状态而未冻结/预占，或先冻结再单独预占，都不通过。

### 材料动作与完工放行

| 动作 | 数量事实 | 必须一并核对 |
| --- | --- | --- |
| 领料/补料 | 从实际仓库转出，增加工单持料，消费本来源相应预占 | 仓库/SKU/批次原身份；库存、批次、来源预占和材料动作一致 |
| 退料 | 减少原领料的持料余量，回原身份或明确授权的退料目的地 | 原领/补料净余量，累计退+耗≤原量，原成本回转 |
| 耗用 | 将该原来源的持料转实际耗料与在制成本 | 已领实物不能第二次扣仓库；实际批次成为追溯来源 |
| 报工 | 增加有效工序产出/工时，保持合格、不良与返工去向 | 前序和本序余量、认证提交人与业务作业人、冻结费率 |
| 完工待检入库 | 有效产出入实存，同时增加对应冻结量，销售可用不增加 | 原报工可入余量、批次、质量来源、暂估或确定价值 |
| 合格放行 | 只释放原待检冻结量，不再增加实存 | 原检验版本/读数/合格余量；批次和总账可用量同步 |
| 不良报废 | 按原隔离实物减少数量并处理相应价值/报废来源 | 不良余量、工单成本去向、期间与原效果引用 |

目前单行动作及部分材料/精确库存已有局部测试；表中是整条业务链的目标行为。若当前代码的“CONSUME”等名称与上述物理语义不一致，先明确原动作账本含义并迁移接线，不能仅为名称一致多扣一次库存。

### 应收核销

规划入口为 `API/finance/service.rs::allocate_payment(context, payment_id, allocations, request_id)`。每行引用原未结单及金额分。锁原命令、收款未分配余额和按稳定ID排序的原债权，复核净可核销量；同事务写核销、余额版本、原回执和必要审计。1000元应收收600并核销600后未结400；20路追加核销不能合计超过未结400，也不能超过收款未分配额。

## 四 性能验收任务

### 固定环境和测量规则

沿用原清单第二十节的候选发布基线：API/Worker合计4专用vCPU/8GiB，数据库独立4vCPU/16GiB/100GiB SSD；客户端Windows 11/4核/8GiB，Web使用固定Edge/Chrome版本及真实WebGPU；公网模拟20Mbps下载/5Mbps上传/80ms RTT。实际处理器、存储、GPU、数据库/浏览器版本、连接池、队列和产物SHA必须写入报告；这些配置不构成性能保证。

数据基线为100个非空独立企业，压力企业10万物料、10万单据和100万库存流水。稳态50会话/20请求每秒，60%列表、15%详情、15%保存、5%认证、5%任务查询；预热5分钟、固定速率30分钟，100请求每秒持续60秒突发另组测试。固定速率发起，排队和超时保留，不能用变慢后减少请求掩盖性能下降。

普通HTTP每种动作、规模和端至少1000样本；慢批任务至少30次完整运行，不能混算不同动作的百分位。P95/P99使用最近秩：升序样本第 `ceil(0.95×N)` / `ceil(0.99×N)` 项，记录P50/均值/最大及错误。批任务的每次完整时长和新增峰值都必须满足对应硬上限。正常组非预期错误率<0.1%，业务不变量错误为0；故意制造的401/409/422/429独立成组。

### 22组任务与通过条件

| 指标 | 实际调用位置和负载 | 起止点和样本 | 目标与必须同时通过的正确性 |
| --- | --- | --- | --- |
| ERP-P01 | 现 `API/auth_handlers.rs::login`；真实密码及有效账号，每次能访问正式业务 | 客户端发登录到取得可用身份/会话；热态≥1000次，冷启动另组 | P95≤1秒/P99≤2秒；不降低密码算法，不使用签发测试JWT代替登录 |
| ERP-P02 | 现各域列表；10万单据/100万流水，常用授权筛选排序，100/500行分别测；四库存GET另测200/2000容量组 | 服务端收完整请求到响应交付，鉴权/池/SQL/编码全计；每动作及规模≥1000次 | 100/200行P95≤500ms/P99≤1秒；500行P95≤1秒/P99≤2秒；库存2000行候选P95≤2秒/P99≤4秒及JSON≤8MiB，未验不开放正式页；全范围先筛选后分页 |
| ERP-P03 | 现各域详情；20行和实际关联摘要，含错权限样本但另组统计 | 服务端完整请求到详情交付；每动作≥1000次，记录SQL/请求数 | P95≤500ms/P99≤1秒；详情与原单/来源一致，N+1网络请求为0 |
| ERP-P04 | ERP-IMPL-07/12/13/15至21/23/25/27/28/32/35；20/100行各正式动作 | 普通写请求受理到数据库提交完成，另记客户端总时长；每组≥1000次 | 20行P95≤1秒/P99≤2秒；100行P95≤2秒/P99≤4秒；回执/库存/价值原子，202不能算提交完成 |
| ERP-P05 | 同库存/来源/版本20路竞争，分别测新命令竞争与同命令重放 | 100轮，每轮证明实际并发/锁等待；全部2000请求保留结果 | 明确成功/冲突完成P99≤3秒；无超量/丢失更新/重复效果/死锁残留，另一连接逐轮复算 |
| ERP-P06 | ERP-IMPL-02/03；1席位和当前8有效会话边界，各20路×100轮 | 真正用户启用/登录配额判定，成功与拒绝分组 | 判定P95≤1秒；免费席位≤1、会话≤配置值；浏览器标签不当新席位 |
| ERP-P07 | ERP-IMPL-03；撤销/改密后再提交受保护请求，≥1000个独立样本 | 撤销事务提交确认到随后新请求的准入结果；不是对机器墙钟直接相减 | 当前逐请求验证，提交后的新请求立即拒绝；引入缓存需重定传播口径，已受理命令保留原回执 |
| ERP-P08 | ERP-IMPL-37；使用组件规范原PERF-01至05数据集，桌面/Web分别执行 | 首有效绘制、持续滚动帧、切换及增量更新各按原定义，不只测空表 | 原PERF阈值500ms/帧P95≤32ms/300ms/20轮有界/200ms；视觉、稳定RowId及选择一致 |
| ERP-P09 | ERP-IMPL-05/37；中文输入、快速改条件、10万物料对象选择 | 输入事件到本地反馈；最终提交到250ms防抖后发请求；选项服务端另计 | 输入P95≤50ms、选项P95≤300ms/≤100项；IME期间0查询，旧响应0次覆盖新条件 |
| ERP-P10 | 现 `REPO/shared_contract/bom_structure.rs::explode_bom_structure`；1万物料/5万边/32层 | 快照读取开始到全部结果及回执提交；2次预热后30次完整运行 | 每次≤5秒/新增峰值≤256MiB；小样本独立复算，环/超预算明确拒绝，结果分页可完整读回 |
| ERP-P11 | ERP-IMPL-26；1万物料/5万边/至少三级/1000需求/90天日桶，30次 | API任务落库、排队、开始执行、全部结果/解释可查分别记录 | 受理P95≤500ms、完整执行每次≤30秒/新增≤512MiB；取消落库后≤2秒停止或返回真实不可中断提交阶段 |
| ERP-P12 | ERP-IMPL-28至31；100行材料动作和100条报工分别测，真实来源和批次 | 收完整请求到整单提交，≥1000样本；20路竞争另按P05 | P95≤2秒/P99≤4秒；末行失败全回滚，原领料/报工余量不超量、不重复入库 |
| ERP-P13 | ERP-IMPL-33；100万追溯边，典型结果≤1000边，正反两向分别测 | 请求到首100项/完整典型结果；每方向≥1000次，超大结果另组 | 首100项≤500ms、完整P95≤2秒；实际来源、完整授权范围，无静默截断 |
| ERP-P14 | ERP-IMPL-15/34至36；100万流水固定5种报表；重算30次 | 请求到完整聚合；重算受理、排队、全部结果分别记录 | 固定报表P95≤2秒；重算受理≤500ms/每次≤60秒；同截止点独立复算数量金额及暂估状态 |
| ERP-P15 | ERP-IMPL-06；1万行×20列≤10MiB，30个独立文件任务 | 服务器文件就绪到预检完成、确认到全部行结果可查；上传另计 | 每次预检≤30秒/导入≤60秒/新增≤128MiB；逐行结果、文件摘要和重复确认无新增 |
| ERP-P16 | ERP-IMPL-38；10万行×20列≤50MiB，CSV/XLSX各30次 | 导出受理后到完整文件可下载；排队、生成和下载分别报告 | 每次CSV≤30秒/XLSX≤60秒/新增≤256MiB；独立解析全行数和金额，授权下载不泄漏他企业数据 |
| ERP-P17 | ERP-IMPL-38；真实A4单20行及单张标签，在线/缺纸/离线分别测试 | 受理、Agent接收、渲染、设备提交、出纸分段，设备身份和原始次数记录 | 渲染≤1秒、在线Agent受理≤5秒；实际出纸另定设备阈值，缺纸不能记PRINTED |
| ERP-P18 | ERP-IMPL-39；1企业连续提交重任务，另9企业保持P02/P04混合负载 | 30分钟固定负载，排队深度、任务租约、各企业请求及拒绝分组 | 普通查询/保存仍符合P02/P04；单企业MRP/导出各≤1、全局重任务先≤2，满队列明确拒绝且可恢复 |
| ERP-P19 | ERP-IMPL-37/39；24小时业务/任务，再20轮切标签回相同空闲状态 | 同状态预热基线与结束工作集；API/Worker最后4小时趋势和请求/任务残留 | 客户端≤基线+10%+20MiB；无任务/请求残留或无法解释持续增长，不清缓存制造低内存 |
| ERP-P20 | ERP-IMPL-40；持续写入后至少3次独立恢复，覆盖数据库、附件和未决任务 | 故障、真实恢复截点、业务重新可用分别记录；事前事后事实清单 | 数据库RPO≤15分钟/服务RTO≤2小时；附件截点另列，截点内余额/流水/回执守恒，无重复重放 |
| ERP-P21 | ERP-IMPL-37/40；真实桌面包和Web包，各30次冷/热启动及登录后首页 | 启动/导航到首可交互；登录提交到正式业务页可操作；网络/加载/绘制分段 | 桌面≤5秒；Web冷≤15秒/热≤3秒；登录后首业务页≤2秒，不能用预先加载夹具计时 |
| ERP-P22 | ERP-IMPL-39/40；真实计量区间、活跃企业、资源和账单，稳态/突发/任务组均保留 | 区间累计CPU/存储/网络/任务资源、基础设施账单及分摊口径 | 每活跃企业月成本可核对；未活跃基础成本单列，预算B/容量发布前固定；没有账单和观测不得报预算达标 |

补充工程预算沿用原清单第三十节：列表数据SQL建议≤4条、20行详情建议≤6条；授权查询另计，实际超限保留原因。典型100行响应≤256KiB/500行≤1MiB，不包含附件实体；每表最多2在途及原有有界缓存。CPU、池等待、锁等待和实际扫描量都记录，不把平均时长代替P95/P99。

### 五种固定报表的负载口径

P14使用下表五种报告分别测量，不允许从大量报表中事后挑最快五种。每组将日期/状态/仓库、实际参与事实数、查询摘要和独立预期值固定到规 `qa/performance/free-erp-report-fixtures.json`。100万流水是预置规模；实际参与扫描/聚合的行数也必须单列，不能用空条件结果代表大范围性能。聚合覆盖完整授权范围，结果明细继续有界分页。

| 报表 | 固定查询和结果字段 | 独立复算方式 |
| --- | --- | --- |
| 销售及毛利日报 | 固定30天、所有授权仓库；按日期汇总有效交付/退货净量、净销售额、原有效出库成本、暂估量/金额 | 原交付和退货分录逐日复算；未知成本不合成确定毛利 |
| 库存收发存 | 固定30天、所有授权仓库/SKU；期初、入、出、调整、期末、预占、冻结、在途和可用精确量 | 原余额截点与期间流水逐桶复算，期初+净变动=期末 |
| 应收应付账龄 | 固定截至日；伙伴、原单未结、未到期/1—30/31—60/61—90/90天以上、预收预付 | 原债权债务与有效核销按同截止重建，分桶之和等于未结 |
| 工单执行和在制数量 | 固定30天活动工单；需求、预占、净领、耗、持料、合格/不良、已入/待入量 | 原工单、来源材料、有效报工和入库逐来源复算，不能用计划量充实际产出 |
| 工单成本汇总 | 同一活动工单集；实际材料/人工/费用、完工/在制/报废去向、暂估和确认版本 | 原实际耗料价值、工时×冻结费率、费用与结转分录复算，无未解释尾差 |

### 实施位置和原始证据

| 产物 | 现有或规划位置 | 必须包含 |
| --- | --- | --- |
| 业务正确性 | 现 `crates/erp-api/tests/free_erp_acceptance.rs`；规其下 `trade_closure.rs`、`manufacturing_closure.rs`、`mrp_closure.rs`、`account_closure.rs` | 正常、失败、并发、重试、未知结果、范围拒绝；独立SQL或文件读回 |
| API混合负载 | 规 `qa/performance/run-erp-api.mjs`、`free-erp-api-fixtures.json`、`free-erp-api.json` | 固定速率请求明细、实际动作/规模/时长/状态码、计划与完成数、环境和SHA |
| 制造批任务 | 现登记BOM目标 `free_erp_bom_performance`；规MRP/成本目标，先登记构建入口 | 30次逐运行输入摘要、完整输出、时间、峰值和独立复算，不以一个最快样本结项 |
| 并发库存及来源 | 现精确库存/材料验收子模块；规 `qa/performance/free-erp-inventory-contention.json` | 100轮20路的真实开始/锁竞争、每命令结果、失败原因、逐轮余额/来源/流水 |
| 双端及恢复 | 规 `qa/acceptance/free-erp-dual-client.json`、`free-erp-hosted-restore.json` | 真实身份/产物/设备/环境、操作记录、故障点、恢复截点与事实对比 |

报告中无实际测量的字段用 `null`，状态写 `NOT_RUN` 并记原因。禁止填估计百分位、把忽略用例计为通过，或把旧产物结果绑定到变化后的源码。源码输入前后SHA不同，保留实际产物结果，当前版本认证为 `PARTIAL`。

BOM性能现有受控执行入口如下；这是命令说明，本次文档维护未运行它。新性能目标必须先登记，不运行原始Cargo，也不另建target。

```powershell
& ./scripts/Invoke-ErpBuild.ps1 -Action test -Scope api -Platform native `
  -IntegrationTest free_erp_bom_performance -Release -RunPerformance
```

业务回归沿已登记 `free_erp_acceptance` 子模块扩展；入口实际强制 `--test-threads=1`，不能把环境变量值冒充测试harness并发。P05并发必须由用例/压测器真实同时发起并证明锁竞争。

## 五 开发批次和交付记录

| 顺序 | 对应原DEV包及本文件单元 | 这一批具体结束条件 |
| --- | --- | --- |
| 1 共享正确性 | DEV-01至05，ERP-IMPL-01至15及37/39必要基础 | 真实账号与范围、主数据、精确库存全部入口、来源/价值、命令及正式工作台；旧写入口成功接线，不能只由触发器拒绝 |
| 2 贸易闭环 | DEV-06/07，ERP-IMPL-16至21/35 | 普通企业无电商店铺完成采购、收退、销售、交退、收付核销；库存、价值和往来独立复算，双端正式保存 |
| 3 基础制造 | DEV-08/09/11/12，ERP-IMPL-22至25/28至34 | BOM/工艺到工单下达、领退耗、报工、完工检验、追溯、在制成本闭环；不足/错来源/后行失败无部分效果 |
| 4 正式MRP及报表 | DEV-10/13，ERP-IMPL-26/27/36/38/39 | 服务端真实快照、后台MRP及取消/恢复/过期、建议真实转单、完整报表文件；P10至18相关组分别验收 |
| 5 官方免费发布 | DEV-14，ERP-IMPL-40及全部F0上线门禁 | 公网和会话安全、真实双端、完整迁移、恢复、告警、容量和成本。免费边界不依赖收费扩展才能闭环 |

### 当前实现与验收边界

2026-10-03源码已有材料物理身份、原来源余量、回执快照和精确库存接线；已记录修复产物的登记目标46项通过，17个新精确库存样本独立读回269项通过。历史43项/210项证据保留；该批次另行复验历史小数激活、共享写入竞争以及库存流水DELETE拒绝。当次选定目标输入一致，另一份集成测试并行变化使整个工作区快照保持PARTIAL；详见 [历史修复证据](../qa/acceptance/free-erp-precision-repair.json)。随后 `REPO/shared_contract.rs` 等实际目标输入继续修改，历史46项结果不能认证当前源码。前一文档批次只检查文档、位置和证据对应关系，没有重跑业务或性能测试。

其后实际修复权限存储故障继续放行及ATP整数夸大可用量：原HTTP反例、修复前后的接口结果保留，完整登记目标55项通过，21项精确库存用例的23个新样本由独立psql核对444项通过；原2项整量FEFO契约、叶文件格式和OpenAPI门禁通过。授权原49项运行另有20项独立事实核对。证据见 [授权修复](../qa/acceptance/free-erp-auth-scope-repair.json)、[精确ATP修复](../qa/acceptance/free-erp-atp-precision-repair.json)；原整数响应兼容投影、权威小数字段、拒绝状态、1000批预算及四组性能任务见ERP-IMPL-09。890个输入中共享库根和另一测试仍并行改变，实际目标源码认证为PARTIAL；严格Clippy仍在仓储及平台适配器依赖失败，API lint未完成。上述为debug源码/API/保护库回归，真实密码/多端席位、正式双端、公网、完整迁移/恢复与ERP-P负载仍须单独验收。

再后一批修复公开列表500、汇总整数可用量夸大及流水显式仓库拒绝，保留原失败日志。四个公开列表、数量/价值聚合及同一Rust报表宿主已接完整小数；最新登记目标68项及独立读回1332项通过，客户端25项定向测试、Web编译和native/Web严格Clippy通过。后台CSV/周转查询、写回执和正式页面尚未全部接线；892个输入中的共享库根仍并行变化，后端实际目标源码认证PARTIAL。详见 [库存读取修复证据](../qa/acceptance/free-erp-inventory-read-repair.json)。

已有 [精确历史回归失败日志](../qa/acceptance/free-erp-precision-history-before.log) 与 [共享写入竞争失败日志](../qa/acceptance/free-erp-precision-shared-fence-before.log) 保留；后者原先实际观察到共享写入没有等待未提交的权威激活，后续批次已证明实际等待和提交后的拒绝。保护库局部迁移及选定回归通过，旧入口成功接线、真实身份与双端、严格仓储Clippy和整条迁移链仍未全部完成；不要据此勾选I01/I02/M09或免费发布完成。

### 一项任务的验收单

| 字段 | Grok必须交付的内容 |
| --- | --- |
| 对象 | ERP-IMPL编号、原功能ID/DEV包、实际文件/函数、现有接口和新增契约 |
| 实现 | 请求字段/类型、来源与权威表、锁与事务边界、版本/幂等、错误码、迁移/索引 |
| 功能 | 正常、边界、失败、并发、重放、未知结果、企业/仓库范围；预期值与独立读回 |
| 双端 | 桌面/Web分别真实登录、查询、编辑、提交、重登；草稿、失败和回执处理 |
| 性能 | ERP-P、阈值版本、环境、数据、动作/规模、样本数、时长和峰值、错误率、正确性 |
| 证据 | 产物SHA、源输入前后SHA、原始日志/请求样本、SQL/文件读回、测试与部署身份 |
| 结论 | 单项PASS/FAIL/NOT_RUN/SIMULATED_ONLY/UNAVAILABLE；部分完成汇总PARTIAL，未通过项逐项列明并回写路线图 |

反馈总结遵守项目500字以内约束，详细证据保存在文档与报告。先完成当前批次的具体接线和可审查结果；不以全仓重写、收费排程、所有行业适配或所有电商平台接入作为基础免费闭环的前置。

## 六 正式页面的入口和实现位置

原清单含72项F0、15项F1、9项收费候选和4项行业扩展。下面按用户每天实际操作组织28个页面宿主，覆盖基础共享、贸易和离散制造；同一页可用底部明细与详情面板承载多个动作，不要求每个按钮单独建菜单。F1动作按原优先级后续接入；收费和行业页面不属于本表首发入口。

表中的新文件、页面scope均为规划。现 `UI/page_registry.rs::PAGES` 仅登记货品查询、订单查询及开发预览/实验台；登记项存在不代表下表已经实现。旧订单查询是既有订单宿主，普通贸易销售须接实际 `API/sales_orders.rs`，不能只改旧网店订单标题。货品复用现有 `Page::Products` 时保留既有稳定身份，不为显示名称另造重复页面。

### 菜单 操作和页面文件

每行的ERP-IMPL指向第二节的API、仓储、字段、事务和性能要求。页内所有主表、子表、选择弹窗和报表列表均使用DataGrid。新文件位于UI业务层，通用控件继续在 `UI/workbench/`、`crates/erp-ui-egui/`，不要把采购或制造动作塞进网格库。

| 页面编号与规划入口 | 必须交付的操作及明细 | 规划UI文件和稳定scope | 后端接续与性能 |
| --- | --- | --- | --- |
| PAGE-01 开始使用 企业开通 | 身份验证、企业名称、贸易/制造选择；初始化进度、重试和首次真实登录 | `UI/pages/account/onboarding.rs`；`account.onboarding`，登录前独立页面 | ERP-IMPL-01；P01/P04/P21 |
| PAGE-02 系统 企业设置 | 企业资料、业务时区、币种、编号、模块开关；保存版本，关闭模块保留历史 | `UI/pages/account/company.rs`；`account.company` | ERP-IMPL-01/04；P03/P04 |
| PAGE-03 系统 账号与设备 | 当前账号/套餐、设备会话列表；撤销指定端、改密、付费席位入口 | `UI/pages/account/sessions.rs`；`account.sessions`；员工档案不列为登录席位 | ERP-IMPL-02/03；P01/P06/P07 |
| PAGE-04 基础资料 物料 | 编码、名称、规格、单位、采购/自制属性；新建、修订、条码、换算、禁用，底部SKU/单位 | `UI/pages/master/items.rs`；新宿主建议 `master.items`；复用现货品宿主时沿用原scope | ERP-IMPL-05；P02/P03/P04/P09 |
| PAGE-05 基础资料 客户与供应商 | 伙伴、联系人、地址、结算条件；客户与供应商身份可共存，详情跳往来 | `UI/pages/master/partners.rs`；`master.partners` | ERP-IMPL-05/35；P02/P03/P04 |
| PAGE-06 基础资料 仓库与库位 | 左树仓库/库区/库位；用途、状态、授权范围，新增/禁用；被历史引用的档案不硬删 | `UI/pages/master/locations.rs`；`master.locations` | ERP-IMPL-04/05；P02/P03/P04 |
| PAGE-07 基础资料 人员与班组 | 员工、班组、状态、业务组织；录入作业人档案，不发登录凭据 | `UI/pages/master/employees.rs`；`master.employees` | ERP-IMPL-05/30；P02/P03/P04 |
| PAGE-08 基础资料 导入与期初 | 选文件、映射、预检、错误行下载、确认；期初库存/批次/往来及启用对账 | `UI/pages/master/imports.rs`、`UI/pages/master/opening.rs`；`master.imports`，任务内按类型隔离 | ERP-IMPL-06/07；P04/P15/P16 |
| PAGE-09 库存 库存查询 | 主表四桶精确量及可用，按仓库/批次/物料筛选；底部来源流水、右详情跳原单 | `UI/pages/inventory/balances.rs`；`inventory.balances` | ERP-IMPL-09/11/14/15；P02/P03/P14 |
| PAGE-10 库存 库存作业 | 单据类型切换通用入出库、调拨、盘点；草稿、确认、发出/接收、差异审批或确认 | `UI/pages/inventory/stock_documents.rs`；`inventory.documents`，类型与页面实例隔离 | ERP-IMPL-10/12/13；P04/P05 |
| PAGE-11 库存 批次与序列号 | 来源、实物、预占、质量、效期及序列；冻结/解冻和来源跳转，不直接覆盖批次余额 | `UI/pages/inventory/batches.rs`；`inventory.batches` | ERP-IMPL-14/32/33；P02/P03/P13 |
| PAGE-12 采购 采购订单 | 新单/修订/确认/关闭；底部订购、收货、待检、退货、应付明细；分批收退入口 | `UI/pages/trade/purchase_orders.rs`；`trade.purchases`；收退可在关联详情开独立实例 | ERP-IMPL-16/17/18/35；P02/P03/P04/P05 |
| PAGE-13 销售 销售订单 | 手工录单、修订、确认、分批交付、合格/不良退货、关闭；自产/转售共用 | `UI/pages/trade/sales_orders.rs`；`trade.sales`；保留原网店订单入口和scope | ERP-IMPL-19/20/21/35；P02/P03/P04/P05 |
| PAGE-14 制造 BOM | 产品与版本主表、左结构树、组件子表；草稿、复制版本、展开、发布、退役 | `UI/pages/manufacturing/bom.rs`；`manufacturing.bom` | ERP-IMPL-22/23；P02/P03/P04/P05/P10 |
| PAGE-15 制造 工艺与工作中心 | 路线/工序/前置/工时；中心和日历关联详情、人工安排及超载提示；不冒称APS | `UI/pages/manufacturing/routings.rs`、`UI/pages/manufacturing/work_centers.rs`；`manufacturing.routings` | ERP-IMPL-24；P02/P03/P04/P14 |
| PAGE-16 制造 生产计划 | 从销售未满足行引入或新增备货；交期、数量、修订、冻结、取消和来源钻取 | `UI/pages/manufacturing/production_plans.rs`；`manufacturing.plans` | ERP-IMPL-25；P02/P03/P04/P05 |
| PAGE-17 制造 MRP | 提交范围与截止点、运行/取消；任务阶段；结果页毛需/供给/净缺/日期、解释、部分转单 | `UI/pages/manufacturing/mrp.rs`、`UI/pages/manufacturing/mrp_results.rs`；`manufacturing.mrp`，运行ID区分结果实例 | ERP-IMPL-26/27；P02/P04/P05/P11/P18 |
| PAGE-18 制造 生产工单 | 草稿/下达/暂停/恢复/短关闭；冻结BOM与工艺、材料/报工/入库/成本子表 | `UI/pages/manufacturing/work_orders.rs`；`manufacturing.work_orders` | ERP-IMPL-28/34；P02/P03/P04/P05 |
| PAGE-19 制造 领退耗料 | 工单、仓库、SKU、批次、原领料；批量领/补/退/耗，逐行校验与原回执 | `UI/pages/manufacturing/materials.rs`；`manufacturing.materials` | ERP-IMPL-29；P03/P04/P05/P12 |
| PAGE-20 制造 工序报工 | 冻结工序、作业人、工时、合格/不良/返工；批量报工、原报工冲正、余量查询 | `UI/pages/manufacturing/reporting.rs`；`manufacturing.reporting` | ERP-IMPL-30；P03/P04/P05/P12 |
| PAGE-21 制造 完工入库 | 有效报工待入量、仓库/批次/质检；分批入库、原效果查回、检验跳转 | `UI/pages/manufacturing/receipts.rs`；`manufacturing.receipts` | ERP-IMPL-31；P03/P04/P05/P12 |
| PAGE-22 质量 检验与处置 | 标准/必检项/读数、合格/不良量；放行、复检、退回、报废及处置来源 | `UI/pages/quality/inspections.rs`、`UI/pages/quality/dispositions.rs`；`quality.inspections` | ERP-IMPL-32；P02/P03/P04/P05 |
| PAGE-23 质量 批次追溯 | 原料向成品、成品向原料；首批结果与分页、跳实际来源，导出完整授权关系 | `UI/pages/quality/traceability.rs`；`quality.traceability` | ERP-IMPL-33；P02/P13/P16 |
| PAGE-24 制造 工单成本 | 实际材料、工时/费率、费用、完工/在制/报废；暂估、确认、重算和成本来源 | `UI/pages/manufacturing/costs.rs`；`manufacturing.costs` | ERP-IMPL-34；P02/P03/P04/P14 |
| PAGE-25 财务 往来与收付 | 伙伴账龄、原单未结、现金未分配；收付款、部分核销、原核销冲销与对账单 | `UI/pages/finance/balances.rs`、`UI/pages/finance/settlements.rs`；`finance.settlements` | ERP-IMPL-35；P02/P03/P04/P05/P14 |
| PAGE-26 财务 费用与期间 | 原费用、分摊依据、期间状态；确认、调整、关期前差异、关期后拒绝/授权重开 | `UI/pages/finance/expenses.rs`、`UI/pages/finance/periods.rs`；`finance.periods` | ERP-IMPL-15/34/35；P03/P04/P05/P14 |
| PAGE-27 报表 经营与制造 | 固定5种报表、截止点、暂估；主表聚合、来源钻取、保存视图、图表与导出同口径 | `UI/pages/reports/business.rs`、`UI/pages/reports/manufacturing.rs`；`reports.business`，报表ID隔离查询 | ERP-IMPL-36/38；P02/P14/P16 |
| PAGE-28 系统 任务审计与帮助 | 导入/导出/打印/MRP阶段及失败、原命令回查；授权审计、反馈、基础数据退出入口 | `UI/pages/system/tasks.rs`、`UI/pages/system/audit.rs`、`UI/pages/system/support.rs`；`system.tasks`，业务任务与运维权限分开 | ERP-IMPL-04/38/39/40；P02/P07/P16/P17/P18/P20 |

### 页面接入必须改哪些现有文件

1. 在现 `UI/page_registry.rs` 为已接通的正式页面登记模块、分组、名称、权限和可用状态；现 `UI/erp_app.rs`、`workspace_shell.rs` 接入页面创建及切换。没有真实API的页面仅保留开发入口，不显示空正式菜单。
2. 共享布局复用现 `UI/workbench/layout.rs`、`toolbar.rs`、`detail.rs`、`subtable.rs`、`theme.rs`。工具栏只放主表上方，左操作、中搜索、右面板设置/帮助/导出；底部不复制工具栏。模块开关只改变可见入口，不删除事实。
3. 页面模型与渲染建议分 `model.rs`、`query.rs`、`actions.rs`、`view.rs`；复用现 `UI/http.rs` 和 `UI/http/web.rs` 的两端适配设施。如后续拆出 `UI/http/native.rs`，属于规划迁移，不能在文档中当作现有文件。
4. 草稿、未知命令与布局复用现 `UI/draft_store.rs`、`action_store.rs`、`leave_guard.rs`；按服务、企业、账号、页面实例隔离。不能把同一共享账号的另一设备草稿当成已提交业务。
5. API接入现 `API/router.rs`，制造现 `API/manufacturing.rs`，采购现 `API/procurement_handlers.rs`，贸易销售现 `API/sales_orders.rs`。DTO与源OpenAPI同步；仓储叶模块接受上层事务，不另立余额或回执真相。

两端各验收真实登录、查询、编辑、保存、失败、冲突、未知结果及恢复。截图、模块夹具、列表只读和单独HTTP成功分别留证，不能替代页面完成。

## 七 页面状态和操作结果的具体规则

### 主表 子表和详情分工

主表展示可筛选的业务对象；底部显示选中对象的明细及关联单据；右侧编辑单头或当前明细。左侧只承载当前页面分类/筛选树，顶部承担模块菜单和页面标签。隐藏面板保留尺寸、查询与草稿；深浅主题切换不重建页面。

新单据明细至少有稳定行ID、物料、规格、业务单位、数量、价格/税额或生产来源。必填、精度、引用和余量错误附在对应行/字段；连续键盘录入支持Tab/Enter移动、对象选择、粘贴预检、撤销未提交编辑。一次批量动作默认整单原子，行级部分成功必须是另有明文契约的操作。

| 页面状态 | 按钮及数据显示 | 离开或恢复规则 |
| --- | --- | --- |
| 待查询/查询中 | 搜索可继续输入；按当前结果执行的批量按钮不可用，旧结果有过期状态；取消旧代次，应用新代次 | 切页取消只读请求；旧回包不覆盖新筛选和详情 |
| 查询失败/无权限 | 显示实际错误与重试入口；不把旧金额/数量显示成新条件的结果 | 保留条件和草稿；401重新登录后恢复，403不自动扩大范围 |
| 干净草稿 | 新建或编辑可操作；列表刷新不改稳定行ID | 保存前本地校验；草稿恢复不自动提交 |
| 脏草稿 | 修改标记与字段错误可见；隐藏面板和切标签保留输入 | 关闭、刷新、批量关标签按实际脏页保护；取消关闭不丢输入 |
| 正在提交 | 同一业务意图的提交按钮不可重复触发；其他只读页面可操作 | 写入不随关页中止；继续等待原回执，不能换request_id再发一笔 |
| 结果未知 | 展示原命令与可回查状态；区分传输失败与后端明确拒绝 | 重登/重启后按原身份查回；仅同ID同内容安全重放，不将未知直接标失败 |
| 已完成 | 只有提交回执确认后显示完成，并刷新实际版本/余量 | 后续余额变化不改写原回执；审计和上下游能找到同一效果 |
| 版本冲突 | 展示冲突字段/新版本与保留草稿的选择；禁止静默覆写 | 用户重新核对后形成新的业务意图，原未知命令先核对 |
| 后台已受理 | 显示任务ID、排队/执行/提交/完成/失败阶段与取消条件 | 202只代表受理；取消不能撤销已经提交的业务，下载再鉴权 |

上述是交互语义，持久化状态枚举沿现有各域契约兼容映射，不为统一文案直接重命名历史值。查询总数、框选求和、勾选汇总与单据合计各用各自完整范围，金额和小数不得用浮点。

### 搜索 分页和选择的验收细节

- 搜索默认展开，250ms防抖，中文输入法组合中不查询，组合结束按最终值查询；金额上下界直接输入，非法范围保留输入并阻止旧结果动作。
- 筛选和排序先作用于完整授权集；排序追加稳定ID。每页100/500等选项必须与真实服务契约对应；四个公开库存读取接口当前上限2000，但正式页面500/2000档仍须有真实行数、响应预算及性能验收，其他接口按各自上限；不全量下载后切片冒充服务器分页。
- 勾选和序号为固定结构列，序号跨页连续且不是RowId；全选当页、全部匹配与明确ID集合分开。跨页批量动作冻结查询身份/截止点，不能静默忽略未加载记录。
- 主表切行后的子表/详情查询带父ID及代次；迟到回包不得串行。改变查询、页面或权限后，原选择是否保留按稳定ID与查询契约判定。
- 网格保留宿主高度，短表滚动条在底部；列宽/顺序/锁列按稳定ColumnId保存。性能验收同时覆盖满表末行裁剪、编辑可见和中文输入。

## 八 可直接执行的业务验收剧本

以下14条均为目标验收方案，并非本次执行记录。使用隔离测试企业和真实API；UI步骤在桌面与Web各执行一遍，事实由另一连接或独立文件解析复算。除专门并发组外使用不同request_id；同意图重试固定原ID。数量均为基本单位，人民币使用分，样本默认不含税、无折扣，另有税价/精度边界用例。各剧本使用独立样本，期初、冻结及预占由对应来源动作建立，不能仅修改余额造出无来源库存；受控测试夹具如需SQL初始化，必须声明事实来源并独立对账。

### UAT-01 企业开通和唯一账号

输入企业甲/乙，各选择贸易和制造开关、人民币、Asia/Shanghai；20个并发请求使用同一企业甲开通身份。期望企业甲只有一个有效企业、一个业务登录账号和一份默认配置，重试返回原企业ID。企业乙独立生成ID；员工档案不新增席位。初始化必需配置中途失败后继续原进度，不生成第二企业或可操作的半初始化身份。接ERP-IMPL-01/02与P01/P04/P06。

### UAT-02 免费多端 会话和改密

企业甲使用真实密码在桌面和浏览器登录，保留不同session_id；同浏览器共享会话的标签不算不同席位。按当前配置开到8个有效会话，第9个明确拒绝且不影响原8个。撤销一个会话后该端新请求失效、其他端继续；改密提交后旧凭据/旧会话新请求拒绝。20路竞争至少100轮均不超过配置；免费新启用第二业务用户拒绝，业务员工仍可登记。所有端沿原命令查已提交结果。接ERP-IMPL-02/03与P01/P06/P07。

### UAT-03 单位 小数和历史兼容

物料基本单位件，换算版本V1为1箱=12件；录入2箱加3件，基本数量27件；领0.5箱即6件，剩21件。允许精确0.000001，0.0000001及非法文本拒绝，全部用十进制字符串传输。改变后续换算版本不改历史27/6/21；旧整数1仍为1件。列表、过滤、合计、导出与来源账本按相同单位重建。缺失换算或超范围不截断数量、不写部分行。接ERP-IMPL-05/08/29与P04/P05/P12。

### UAT-04 采购 销售 退货和往来闭环

本样本成本采用已确认的每件30元，无附加费用。每一步记录原单、原动作、余额、成本和往来；同ID重放不新增效果。接ERP-IMPL-16至20/35/36，P02/P03/P04/P05/P14。

| 步骤 | 用户操作 | 必须独立核对的结果 |
| --- | --- | --- |
| 1 | 采购10件，每件30元，确认后分批收6件和4件，均待检 | 收货10、实物10、待检冻结10、可用0；不能销售或领料 |
| 2 | 检验合格8件放行，不良2件退供应商；本单不补货，明确关闭剩余2件 | 实物8、冻结0、可用8；净采购8件/240元，应付240；原收10和退2均保留，关闭量2可查 |
| 3 | 销售6件，每件50元，分批交付4件及2件 | 实物2、净交付6、应收300元、出库成本180元；订单余量0 |
| 4 | 按原交付合格退1件并冲减客户欠款50元 | 实物3、净销售5件/250元、净成本150元、确定毛利100元、应收250元 |
| 5 | 付款240元核销供应商应付；收款200元核销客户应收 | 应付0、应收50元；收付款未分配均0；核销可回查，不用已付款状态替代账本 |
| 6 | 查询收发存、销售日报、往来及导出，试删已执行原单 | 数量、金额与上述结果一致；原单删除拒绝，更正沿引用冲正；当前页/全部范围统计一致 |

### UAT-05 精确可用量和共享库存竞争

同SKU实物1.900000、预占0.900000、冻结0.900000，可用必须0.100000；不得按兼容整数显示1。请求出库0.100001应明确不足；出库0.100000后可用0，冻结/预占仍各0.900000。另建可用1件样本，20路各扣1件，最多1成功，19明确不足；100轮独立重建样本并证明实际锁竞争。贸易、制造、WMS、质量、批次及直接SQL规则都不能绕开权威服务；拒绝后流水/回执/来源无半次效果。接ERP-IMPL-08至13/29与P02/P04/P05。

### UAT-06 基础制造到质检和成本

隔离企业使用成品F，BOM每1件用A=2、B=0.5；计划10件，A库存25/B库存6，成本A每件10元/B每单位20元。仅有这一工单、无其他占用，全部来源可独立核对。接ERP-IMPL-22至24/28至34，P04/P05/P10/P12/P13/P14。

| 步骤 | 用户操作 | 必须独立核对的结果 |
| --- | --- | --- |
| 1 | 发布BOM/工艺，建计划10件工单并下达 | 固定原BOM/工艺版本；预占A20/B5，仓库实物仍25/6，可用5/1 |
| 2 | 按本工单预占领A20/B5，再退A2/B0.5到原批次 | 仓库实物A7/B1.5、预占0；工单持料A18/B4.5；原领、退、批次及回执一致 |
| 3 | 记录实际耗A18/B4.5；报有效产出9件，实际人工45元、费用45元 | 不第二次扣仓库；工单持料0，材料成本180+90=270元，总成本360元 |
| 4 | 按原有效报工入库9件，全部待检 | 成品实物9/冻结9/可用0；同报工再入10件或重复原命令不能多入 |
| 5 | 检验8件合格放行，1件不良报废；按明确短产原因关闭计划差1件 | 成品实物8/冻结0/可用8；计划10、有效报产9、入库9、检验报废1、短产关闭1分别可查 |
| 6 | 确认成本并双向追溯 | 入库9件每件40元，留存成品320元、报废40元、在制0；成本合计360，关联实际A/B批次，不用计划材料代替实际耗用 |

本例的报工产出与后检验判定是不同阶段，不把后检验不良反写成原报工不存在。其他合格/不良/返工、部分完工、暂估及晚费用另建样本；初次成本未确认不得先显示确定每件40元。

### UAT-07 工程版本 工序前置和冻结

工单甲下达时固定BOM V1/路线R1；复制V2改变用料并发布，仅后续符合生效政策的工单乙引用V2。甲实际材料仍按原V1，不能刷新后自动替换。路线两工序有明确前置，前序未提供允许的有效产出时后序报工拒绝；已发布版本覆写、交叉企业组件、结构环及33边深度拒绝。允许合法32边结构并检查展开结果。接ERP-IMPL-22至24/28/30与P03/P04/P05/P10/P12。

### UAT-08 MRP 数量 供给和日期

制造需求F=10件，F可用0；每件用A2/B0.5，损耗/安全库存/最小批量为0，不额外整量舍入。A实物5、预占3、冻结1，可用1；另有A确定采购4于2026-10-07可用。B实物1、冻结0.5，可用0.5，无确定供给。所有材料需要日2026-10-12；本测试自建周一至周五日历且无额外假日，产品提前期0，A采购提前期2工作日、B1工作日。

期望A毛需20、净缺15、建议下达2026-10-08；B毛需5、净缺4.5、建议下达2026-10-09。待检/在途未确定可用部分和他来源已分配供给不能重复抵扣；保存每项供给及日期解释。读取由服务器固定的真实快照，不接客户端自报库存。接ERP-IMPL-25/26与P03/P11/P18；这是测试日历，不声称等同实际节假日安排。

### UAT-09 建议部分转单和运行过期

沿UAT-08，把A建议15先转采购10，再转5，两次分别查目标采购行与原转换；剩余为5再为0。相同命令重试返回原目标；第三次转1拒绝。20路竞争同余量至少100轮，合计转量≤15。MRP后修改相关库存或需求输入，旧建议明确过期并拒绝继续按旧快照确认；重新计算得到新运行，已提交原采购事实保留，不自动撤销。B4.5保持小数，不能截成4。接ERP-IMPL-26/27与P04/P05/P11。

### UAT-10 报表 导出和真实打印

以UAT-04/06来源核对固定5种报表；相同查询身份/截止点的屏幕、导出、图表和钻取一致，未知成本另列。准备跨多页10万行文件，独立解析行数、单位和总额；不是导出当前页。CSV/XLSX公式开头文本按安全策略处理，附件/下载身份跨企业拒绝。A4与标签在真实约定设备打印，缺纸/离线不记已出纸；对重复发送分别核对任务去重和设备可能产生的物理结果。接ERP-IMPL-33/36/38与P13/P14/P16/P17。

### UAT-11 多端编辑 搜索和草稿

同账号桌面/Web打开同单同版本，各改不同字段同时保存；版本竞争后只有一个成功，另一个保留输入并明确冲突，不静默合并。输入中文、连续修改筛选并人为延迟旧请求，组合中0查询，最终250ms后请求，旧结果0次覆盖。批量粘贴最后行非法时整批保存拒绝；隐藏详情、切主题、切标签、重启恢复草稿，未决写回执不丢失。接ERP-IMPL-37与P05/P08/P09/P19/P21。

### UAT-12 原子回滚和丢响应

100行库存或材料动作的最后一行余额不足/引用不合法，全部业务数量、状态、价值、来源、回执与Outbox无部分提交。另一次动作在服务端提交成功后丢弃响应，客户端显示结果未知；重启/重登后沿原request_id回查得到原效果和原快照，后续余额变化不改原响应。提交前、提交后、Outbox消费前后分别注入故障，外部效果依契约去重，不能仅用HTTP200证明一次效果。接ERP-IMPL-09/29/35/39与P04/P05/P12/P18/P19。

### UAT-13 企业 仓库和导出范围

企业甲/乙使用真实账号及限制仓库权限；互换单据、SKU、批次、原领料、工单、任务、附件和打印身份。不得漏读、写入或以他企业引用生成本企业效果；按公开契约返回403或隐藏性的404。空授权仓库集合表示无仓库访问权，不等同无限制；全部匹配和汇总先应用授权再分页。撤权后新请求拒绝，下载与后台执行再次检查适当范围。接ERP-IMPL-02/03/08/33/38/39与P02/P05/P07/P13/P16。

### UAT-14 完整迁移 恢复和免费退出

隔离库带旧整数库存、历史小数、来源预占、未完工单、收付核销、附件和未决任务，走完整注册迁移链；迁移前后按同截点比较所有事实，来源不明须显式对账，不能补0通过。至少3次独立恢复演练测RPO≤15分钟、RTO≤2小时，截点内已提交效果和原回执不丢、后台恢复不重复做业务。退出下载可独立解析基本档案、业务与台账；免费用户不必先购买席位才能导出基本数据。公开服务、恢复产物、附件截点和双端运行另留证。接ERP-IMPL-04/39/40与P16/P18/P20/P21/P22。

## 九 性能埋点和资源超限的实施规则

本节落实第四节的22组指标，不另建相互矛盾的阈值。配置预算是候选发布方案，须实际压测和成本计量后固定；免费仓库数、订单额度、付费价格仍未决定，不把技术保护预算直接写成商业收费政策。

### 计时字段和代码落点

| 计量段 | 必须记录的字段与边界 | 现有接续和规划实施位置 |
| --- | --- | --- |
| 压测器调度和公网 | 计划发起、实际发起、客户端收到完整结果；发起迟到、DNS/TLS/传输、超时。公网端到端包含客户端等待 | 规 `qa/performance/run-erp-api.mjs`；输入固定动作/规模/速率，输出逐请求样本 |
| API总处理 | 收到完整请求到最后响应字节交给传输层，包含身份、企业范围、池等待、SQL、事务、编码；另记实际HTTP状态 | 现 `crates/erp-common/src/telemetry.rs`；规 `API/telemetry/business.rs`，在路由外层与响应流完成点埋点 |
| 数据库池 | 等待开始、取得连接、释放；业务动作、连接等待数、超时、在用/空闲数 | 规 `REPO/db_observability.rs`；池获取处记录，不能只计取得连接后的SQL |
| 行锁与事务 | 锁申请/取得、事务开始/提交完成；锁等待、事务耗时、回滚原因、重试次数；锁竞争测试另保存阻塞关系 | 现精确库存及各域事务；规统一事务观测包装，保留域名/动作名，不记录敏感SQL参数 |
| 写入效果 | 原命令、对象、动作、提交版本、提交成功时点、回执状态；202的受理时点独立 | 现 `REPO/shared_contract.rs` 及其叶模块；API以真实提交回执决定完成 |
| 后台任务 | 创建/入队、开始、快照就绪、计算结束、提交结束、产物就绪；排队/执行/总等待、租约、取消与失败 | 现 `WORKER/runtime.rs` 与具体jobs；规MRP/成本/admission叶模块 |
| 客户端绘制 | 输入、本地反馈、请求发起、完整回包、首次应用有效数据、首可操作帧；UI代次及父对象ID | 现 `UI/http.rs`、`UI/performance.rs`、`UI/query.rs`；规各正式页面状态记录 |
| 内存和队列 | 进程工作集、任务用途计数、页面驻留块/行/字节、在途请求、队列深度；基线和峰值均留原样本 | 现UI/远端资源计数与 `qa/performance/`；规进程采样器和运行仪表板 |

持续时长用本进程单调时钟，跨机器通过trace关联各段，不直接相减未校准墙钟。总P95/P99从同一请求总时长计算，不能把各段P95相加当作总P95。API流完成不等于浏览器可操作；客户端另测解码、应用数据和绘制。

指标标签限定域、动作、状态码、套餐等有界集合；企业ID/单据ID/命令ID只进入受控trace或抽样日志，不作为无限增长的监控标签。不记录密码、访问/刷新令牌、附件内容或完整客户资料。

### 请求样本和错误必须怎样统计

20请求/秒持续30分钟的稳态目标为36000次计划请求：列表21600、详情5400、保存5400、认证1800、任务查询1800。预热5分钟另有6000次，不混进稳态；100请求/秒持续60秒的突发另有6000次。计划、发起、完成、超时、未发出的迟到请求均保存；压力发起器本身不足时本组无效，不能直接把少发请求算服务通过。

50会话分布在100个非空企业的数据基线上，每个免费企业仅一个业务账号，每账号有效会话不超过配置。认证样本真实调用密码登录，并按测试计划撤销或回收旧会话；不为压测关闭席位和会话准入。强占席位/第9会话等预期拒绝单独成组。

混合负载用于验证系统容量与公平性；每一个待发布列表/动作、100/500行规模和两端环境仍需分别满足至少1000样本，不能拿36000个混合总请求声称所有页面都测了1000次。P10/P11/P15/P16等慢任务每类30次完整运行，冷/热和格式分别分组。每组保留固定随机种子、数据摘要、查询形状和成功/失败数量。

非预期错误率=非预期失败请求数/实际发起请求数，5xx、超时、传输失败及正常组不应发生的4xx都计入。迟到未发出的请求单列容量缺口。正常组要求<0.1%，因此36000次稳态最多35次非预期失败；业务不变量错误仍为0。故障/越权/限流组可以有预期拒绝，需验证拒绝原因及无部分效果，不能从正常组移出失败来改变结论。

延迟文件不能只保留成功请求。超时保留实际截止耗时与结果未决标记；超时是观测下界，不能按普通已完成样本宣称精确P99。成功分布、包含失败的观测分布、未决数及错误率分别公开；还有未决业务时本组不具备完整业务验收结论。

### 资源预算和超限结果

下表区分现有硬限制与候选预算。配置变更必须同步API/DTO/界面可选项和压测报告。限制后的拒绝、恢复和其他企业正常访问都属于验收。

| 资源 | 现有或候选值 | 到达边界的行为与验收 |
| --- | --- | --- |
| 免费业务席位/会话 | 现1席位；有效会话当前默认8，按服务端有效配置 | 第二业务账号或额外会话明确拒绝；员工档案不收费占位，单端撤销不踢其他端 |
| 远端表缓存/在途 | 现CORE契约每表2在途、10块/1000行/8MiB | 取消旧代次和淘汰可重载块；脏草稿/原命令不随页缓存淘汰；不无限驻留未加载全选数据 |
| 普通查询响应 | 候选100行≤256KiB、500行≤1MiB；现 `UI/http.rs::MAX_BODY` 普通响应硬上限8MiB | 列表不夹带附件实体；超限报告明确错误，不静默截断；大文件使用独立流式下载 |
| 普通请求体/明细 | 候选JSON体≤2MiB、单据≤1000行；20/100行按P04基准 | 体超限按契约413、行预算超限422；需大批时持久化后台处理，不能分多笔独立提交冒充一单原子 |
| 普通客户端等待 | 现HTTP普通超时15秒；列表和保存性能目标仍为P02/P04 | 超时取消只读请求；写入标结果未知并回查原命令。慢任务在≤500ms受理后查任务，不长挂一个普通HTTP |
| MRP/BOM计算 | 候选1万参与物料、5万边、32层、90天；P10新增≤256MiB/P11≤512MiB | 超结构或解释预算明确拒绝/分页；取消按P11，不能把未算部分显示成功；图上共享子树不复制指数路径 |
| 导入 | 候选1万行×20列、输入≤10MiB，新增≤128MiB；P15预检30秒/导入60秒 | 明确受理及逐行预检；超限拒绝，确认结果可查；不能只导前1万行而报整文件成功 |
| 导出 | 候选10万行×20列、输出≤50MiB，新增≤256MiB；CSV30秒/XLSX60秒 | 更大范围显式拆文件或扩展已验预算；下载流式，过期可重建授权任务；不能仅导出当前页 |
| 重任务并行/排队 | 候选每企业MRP1/导出1、全局重任务2；待执行每企业10/全局100 | 满队列429并给可理解的重试信息，任务查回不重复入队；另一9企业查询/写入保持P02/P04 |
| 连接池 | 候选各副本API池合计≤16、Worker≤8、其他业务≤8，总≤32，另预留至少8维护连接 | 配置按所有副本求和，池等待完整计时；排队有截止，失败不遗留事务，不能无限加连接掩盖慢SQL |
| SQL与访问数 | 候选列表数据SQL≤4、20行详情≤6，授权查询另列，响应字段按业务需要 | 关联数据集合读取，不逐行网络查询；确需超预算解释查询计划和实际证据，P02/P03仍须通过 |
| 长时驻留 | P19客户端同状态≤预热基线+10%+20MiB；服务器24小时末4小时无无法解释增长 | 任务/请求/文件句柄有界；保留增长样本，不能清缓存或重启后替换失败结果 |

CPU和服务器内存同时记录总量及用途；当前4vCPU/8GiB服务基线里包含API/Worker合计资源，不能给每个进程各用8GiB却仍称同基线。进程内存峰值至少每100ms采样一次并公布工具和粒度；更短瞬时峰值由用途计数/分配工具补测。Web额外报告JS堆、浏览器进程和GPU信息，不能用单个堆数字代替总内存。

### 上线前必须固定的容量和成本参数

免费仓库数、累计单据额度、附件容量和保留期、活跃企业容量、月预算B尚未确定。Grok先交计量、配置、达到配额时的可解释行为与历史数据保护，不擅自删历史、不以付费开关封锁基础导出。产品发布清单填好这些参数及实测容量后才向用户公布；本规范的100企业数据基线和50在线会话是测试条件，不是已验证的正式用户上限。

P22计算月度基础设施总成本、固定空闲成本及按可核对计量分摊的活跃企业成本；API、数据库、备份/附件存储、出口流量和任务资源一起列入。收费候选与公共资源配置分别实现，免费账号升级沿原企业ID和原业务事实，不复制一套账。

## 十 交给 Grok 的单批开发输入

每批沿第五节顺序，范围用原功能ID、ERP-IMPL、PAGE和UAT绑定。下面是可直接使用的任务格式，实际目标按本批填写；这段是交付要求，不是已经运行的报告。

```text
阅读 AGENTS.md、PRODUCT_SCOPE、ROADMAP、FREE_ERP_CHECKLIST 与本规格。
本批功能：填写原功能ID / ERP-IMPL / PAGE / UAT，列清依赖及结束条件。
接续：列现有文件、函数、HTTP方法/路径、权威表；新文件注明规划职责。
实现：真实查询+编辑+保存+原回执；库存/价值/状态同事务，范围和版本不跳过。
界面：Rust egui+DataGrid，正式页面真实接入；保留草稿和未知结果。
测试：正常、边界、最后行失败、20路竞争、原命令重放、跨企业/仓库及独立读回。
性能：指定ERP-P、固定规模/环境/Release产物、原样本数和时间/内存边界。
执行：只用统一构建入口，按受影响范围验证；完整迁移和真实双端分开留证。
交付：变更文件/函数与接口、迁移/索引、原始日志、独立预期与读回、产物及输入SHA。
结论：每个门禁PASS/FAIL/NOT_RUN等；历史通过不得绑定变化后的源码。
回写：ROADMAP记本批事实与未完成项，反馈≤500字；不直接勾选整套免费ERP上线。
```

一个批次只有API或只有页面时，可以记录这一层完成，但对应功能整体仍为PARTIAL。验收单位是用户能完成的业务及其真实事实；文档写得完整、函数有实现、接口能返回和线上业务可用分别记录。

## 十一 批次库存逐动作实施规格

本节进一步细化I02/I04/I08、T02/T04/T05、M09/Q03，以及ERP-IMPL-10/14/17/20/21/29/33；沿用PAGE、UAT和ERP-P，不另建产品范围。2026-10-04已实施共用精确批次内核、真实SKU流水ID返回、UUID原命令、制造材料和原OMS接线；首轮实际HTTP/数据库回归13通过，覆盖范围和未通过项见本节交付记录。后续采购收退、销售退货、调拨、历史核对、正式双端及Release性能仍须分别实施验收。

### 当前写入口和必须修复的差距

| 入口 | 实际文件与符号 | 本次源码核对结果 | 开发完成条件 |
| --- | --- | --- | --- |
| 无原编号的入库与调整 | 现 `API/inventory_print_report_handlers/inventory_core.rs::apply_inventory_movement`；现 `REPO/inventory.rs::{apply_adjustment_with_tx,apply_adjustment_effect_with_tx}` | 已委托共同精确服务和批次内核；不再直接修改旧整数列，错企业/错SKU批次404，未知精度409 | 合法小数批次在旧整单位动作后保留余数；SKU、批次、两类日志在同事务，其他收退入口分别接线验收 |
| 有原编号的入库、出库与调整 | 现 `API/inventory_print_report_handlers/stock_movement.rs::{inbound_inventory,outbound_inventory}`；现 `API/inventory_print_report_handlers/inventory_core.rs::adjust_inventory` | 三个handler已接受实际UUID `batch_id`；nil与未映射的库位/字符串混合400；整单位1仍是1000000微单位 | 原批次效果与GET回查一致；正式桌面/Web DTO、提交前原ID持久化和未决恢复仍待验收 |
| 原命令输入和结果 | 现 `REPO/inventory_movement_commands.rs::{InventoryMovementCommandInput,InventoryCommandSnapshot,execute,find,verified_snapshot}` | 新UUID字段缺省时不序列化；非空参与摘要；快照保存真实`batch_effects`，回查核对原流水和不可变子效果 | 旧请求摘要与无UUID快照兼容须在完整回归验证；原结果不因后续余额、批次名称变化而漂移 |
| 制造领补退料 | 现 `REPO/shared_contract/material_moves.rs::{post_material_move,apply_stock}` | 已委托同一内核；NULL精度不推算；实际领退效果绑定真实SKU流水；CONSUME保留原持料来源且仓库无第二次扣减 | 有实际来源映射前不消费批次已有预占；多工单、多原领料、原材料批量和正式页面仍须逐动作验收 |
| 原OMS的FEFO发货 | 现 `REPO/inventory.rs::consume_batches_by_fefo`；现 `REPO/inventory_batch_consumption.rs::consume`；现 `REPO/inventory_order_consumers.rs::apply_order_terminal_items` | 将实际SKU前后量和RETURNING流水ID交给同一批次内核；保留归属、精度、总量、效期、自由量和1000条预算 | 跨批子效果之和等于实际SKU物理delta；完整OMS成本/来源/状态/日志/事件失败须整单回滚 |
| SKU库存流水 | 现 `REPO/inventory_precision.rs::{InventoryPreciseEffect,apply_precise_effect_with_tx,apply_precise_with_tx}` | 新效果入口使用INSERT RETURNING；现余额返回方法委托并保留兼容 | 实际ID对应完整所属键与原来源；不查询“最新流水”猜归属；后续批次/审计/回执失败连同该流水回滚 |
| 逐批效果与历史防改写 | 现 `REPO/inventory_batch_effects.rs::{InventoryBatchEffect,apply,verify}`；现迁移`migrations/0288_inventory_batch_movements.sql`、`migrations/0289_inventory_batch_movement_integrity.sql` | 真实批次余额和资料锁内核对；有界VALUES更新/插入；不可变子表、完整所属键外键、提交时物理delta守恒、父流水防改写 | 大微单位以十进制字符串返回；新空批次明确零开账；资料快照不随当前资料改写；容量、锁竞争与失败留独立事实 |


`sku_batches.batch_id`是实际SKU批次UUID；`shared_batches`是按物料编码/批次码建立的契约资料；共享库位流水还含`location_id/batch_code/quality_status`。只有具备企业、SKU和原来源的真实映射时才能贯通。实现期间可以明确拒绝暂不支持的混合维度，免费首发验收仍要求选定业务的真实批次闭环。

### 请求字段 页面操作和原结果

| 字段或动作 | 输入与类型要求 | 服务端必须核对 | 页面和回执必须表现 |
| --- | --- | --- | --- |
| 企业与操作者 | 取已认证上下文，不接受客户端任意指定有效actor | 当前企业、账号、权限和仓库范围；不能靠缓存页面判定 | 页面使用当前身份；登录变化后保留未决命令原ID |
| `request_id` | 沿现1至128可见ASCII字节；头/体同时给出时必须一致 | 同ID同规范负载重放；变仓、SKU、批次、数量或actor拒绝 | 提交前生成并保存；超时显示未确认，通过原GET恢复 |
| `warehouse_id/sku_id/batch_id` | UUID；批次UUID已接入现原命令字段 | 批次实际归属企业和SKU；仓库中余额身份完整；拒绝nil与错归属 | 批次选择先按实际SKU/仓库筛选；不能展示他企业元数据 |
| 数量 | 内核使用六位精确定点；现库存移动整单位整数1继续换算为1000000微单位 | checked运算、非零及动作方向；小数新契约必须明确字段/版本并更新OpenAPI和双端DTO | 展示真实小数及基本单位；不将旧整数响应当作无损小数结果 |
| 库位/批次码/质量 | 沿已有库位请求字段；不能仅因文字批次码相同就等同UUID | 映射唯一且归属正确；未支持的混合输入明确4xx且0效果 | 校验错误保留输入；质量冻结或过期原因附对应控件/行 |
| 原领料/收货/交付 | 退料、采购退货、销售退货分别引用实际原行和原效果 | 原企业、SKU、仓库、批次及尚可退余量；累计量锁内复核 | 选原来源后带出批次；不让操作者换一个批次承接原成本 |
| 版本与原因 | 单据版本沿其正式契约；调整和异常动作必须给原因 | 当前版本、有效状态、期间和允许动作；不可覆盖历史事实 | 冲突保留草稿并显示当前版本；原成功结果独立于当前余额 |
| `batch_effects` | 现结果数组；每项含真实库存流水ID、批次效果ID、精确delta、原前后量及决策快照；六个微单位量使用十进制字符串 | 验证原命令身份及实际效果关联；金额另引用原成本事实 | FEFO多批分行；GET返回原结果，不重新执行推荐或重算成本 |
| 查询/保存/导出 | 复用现工作台与DataGrid；规 `UI/pages/inventory/batches.rs` 和相应正式单据页 | 完整授权查询先筛选/排序再分页；后台导出保留实际查询范围 | 桌面与Web各自验查询、选择、保存、失败及文件落地，夹具不计通过 |

### 可交给 Grok 的八个实施任务

下表按现有实现更新实际落点；标为“规”的部分仍是待实施职责。已经有内核和局部测试不代表整个任务包完成；每批结束条件具体到独立事实，不能只填写“批次管理已完成”。

| 任务与前置 | 要修改的现有位置和拟新增位置 | 必须实现的行为 | 必须交付的验收 |
| --- | --- | --- | --- |
| BATCH-DEV-01 返回真实库存效果 | 现 `REPO/inventory_precision.rs::apply_precise_with_tx`；现同文件`InventoryPreciseEffect/apply_precise_effect_with_tx`；现余额接口保留委托 | INSERT RETURNING真实ledger_id和落库事实；效果在当前事务内返回，不独立commit，不查询最新行 | 实际ID对应本租户/仓库/SKU/原来源；库存在后续错误时连同该流水回滚；CASE-01/07/10 |
| BATCH-DEV-02 统一批次写内核，依赖01 | 现 `REPO/inventory_batch_effects.rs::{apply,verify}`；现 `REPO/inventory.rs`及`REPO/lib.rs`委托/登记 | 完整键、精确量、已有预占归属、元数据和有界计划；写前验证，稳定锁序，检查实际影响行数 | CASE-01至06/11/15；错误后SKU、批次、来源及必需记录全无新效果 |
| BATCH-DEV-03 不可变逐批流水，依赖01/02 | 现迁移0288/0289建立并保护`inventory_batch_movements`；现同叶`apply/verify`；现 `migrations/0283_inventory_precision.sql`守卫保留 | 新表关联真实SKU流水；记录前后量和实际元数据；业务应用不允许UPDATE/DELETE历史效果 | CASE-07/09/10/14；独立连接双向核对，缺一项效果时整单失败，不伪造历史追溯 |
| BATCH-DEV-04 原命令UUID批次，依赖02/03 | 现 `REPO/inventory_movement_commands.rs::{normalize,execute,find,verified_snapshot}`及输入/快照；现三个库存handler；OpenAPI源和两端DTO | 可选batch_id及原批次效果参与命令验证；无批次旧输入序列化/摘要保持兼容；新混合维度规则显式 | CASE-08/09/16；20路同ID只一次写入，原GET不依赖当前余额或变更后的批次名称 |
| BATCH-DEV-05 制造材料接线，依赖02/03 | 现 `REPO/shared_contract/material_moves.rs::{post_material_move,apply_stock}`；现材料原来源和回执；规正式材料页面 | 领/补减真实仓库批次，退加原批次；耗用只减已领持料来源，不再次扣仓库；原预占消费有批次来源证明 | CASE-03/06/12/13；同工单不同原领料、同SKU不同工单及不同批次互不借用 |
| BATCH-DEV-06 FEFO与收退调拨，依赖02/03 | 现 `inventory_batch_consumption.rs::consume`；现采购/销售/调拨实际服务；复用原OMS事务 | 冻结已选批次和数量后共同提交；采购收退、销售交退、调拨两端及盘点差异追加实际效果；原已提交动作不重做 | CASE-07/09/10/14；每种业务分别从真实原行到库存/批次/价值读回，不以一个OMS场景代替 |
| BATCH-DEV-07 历史核对与迁移，依赖02/03 | 规 `REPO/inventory_batch_reconciliation.rs`；规独立QA预检/读回；0288/0289仅已在受保护QA库应用；后续迁移编号从当前目录分配 | 按事实区分新零余额、可证明历史、未知/矛盾历史；形成核对清单和有来源的期初处理，不能全表直接乘1000000 | CASE-05/11/14/15；旧企业完整选定事实不变，迁移与恢复分别留证，未知精度保持明确状态 |
| BATCH-DEV-08 正式查询 双端与性能，依赖已完成业务接线 | 规 `API/inventory_batch_handlers.rs`及`UI/pages/inventory/batches.rs`；现页面注册、查询宿主和DataGrid；规性能目标/报告 | 当前余额、原效果和追溯分开查询；输入防抖、取消、原命令恢复、授权下载；先登记性能执行入口 | UAT-03/04/05/06/10至14；下列8组性能全部有真实原样本，不能由debug回归秒数代填 |

### 数据库字段 索引和共同提交

现迁移0288已建立`inventory_batch_movements`，0289增加元数据身份和父流水守卫。下列最小字段已存在；更多原行/成本关联仍按相应业务补齐。数量字段统一bigint微单位；禁止浮点和饱和运算；未过账的计划不能作为真实效果返回。

| 实际字段或待补关联 | 作用和约束 |
| --- | --- |
| `batch_movement_id/tenant_id/warehouse_id/sku_id/batch_id` | 稳定效果ID与完整物理身份；实际归属核对，不能仅靠全局batch_id外键推断企业/SKU |
| `inventory_ledger_id` | 关联本事务实际INSERT RETURNING的SKU库存流水；数据库约束与应用验证共同限定同企业、仓库和SKU |
| `delta_on_hand_micros/delta_reserved_micros` | 真实本批变化；同一SKU效果的批次子效果分别累计，不能用原订单量代替实际扣减 |
| `before_on_hand_micros/after_on_hand_micros/before_reserved_micros/after_reserved_micros` | 由锁内实值生成；after=before+delta，非负且预占≤手量；不接受客户端提交余额 |
| `source_kind/source_id/source_line_id/original_effect_id` | 沿实际业务和可退来源保存；没有行身份的旧入口明确NULL，不捏造采购/工单行 |
| `decision_at/batch_metadata_snapshot` | 锁后实际决策时点和已读取元数据：批次标识、到期及资料版本等；当时未记录的字段保持NULL |
| `created_at`及唯一键 | 保存实际落库时间；同一SKU流水/批次合并为一个物理子效果，唯一`(tenant_id,inventory_ledger_id,batch_id)`，重放不新增 |

索引至少覆盖`(tenant_id,inventory_ledger_id,batch_id)`原效果回查、`(tenant_id,batch_id,created_at,batch_movement_id)`按批次游标，以及实际原来源查询键。查余额继续用现`inventory_batch_balances`完整主键；批次元数据使用真实企业/SKU谓词。用真实规模的EXPLAIN ANALYZE与返回行数证明查询边界，不能只列“已建索引”。不随迁移给旧发货记录填猜测的FEFO批次。

不得将不同原业务行净额合并后丢失来源。上述唯一键描述一项SKU流水下的批次物理效果；服务若归并了多个来源行，须另保存逐来源子明细及精确分摊，其合计等于该物理效果。同批多原行、正反动作同时出现时，也要能逐原行复算可退量，不能只剩净变化。

共同顺序为：规范化完整命令→原业务/来源锁→按完整物理键排序的库存锁→SKU配置/成本所需锁→按稳定批次ID锁余额→锁实际归属元数据→以锁后决策时点验证全部计划→SKU精确流水与逐批效果→来源/价值/状态→必需operation/audit/必要Outbox→原结果快照→一次commit。多仓、多SKU先收集有界键，不能写到第二行才追加相反锁序；配置和批次资料修改入口也必须审查相容锁序并验真实竞争。

批次手量可独立复算时，应与本SKU同仓手量一致；批次预占不能超SKU预占，但这个合计关系不能证明某工单拥有某一批预占。业务消费必须证明自己的来源。只有已有批次精度和来源均可信，才能按本源释放/消费；质量/冻结/过期依实际策略拦截。制造CONSUME使用原持料来源追溯，仓库变化为0，不为满足关联表而伪造第二次仓库出库。

精确列NULL属于尚未证明的历史状态。新建空批次余额可以明确从0建立；已有非零SKU但没有批次映射时不能凭本次入库补成“已核对”。未知/矛盾历史409并生成待核对结果，恢复已核对精度必须有完整证据。权限拒绝403，当前范围内对象不存在404，畸形/不支持维度沿400，数量运算或资源新错误按明确422契约，必需写入失败500且回滚；同步OpenAPI源、生成契约和客户端行为，保持现有兼容错误的约定。

### 十六个具体业务样本

以下样本统一接续现登记`crates/erp-api/tests/free_erp_acceptance.rs`下的`inventory_batch_writes.rs`，制造/履约已有子模块继续复用。标题中的CASE是本节BATCH-CASE，不能冒充前文ERP-CASE；每个独立样本固定企业/仓库/SKU及原ID，另一个非空企业用于越权对照。

| 样本 | 初始事实与动作 | 必须得到的结果和独立核对 |
| --- | --- | --- |
| BATCH-CASE-01 整单位入库保留小数 | 同仓SKU=4.9，A批=1.9、B批=3.0；旧兼容入库整数1到A | SKU=5.9、A=2.9、B=3.0；delta=1000000，不抹0.9；真实流水/批次效果/日志一次提交 |
| BATCH-CASE-02 跨企业批次 | 请求企业A真实SKU/仓库，传企业B的batch_id | 按当前范围拒绝；A/B余额、原来源、日志/成功事件及原回执均无新成功效果；响应不带B资料 |
| BATCH-CASE-03 同企业错SKU | SKU甲库存足够，传SKU乙批次；入库、出库及制造领退分别测 | 各入口均拒绝；不能只验证batch_id全局存在，也不能用甲总库存掩盖乙批次 |
| BATCH-CASE-04 批次超可用 | A批手1.9、预占0.9，可用1.0；小数新契约请求普通出1.1 | 整单拒绝；另合法出1.0后A=0.9/预占0.9；含冻结时按真实冻结准入，不假定冻结为0 |
| BATCH-CASE-05 未知精度 | 批次旧整数有值但微量为NULL，SKU精度可信 | 不按整数自行激活批次；明确409，SKU/批次/来源无部分更新；预检记录与核对方式可查 |
| BATCH-CASE-06 本源预占 | A工单和B工单在同SKU有不同来源，尝试A领取B拥有的预占批次 | A只能消费有证明的本源或真实自由量；不能因SKU总预占足够就占用B；各源原量/余量和批次独立复算 |
| BATCH-CASE-07 FEFO跨两批 | 手量2.0，A=0.6先到期、B=1.4后到期且均可用；原OMS整单位出1 | A扣0.6、B扣0.4，剩A=0/B=1.0；两个子效果合计1000000，关联同一个真实SKU出库流水 |
| BATCH-CASE-08 原命令20路重放 | 20路同时提交同ID/同批次/同负载 | 全部结果收齐；一次库存/逐批效果/成本/必需日志，20结果均指向原效果；换批次复用ID409 |
| BATCH-CASE-09 提交后断响应 | 成功提交后阻断响应，再修改当前批次名称或余额，重新登录用原ID回查 | 返回原SKU/批次余额、效果及决策快照；不返回新余额冒充原结果，不产生新库存/成本；仍验证当前权限 |
| BATCH-CASE-10 后序写失败 | 两SKU/两批次，注入末项批次效果、COGS、审计、Outbox或回执INSERT失败 | 每个真实故障分别跑；整单SKU/批次/来源/状态/成本均回滚，无成功回执或成功事件 |
| BATCH-CASE-11 总量矛盾 | SKU=3.0，可信批次合计2.9；尝试领料或发货 | 明确核对冲突，不能只修改被选批次使请求成功；旧事实原样保留，不生成猜测补差流水 |
| BATCH-CASE-12 制造领退原批 | 同原领料从A领0.25，再从该原领料退0.125；与B批及第二原领料对照 | A净减0.125，原持料0.125；错批/错原领料拒绝；真实SKU流水与A批效果逐笔一致 |
| BATCH-CASE-13 耗用不重复仓库出库 | 原已领A批0.25持料，确认耗用0.125 | 持料剩0.125、耗用0.125；仓库SKU和A批余额不再减，追溯指向原领料效果，不制造仓库流水 |
| BATCH-CASE-14 资料与历史追溯 | 原收货/交付/退货及资料修改；包括到期临界点、制造批次去向 | 成功原效果保存当时资料；过期普通出库拒绝；实际锁竞争后依据锁后时间/资料判定；原收退/交退量及原成本有双向链 |
| BATCH-CASE-15 容量边界 | 同SKU完整批次1000条、1001条分别测，另含外企业批次UUID攻击 | 1000按完整事实决策；1001明确超限且0效果；不截断取前1000后宣布成功；旧企业完整选定事实不变 |
| BATCH-CASE-16 旧原结果兼容 | 保存已有无UUID批次原请求/摘要/快照，再上线可选batch_id字段 | 老请求序列化/摘要与GET验证不漂移；新UUID批次有独立原效果；不存在真实映射的UUID/字符串混合输入明确拒绝 |

验收器使用独立只读连接：完整SKU余额与其流水累计、批次前后/delta、来源余量、真实原行、价值、状态、回执及两类日志/事件双向核对。没有新批次表时不能把空查询计PASS；演示/测试夹具身份写入报告。保留原失败和原事实，不覆盖为修后通过。全路径修复、迁移、正式双端和发布性能均通过后才可勾I04及相关完整功能。

故障注入样本须建立使目标INSERT真实执行的前置，例如确实需要库存通知的启用渠道或真实发货成功事件；保留实际故障点、错误与后续完整读回。没有命中必需写入的用例不能证明共同回滚。

### 八组性能负载和明确通过规则

全部沿第四节固定环境/数据及最近秩，使用登记入口的Release产物；每组做3轮，报告最差轮，不把三轮样本混成一个漂亮百分位。每动作/规模/方向成功组至少1000次，故意的400/403/404/409/422、合法重放和故障注入分组。20路竞争每组100轮；慢迁移/恢复另按实际完整运行记录。以下是目标，当前测量状态NOT_RUN。

| 性能任务 | 输入、调用与计时 | 通过阈值及必须同时满足的事实 |
| --- | --- | --- |
| BATCH-PERF-01 单键入出调，ERP-P04 | 各实际handler→原命令/兼容服务→共同提交；分别1/10/100/1000候选批次，含正常/已知零成本 | 单SKU普通动作P95≤1秒/P99≤2秒；1000候选组单列结果；每成功动作实际SKU/批次/来源一致，无静默跳过 |
| BATCH-PERF-02 多行领退/交付，ERP-P04/P12 | 20/100行各动作；1/10/100批次选择；记录原来源/物理/资料锁、SQL、效果和commit | 普通20行P95≤1秒/P99≤2秒；100行及100条材料P95≤2秒/P99≤4秒；末行失败全回滚；1000行资源边界另测，不以100行代替 |
| BATCH-PERF-03 同键和同源竞争，ERP-P05 | 同SKU不同命令、同原ID、不同SKU、领/退/取消及元数据更新各20路×100轮 | 明确成功/冲突P99≤3秒；0超量、0借用他源、0重复成本/效果；每轮独立核对，死锁/超时单列且原ID可恢复 |
| BATCH-PERF-04 原效果回查，ERP-P03 | 真正原命令GET及批次效果明细，20/100个效果分别测；先另改当前余额/资料 | P95≤500ms/P99≤1秒；原结果不漂移，当前授权拒绝另组；禁止通过读取最新库存减少校验成本 |
| BATCH-PERF-05 批次列表，ERP-P02 | 10万物料/100万SKU流水、完整授权筛选/排序；100/500行及查询计划分别记录 | 100行P95≤500ms/P99≤1秒；500行P95≤1秒/P99≤2秒；稳定游标/RowId，响应预算沿第四节；不客户端全量切片 |
| BATCH-PERF-06 双向追溯，ERP-P13 | 100万真实追溯边；首100项、典型完整≤1000边，正反向各至少1000请求 | 首100项≤500ms、完整P95≤2秒；无遗漏和跨企业链；超过同步预算返回可继续任务/游标，不静默截断 |
| BATCH-PERF-07 计划内存与超限，ERP-P18/P19 | 1000/1001批次、20/100行、多仓组合；另9企业普通查询/保存持续30分钟；24小时资源另跑 | 有界计划在分配前检查；1001拒绝0效果；其他企业保持P02/P04；应用连接/任务/缓存有界；峰值记录实际基线与增量，超过已固定预算FAIL，未固定预算NOT_RUN |
| BATCH-PERF-08 迁移恢复与出文件，ERP-P16/P20 | 10万逐批效果×20列导出；至少3次数据库/附件/原命令恢复；未知历史与已核对企业分组 | CSV≤30秒/XLSX≤60秒、新增峰值≤256MiB；数据库RPO≤15分钟/RTO≤2小时；恢复后余额/流水/原效果守恒、0重复入账，历史缺来源仍明确未知 |

规划原始报告为`qa/performance/free-erp-batch-inventory.json`；新增性能测试目标先登记`Invoke-ErpBuild.ps1`，当前只有已登记BOM性能入口可按其准入运行，不能直接执行新测试二进制或原始cargo。报告必须写真实动作/规模、请求数、开始/结束与排队、鉴权/池/锁/SQL/提交分段、百分位与错误、峰值、环境、源码及产物SHA；debug测试耗时不填入发布性能指标。

交给Grok时可先领取BATCH-DEV-01至03，完成原失败留证、内核/流水和独立读回，再领取04至06的实际入口，最后07/08的迁移、正式客户端及性能。每批同时写具体尚未通过项；当前文档状态、后续源码实现和最终免费上线资格分别记录。

### 本轮实际交付记录与后续交接

原失败保留在[原测试日志](../qa/acceptance/free-erp-batch-writes-original-test.log)：13实际失败，0忽略，耗时4.92秒。独立[原失败复核](../qa/acceptance/free-erp-batch-writes-original-parser-fixed-independent.json)确认错归属200、历史NULL被推算和缺逐批流水；4441个原企业37表完整选定事实不变。原运行期间6项其他开发输入变化，所以原源码绑定为PARTIAL；该报告的bug事实与实际HTTP/数据库读回分别保留。早期证据解析器漏了Rust摘要的`0 measured`字段，原FAIL报告保留，修正解析器后的报告另存。

首次修后[13项独立复核](../qa/acceptance/free-erp-batch-writes-fixed-focused-independent.json)为PASS：13通过、0失败、0忽略，6.32秒；4459个原企业37表完整事实不变，构建前后931项源码输入没有变化。20路同原ID全部200，只产生一笔实际SKU/批次效果。制造领0.25、原批退0.125、耗0.0625后A批1.775，耗用没有第二次仓库扣减。

扩展[23项实际回归复核](../qa/acceptance/free-erp-batch-writes-fixed-expanded-retry-independent.json)：23通过、0失败、0忽略，11.33秒；4477个原企业37表完整事实不变。1000完整候选通过，1001明确422；跨批FEFO两个子效果关联同一实际SKU流水；两个真实必需INSERT故障整笔回滚并用原ID恢复；PostgreSQL实际元数据锁等待后过期409；大于JavaScript安全整数的量以十进制字符串返回；3次子/父流水改写实际拒绝。该窗口只有另一个登记目标的`shared_contract.rs`测试源码变化，保留全输入PARTIAL和本目标实际结果。第一次扩展编译因两个测试辅助函数的错误转换失败，业务0执行，失败日志及修正后重跑分别保留。

首次完整[239项回归记录](../qa/acceptance/free-erp-batch-writes-fixed-full-independent.json)：232通过、7失败、0忽略，153.52秒；4504个原企业37表完整事实不变。7项分别是两个FEFO兼容错误码、一项旧UUID拒绝断言、两个NULL精度成功夹具、两个未覆盖工单锁的旧并发观测。已恢复FEFO的精确可用/请求量及原`inventory_insufficient`码；已知成功夹具明确声明精确量，未知历史拒绝用例保留；当前并发探针沿真实blocker PID递归观察阻塞链，并在断言前收齐20个结果。原失败记录保留，不以修后结果覆盖。

修后完整[239项独立复核](../qa/acceptance/free-erp-batch-writes-fixed-full-retry-independent.json)：239通过、0失败、0忽略，171.31秒；4778个原企业的37张选定表全部列保持不变，新增25个批次场景的实际HTTP与精确台账复核无错误。构建前后932项输入盘点中，制造API、另一个登记目标的`shared_contract.rs`测试及三份OpenAPI文件发生并行变化，整体源码绑定为PARTIAL；239通过只绑定该次实际构建，不能直接认证变化后的整仓或发布产物。15个受影响叶模块固定入口格式检查通过；第一次格式检查因三个路径未包含真实`inventory_print_report_handlers`目录被入口拒绝，0格式执行，日志保留。严格API范围Clippy仍FAIL：本批内核3项已修，仓储库234项与四个适配器27项阻断，API lint未到。上述debug测试秒数只说明测试执行，不能填入BATCH-PERF或ERP-P发布性能。

当前[OpenAPI源与生成契约核对](../qa/acceptance/free-erp-batch-writes-openapi.json)为静态PASS：三个库存动作的实际UUID、原回执/逐批效果、六个精确量字符串及404/409/422边界一致，明确1000候选预算。清除原入/出库接口遗留的“UUID不支持”说明后，按现有入口重新生成契约；所有源路径及组件与生成文件相同，核对期间输入未变。此项记录当前契约SHA，不能将此前239项运行绑定到新OpenAPI或后续制造变化。

迁移仅在`127.0.0.1:15438/erp_shared_contract_test`以`erp_grid_test`执行；预检/读回脚本同时锁定数据库、角色、服务器IP/端口。0288/0289增加表、索引、外键和守卫，不全表推算/回填历史量；未知历史保持未知。原数据、失败日志、前后完整事实和源码摘要均保留。

Grok接续边界：先复用现`InventoryPreciseEffect`和`inventory_batch_effects::apply/verify`，不要重新建立另一套批次余额写入；库位`batch_code`与UUID无真实映射时明确拒绝。下一批补原采购/销售/调拨收退、批次来源预占映射和原成本关联；之后完成历史核对、正式批次查询及双端页面。I04/I05、M09完整业务、全量工具链门禁和免费上线资格继续未勾选。

## 十二 生产收货放行逐动作规格

关联M10/M11、Q01/Q02、I02/I04/I05、F01、S03/S04。贸易公司使用同一库存事实，采购转售不要求建立工单。制造完工按有效原报工接收入库；待检时增加实存并冻结同量，质量放行仅解除原待检量，不增加净实存。整笔放行、部分放行及不良处置分别定义累计余量，不允许用检验状态字符串代替实物来源。

生产放行依据原待检实物、服务器必检判定及当前权限。当前质量审核回执入口只保存质量记录，实际库存解除冻结须由下列收货放行内核共同提交；不能把质量回执200当作库存已经可用。

### 十二A 具体功能和实现位置

| 任务 | 具体输入、动作和输出 | 实现位置与权威表 | 验收方向 |
| --- | --- | --- | --- |
| RECEIPT-01 原报工准入 | 引用同企业、工单、产品及有效末工序报工；按报工净合格量计算可入库余量；冻结路线版本、冲正及已收量进入复核 | 现 `API/manufacturing.rs::save_production_receipt_route`、`REPO/shared_contract.rs::save_production_receipt`、`mfg_reports/mfg_receipts`；规收货服务接产品/SKU及末工序来源 | 合格10分收6+4，第三笔拒绝；其他工序、另工单、冲正后不足、错产品拒绝；全字段独立读回 |
| RECEIPT-02 待检实物 | 接收量为六位小数；原仓库、库位和批次进入库存键；待检实存+q/冻结+q，直接合格由服务器检验策略准入 | 现精确库存服务、`mfg_qty_ledgers/shared_stock_ledgers/inventory_ledgers`；规接收货原命令快照、UUID批次余额及成本分录 | 收1.25只入一次；同命令无新增；库存、质量、批次、成本、日志末项失败全部回滚；未知历史不补猜 |
| RECEIPT-03 整笔放行 | 请求体仅 `request_id`；工单和收货ID来自路径；认证提交人来自服务器；复核原正数待检、报工、检验及三类来源流水 | 现 `REPO/shared_contract/production_receipt_release.rs::{release_production_receipt_with_actor,source_proof}`；原内部入口委托同一服务 | 错工单、缺检验ID、待检却已有可用量、错仓库/SKU、错来源量和重复来源全部拒绝且完整事实不变 |
| RECEIPT-04 必检与判定 | 服务器冻结要求覆盖工单历史全批规则及当前批；读数存在、责任人、上下限和判定同时有效；原标准不由客户端覆盖 | 现同叶模块 `inspection_proof/lock_quality_source`；`shared_inspection_freezes/shared_inspection_freeze_lines/mfg_quality_items`；4个现检验写入口锁同工单。当前冻结创建仍要求非空批次，全批规则样本为独立旧配置 | 必检未读、伪造passed、超标准、错冻结项目/来源拒绝；无必检的服务器策略作为原快照记录；检验写入与放行竞争须有实际锁证据 |
| RECEIPT-05 权限和必需记录 | 重新核对当前账号ACTIVE及原仓库授权，用户锁保持到提交；HTTP记录真实认证用户，内部调用记录SYSTEM | 现 `API/auth_context.rs::production_release_actor`、同叶模块 `ProductionReleaseActor::{require_warehouse,record}`；`users/user_warehouse_access/operation_logs/audit_logs` | 非空仓库范围越权403；权限撤销竞争、原命令重试越权同样拒绝；实际日志INSERT故障无库存效果；空范围沿既有“全部仓库”策略 |
| RECEIPT-06 原效果和重试 | 同请求与同载荷返回原工单可用总量；两个质量桶效果引用真实RETURNING流水ID；质量快照和来源快照在提交前保存 | 现迁移 `0290_production_receipt_release_sources.sql`、`mfg_receipt_release_commands/mfg_receipt_release_effects/command_receipts` | 原1.25放行后另收0.5，重试仍返回原1.25；同ID改工单/来源冲突；20路同命令一笔；父子流水、原回执与快照不可改写 |
| RECEIPT-07 部分放行及正式页面 | 规请求 `qty`、检验结果ID及版本；累计已放行≤原待检/有效合格余量；剩余待检保留；查询与草稿按页面实例隔离 | 规 `API/manufacturing/receipts.rs::release_quantity`、现放行叶模块的同事务内核、`UI/pages/manufacturing/receipts.rs` 和自研DataGrid | 待检10放行6剩4，重复6无变化，再放5拒绝；桌面/Web分别真实登录、保存、断网后查原命令、重登读回 |
| RECEIPT-08 原成本与批次 | 规连接实际产品SKU、UUID批次、序列/库位、原有效成本、暂估状态及半成品再投料来源；使用原库存与价值内核 | 规收货服务接 `inventory_batch_effects`、原成本分录和原收货来源；两端页面显示权威精确值与未知状态 | 源批次与SKU余额相等；原成本/尾差独立复算；未放行不能销售；再投入不借用他仓/他批/他笔收货 |

0290只为新放行建立不可变快照、实际效果外键、守恒约束及索引，没有回填历史原结果。当前整笔服务仍不能证明批次启用SKU的UUID映射，遇到此类旧收货明确要求对账；接续任务必须补真实映射。原收货创建的幂等结果快照、末工序/产品校验、部分放行、原价值、必要库存通知/Outbox、快照字节及峰值预算、正式页面及公网托管均待验收，M11/Q02整项不勾选。库存通知须使用真实启用渠道命中生产者及实际INSERT故障，不以空Outbox证明成功或回滚。

### 十二B 可直接执行的样本和失败证据

真实HTTP位置为现 `crates/erp-api/tests/free_erp_acceptance/production_receipt_release.rs`，已登记在固定入口 `free_erp_acceptance`。每个样本新建自己的QA企业、账号、仓库、SKU、工单和报工；待检通过实际收货POST创建，不能只塞一行孤立收货冒充真实来源。另一psql连接的完整事实由现 `qa/acceptance/free-erp-production-release-{capture,verify}.py` 捕获与复核，原结果文件不覆盖。

原代码实际执行9项，0通过/9失败/0忽略，3.75秒；[原反例独立复核](../qa/acceptance/free-erp-production-release-original-independent.json)确认5075个既有QA企业44张选定表全部列不变。7个拒绝样本均误放行：未完成必检、超界伪造通过、错工单、无权仓库、来源量不符、缺原检验、待检可用量异常；另2项证明重试返回后来总量及放行缺操作/审计记录。报告中的证据完整PASS表示反例复核完成，原功能测试仍为9失败。

修后[9项专项独立复核](../qa/acceptance/free-erp-production-release-fixed-focused-independent.json)为PASS：9通过、0失败、0忽略，3.66秒；5088个既有企业45张选定表全部列不变。扩展23项首次22通过/1失败，24.96秒，失败发生在准备夹具时：现有精度守卫正确拒绝改写已证明历史流水，没有执行该例的HTTP。原失败日志和报告保留，改用独立未对账旧余额样本验证409，不放宽守卫。

修后完整[263项独立复核](../qa/acceptance/free-erp-production-release-fixed-full-independent.json)为PASS：263通过、0失败、0忽略、0筛除，170.16秒；5125个既有QA企业45张选定表全部列不变。24个本批场景覆盖历史全批必检、冻结标准、报工冲正、1000/1001检验预算、未知精度、内部旧入口、原快照、当前撤权及检验变化竞争；两组20路均有真实PG阻塞链并收齐结果；两个必需日志INSERT实际故障整笔回滚，原ID重试成功；六次快照/源流水改写实际拒绝。两份完整事实文件、原始HTTP及事务来源分别保留，debug用例秒数不作为发布性能。

前后源码清单包含所有Rust、迁移、依赖锁及OpenAPI输入，其他并行开发发生变化时记录PARTIAL；不能把旧二进制通过直接认证当前新源码。合成QA JWT不代替真实密码、席位、会话和双端验收。格式、严格Clippy、当前契约及直接旧目标分别留证，未通过门禁不由263项回归替代。

原登记HTTP目标另见[直接入口独立复核](../qa/acceptance/free-erp-production-release-direct-independent.json)：1通过、0失败、0忽略，0.46秒；5426个既有QA企业45表完整事实不变，34项独立断言验证原6件收货、仅一笔放行、净实存仍6/冻结0、真实效果ID和认证用户日志。4个受影响叶的固定入口格式最终PASS；第一轮另一质量处理函数出现格式差异，修后复核，原FAIL日志保留。严格Clippy仍为FAIL：仓储234项及适配器27项阻断API lint；[当前OpenAPI源/生成核对](../qa/acceptance/free-erp-production-release-openapi-current-first.json)单独静态PASS，不能代替运行及双端验收。

### 十二C 专属性能任务

下列为候选门槛，全部仍为NOT_RUN，复用第四节环境和统计方法。普通动作每组至少1000次；权限、冲突和预算拒绝独立统计。锁等待、鉴权、连接池、原来源预检、必检判断、精确库存、投影流水、Outbox、两类日志及commit全部计时；成功必须满足来源、守恒及原结果检查才能纳入成功组。debug用例秒数不能填入本表。

| 性能组 | 真实负载和位置 | 目标与同时验收 |
| --- | --- | --- |
| RECEIPT-PERF-01 接收入库 | 实际收货POST，20/100行规划批量分别测；10万物料/100万流水背景 | 沿P04/P12：20行P95≤1秒/P99≤2秒，100行P95≤2秒/P99≤4秒；原报工余量和整单原子 |
| RECEIPT-PERF-02 整笔放行 | 同叶模块，0/1/100/1000条有效必检及冻结规则分别测，正常用户20并发 | P95≤1秒/P99≤2秒；单请求新增峰值候选≤16MiB，持久化原快照≤4MiB；1001预算拒绝另组，不能截断必检 |
| RECEIPT-PERF-03 同工单竞争 | 同收货不同ID、同ID、同工单不同收货分别20路×100轮；实际PG阻塞链证明竞争 | 沿P05：结果完成P99≤3秒；全部2000响应保留，数量不超、同来源只放一次，无死锁残留 |
| RECEIPT-PERF-04 跨工单隔离 | 20个工单/20个不同库存键和同库存键分别测；记录单键等待和用户共享锁 | 沿P04/P05；独立工单不因全企业锁串行；连接池上限、队列、吞吐、锁占用分别留证 |
| RECEIPT-PERF-05 原结果重试 | 原放行后另收货/再投料，再查原命令；1/100/1000检验快照分别测 | P95≤500ms/P99≤1秒；不重新计算当前可用总量，不重写原效果；响应精确字符串与原快照一致 |
| RECEIPT-PERF-06 来源/权限拒绝 | 1/10/100/10000仓库授权；错来源、未知历史及撤销后新请求分别测 | 候选拒绝P95≤500ms/P99≤1秒；无业务效果，10001授权超预算单列；错误率不与成功组混合 |
| RECEIPT-PERF-07 正式双端 | 规划正式页面100/500行收货、原检验和关联子表，真实桌面/Web、固定网络 | 详情沿P03、查询沿P02、输入沿P09；帧/输入/网络/提交各统计，读取原命令后显示真实状态 |

Release压测必须先登记该专属目标并使用固定构建入口；当前只登记BOM性能目标，不能借其名称运行收货压测或创建新target。最终报告保留原始每请求时延、环境/产物SHA、SQL计划、扫描/返回行数、峰值内存及独立业务结果。

## 十三 质量来源与冻结标准逐动作规格

本节细化Q01/Q02和RECEIPT-04，制造和贸易共用检验模板与冻结契约。前期支持真实工单、生产收货和现有销售单；采购来料检验须接实际采购收货，不能传一个任意UUID或借用工单身份。下列“现”表示当前代码位置，功能是否验收通过须看实际报告；“规”表示后续开发位置。

### 十三A 功能、入口和实现位置

| 任务 | 具体行为与最小字段 | 实现位置 | 验收方向 |
| --- | --- | --- | --- |
| QSOURCE-01 来源规范化 | `source_kind/source_id`；支持WORK_ORDER、PRODUCTION_RECEIPT、SALES_ORDER，RECEIPT兼容归一到生产收货；去首尾空白、ASCII大小写归一，未知类别和零UUID拒绝 | 现 `REPO/shared_contract/inspection_sources.rs::{SourceKind,parse,as_str,lock_source}`；parse/as_str属于SourceKind；现 `API/manufacturing.rs::freeze_inspection_template_route` | 三种真实来源分别成功；别名与小写不绕过必检；任意来源拒绝且无冻结、回执或库存效果 |
| QSOURCE-02 真实来源归属 | 工单读实际`product_code`；生产收货读实际工单、批次和原量；销售单读当前真实版本中正数量的货品行；全部使用tenant谓词 | 现同叶模块 `LockedSource::require_item`、`lock_source`；`mfg_work_orders/mfg_receipts/shared_sales_revisions/shared_sales_lines` | 缺来源、另一企业来源、错产品、错收货批次、销售单不存在货品均拒绝；生产产品代码到SKU的正式映射仍需另做 |
| QSOURCE-03 版本与完整复制 | 引用同企业真实模板及版本；复制所有检验项目的`item_code/required/min_micros/max_micros/sample_count`；不能只复制前1000项 | 现 `REPO/shared_contract.rs::freeze_inspection_template`、叶模块 `insert_frozen_lines_and_binding`；`shared_inspection_templates/shared_inspection_template_lines/shared_inspection_freezes/shared_inspection_freeze_lines` | 1/100/1000项目逐项相等；1001明确拒绝且整笔回滚；后来新模板版本不改变原冻结上下限 |
| QSOURCE-04 保存原来源证明 | 保存来源类型及ID、工单/收货/销售版本真实外键、模板版本、产品、批次、完整行副本、实际来源快照、原request_id/payload_hash | 现迁移 `migrations/0291_inspection_source_bindings.sql`、`0292_inspection_command_identity.sql`、`0293_inspection_bounded_command_keys.sql`；`shared_inspection_source_bindings`及`verify_inspection_original_command` | 冻结、来源证明、原成功回执一次提交；同ID重放不新增；不同ID同来源同批次冲突；跨企业和不存在的外键不能提交；原负载完整核对，不把长文本放进全局Btree |
| QSOURCE-05 保护冻结标准 | 新证明、关联冻结头/行/读数及原命令禁止改写/删除；冻结行封存后禁止新增；提交时核对完整副本 | 现同迁移的 `guard_inspection_bound_copy/guard_inspection_bound_command/lock_inspection_copy_parent/verify_inspection_bound_copy` | 用实际SQL分别改头、行、读数、证明和原回执，或补入一条新项目，均拒绝；故障回滚完整，不能关闭守卫使测试通过 |
| QSOURCE-06 生产放行统一判定 | 当前工单的全批/当前批规则，加当前生产收货的全批/当前批规则；必检须有匹配来源、项目、批次、实际读数与责任人且在原界限内 | 现 `REPO/shared_contract/production_receipt_release.rs::inspection_proof`；实际库存入口 `SharedContractRepo::release_production_receipt_with_actor` | 收货来源必检不得漏读；空冻结和不完整旧副本要求核对；另一收货、另一批次不能替代当前检验；无必检策略必须保存为真实原快照 |
| QSOURCE-07 质量审核保存证明 | 审核入口只记录质量结论；按同一`inspection_proof`检查工单与原收货批次，保存原审核快照，并保持工单先于收货的锁序 | 现 `REPO/shared_contract.rs::release_after_quality`；现 `API/manufacturing.rs::release_after_quality_route`；`mfg_quality_releases/mfg_quality_release_proofs` | 必检未读时无审核结论；通过后只新增一份结论和证明，库存仍HOLD；原ID重试返回原结论；之后实际放行才解除原冻结量 |
| QSOURCE-08 贸易销售来源与竞争 | 冻结绑定真实销售版本和货品行；发货与冻结/修订使用同一`sales-revision:{so_id}`来源锁，保护“当前没有必检”的检查 | 现 `REPO/shared_contract.rs::{save_sales_revision,advance_sales_delivery}`和来源叶模块；`shared_sales_revisions/shared_sales_lines/shared_sales_deliveries` | 合法销售货品可冻结；错版本/错货品/外企业拒绝；实际并发冻结与发货不能同时绕过检验。销售整项仍须验证原出库、成本及双端页面 |
| QSOURCE-09 读数与正式检验页 | 展示原版本、项目、单位、样本要求、读数、检验人、审核状态及实际结果；支持未读、合格、不合格、复检和附件；异常读数须可保存为失败事实 | 现 `API/manufacturing.rs::save_frozen_inspection_reading_route`、现 `API/manufacturing/inspection_outcomes.rs`及现 `REPO/shared_contract/inspection_outcomes.rs`；规 `UI/pages/quality/inspections.rs`及样本/附件表 | 当前HTTP已保存FAILED/MISSING及实际提交账号，追加复检、账号审核和只读历史已接线，具体字段、样本和证据见第十四章；旧内部方法仍沿旧准入。正式样本齐套、单位/公差、正常账号会话、附件和桌面/Web继续待验，不能据此勾选整个Q01 |

现有接口保留：`POST /v1/tenants/{tenant_id}/quality/inspection-templates`、`quality/inspection-freezes`、`quality/inspection-freezes/{freeze_id}/readings`、`quality/receipts/{receipt_id}/releases`。质量审核与制造收货实际放行分别验收，实际库存入口是`POST /v1/tenants/{tenant_id}/manufacturing/work-orders/{work_order_id}/receipts/{receipt_id}/release`。当前冻结来源输入错误沿422，制造实际放行发现未检/历史矛盾沿409，质量审核当前拒绝沿422；三种检验写入的数据库失败由现`API/manufacturing.rs::inspection_write_error`返回500，不把数据库故障误报为用户参数错误，也不向客户端泄露SQL内部信息。变更错误码时须同步真实handler、OpenAPI源/生成契约和两端客户端。

### 十三B 表、字段与数据边界

`shared_inspection_source_bindings`主键`(tenant_id,freeze_id)`，唯一`(tenant_id,source_kind,source_id,batch_code)`。保存`request_id/payload_hash`，由freeze_id生成命令效果身份；延迟外键只引用真实`command_receipts`既有主键`(tenant_id,request_id)`，提交时另由`verify_inspection_original_command`逐字核对完整payload_hash、效果ID及SUCCEEDED状态。现有payload_hash列在部分入口保存的是完整规范化负载，不能假设只有固定长度摘要，更不能截断、改成新协议或为该长文本创建全局Btree。父来源分别使用真实工单键、`(tenant_id,receipt_id,work_order_id)`收货键或`(tenant_id,sales_revision_id,source_id)`销售版本键。原模板以`(tenant_id,template_id,version_no)`引用；类型不同的来源槽位按检查约束互斥，不能仅保留一个没有归属的字符串。

完整标准为`line_count/frozen_lines`，原事实为`source_snapshot`；两者不是由客户端提交的“已通过”声明。冻结行由一次客户端批量INSERT写入，数据库仍执行逐行父锁与约束检查；不能据“批量SQL”推断1000项耗时已经达标。提交时完整复制校验只对封存记录聚合一次，不逐行重复聚合整个1000项JSON数组。

`mfg_quality_release_proofs`主键`(tenant_id,release_id)`，同企业收货和原request_id分别唯一，引用真实审核父行、收货/工单及原命令。`snapshot`保存tenant、原审核/收货/工单ID、实际batch_code、inspector及quality判定；原证明与关联新审核行不可改写。该表不代表库存放行效果，也不补填旧审核的猜测证明。

销售读数的work_order_id允许NULL，实际来源由冻结的销售单/版本证明绑定；生产两种来源仍须真实工单。0292的`inspection_reading_owner_shape`对新写入约束来源形态，并以NOT VALID保留旧行核对工作，不能宣称全库历史数据已验证。0293为已应用第一版QA草稿的前向修复，只替换两个新证明表的原命令外键和相关索引；未发布的0291/0292文件也同步为安全结构。第一版实际应用DDL的字节副本、SHA和失败日志保留，不覆盖其历史身份。

来源类型最多32个UTF-8字节、产品/检验项目最多1024字节、批次最多512字节、检验人和reviewer输入最多512字节；冻结复制最多1000项目，放行读取的冻结头、项目和读数各最多1000，查询取1001用于显式判超限。上述是当前结构准入；持久化快照字节上限、进程峰值和发布时延仍需实际测量及相应门禁。

旧冻结不自动加新来源证明。当前放行重新检查真实来源、产品和批次，并要求旧完整行副本与其真实模板版本一致；空副本、漏项目、父版本不存在或无法核对，均保留旧事实并拒绝放行。不能把旧空数据解释为“无必检”，也不能因为新表存在就宣布所有历史资料已可信。

索引覆盖来源/批次查询、冻结项目键以及工单、生产收货、销售版本、模板版本、原命令的外键回查。还须对10万工单/销售单、100万冻结项目及读数背景做EXPLAIN ANALYZE，保留扫描行、返回行和实际索引；索引DDL存在不等于生产查询已通过。

### 十三C 逐条验收样本和当前证据

现专项目录 `crates/erp-api/tests/free_erp_acceptance/production_quality_sources.rs`接入已登记的`free_erp_acceptance`；准备待检实物继续复用实际生产收货HTTP，不用孤立假收货。独立完整事实由 `qa/acceptance/free-erp-production-quality-source-{capture,verify}.py`读回，迁移另由`free-erp-production-quality-source-schema-verify.py`核对。

| 样本 | 必须验证的实际结果 | 当前证据与剩余部分 |
| --- | --- | --- |
| QSOURCE-CASE-01 任意来源类型 | UNKNOWN来源422；原请求无冻结、证明和成功回执 | 专项通过 |
| QSOURCE-CASE-02 错产品 | WORK_ORDER存在但item_code不等于原product_code，422且0效果 | 专项通过，另覆盖零UUID及外企业工单 |
| QSOURCE-CASE-03 缺生产收货 | PRODUCTION_RECEIPT引用不存在ID，422且0效果；外企业同样验证 | 专项通过，缺来源和真实外企业收货分别测 |
| QSOURCE-CASE-04 收货别名必检 | RECEIPT归一为PRODUCTION_RECEIPT；其必检未读时实际库存放行409 | 专项通过 |
| QSOURCE-CASE-05 小写工单来源 | work_order成功归一为WORK_ORDER；未读必检仍拦截放行 | 专项通过 |
| QSOURCE-CASE-06 实际收货读数 | 原范围1至2、实际读数1.5；放行成功，原快照必检=1且包含原冻结、读数1500000和原来源证明 | 专项通过，另连接核对实际原副本 |
| QSOURCE-CASE-07 空旧冻结 | 新QA企业独立准备有真实父模板但未复制项目的旧配置；放行409、旧事实不变 | 专项通过 |
| QSOURCE-CASE-08 改冻结头 | 实际SQL修改新冻结产品或批次拒绝，完整事实不变 | 专项通过 |
| QSOURCE-CASE-09 改冻结行 | 实际SQL扩大新冻结上限拒绝，完整事实不变 | 专项通过 |
| QSOURCE-CASE-10 错批及其他收货 | 创建收货来源时错实际batch_code拒绝；同工单另一收货的未读规则不替代本收货规则，也不错误阻断无关联合法收货 | 错实际批次及另一批审核隔离通过；同工单另一收货的完整独立样本待补 |
| QSOURCE-CASE-11 销售真实版本 | 真实销售版本内正数量货品可冻结；缺销售单、错货品、外企业版本拒绝且不泄露资料 | 实际销售单冻结/读数、错货品、外企业版本通过；缺销售单及冻结与SHIP竞争待补 |
| QSOURCE-CASE-12 历史来源核对 | 合法旧别名、全批和完整副本参与必检；错产品、模板版本或漏项目拒绝；不改旧行使它看起来通过 | 旧收货别名、完整副本和错产品通过；父版本失配、部分漏项反例待补 |
| QSOURCE-CASE-13 原命令和证明保护 | 实际改/删原回执、读数或来源证明，以及封存后增加项目拒绝；延迟外键拒绝缺原成功回执 | 六种实际SQL改写/删除/补项拒绝通过；缺原命令、完整负载/效果不符的提交反例待补 |
| QSOURCE-CASE-14 后序真实失败 | 注入实际证明INSERT失败，冻结头/行/原回执全部回滚；移除本用例故障后原ID重试成功，再重放不新增 | 来源绑定和审核证明两个真实INSERT故障、500、完整回滚及原ID重试均通过 |
| QSOURCE-CASE-15 审核与库存 | 收货必检未读时质量审核拒绝；真实读数通过后审核只保存原结论及证明，待检实存/冻结均不变；原ID重试不新增 | 专项通过，责任人来自真实请求身份；真实密码与角色授权仍待验 |
| QSOURCE-CASE-16 真实竞争与容量 | 工单、收货、冻结、读数、审核及放行按相容锁序；真实PG阻塞链和所有响应留证；1000项完整复制、1001拒绝，不把被过滤或忽略用例算通过 | 真实工单锁/销售版本锁阻塞链与1000/1001通过；跨审核、读数、放行的20路混合竞争待补 |

原代码实际执行9项，0通过/9失败/0忽略，4.62秒；[原反例独立复核](../qa/acceptance/free-erp-production-quality-source-original-independent.json)验证5,438个既有QA企业51张选定表全部列不变。证据PASS表示反例已复核，原功能仍是9失败。原日志、完整事实、5份原源码副本及前后输入清单保留。

[本次迁移独立复核](../qa/acceptance/free-erp-production-quality-source-migration-independent.json)确认QA-only应用第一版0291后5,457个既有企业52表完整事实不变，新来源证明和审核证明的历史回填均为0。一次数据库暂时不可达的捕获失败也保留。第一版实际DDL身份见[原迁移字节与摘要](../qa/acceptance/free-erp-production-quality-source-first-applied-ddl.json)；当前0291—0293另有[回滚式子链复核](../qa/acceptance/free-erp-production-quality-source-chain-independent.json)：实际32,000字节负载及32,000字节效果文本原样保留、没有全局长文本Btree、两个原命令外键使用原主键、延迟守卫存在、结束后专属临时schema不存在。它只覆盖九张克隆父表的子链，不代表全ERP新库/升级链或生产迁移已验收。

修后9项专项通过；首次独立报告只因SQLx与psql对同一created_at使用UTC/+08时区表示而报错，原FAIL保留，[按同一精确时刻复核](../qa/acceptance/free-erp-production-quality-source-fixed-focused-timezone-independent.json)为PASS，其他字段仍逐字核对。扩展首轮25通过/3失败，真实失败暴露第一版回执长文本索引超限以及销售读数错误要求工单非空；修后[28项专项独立复核](../qa/acceptance/free-erp-production-quality-source-fixed-final-independent.json)为PASS：28通过、0失败、0忽略，17.26秒；5,505个既有企业52表所有列不变，31个新企业场景、6,628项独立断言。原负载不缩短、不补假工单、不关闭数据库守卫。CASE-10至16仅按表中已执行部分记录，不能整组勾选通过；debug耗时不计发布性能。

完整登记目标首轮在执行测试前因采购确认函数使用await但没有async而编译失败，实际测试数为NOT_RUN，原失败日志保留。补齐现`API/procurement_handlers.rs::confirm_trade_purchase_http`的异步声明后，[完整291项独立复核](../qa/acceptance/free-erp-production-quality-source-full-retry-legacy-command-independent.json)为PASS：291通过、0失败、0忽略、0筛除，195.10秒；5,539个既有企业52表所有列不变，55个本批及直接相关企业场景、6,707项独立断言。第一份全量独立报告将“刻意预置旧成功回执、但缺原结果”的反例误判为新增回执；其FAIL保留，修正后单独核对原请求、完整负载、效果身份、HOLD状态及不存在新放行/审核证明，不放宽一般拒绝请求的零效果要求。

六个受影响叶模块的固定入口格式有PASS记录；并行开发之后的新格式差异不能沿用该结论。严格Clippy仍FAIL：仓储236项和四个渠道适配器合计27项阻断API lint；本批新增来源/放行叶无该次诊断，但API lint尚未跑到，不能据此报整项通过。完整回归前后940个输入中制造API、共享测试和3个OpenAPI文件发生变化，整体当前源码认证仍PARTIAL；编译窗口的测试通过不能直接认证后来新增入口。真实密码/角色、正式桌面/Web、托管、全迁移链及专属Release性能仍待验。

旧登记目标另有[四入口独立复核](../qa/acceptance/free-erp-production-quality-source-direct-current-independent.json)：模板、冻结、读数和质量审核各1通过/0失败/0忽略，分别0.42/0.78/0.49/0.45秒；8个实际HTTP状态保留，5,872个既有企业52表所有列不变、5,931项独立断言通过。模板的两个不同新请求合法创建版本1/2及各自原回执；冻结/读数/审核则各只接受一个效果。审核后权威inventory_balances精度列仍实存/冻结各1,000,000、原收货HOLD/可用0。最初独立脚本误用模板同版本假设及mfg_qty_balances表，FAIL和原脚本保留，按实际既有契约修正后复核；没有为通过检查改变业务行为。这四例的双请求不替代20路真实阻塞竞争验收。

### 十三D 专属性能组

复用第四节环境、三轮最差结果和最近秩统计；每个动作/规模成功组至少1000次，故意拒绝、合法重放和故障分别统计。以下均为候选目标，当前全部NOT_RUN，必须登记专属Release性能目标后再测，不借用BOM目标、不新建构建缓存。

| 性能组 | 实际负载和计时边界 | 目标与结果核对 |
| --- | --- | --- |
| QSOURCE-PERF-01 冻结整单 | 三种真实来源×1/100/1000项目，计鉴权、来源锁、模板查询、复制、约束、原证明和commit | 1/100项P95≤1秒/P99≤2秒；1000项候选P95≤2秒/P99≤4秒；完整原副本和回执一致 |
| QSOURCE-PERF-02 来源拒绝 | 未知种类、缺/跨企业来源、错产品/批次及1001项目分别测 | P95≤500ms/P99≤1秒；1001不得先写部分成功效果；错误不混入成功百分位 |
| QSOURCE-PERF-03 原读数 | 1/100/1000项目冻结的单项目读数，含0、小数、未读和越界失败事实；现HTTP接新实测内核，复检/审核/历史负载另见QREAD-PERF | 合法单写P95≤1秒/P99≤2秒；保存原范围、声明责任人、实际账号和失败事实；当前只有功能样本实测，Release延迟/吞吐/峰值保持NOT_RUN |
| QSOURCE-PERF-04 审核和实际放行 | 分别调用两个真实入口，0/1/100/1000项目，10万来源/100万项目背景 | 沿RECEIPT-PERF-02；候选新增峰值≤16MiB、原快照≤4MiB，分别测量；仅正确来源且完整必检通过的结果纳入成功组 |
| QSOURCE-PERF-05 20路竞争 | 冻结同源同批、同项目读数、审核同收货各20路×100轮；另测同时放行及跨工单 | 全结果完成P99≤3秒；真实阻塞链、无死锁、一次原效果和原回执；独立来源不得使用全企业锁串行 |
| QSOURCE-PERF-06 原证明查询 | 规划授权查询原冻结、来源及审核证明，20/500行，1/100/1000项目详情 | 沿P02/P03/P18，分页作用于完整授权范围；返回有界详情，不全量下载后切片；原命令不返回当前变化冒充原结果 |
| QSOURCE-PERF-07 正式两端检验 | 规划正式检验页、自研DataGrid编辑和关联明细，真实桌面/Web分别登录保存 | 沿P02/P03/P09；输入、IME、帧和网络分别计时，查询/保存失败与重登读回验收；本地夹具不计双端通过 |

报告必须同时附产物及源码SHA、运行环境、每请求时延、SQL计划、连接池/等待/吞吐、原快照字节、峰值内存和独立业务读回。debug测试耗时和“1000条上限已编码”都不能填充发布性能成绩。

## 十四 实测读数、复检和审核逐动作规格

归属Q01/Q02，接第十三节的三种真实来源。保存请求成功只说明事实已提交；响应中的`reading.verdict`分别为PASSED、FAILED、MISSING，不能用HTTP 200或SUCCEEDED替代检验结论。正式桌面/Web、完整多样本、单位和附件、正常账号与公网托管仍单独交付，本章保持PARTIAL。

### 十四A 功能、文件、函数和边界

| 任务 | 具体行为和字段 | 实现位置 | 验收方向 |
| --- | --- | --- | --- |
| QREAD-01 保存实测事实 | `item_code/reading_qty/inspector/reviewer`；十进制最多6位小数，可为负数；超出原范围仍保存FAILED；空值保存MISSING，必检不放行；0是已测值 | 现 `DOMAIN/manufacturing/inspection_outcomes.rs::{record_measurement,InspectionOutcome,InspectionVerdict}`；现 `API/manufacturing.rs::save_frozen_inspection_reading_route`；现 `REPO/shared_contract/inspection_outcomes.rs::save_inspection_outcome` | -0.25原样存-250000；未读与0分别核对；原1至2范围内1.5通过，3失败；过多小数400且没有效果 |
| QREAD-02 区分姓名声明与账号责任 | `inspector/declared_reviewer`为输入声明；`actor_source/actor_user_id`从认证上下文及实际ACTIVE用户取得，不从JSON取得 | 现仓储叶的 `InspectionActor::require_active`；现API叶 `API/manufacturing/inspection_outcomes.rs::{actor,require_quality}`；现 `shared_inspection_reading_events` | 同企业实际账号、停用账号、跨企业身份及伪造actor字段分别测；声明reviewer不能冒称另一真实账号已审核。真实密码、角色变更、会话和多端仍待验 |
| QREAD-03 追加复检 | `previous_item_id/reason`必须引用当前最新项目读数；同冻结/同项目追加，保留原范围和旧值；失败、未读或被实际REJECT的读数可复检 | 现仓储叶 `save_inspection_outcome`；现API叶 `reinspect_route`；原 `mfg_quality_items`加新不可变事件 | 原失败不改写；过期/其他项目/其他企业引用拒绝；合格但未被拒绝不能随意换值；当前单项目含首次最多20次，超限明确拒绝且不删历史 |
| QREAD-04 真实账号审核 | `decision=APPROVE/REJECT`，REJECT必填`reason`；审核人从认证账号取得；只审核最新读数，APPROVE必须有已测合格值 | 现仓储叶 `review_inspection_outcome`；现API叶 `review_route`；现 `shared_inspection_reading_reviews` | 不合格或未读不能被APPROVE改成合格；每次读数只保留一份不可改写审核；被拒绝后追加复检，再审核新读数。免费单账号允许同账号检测和审核，双人复核属于后续政策 |
| QREAD-05 接实际库存放行 | 第2次起的合格复检必须被实际APPROVE；原失败仍进入原证明，当前有效读数只取最新一条；不以旧合格值覆盖后来的拒绝 | 现仓储叶 `effective_readings`；现 `REPO/shared_contract/production_receipt_release.rs::inspection_proof`；原实际制造release入口 | 失败→复检合格仍409→审核成功仍HOLD→实际放行只解除原1.25冻结；原证明同时保留两次实测、一份审核及当前有效值 |
| QREAD-06 接贸易与领料闸门 | 同源/批次检查完整原标准、真实数值、责任声明、最新事件及实际审核；旧全批未读保留阻断 | 现 `REPO/shared_contract.rs::inspection_blocks`；现 `advance_sales_delivery/issue_work_order_material` | 旧合格不能掩盖被拒绝或未审核复检；空/部分副本、销售实际SHIP与来源锁竞争补验见第十五章，完整贸易成本、退货、正式两端仍按T04独立验收 |
| QREAD-07 查询授权原历史 | 单项目GET返回按attempt_no排列的最多20条原读数、原request_id、created_at及实际审核；注明旧无来源事件的条数和证据缺口 | 现仓储叶 `read_inspection_history`；现API叶 `history_route/InspectionReadingHistoryQuery` | REPEATABLE READ READ ONLY；工单被真实写锁占用时查询仍可完成；只查企业/冻结/项目授权范围，停用和无quality.read拒绝；查询不新增日志、回执或库存效果 |
| QREAD-08 原重试和后续状态 | 首次读数v1规范负载不变；复检和项目审核使用独立v2负载；合法重放返回原快照，不返回后来审核或当前库存冒充原结果 | 现仓储叶 `lock_request/save_inspection_outcome/review_inspection_outcome`；`command_receipts`和事件snapshot | 数字1.5与1.500000视为同值；同ID改值409；销售新版本不改变原重试/历史；实际放行后拒绝新检验修改，合法旧ID仍可重放 |
| QREAD-09 数据库完整性 | 真实读数、冻结项目/来源证明、原成功回执和实际账号均有外键；前后读数同项目递增，原值/事件/审核/回执不可改写 | 现 `migrations/0294_inspection_reading_outcomes.sql`；`verify_inspection_outcome/verify_inspection_reading_review`及延迟原负载校验 | 实际commit分别验证缺回执、负载不符、效果不符；删字段不能利用SQL NULL绕过校验；外企业账号不能被借为审核人；只拒绝，不猜测修补旧事实 |
| QREAD-10 共同提交与失败 | 实测/事件或审核、operation_logs、audit_logs及成功回执一次提交；数据库失败500，原ID可在故障解除后重试 | 现仓储叶 `log_effect`；现 `API/manufacturing.rs::inspection_write_error`；原日志仓储 | 两新表和audit_logs的实际INSERT故障分别触发；证明故障位置被真正命中，整笔回滚；移除本用例故障后原ID成功，重放不新增，不泄露数据库内部错误 |
| QREAD-11 多样本、单位和扩展 | 规划样本序号/每样本原值、单位和精度、设备/校准、缺陷/附件及检验计划；超过当前20次和1000条原证明预算的有界接续必须设计，不能删旧失败 | 规 `UI/pages/quality/inspections.rs`及新的样本/计划表；继续复用现冻结版本与来源 | 当前每项目一条代表值不等于已完成sample_count个实测；当前必检0/0范围仍沿旧准入限制。多样本齐套、量纲、公差、更多历史、附件授权和样本判定必须另验 |
| QREAD-12 正式双端 | 展示真实原范围/值/状态、声明责任人、提交账号、原审核、复检原因；编辑过期时保留草稿并提示重查；业务表格只用自研DataGrid | 规 `UI/pages/quality/inspections.rs`、两端HTTP适配/DTO与页面注册；公用表/子表/详情沿现工作台 | 桌面/Web真实登录→保存失败值→重登读回→复检→审核→实际放行；IME、网络失联、未决保存、过期引用及查询取消分别测，夹具和只读页不能代替 |

`inspector/reviewer`各最多512个UTF-8字节，项目最多1024字节，复检/审核原因最多2048字节。MISSING仍保留`reading_present=false`，原表兼容存0，但新快照中的`reading_micros`为空字符串；已测0的快照为字符串`"0"`、presence=true。原范围和已测值在响应中使用精确字符串；不改变其他数量账的一百万分之一含义。

现内部旧方法`SharedContractRepo::save_frozen_inspection_reading`仍沿原准入拒绝越界/必填空值，供既有内部契约使用；当前HTTP handler已改接`save_inspection_outcome`。内部旧成功读数没有新事件时，不能补造原reviewer、提交账号或审核；历史证据缺失保持LEGACY/待核对。本章不宣称所有内部写入口完成新流程迁移。

首次v1合格读数保留既有放行政策，项目级审核不作为新前置；一旦实际REJECT则阻断。复检从第2次起必须有实际APPROVE。两者均区别于原整笔质量审核`quality/receipts/{receipt_id}/releases`以及实际库存放行；不能把声明reviewer、项目审核或整笔审核直接显示为库存已放行。

### 十四B HTTP字段和数据库位置

| 动作 | 实际HTTP位置 | Request/Query与最小字段 | 成功结果 |
| --- | --- | --- | --- |
| 首次实测 | POST `/v1/tenants/{tenant_id}/quality/inspection-freezes/{freeze_id}/readings` | 现父模块 `InspectionReadingRequest`：`request_id/item_code/reading_qty/inspector/reviewer` | 原 `effect_id/replay/status`加`reading`原快照；SUCCEEDED可以对应FAILED/MISSING |
| 复检 | POST `/v1/tenants/{tenant_id}/quality/inspection-freezes/{freeze_id}/reinspections` | 现API叶 `InspectionReinspectionRequest`：首次字段加`previous_item_id/reason` | 新item_id、递增attempt_no、previous_item_id、review_required=true及原判定 |
| 项目审核 | POST `/v1/tenants/{tenant_id}/quality/inspection-freezes/{freeze_id}/readings/{item_id}/reviews` | 现API叶 `InspectionReadingReviewRequest`：`request_id/decision/reason`；不能提交reviewer_user_id | 新review_id、真实reviewer_user_id、原读数和审核原快照；原测量仍不变 |
| 项目历史 | GET `/v1/tenants/{tenant_id}/quality/inspection-freezes/{freeze_id}/readings?item_code=WIDTH` | 现API叶 `InspectionReadingHistoryQuery`：`item_code`；当前一个项目固定有界历史，未实现通用全量检验分页 | `history/attempt_limit/legacy_reading_count/legacy_provenance_complete`；空历史与旧证据缺口分别表示 |

三个新增handler位于`crates/erp-api/src/manufacturing/inspection_outcomes.rs`，由现父`manufacturing.rs::router`注册；首次handler保留在父模块。新写入和项目审核要求quality.write，历史要求quality.read，同时查询当前实际ACTIVE用户。停用/无权限403，格式非法400，过期引用/重复新请求/超限等语义422，同ID负载冲突或来源已完成409，必要写入失败500。真实角色与会话政策需正常账号验收，不能以合成JWT测试代替。

`shared_inspection_reading_events`以`(tenant_id,item_id)`为主键，外键连接原`mfg_quality_items`、冻结项目及来源证明。保存`freeze_id/item_code/attempt_no/previous_item_id/review_required/verdict/declared_reviewer/actor_source/actor_user_id/reason/request_id/payload_hash/snapshot/created_at`。同项目/次数、原请求以及上一item_id分别唯一，既保留顺序也禁止分叉。快照不是由客户端提交的“已通过”声明。

`shared_inspection_reading_reviews`以`(tenant_id,review_id)`为主键，原item_id及原request_id分别唯一，保存`reviewer_user_id/decision/reason/payload_hash/snapshot/created_at`；snapshot包含实际原读数。账号外键通过新`users(tenant_id,user_id)`唯一索引核对企业；现用户原主键是全局user_id，不能误认为已有复合键。原回执仍引用现主键`(tenant_id,request_id)`，延迟触发器逐字核对完整payload_hash、SUCCEEDED和效果ID，不给长文本建立全局Btree。

放行原证明新增`effective_readings/reading_events/reading_reviews`，同时保留原`readings`数组的失败历史。当前原证明总读数仍最多1000，读取1001显式拒绝；20次单项目限制不等于每个1000项冻结都可再复检20次。更大历史的有界取证和分页必须接续实现，不能截断失败行来通过放行；快照字节、峰值内存及Release时延尚未验收。

### 十四C 可执行样本和当前证据

现测试文件为`crates/erp-api/tests/free_erp_acceptance/inspection_outcomes.rs`、`inspection_outcome_edges.rs`、`inspection_outcome_commands.rs`、`inspection_outcome_sales.rs`，均在固定入口`free_erp_acceptance`登记。用实际生产收货处理创建HOLD库存、真实版本/冻结和新QA企业，再调用Axum实际HTTP请求处理及真实PostgreSQL；公网TCP/网关、真实密码和物理双端另验。27项样本包含以下组：

| 样本组 | 必须保存的结果和失败证据 |
| --- | --- |
| QREAD-CASE-01 实际失败与未读 | -0.25保存FAILED、None保存MISSING，二者均阻断实际放行；已测0与未读不同；原事实、原量和实际回执逐项核对 |
| QREAD-CASE-02 原责任与权限 | 声明reviewer和实际提交UUID都落库；quality.read不能写、quality.write不能冒充read；停用账号403、跨企业403/422且无效果 |
| QREAD-CASE-03 完整复检闭环 | 原失败→合格复检仍拦截→实际APPROVE只保存审核且库存仍HOLD→实际release只产生一份原实物效果；原失败永不改写 |
| QREAD-CASE-04 审核不能造合格 | FAILED/MISSING不能APPROVE；合格被REJECT后仍保留原值和数值结论，再追加复检和审核新值 |
| QREAD-CASE-05 过期及错归属 | 旧previous、旧item审核、另一项目/企业引用分别拒绝，不能偷偷换到当前行 |
| QREAD-CASE-06 原协议和重试 | 逐字保留v1规范负载；数字格式等值重放、改值409；新审核或销售修订后仍返回原快照；放行后新审核拒绝，原重放成功 |
| QREAD-CASE-07 只读历史 | 真实持有工单写锁时GET仍返回原失败/复检/审核；只读快照，前后没有新增回执、日志或库存 |
| QREAD-CASE-08 深度与非法输入 | 20次全部历史可读，21次明确拒绝且0效果；7位小数和伪造actor字段拒绝 |
| QREAD-CASE-09 不可变与提交守卫 | 实际改/删读数、事件、审核、原回执7种动作拒绝；真正commit拒绝缺原回执、完整负载不符及效果不符；遗漏snapshot账号不能利用NULL绕过；外企业审核UUID被真实复合外键拒绝 |
| QREAD-CASE-10 后序实际故障 | 事件、审核、audit_logs三处真实INSERT故障各有非事务命中标记，500且完整回滚；移除仅本用例故障后原ID成功，重放不新增，不返回内部SQL错误 |
| QREAD-CASE-11 20路真实竞争 | 同项目不同ID、同ID、同读数审核，以及10审核/5复检/5放行混合分别20路；实际PG阻塞关系和全部20响应保留，不把tokio::join或过滤用例当充分竞争证据 |
| QREAD-CASE-12 销售原版本 | 销售读数work_order_id为NULL；新真实销售版本替换产品后，原重试/历史不变，新不匹配审核拒绝；真实销售库存和SHIP流程仍另验 |

修前[3项原反例](../qa/acceptance/free-erp-inspection-outcomes-original-encoding-corrected-independent.json)为0通过/3失败/0忽略，1.49秒；独立复核5,883个既有QA企业54张选定表全部列不变，FAIL结果不改为PASS。首个读回脚本的编码名错误导致未完成复核，原脚本字节保留；更正编码后按同一真实日志和完整事实重新核对。

修后[3项专项事实](../qa/acceptance/free-erp-inspection-outcomes-fixed-focused-independent.json)为3通过/0失败/0忽略，2.79秒；独立复核5,890个既有企业54表不变，真实事件、精确值、原回执和HOLD状态一致。[首轮扩展](../qa/acceptance/free-erp-inspection-outcomes-fixed-expanded-strengthened-independent.json)实际24通过/3失败：旧制造共用校验误要求quality.read用户具备制造写权限，另有销售测试错用SUBMIT修订。失败日志和原脚本保留，未改为PASS。

修正质检入口独立权限及销售真实REVISE/expected_version后，[27项扩展独立事实](../qa/acceptance/free-erp-inspection-outcomes-fixed-permissions-independent.json)实际27通过/0失败/0忽略，50.04秒；另一psql连接独立核对7,545项，5,934个既有企业54表全部列不变。规范负载按实际HTTP重新计算，四份真实20路PG阻塞图、三处INSERT命中、七种不可变SQL及五种原回执/账号守卫逐项核对；原失败与最新审核后的实际库存效果一致。运行期间另有shared_contract测试根变动，948个源码输入资格整体PARTIAL，本批实测叶与生产接线稳定。完整登记回归、正常登录/角色会话、正式两端及发布性能仍独立验收，不能据此认证全部Q01或上线；debug总秒数不作为P95/P99结果。

[完整登记回归](../qa/acceptance/free-erp-inspection-outcomes-fixed-full-independent.json)实际318通过/0失败/0忽略/0筛除，263.52秒；独立7,575项核对覆盖本批29个新企业场景及5,964个既有企业54表保护，其他登记场景以各自实际测试断言为证。本窗口制造API根、共享仓储根/测试根及三份OpenAPI输入发生并行变更，整体当前源码资格仍PARTIAL，不把旧产物的318通过认证为后来所有代码。四个当前质检契约及95份成功实测响应见[OpenAPI复核](../qa/acceptance/free-erp-inspection-outcomes-openapi-fixed-permissions-refreshed.json)。严格Clippy实际仍FAIL：仓储lib 237/lib test 239、四渠道合计27；当前未报本批新仓储叶诊断，API lint被依赖阻断，不能写API已通过。全迁移、正常登录/权限会话、正式两端及Release门禁继续待验。

原公开零读数兼容入口`shared_contract::inspection_reading_http_race_read_back`补入实际ACTIVE账号夹具后，实际1通过/0失败/0忽略，0.80秒；[独立6,337项复核](../qa/acceptance/free-erp-inspection-outcomes-direct-reading-independent.json)确认6,324个既有企业54表不变，原0值是已测且合格，只一份读数/事件及实际成功命令，另一新请求422且无回执。首次构建被并行新增导出测试的JWT移动错误阻断，未执行此业务用例；当前源码已由并行开发修正后才实际重试，原失败日志保留，不能把首次编译失败说成两轮业务测试。9个涉及叶模块的固定入口格式检查通过，父根未全仓格式化。

首轮0294因现users没有复合唯一键失败，事务整笔回滚；[首次迁移回滚复核](../qa/acceptance/free-erp-inspection-outcomes-first-migration-rollback-independent.json)验证5,889个既有企业不变、新表不存在。保留首版实际DDL字节与失败日志，补索引后的[QA迁移复核](../qa/acceptance/free-erp-inspection-outcomes-migration-retry-independent.json)确认同一组企业原事实不变、两个新表没有猜测回填。当前只应用受保护QA子链，全ERP新库、升级链和生产迁移均未验。

### 十四D 专属性能指标

下列为候选目标，当前全部NOT_RUN。遵循第四节固定环境、三轮最差结果和最近秩统计；普通动作每规模每轮至少1000次，重放、业务拒绝、锁等待和故障独立统计。当前仅登记BOM专属Release性能目标，须先登记检验目标再运行，不能借BOM名字、debug秒数或另建target。

| 性能组 | 具体负载和实际位置 | 目标和同时核对 |
| --- | --- | --- |
| QREAD-PERF-01 实测保存 | 单项目首次PASSED/FAILED/MISSING三组，1/100/1000项原冻结；现save_inspection_outcome，含鉴权、来源/项目/原请求锁、原值/事件、两日志、回执及commit | P95≤1秒/P99≤2秒；只有原值、数值结论、声明/账号及原回执一致才算成功；业务拒绝不混入成功组 |
| QREAD-PERF-02 复检追加 | 第2/10/20次分别测，真实prev/reason、原失败及当前审核；21次拒绝另组 | 合法P95≤1秒/P99≤2秒；不分叉、不覆盖；单行查询有索引，记录扫描/返回量、池和锁等待 |
| QREAD-PERF-03 项目审核 | 初次及复检APPROVE/REJECT，实际ACTIVE账号；失败/过期/停用等另组 | P95≤1秒/P99≤2秒；原读数不改写，仅一份实际审核，审核阶段库存不变 |
| QREAD-PERF-04 原结果重试 | 首次/复检/审核原ID各测，后来审核、销售修订或实物放行背景下分别测 | P95≤500ms/P99≤1秒；完整原快照一致，不返回后来状态，不新增任何效果 |
| QREAD-PERF-05 历史查询 | 单项目1/10/20条，10万冻结/100万读数背景；真实只读GET，持有工单写锁和独立查询两组 | 候选P95≤300ms/P99≤600ms；不得获取工单写锁；原历史完整，未授权拒绝不计成功 |
| QREAD-PERF-06 取证和实际放行 | 1/100/1000条原读数、不同复检深度及真实审核；inspection_proof与实际制造release分别计时 | 沿QSOURCE/RECEIPT发布目标；候选新增峰值≤16MiB、原快照≤4MiB；1001明确拒绝，不能截断旧失败；字节和峰值需实际测量 |
| QREAD-PERF-07 20路竞争与背压 | 同ID、不同ID、审核、混合各20路×100轮；另测不同工单/销售单；正式共享连接池和原有队列 | 全结果完成P99≤3秒；2000响应和实际PG阻塞关系齐全，无分叉、漏/多效果、死锁残留；QA每请求连接池不冒称生产单池背压通过 |
| QREAD-PERF-08 正式两端 | 实际质量页DataGrid编辑/历史/详情，固定网络桌面/Web各测；真账号和断网重登 | 沿ERP-P02/P03/P09；IME、输入、帧、网络和保存分别统计；成功回执、重登原事实和真库存一致，当前未接正式页面 |

每组报告必须附源码/产物SHA、服务器/PG版本与参数、CPU/内存/磁盘、数据基数、连接池/队列/锁等待、SQL计划、原始每请求时延、成功/拒绝/故障分类、快照字节/峰值内存及另一连接的独立事实核对。没有登记或没有实测的项目保持NOT_RUN。

## 十五 消费者质量闸门与精确贸易库存

归属T04、M08/M11、Q01/Q02、I02/I03。贸易SHIP、制造ISSUE/SUPPLEMENT与直接RELEASED收货不能只检查已经存在的必检行：冻结头存在但明细为空、只复制WIDTH而缺HEIGHT时，同样需要拒绝。完整标准、原来源和有效实测必须一并核对。以下现有内核及14个真实QA样本与待补正式页面、账号授权、余下库存写入口和Release性能分别记账，整体仍为PARTIAL。

### 十五A 逐项功能与实际落点

| 任务 | 具体功能与规则 | 实现位置 | 验收方向与剩余工作 |
| --- | --- | --- | --- |
| QGATE-01 统一消费者取证 | 按企业和真实来源锁定，再读取冻结头、完整冻结项目、来源绑定、原实测、复检和实际审核；完整取证成功才能继续提交 | 现 `REPO/shared_contract/production_receipt_release.rs::{source_inspection_proof,scoped_inspection_proof,inspection_proof}`；现 `REPO/shared_contract.rs::inspection_blocks` | 真实贸易SHIP、制造ISSUE/SUPPLEMENT和直接RELEASED分别调用；缺项拒绝而非空集合通过。`inspection_proof`继续复用原制造放行入口 |
| QGATE-02 核对旧完整副本 | 无来源绑定的旧冻结，必须找到同企业原模板及准确version_no，项目集合、required、min/max和sample_count逐项一致 | 现取证叶 `scoped_inspection_proof` 的template_valid、template_lines和原副本比较；现模板/冻结两组表 | 旧完整副本可以继续；空、部分、原版本不存在拒绝。不得替旧数据猜测绑定、真实审核账号或缺失读数；sample_count的副本一致不等于多样本实测已实现 |
| QGATE-03 来源与批次范围 | source_kind按trim/大写和既有RECEIPT别名解析，必须属于真实工单/销售单/收货；制造领料沿现有全批政策，直接完工及贸易SHIP按本批和旧空批核对 | 现 `REPO/shared_contract/inspection_sources.rs::{parse,lock_source}`；现 `REPO/shared_contract.rs::inspection_source_required`；现取证叶 | 小写/空格的合法旧来源不绕过也不误拒绝；另一批的空副本不阻断本批直接收货。采购来源、所有行业的批次政策和正常账号范围另验 |
| QGATE-04 接精确贸易流水 | shared_stock的ON_HAND/RESERVED/FROZEN/IN_TRANSIT先通过权威InventoryService，再在同一事务写一份带真实关联ID的stock投影；非GOOD实物同时冻结 | 现 `REPO/shared_contract/trade_stock.rs::{StockPosting,post}`；现 `REPO/inventory_precision.rs::apply_precise_with_tx`；父入口 `append_stock_ledger/reserve_stock_line/release_stock_line` | 逐笔对照tenant/warehouse/SKU、四个精确delta及真实ref_id，不关闭0283守卫。UNAVAILABLE/HELD/WIP、旧cancel/return及其他内部写入口仍需单独接续，不能报告所有库存入口已迁移 |
| QGATE-05 零可用量销售审核 | REVIEW在同一事务先减本单预占，再减对应GOOD实物；两笔使用原source_line/request_id。ISSUE和SHIP不再次扣货 | 现 `REPO/shared_contract.rs::advance_sales_delivery` 的REVIEW分支；现trade_stock叶 | 真实available=0且本单reserved=1仍可审核；提交后reserved及on_hand各少1、available仍0。拒绝、重放与另一来源竞争须保证原子；生产共享连接池及Release未验 |
| QGATE-06 失败与原请求重放 | 质量拒绝不写材料、收货、交付状态、库存、日志或成功回执；合法原ID重放不再产生第二份数量效果 | 现原 `admit_retry/load_receipt/insert_receipt`；现材料、收货与交付入口；现QA `inspection_gates.rs::{facts,gate_http}` | 销售当前拒绝409，制造当前拒绝422；旧同ID不同负载仍409。当前release_stock_line只允许一个负释放记录，连续多次部分释放与cancel竞争尚待改造；本批0.5→释放0.2→剩余0.3及原重放已测 |
| QGATE-07 来源锁和有界读取 | 在读冻结头之前取得真实sales-revision或工单来源锁；头/行/原读数/绑定以1001哨兵拒绝超过1000，原绑定单份≤4MiB、总量≤8MiB | 现 `scoped_inspection_proof`、`binding_bytes`；现 `inspection_sources.rs::lock_source`；现真实QA20路SHIP锁样本 | 20个请求实际被PG阻塞，锁持有事务插入旧空副本并提交后，20个都409且无成功命令。预算是源码边界；1000/1001及4/8MiB边界、峰值内存仍须专测，不以限制常量冒称性能通过 |
| QGATE-08 正式页面与业务闭环 | 在销售交付、领补料、完工和质量页展示真实来源、冻结标准、当前有效值与原审核；错误保留草稿，恢复原请求回查，不将HOLD或审核显示为已放行 | 规 `UI/pages/trade/sales_orders.rs`、`UI/pages/manufacturing/materials.rs`、`UI/pages/manufacturing/receipts.rs`、`UI/pages/quality/inspections.rs`及统一页面注册/双端HTTP适配 | 沿第六章PAGE-13/19/21/22的建议宿主及独立交付实例，尚未接线；全部表用自研DataGrid。桌面/Web真实登录、编辑、保存失败、重登和真实库存独立验，不把QA JWT、每请求新池或夹具当公网产品 |

接口沿既有路径，不新增平行写入：POST `/v1/tenants/{tenant_id}/sales-orders/{sales_order_id}/deliveries`的SHIP/REVIEW、POST `/v1/tenants/{tenant_id}/manufacturing/work-orders/{work_order_id}/materials`的ISSUE/SUPPLEMENT、POST `/v1/tenants/{tenant_id}/manufacturing/work-orders/{work_order_id}/receipts`的直接RELEASED。精确贸易叶是内部仓储接线；没有新增公开“万能库存写入”API。本章不改变免费单账号多端和后续收费边界。

数据库沿现表：`shared_inspection_freezes/lines/source_bindings`保存原标准和来源；`mfg_quality_items`与`shared_inspection_reading_events/reviews`保存原实测/复检/审核；`shared_stock_ledgers`为批次/质量投影，`inventory_ledgers/balances`为当前精确库存。原制造收货还会保留一份MFG_RECEIPT兼容流水。独立重建余额仅能排除经真实receipt→stock→canonical引用、四数量的整数/微量值、企业/仓库/SKU、reason及occurred_at完全相符的那份兼容投影；不能因同值、同reason或字符串像receipt就任意去重。批次/库位/质量明细与仓库/SKU汇总分别核对，不把粗粒度汇总当作所有批次可用量。

### 十五B 十四个真实验收样本

所有样本在受保护QA端口15438创建新企业，不修改既有企业业务；测试入口为 `crates/erp-api/tests/free_erp_acceptance/inspection_gates.rs`，登记在 `free_erp_acceptance.rs`。下列是本批实际测试目标，不覆盖全部首发业务。

| 样本 | 输入和实际调用 | 预期独立事实 |
| --- | --- | --- |
| QGATE-CASE-01 贸易空副本 | 真实SUBMIT→FULFILL、入库1、预占1、交付RESERVE/PICK/REVIEW/ISSUE；冻结WIDTH/HEIGHT标准但0行，再实际SHIP | 409；保持ISSUED、无SHIP回执、GOOD开仓/预占/解除/扣货只有原四笔 |
| QGATE-CASE-02 贸易部分副本 | 同上，仅复制WIDTH且真实1.5合格；HEIGHT缺失 | 409；可见合格WIDTH不能代替缺失HEIGHT；原事实不变 |
| QGATE-CASE-03 制造空副本 | 真实工单、报工和待检收货；工单原标准空副本，ISSUE 0.1 | 422；没有mfg_material_moves、材料数量及命令副作用 |
| QGATE-CASE-04 直接完工空副本 | 真实原报工，直接RELEASED 0.1、本批QA-PRODUCTION；同源原标准0行 | 422；没有新增可用收货或库存、回执效果 |
| QGATE-CASE-05 旧贸易完整副本 | source_kind=` sales_order `，真实WIDTH/HEIGHT两项1.5；实际SHIP及原ID再发 | 两次200，最终SHIPPED；只一份成功命令，重放不重复数量效果，无猜测来源绑定 |
| QGATE-CASE-06 旧制造完整副本 | source_kind=` work_order `，两必检真实通过；ISSUE及SUPPLEMENT各0.1，各原ID重放 | 只两个材料动作各100000微量，四次HTTP200；原事实在重放前后完全相同 |
| QGATE-CASE-07 完整标准直接完工 | 本批完整两项标准、原报工，RELEASED 0.1 | 实际available_micros=100000；精确流水与兼容投影明确关联 |
| QGATE-CASE-08 未设规则 | 本工单没有任何冻结头，实际ISSUE 0.1 | 200并只一笔材料动作；不凭空新增检验前置或冻结 |
| QGATE-CASE-09 旧版本失配 | 旧头version_no=99，实际模板仅v1；ISSUE 0.1 | 422；无材料动作及失败请求的成功回执 |
| QGATE-CASE-10 不同批次 | 旧空副本批次QA-OTHER-BATCH；直接收货批次QA-PRODUCTION 0.1 | 按既有批次政策200及available=100000；不扩大另一批的检验规则 |
| QGATE-CASE-11 真实拒绝和复检链 | 实际冻结销售来源，WIDTH/HEIGHT首次1.5；账号REJECT WIDTH→SHIP；复检WIDTH 1.5未审核→同ID SHIP；实际APPROVE→同ID SHIP及重放 | 顺序409/409/200/200；原3个实测事件和2个真实审核完整保留，只一份SHIP成功效果 |
| QGATE-CASE-12 真实来源锁竞争 | 同销售来源20路不同ID SHIP；用PG阻塞图确认20路等待，在持锁事务插入空副本再commit | 20个409、0个成功SHIP；全部请求完成、旧交付保持ISSUED、失败ID无命令，不只用Barrier证明并发 |
| QGATE-CASE-13 零可用量审核 | 正常预占后通过精确服务冻结剩余自由量，独立测available=0，再实际REVIEW/ISSUE | 合法200；本单reserved与实物各减1且available仍0，无重复扣货 |
| QGATE-CASE-14 精确部分释放 | 新来源预占0.5，调用release_stock_line释放0.2，用相同原ID重放 | 只一笔reserved=-200000，source剩300000，实物不变，只有一份原成功命令及同effect_id |

修前、修后、完整登记回归、独立快照和输入SHA随本批证据记录；原失败、编译阻断和测试读回错误必须保留。测试秒数属于debug功能运行耗时。QA使用实际ACTIVE企业用户与合成认证，正常密码/角色/会话撤销、公网服务和生产共享池没有在这些样本中验收。

修前原4样本中两个制造入口实际200并有副作用，贸易准备先被0283精度守卫拒绝；[首轮失败日志](../qa/acceptance/free-erp-inspection-gates-original-test.log)保留，因此不能称首轮四个都完成质量反例。接精确贸易流水后、质量仍未修的[实际反例复核](../qa/acceptance/free-erp-inspection-gates-quality-unfixed-readback-independent.json)确认四个都实际200并产生错误效果，0通过/4失败/0忽略，3.51秒；保护6,337个既有企业56张选定表全部列。此前编译被并行新增domain符号阻断，业务0执行；修正独立解析器对精确制造收货兼容投影的重复求和后另存复核，原编译/读回失败仍保留。

修后首轮扩展11通过/1失败；唯一失败是测试facts遗漏mfg_material_moves，实际ISSUE/SUPPLEMENT及原重放均200，失败日志保留。[14项专项独立复核](../qa/acceptance/free-erp-inspection-gates-fixed-final-independent.json)实际14通过/0失败/0忽略，17.85秒；独立19,660项核对及6,358个既有企业56表全部列保护，14个新场景的原副本、实际HTTP、真实审核链、20路PG阻塞及精确余额分别检查。一次功能竞争不等于Release百分位、正式共享池背压或公网认证已通过。

[完整登记复核](../qa/acceptance/free-erp-inspection-gates-full-independent.json)实际332通过/0失败/0忽略/0筛除，250.70秒；独立20,065项核对，保护6,374个既有企业56张选定表全部列。本批14个新场景逐项做独立事实核对，其他登记场景以各自真实Rust断言为证。950个输入的[源码窗口复核](../qa/acceptance/free-erp-inspection-gates-source-audit.json)显示专项窗口1个、完整窗口7个输入变化；本批三个实现/测试叶不变，但生产接线所在父根也有并行修改，整体当前源码资格PARTIAL，不能把旧构建认证为后来所有代码。

销售交付此前缺少对应OpenAPI，现补入实际`sales_order_id`路径、13字段`SalesDeliveryRequest`和计划/动作/回查/取消四种成功形态，保留现数字微量HTTP及旧分支优先级。OpenAPI源与bundle、六个消费者/实测POST的当前Rust DTO、54份成功响应和27份实际拒绝匹配，见[本批契约复核](../qa/acceptance/free-erp-inspection-gates-openapi-full.json)；项目v2门禁也通过。计划/回查/取消形态仅补契约，未由本批质量样本调用，不能宣称相应完整业务通过。

本批三个叶的固定入口格式检查通过。严格[仓储Clippy](../qa/acceptance/free-erp-inspection-gates-clippy-repo.log)实际lib 239/lib test 241项失败；[API Clippy](../qa/acceptance/free-erp-inspection-gates-clippy-api.log)被仓储依赖239项失败阻断。未报这三个叶的诊断，不表示它们已获独立lint或API/全包通过；不压制警告，不终止其他构建或换缓存绕过锁。

### 十五C 八组性能和资源任务

下列全部是候选发布目标，状态NOT_RUN；复用第四/九章环境、三轮最差、最近秩统计及原始每请求样本规则。普通单写/查询每规模每轮至少1000次，竞争20路×100轮；错误、原重放、来源锁等待及业务成功各自统计。必须先在统一入口登记专属性能目标，当前不能借BOM性能目标运行检验或贸易负载。

| 性能组 | 具体负载、埋点和位置 | 目标与同时满足的事实 |
| --- | --- | --- |
| QGATE-PERF-01 完整取证 | 0/1/100/1000行冻结×真实WorkOrder/Sales；10万来源、100万实测背景；scoped_inspection_proof分别计来源锁、头/行/绑定/实测查询、校验及JSON | 1/100行取证P95≤500ms/P99≤1秒；1000行候选P95≤1秒/P99≤2秒；完整标准/旧历史和当前有效审核一致 |
| QGATE-PERF-02 实际消费者提交 | SHIP、ISSUE/SUPPLEMENT、直接RELEASED分别测1/100/1000行标准；包含当前身份/仓库检查、锁、取证、库存、日志、回执和commit | 常规1/100行P95≤1秒/P99≤2秒；1000行候选P95≤2秒/P99≤4秒；不能只测叶函数替代HTTP及实际效果 |
| QGATE-PERF-03 明确拒绝与预算 | 空/部分/版本错、1001行、单绑定>4MiB或总绑定>8MiB各单独负载；SQL扫描行/返回字节与构造JSON耗时同时计 | 拒绝P95≤500ms/P99≤1秒；不截断、不部分保存，返回字节有界。预算上限已编码，边界及Release资源仍未实测 |
| QGATE-PERF-04 精确贸易库存 | 0.000001/0.2/1/500单位开仓、预占、单次部分释放与REVIEW；available=0和正常自由量分别测；trade_stock::post含权威锁/流水/投影 | 单动作P95≤1秒/P99≤2秒；零可用审核合法、四精确delta重建一致，无负余额、舍入或兼容流水重复计数 |
| QGATE-PERF-05 原请求与失败重试 | 预占/释放/REVIEW/SHIP原ID重放，拒绝后标准补齐用原ID重试；后来交付状态与库存变化作为背景 | 合法重放P95≤500ms/P99≤1秒；原effect_id及数量不变，失败无成功命令；现旧接口返回形态逐个注明，不冒称已具完整原快照 |
| QGATE-PERF-06 20路混合竞争 | 冻结与SHIP、读数/审核与SHIP、领料与冻结、同/异库存键REVIEW及release；正式共享池和实际有界队列 | 每轮全部20结果完成P99≤3秒，无死锁、失联未决或多效果；独立来源不采用全企业大锁；保留实际PG阻塞图与池/队列等待 |
| QGATE-PERF-07 原证明资源 | 1000行与复检深度1/10/20、绑定接近4/8MiB；同时记录源JSON、最终证明、RSS峰值、分配及SQL返回字节 | 候选新增峰值≤32MiB；最终持久化原证明沿原4MiB预算另测；超过预算明确拒绝，不删旧失败或静默缩短历史。8MiB输入预算不表示最终证明已达4MiB |
| QGATE-PERF-08 正式两端业务 | 销售交付/制造领料/完工/检验页，各500/2000行与1/100/1000项明细，固定网络桌面/Web分别真实登录 | 沿ERP-P02/P03/P08/P09；输入P95≤50ms、滚动帧P95≤32ms，查询/保存分项计时；真实失败、重登、回执与库存一致；页面未接不填PASS |

性能报告必须保存源码/产物SHA、CPU/内存/磁盘、PG/服务版本与参数、数据规模、SQL计划、每请求原始耗时、成功/拒绝/故障数量、池/锁/队列等待、吞吐及峰值内存，附独立连接业务读回。源码预算、一次20路功能竞争和debug总耗时不能代替这些发布指标。

### 十五D 并行新增任务、报表和命令入口的源码盘点

2026-10-07核对同一制造路由模块的33个tasks/reports/commands/work/documents/access/health路径、39个HTTP操作。下面记录当前注册与Request/Query字段，方便Grok接续，均位于现 `API/manufacturing.rs`；新增两个GET为窗口关闭原记录和文档冲突输入回查，这些入口不据前面14项质量闸门测试认定完成。正常授权、调用方自报字段与权威事实的区分、原回执、正式列表、真实报表计算、OpenAPI和发布验收须各自提供证据。源码有路由或HTTP能返回，不表示已经能接收运营中的真实任务或提供完整经营报表；命令原结果的局部修复另见第十六章。

| HTTP与路径 | 现handler / DTO | 真实请求或查询字段 | 后续验收方向 |
| --- | --- | --- | --- |
| POST `/v1/tenants/{tenant_id}/tasks/exceptions` | `save_task_exception_route` / `TaskExceptionRequest` | `request_id/kind/object_type/object_id/command_id/query_generation/error_text` | 原命令/对象及异常事实可信；未知、跨企业引用拒绝；异常状态与真正业务失败一致 |
| POST `/v1/tenants/{tenant_id}/tasks/exceptions/{exception_id}/actions` | `act_on_task_exception_route` / `TaskExceptionActionRequest` | `request_id/command_id/query_generation/action` | 重试和处置引用正确原命令及代次；过期/已完成/不允许动作不产生新效果 |
| POST `/v1/tenants/{tenant_id}/tasks/exceptions/{exception_id}/effects` | `save_task_effect_route` / `TaskEffectRequest` | `request_id/effect_text/success_banner` | 文本或success_banner不能替代权威已提交效果；真实对象读回、重放和故障回滚 |
| POST `/v1/tenants/{tenant_id}/tasks/exceptions/{exception_id}/cancellations` | `cancel_committed_task_route` / `TaskCancelRequest` | `request_id/command_id` | 取消受理、执行中和已提交分别定义；不能将数据库已提交事实直接改为未执行 |
| GET `/v1/tenants/{tenant_id}/tasks/effects` | `read_task_effect_route` / `TaskEffectQuery` | `object_id/command_id/query_generation` | 查询原效果及原代次；无副作用；缺失、跨企业及旧代次有明确结果 |
| POST `/v1/tenants/{tenant_id}/tasks/dead-letters` | `post_dead_letter_once_route` / `DeadLetterPostingRequest` | `request_id/object_type/object_id/command_id/amount_micros` | 实际死信、原数量与原效果约束，只一次合法补记；调用方amount不自行成为权威账目 |
| GET `/v1/tenants/{tenant_id}/tasks/dead-letters` | `read_dead_letter_posting_route` / `DeadLetterPostingQuery` | `object_id/command_id` | 只读取企业授权原补记事实；已补记/不存在清晰，不能把查询当补记 |
| POST `/v1/tenants/{tenant_id}/reports/saved-views` | `save_report_view_route` / `SavedReportViewRequest` | `request_id/report_scope/name/date_from/date_to/dimension/measure/chart_type/formula` | 维度/度量/公式与真实字段目录一致；保存视图和实际计算分别验收；非法公式、范围和权限拒绝 |
| POST `/v1/tenants/{tenant_id}/reports/field-catalogs` | `save_report_field_catalog_route` / `ReportFieldCatalogRequest` | `request_id/report_scope/fields` | 字段目录来自实际可访问数据；禁用字段引用和旧视图处理，不信任任意客户端字段即合法 |
| POST `/v1/tenants/{tenant_id}/reports/saved-views/{view_id}/stale-fields` | `mark_cited_report_stale_route` / `ReportStaleRequest` | `request_id/caller_fields` | 对照真实原目录和视图引用判失效；记录实际失效原因，调用方自报列表不能取代核对 |
| POST `/v1/tenants/{tenant_id}/reports/saved-views/{view_id}/restores` | `restore_cited_report_pair_route` / `ReportRestoreRequest` | `request_id/session_id/caller_query` | 原视图/会话配对和当前授权重检；保留有效查询、草稿和非法条件，恢复不越权 |
| POST `/v1/tenants/{tenant_id}/reports/saved-views/{view_id}/trends` | `save_saved_view_trend_route` / `SavedViewTrendRequest` | `request_id/grain/caller_amount` | DAY/WEEK/MONTH实际日历与原查询统一；趋势金额从真实账计算，不把caller_amount直接显示为实测结果 |
| POST `/v1/tenants/{tenant_id}/reports/saved-views/{view_id}/shares` | `share_saved_report_view_route` / `SavedReportShareRequest` | `request_id/recipient_user_id/caller_claimed_share` | 当前有效接收账号、企业和数据范围重检；分享声明不能代替授权。免费一个登录账号政策继续有效 |
| GET `/v1/tenants/{tenant_id}/reports/saved-views/{view_id}/shares` | `read_shared_report_view_route` / `PathOnly` | 无Query；view_id来自路径，接收人来自认证上下文 | 真账号读回、撤权和失效链接分别验；无额外客户端接收人字段，查询不创建分享 |
| POST `/v1/tenants/{tenant_id}/commands` | `open_command_route` / `OpenCommandRequest` | `request_id/object_type/object_id/version_no` | 原对象和版本归属、命令状态由服务端确认；开启命令不等于原业务已提交 |
| POST `/v1/tenants/{tenant_id}/commands/{request_id}` | `advance_command_route` / `AdvanceCommandRequest` | `reported_status/error_text/recoverable/payload_hash` | 不能以客户端reported_status替代数据库提交或真实失败；原负载、状态迁移和业务效果共同核对 |
| GET `/v1/tenants/{tenant_id}/commands/{request_id}` | `read_command_route` / `PathOnly` | 无Query；原request_id来自路径 | 当前身份和原对象范围校验，只读返回原状态/效果，缺失与未知区分，不重发未决业务 |
| POST `/v1/tenants/{tenant_id}/commands/{request_id}/lost-lookups` | `lookup_lost_command_route` / `LostCommandLookupRequest` | `payload_hash/revoked_session_id/current_session_id/client_unknown/warehouse_id/sku_id/qty_units` | 真会话撤销/当前授权及原负载读回；客户端unknown、会话ID和数量不能自动成为原已提交证据；回查不能产生第二笔库存效果 |
| GET `/v1/tenants/{tenant_id}/commands/{request_id}/restarts` | `read_restarted_command_route` / `CommandRestartQuery` | `restarted` | 请求标记不证明真实客户端重启；只读返回原效果，缺失不创造失败或占位 |
| POST `/v1/tenants/{tenant_id}/commands/{request_id}/window-closes` | `record_window_close_route` / `WindowCloseRequest` | `request_id/client_unknown` | 关闭窗口意图不能清除已提交业务、重发未决动作或替代真实进程退出；具体客户端尚须独立验 |
| GET `/v1/tenants/{tenant_id}/commands/{request_id}/window-closes` | `read_window_close_route` / `WindowCloseRead` | `close_request` | 请求路径与原关闭编号绑定；只读返回原decision/effect/effect_count，executed=false。当前身份/对象范围、不存在及跨企业拒绝独立核对，不把关闭记录当真实进程退出或新执行 |
| POST `/v1/tenants/{tenant_id}/tasks` | `enqueue_durable_task_route` / `DurableTaskRequest` | `request_id/event_id/effect_id` | 任务必须来自真实事件和原效果；客户端自报ID不证明已提交，幂等接单和执行效果分开验 |
| GET `/v1/tenants/{tenant_id}/tasks` | `read_durable_task_page_route` / `DurableTaskPageQuery` | `status/offset/limit/count_exact` | 当前默认limit=100、count_exact=true，按status/task_id分页；实际仍独立COUNT且响应version=1，不能当真实提交版本或同一快照。当前身份/来源仓库范围、计数策略、稳定查询及正式客户端另验 |
| POST `/v1/tenants/{tenant_id}/tasks/{task_id}/claims` | `claim_durable_task_route` / `DurableClaimRequest` | `request_id/owner/lease_seconds` | owner从执行身份核对，租约竞争和到期读真实任务；领取不能等同执行成功 |
| POST `/v1/tenants/{tenant_id}/tasks/{task_id}/heartbeats` | `heartbeat_durable_task_route` / `DurableHeartbeatRequest` | `request_id/owner/progress` | 原有效租约和当前执行者校验；客户端progress不证明业务提交，续约不产生库存效果 |
| POST `/v1/tenants/{tenant_id}/tasks/{task_id}/failures` | `fail_durable_task_route` / `DurableFailureRequest` | `request_id/owner/transient/known/error_text` | 原有效执行租约和真实失败事实核对；客户端known/transient不能代替服务端失败证明，也不能重复原效果 |
| POST `/v1/tenants/{tenant_id}/tasks/{task_id}/cancellations` | `cancel_durable_task_route` / `DurableCancelRequest` | `request_id/owner/posted/work_order_id/qty` | 由真实原任务及已提交事实决定取消，不能信任客户端posted/工单/数量而清除终态或另记库存；原效果、取消意图与补偿动作分开验 |
| POST `/v1/tenants/{tenant_id}/tasks/{task_id}/restores` | `restore_dead_task_route` / `DurableRestoreRequest` | `request_id` | 核对真实原任务、当前权限和原已提交效果；恢复任务不能直接重做库存或清除原失败事实 |
| POST `/v1/tenants/{tenant_id}/work/admissions` | `admit_tenant_work_route` / `WorkAdmissionRequest` | `request_id/kind/command_id/retry_after_seconds` | 准入与退避应来自真实服务资源及队列，客户端kind/等待值不能代替服务端观测，也不证明实际任务已提交 |
| POST `/v1/tenants/{tenant_id}/documents` | `open_versioned_document_route` / `VersionedDocumentOpenRequest` | `request_id/body_text` | 当前仅保存通用文本和版本；不能代替采购、销售、BOM或工单的字段校验及业务提交；正文大小、当前操作者、合法对象类型与原结果另验 |
| POST `/v1/tenants/{tenant_id}/documents/{document_id}` | `save_versioned_document_route` / `VersionedDocumentSaveRequest` | `request_id/expected_version/body_text` | 实际操作者来自认证上下文；旧版本保留输入与冲突，不能把冲突写作业务提交成功。重试须返回原版本和原冲突，正式双端草稿另验 |
| GET `/v1/tenants/{tenant_id}/documents/{document_id}` | `read_conflict_input_route` / `PathOnly` | 无Query；document_id来自路径，actor来自认证上下文 | 返回原version_no/stored_body/kept_input且不写入；另一操作者、企业及缺失分别验，不能用通用文本回查冒充完整采购/销售业务或双端草稿恢复 |
| POST `/v1/tenants/{tenant_id}/access/grants` | `grant_access_route` / `AccessGrantRequest` | `request_id/permission/warehouse_id` | 赋权者实际管理资格、授权目录及本企业真实仓库核对；不能让普通业务使用者请求一个权限名就取得该权限。沿原RBAC作为权威，不另造自由授权旁路 |
| POST `/v1/tenants/{tenant_id}/access/objects` | `register_access_object_route` / `AccessObjectRequest` | `request_id/object_kind/warehouse_id` | 真实业务对象和授权仓库绑定；只注册类型/仓库不证明已建立采购、库存或生产来源，也不证明调用者可访问原对象 |
| POST `/v1/tenants/{tenant_id}/access/actions` | `authorize_audited_action_route` / `AccessActionRequest` | `request_id/permission/object_id/summary/platform_operator/retention_days/free_plan` | 当前角色、对象范围、平台运营身份、套餐及留存策略来自服务端；客户端platform_operator/free_plan不能提权或改权益。真实业务效果与必需审计同事务另验 |
| POST `/v1/tenants/{tenant_id}/access/frozen-scopes` | `freeze_task_scope_route` / `FrozenScopeRequest` | `request_id/permission/warehouse_id` | 授权范围冻结不扩大调用者当前范围；绑定真实任务/提交人/原权限资料。新赋权不能扩大已受理任务快照，当前撤权应拒绝下载 |
| POST `/v1/tenants/{tenant_id}/access/frozen-scopes/{task_id}/downloads` | `authorize_frozen_download_route` / `FrozenDownloadRequest` | `request_id` | 原任务及文件来自当前企业和真实提交人；当前角色/仓库与原范围重检，授权通过不等于文件已安全完整交付；实际流、长度/SHA及竞争另验 |
| POST `/v1/tenants/{tenant_id}/access/warehouses/{warehouse_id}/revocations` | `revoke_warehouse_scope_route` / `WarehouseRevokeRequest` | `request_id` | 实际管理资格、被撤对象及当前范围核对；撤权事务提交后的新请求/下载拒绝。不能只删除泛用access记录却保留正式RBAC可访问事实 |
| POST `/v1/tenants/{tenant_id}/health/signals` | `record_health_signal_route` / `HealthSignalRequest` | `request_id/kind/value/threshold/duration_seconds/command_id/action_text` | 信号值、阈值、持续时间及动作必须绑定服务端真实观测和配置。客户端提交记录不是实际延迟/容量测量，不凭它证明服务恢复；运营写入资格、脱敏、告警和任务/命令来源另验 |

性能继续按ERP-P02/P03/P04/P05/P07及报表相关门禁执行；这37个入口当前分别按各自真实证据结项，不由质量闸门的14项通过替它们结项。制造最新盘点另包含看板领取BoardClaimRequest及atomic-effects验证入口，合计37个POST/11个GET、23个新增写DTO；客户端prepared_lines/failing_line不代表真实整单校验或实际故障，不能把通用效果验证入口当作正式制造闭环完成。制造整单、六类持久任务动作、准入和两个泛用单据入口的具体接续见第十七章；六access入口必须继续核对真实RBAC、当前仓库、服务端权益和文件事实，新增写入口不自动获得安全验收资格。

## 十六 命令原结果和丢失响应逐动作规格

本章将“保存后断线、重登、查询原结果”细化为可领取任务。它适用于制造和贸易的共同可靠性边界；已提交的业务动作由原业务事务证明，客户端超时属于客户端未拿到响应。查不到原编号时返回缺失，不能创建失败或待核对占位，不能替用户重新领料、出库或增加库存。

### 十六A 具体功能、代码位置和剩余任务

下表的“现”表示当前接线位置，“规”表示后续落点。当前兼容命令入口保留旧请求字段，不表示所有对象、角色、仓库和业务结果已经具备统一可信来源；免费一个账号、多端在线也不免除当前授权检查。

| 任务 | 具体行为与字段规则 | 实现位置 | 验收与交付方向 |
| --- | --- | --- | --- |
| CMD-R01 客户端状态观察 | `reported_status=UNKNOWN`或与服务器当前状态一致，仅返回原回执；试图变成RUNNING/SUCCEEDED/FAILED/RECONCILING时409。客户端`error_text/recoverable`不覆盖原错误、状态或完成时间。原编号不存在404，不抢占后续真实动作身份 | 现 `API/manufacturing.rs::advance_command_route` → 现 `REPO/shared_contract.rs::advance_command` → 现 `REPO/shared_contract/command_readback.rs::observe_command_status` | 对ACCEPTED真实回执上报四种新状态均409；UNKNOWN和相同状态200/replay=true。另一连接核对回执、对象、库存、流水、日志、Outbox全部列不变 |
| CMD-R02 原效果与对象分开读取 | `effect_id`取原`command_receipts.effect_id`，`object_id`取原对象信息；真实领料通过原move_id返回所属work_order_id。没有泛用command_objects的真实业务也能读取。缺失对象版本时兼容`version_no=0`表示未知，不取当前工单版本冒充提交版本 | 现 `API/manufacturing.rs::{read_command_route,read_restarted_command_route}` → 现 `REPO/shared_contract/command_readback.rs::{read_command_result,stored_receipt}`；现 `mfg_material_moves`的企业+move_id键 | 实际领料0.1后GET返回同一move_id和真实工单；库存以后变化也不改原效果ID。普通读取和restarts入口均只读；restarted=true只是兼容请求字段，不证明真的杀进程或恢复了桌面 |
| CMD-R03 丢失响应回查 | 原企业+原request_id+原payload_hash匹配；同一actor的原session必须REVOKED，新的session必须ACTIVE、未过期、凭证代次当前、对应用户ACTIVE。`client_unknown`真假都不执行；兼容warehouse_id/sku_id/qty_units仅为旧形态，不作为实际来源、授权仓库或效果依据 | 现 `API/manufacturing.rs::lookup_lost_command_route` → 现 `REPO/shared_contract/command_readback.rs::lookup_lost_command_readonly`；现 `browser_sessions/credential_epochs/users` | Accepted回查仍Accepted；已完成实际领料返回原Succeeded/replay=true；改数量至i64最大值、换兼容库存ID也不写库存。过期、旧代次、其他actor会话403；改原指纹409 |
| CMD-R04 历史泛用完成待核对 | 历史泛用command_objects曾允许客户端造终态，或由COMMAND_LOOKUP新建库存。没有可靠原业务证明的泛用终态读回409/command_reconciliation_required；不能读时自动修状态、删流水、补效果或认同成功。新泛用受理保留对象引用但effect_id为空 | 现 `REPO/shared_contract/command_readback.rs::stored_receipt`；现 `REPO/shared_contract.rs::open_command`；现 `API/manufacturing/command_readback.rs::error_response` | 自有QA中明确构造旧错误形态；GET、UNKNOWN观察和丢失回查均409且完全不变。存量核对和修复另领数据任务，不能以首次请求时自动猜测代替 |
| CMD-R05 全域权威来源与原快照 | 后续为每个实际业务动作冻结actor、对象类型/ID、动作、仓库范围、完整语义负载、提交版本及原结果。先证明来源，再开放通用读回；原OpenCommand不能用任意客户端object_id制造真实业务受理。终态、原明细和业务效果同一事务提交 | 现 `REPO/inventory_movement_commands.rs::{execute,find,verified_snapshot}`可参考；规 `REPO/shared_contract/command_readback.rs`逐域验证；规对应动作原结果迁移与服务 | 正常账号、跨企业、同企业另一使用者、撤仓库权、原对象删除、同ID跨动作、半提交/损坏结果分别验。当前材料原ID可读不表示全域权限/原快照统一完成；不可用的来源明确拒绝 |
| CMD-R06 双端保存恢复 | Rust共享页面保存前生成稳定原ID；传输丢失保留原ID、原负载和草稿，当前结果显示未收到响应，主动回查原动作；不能生成新ID或在客户端把超时判成业务失败。注销/改密保留已提交结果并重新按当前身份查询 | 规 `HOST/command_recovery.rs`与各正式业务宿主；复用现 `HOST`的native/Web适配及DataGrid；规真实类型业务回查接线 | 桌面与Web各做真实登录、保存、丢响应、重登、恢复草稿、查原结果及明确失败；实际关闭/重开客户端和服务端提交断线独立留证。HTTP/restarts=true不代替实际进程恢复 |
| CMD-R07 原命令列表与详情 | 正式任务/命令列表按授权全集筛选分页，默认500行、最高2000候选；稳定RowId为企业+原编号，状态、对象、原效果、错误、时间独立字段。无真实源版本就显示未知；翻页/查询取消不撤销已受理写入 | 规 `GET /v1/tenants/{tenant_id}/commands`查询，现同路径POST保持原受理操作；规 `HOST/command_recovery.rs`列表宿主；现自研DataGrid及工作台 | 10万/100万原命令下真实分页；新查询取消旧读，旧结果不能覆盖；批量动作不能基于旧结果。无真实列表API时不注册正式空菜单；通用网格不反向依赖业务 |
| CMD-R08 契约、回归与负载证据 | 明确200原读回、400非法状态、403会话范围、404缺失、409指纹/客户端迁移/待核对。普通读取、状态观察、丢失回查三种请求都不能写任何业务效果；读事务使用一致快照和READ ONLY，不拿库存写锁 | 现 `openapi/src/{paths,components}.yaml`及生成bundle；现 `crates/erp-api/tests/free_erp_acceptance/command_readback.rs`；现 `qa/acceptance/free-erp-command-readback-{capture,verify}.py`；规 `qa/performance/free-erp-command-readback.json` | 实際HTTP与另一PG连接核对，登记目标经统一入口运行；源码SHA、产物、数据库身份、原始响应、分位数与内存分别交付。共享父文件并行变化时给出变化清单，不能宣称全部当前版本已验证 |

只读实现固定位置为 `REPO/shared_contract/command_readback.rs::begin_read`：REPEATABLE READ + READ ONLY。状态观察和回查没有库存/报工/往来执行分支，也不等待库存行的FOR UPDATE。正式业务执行继续走各自原事务，不能调用兼容状态观察来“补完成”。当前保留旧LostCommandLookupRequest七字段的理由是兼容；后续简化字段或增加来源绑定要更新API契约和客户端，不能悄悄赋予这些旧数量字段写入含义。

### 十六B 十五个具体验收样本

| 样本 | 输入或准备 | 期望HTTP/原事实 |
| --- | --- | --- |
| CMD-U01 Accepted丢响应 | 真实工单引用受理；原会话撤销、新会话有效；兼容qty_units=3，client_unknown=false | 200/ACCEPTED；零新增COMMAND_LOOKUP库存流水，命令、工单、余额不变 |
| CMD-U02 客户端成功 | 已受理后reported_status=SUCCEEDED | 409/command_client_transition_denied；回执仍Accepted，完成时间未填 |
| CMD-U03 实际领料GET | 实际ISSUE 0.1成功，无泛用command_objects | 200/SUCCEEDED；effect_id等于实际move_id，object_id等于实际work_order_id |
| CMD-U04 实际领料丢响应 | 与U03同类真实效果，撤旧会话后查询原hash | 200/SUCCEEDED/replay=true；同一原move_id，库存不变 |
| CMD-U05 同状态和Unknown | Accepted上报UNKNOWN与ACCEPTED，同时伪造error_text/recoverable | 200原Accepted；原错误/可恢复状态不被覆盖，effect_id为空 |
| CMD-U06 四种客户端迁移 | Accepted分别上报Running/Succeeded/Failed/Reconciling | 四种均409，不改变任一业务事实 |
| CMD-U07 不存在原编号 | GET、UNKNOWN观察、丢失回查同一缺失编号 | 三种均404；command_receipts/command_objects不新增占位 |
| CMD-U08 实际结果观察与重启字段 | 实际领料后上报UNKNOWN和SUCCEEDED；请求restarts?restarted=true | 返回原Succeeded、真实原效果；未知源版本保持0，回查不执行。实际杀进程恢复仍另验 |
| CMD-U09 原指纹冲突 | 实际领料原ID上报另一个payload_hash | 状态观察与回查409；原效果保持 |
| CMD-U10 不可信库存字段 | 实际原结果回查，兼容warehouse_id/sku_id改成随机ID，qty_units=i64最大值；unknown真假各一次 | 返回同一原结果；这些字段不能创建任何库存效果，也不能代替后续R05的真正来源权限 |
| CMD-U11 过期新会话 | ACTIVE状态但expires_at已过 | 403，零业务和回执变化 |
| CMD-U12 旧凭证代次 | 新会话的credential_epoch不等于当前代次 | 403，零变化 |
| CMD-U13 其他使用者会话 | 请求认证actor正确，但新会话actor属于另一人 | 403，零变化 |
| CMD-U14 旧错误泛用终态 | 只在自有QA明确构造旧客户端成功形态，缺权威原效果 | 三个读/观察入口409待核对；不更正、不删或补写原数据 |
| CMD-U15 持有库存写锁时20路回查 | 真实领料原结果；另一个实际PG事务锁住对应余额，20路回查同时运行 | 全部返回原效果且不等库存锁释放；20次仍一笔原领料、零新效果。功能超时界限15秒用于发现阻塞，不是发布P99成绩 |

每组实际结果须比对原回执、对象、会话、余额、精确/旧投影流水、领料/报工/收货、共享库存、质量、来源预占、操作/审计、Outbox等选定事实的全部列。私有QA捕获当前60表并保护所有既有企业；测试使用合成签名与真实QA用户，不等于普通密码登录、完整会话中间件、公网托管、双端页面或发布压测通过。

### 十六C 八组性能和资源任务

沿用发布候选基线：API/Worker合计4专用vCPU/8GiB，PG独立4vCPU/16GiB/100GiB SSD；网络20Mbps下载/5Mbps上传/80ms RTT。100个企业，每企业一个实际登录使用者、两个有效端会话，配100万命令和实际效果关联；最大单企业10万，另测极端100万。正常结果、Accepted、实际失败、缺失、冲突、待核对分组报告，不能用快速拒绝稀释真实原结果的分位数。以下均为Release待测目标，debug测试总秒数不是性能成绩。

| 任务/现有门禁 | 负载、计时与资源 | 量化目标 | 具体执行和证据位置 |
| --- | --- | --- | --- |
| CMD-P01 / ERP-P03 单笔原结果 | 100万命令，单企业10万；真实材料/库存/贸易结果分别至少5000请求。计时含鉴权、池等待、SQL、编码至最后字节；冷态与预热态分开 | 热态服务端P95≤500ms/P99≤1秒；一客户端请求返回单结果≤64KiB；业务SELECT≤4，BEGIN/SET/ROLLBACK另计；原结果错误0 | 现原读函数；规 `qa/performance/free-erp-command-readback.json`保存独立样本、SQL次数、查询计划和响应字节 |
| CMD-P02 / ERP-P02 列表 | 10万与100万全集真实筛选/排序；500行与2000行分开测，禁止全量装载后切片 | 500行P95≤1秒/P99≤2秒；2000候选P95≤2秒/P99≤4秒且JSON≤8MiB；未达标不开放2000正式页 | 规R07查询/宿主及同负载报告；列表尚未实现，不把现单笔GET算列表通过 |
| CMD-P03 / ERP-P05 写锁隔离 | 20路同原ID回查，另一个实际事务持对应库存锁10秒，至少100轮；客户端真假unknown分组 | 返回原结果不等待库存锁；P99≤3秒，错状态/重复效果/死锁0；另测服务端真的未提交时读到原已提交版本 | 现只读内核；规Release竞争负载，保留实际锁会话、开始/提交时刻及结果SHA |
| CMD-P04 / ERP-P04 观察和回查 | 三类入口各5000次；真实终态、Accepted、缺失及冲突分开 | 原读/观察P95≤500ms/P99≤1秒；三个入口每请求业务INSERT/UPDATE/DELETE=0；新回执/新库存/新日志0 | 现观察/回查；规SQL事件与前后全事实比较；HTTP200不能代替零效果证明 |
| CMD-P05 / ERP-P07 会话变化 | 真登录、撤销、改密、过期和代次更换，各100轮；变更commit后立即请求 | 新请求立即按当前会话拒绝；不得重复已经完成的业务；异常/跨企业泄漏0 | 现会话中间件与本章回查条件；规正常登录/设备与Session负载报告，合成JWT测试独立标注 |
| CMD-P06 / ERP-P02 多企业读 | 100企业×2端，50RPS固定到达率持续10分钟，70%GET/20%lost/10%观察；每类保留分位数；记录延迟请求而不漏样 | 正常原读P95≤500ms/P99≤1秒；500/超时≤0.1%，业务错误和重复效果0；队列有界，过载明确拒绝 | 规公共压测任务；API峰值RSS≤2GiB，Worker另计，连接池上限和排队深度写报告 |
| CMD-P07 / ERP-P02 有界响应 | 原错误文本、来源字段及历史数据异常分别测，单笔/列表字节与峰值分开 | 单笔≤64KiB，列表≤8MiB；非法/超预算结果明确拒绝；不能靠静默截断变成“正确原结果” | 规原快照/错误长度边界及负载；当前兼容元数据尚须增加完整大小预算验收 |
| CMD-P08 / ERP-P03 双端恢复 | 桌面/Web各50次正常保存、实际丢响应、重登和原结果恢复，80ms RTT；真实显示到可操作绘制 | 收到原结果后P95≤100ms完成状态更新；端到端另列；保存身份和草稿丢失0、二次业务提交0、UI阻塞0 | 规R06正式宿主、两端文件/网络/生命周期适配；记录屏幕/设备、原ID、结果与服务器commit时刻 |

索引和往返不能只靠SQL名称推断：原回执使用企业+原编号主键；会话使用企业+session主键、代次使用企业+actor主键，实际领料使用企业+move_id主键；提供各负载实际EXPLAIN/扫描行数、数据库版本/池大小和完整源码/产物SHA。R05的真实来源、授权和原快照补齐后要重测，当前原型的快响应不构成完整可靠性或性能结项。

### 十六D 当前证据与Grok接续

原四个反例已实际复现：Accepted回查200后新增3个基本单位库存并变成Succeeded；客户端成功上报200后伪造终态；真实ISSUE 0.1的GET为404；真实原结果lost-lookups为422。原日志保留[实际反例](../qa/acceptance/free-erp-command-readback-original-test.log)，另一PG连接及既有企业60表保护见[反例独立复核](../qa/acceptance/free-erp-command-readback-original-independent.json)。该定向过滤另匹配一个既有原库存命令测试，其通过与这四个失败分开记录。

本批15项专项实际通过/0失败/0忽略，12.36秒；完整登记`free_erp_acceptance`目标347通过/0失败/0忽略/0筛除，264.96秒。专项[独立读回](../qa/acceptance/free-erp-command-readback-fixed-focused-verified-independent.json)34,479项核对、6,763个既有企业60表保护；完整[独立读回](../qa/acceptance/free-erp-command-readback-full-independent.json)35,303项核对、6,779个既有企业60表保护，同时独立核对本批15组；其他回归用例沿各自真实Rust断言。20路在库存锁仍持有时897ms全部返回，只是此功能样本，不能当Release P95/P99。

首次独立比较因QA连接用UTC、psql用+08:00而把同一时刻的文字差别误报为变化，原[失败报告](../qa/acceptance/free-erp-command-readback-fixed-focused-independent.json)与脚本保留。复核只对实际PG目录确认的timestamptz列转成精确UTC时刻，其他全部字段、内嵌负载和原始文件不改；校正后上述两份独立读回通过。原时间目录、原/修后日志、60表捕获SQL、源码与报告均分别绑定SHA。

当前OpenAPI与本批实际57份响应（40成功、17拒绝）、51份写请求匹配，见[专项契约](../qa/acceptance/free-erp-command-readback-fixed-focused-openapi.json)和[完整契约](../qa/acceptance/free-erp-command-readback-full-openapi.json)；项目v2门禁通过。三个实现/测试叶和两个domain叶格式检查通过，修改后的domain状态决策用例实际1通过；严格Clippy仓储lib 240/lib test 242项失败，API被仓储及JD/Taobao/PDD/Douyin合计27项依赖诊断阻断，不宣称整个lint完成。

[源码资格](../qa/acceptance/free-erp-command-readback-source-audit.json)显示原反例/专项/完整窗口分别4/6/5个输入并行变化；三个本批叶在专项和完整窗口不变。共享父接线、巨型shared_contract测试及OpenAPI继续有并行变化，整份当前代码保持PARTIAL；`shared_contract`另一个登记测试目标没有由本批运行，不能由347项替它结项。正常密码/真实会话、全域角色/来源/仓库、原快照、正式桌面/Web与官方托管仍未完成，八组Release指标和峰值资源保持NOT_RUN。

文档固定快照的100功能/40实施单元/28宿主/14业务剧本/22性能任务，以及本章8/15/8结构、96个源码文件与27个辅助HTTP操作核对通过，见[固定快照文档核对](../qa/acceptance/free-erp-command-readback-docs-fifth.json)。持续新增接口另纳入后续盘点；本次最终[交付时文档检查](../qa/acceptance/free-erp-command-readback-docs-delivery.json)保持本章结构与7项源事实通过，但报告`/documents`及`/documents/{document_id}`两条后来新增泛用路由尚未分类，不能称整个不断变化的路由文档已验证完成。

Grok后续优先CMD-R05真实来源/当前授权/原快照、CMD-R06真实双端恢复、CMD-R07正式列表和CMD-R08/本章八组Release负载。`commit_atomic_effect`的客户端来源/金额与整数投料准入、真实终态失败证明，以及其他域的原结果完整性继续分别审计，不能因本章只读修复替它们结项；旧测试如预期客户端任意终态或回查新入账，须按原业务效果重新构造验收，不恢复不可信的旧行为。

## 十七 制造整单任务执行和多端单据开发细目

本章进一步拆开ERP-IMPL-04/28/29/34/39，对应A03/M09/M11/M12/O01至O04/O08。贸易公司的采购、库存、销售同样复用原命令、版本冲突和后台任务能力；贸易业务无需建立制造BOM。下列20个任务、16组样本和8组性能负载是待开发及待验要求，没有由第十六章347项回归验证。`ATX`表示制造整单任务，`TASK`表示持久任务，`DOC`表示多端单据；这些编号细化原功能，不新增收费限制。

### 当前接线与必须补齐的事实

2026-10-04原反例复核：旧`commit_atomic_effect_route`接旧`commit_atomic_effect`，实际接受关闭工单、伪造999999分金额、未授权仓库及未读必检的四个请求，均返回200并生成库存/领料效果。原HTTP和独立数据库证据保留于[原问题复核](../qa/acceptance/free-erp-atomic-effects-original-independent.json)，不能把旧事务存在当作来源可信。

当前单行入口已移至现 `API/manufacturing/atomic_effects.rs::commit_atomic_effect_route`，调用现 `REPO/shared_contract/atomic_effects.rs::commit_atomic_effect_with_actor`；父模块只登记路由，原仓储方法保留内部SYSTEM兼容委托。HTTP仍接收`request_id/warehouse_id/sku_id/source_action/action_no/qty/amount_cent/prepared_lines/failing_line`，但动作仅ISSUE，prepared_lines必须1，failing_line必须false；成功明确`applied_lines=1`，不声称整单完成。服务端核对认证用户ACTIVE及当前仓库权限、真实RELEASED工单、冻结必检、工单绑定版本/真实组件及需求余量。复用现 `REPO/shared_contract/material_moves.rs::append_material_move_with_tx`完成精确库存和原工单预占消费；读取当前SKU.cost_cent计算实际精确声明值，缺成本409待核对，合法零成本保留。

精确价值以`qty_micros × unit_cost_cent`存现`atomic_material_values.value_cent_micros`的NUMERIC(38,0)，百万个该单位等于1分；`amount_cent`仅按半分向上舍入后的整数分投影，并与请求声明比较。原材料数量/余额、原库存余额、实际物理/材料效果ID、认证提交人、成本/版本证明以及两个日志/Outbox/待处理事件ID写入不可变原快照。现迁移`migrations/0295_atomic_material_sources.sql`约束新ISSUE效果/回执共同提交，旧未知行不猜补；重放核对真实原效果、日志与事件，返回原快照，不用当前价格、库存或工单状态重算。当前SKU成本声明与该精确价值事实不等于原库存成本层或车间/在制价值总账已完成；原子待处理事件不表示供应商应付或财务过账成功。完整100行材料单、批次、多层及下达时冻结需求、退耗原价值转移、普通密码/角色/会话、真实Worker消费和双端页面仍继续按下表开发，整体PARTIAL，Release性能NOT_RUN。

单行专项实际22通过/0失败/0忽略，20.60秒；[独立读回](../qa/acceptance/free-erp-atomic-effects-repaired-third-independent.json)37,344项核对，7,218个既有企业65表全部列保护，55次实际原子HTTP请求。0.250000 × 1200分的精确声明值300000000、0.000001 × 1200分的精确值1200（整数分投影0）、合法零成本、未知/负成本、真实BOM缺失/超量/错误组件、撤权重放、原预占消费、20路真实PG并发以及五个必需INSERT故障均在该专项内；重试返回原成本/余额快照，不重新入账。[当前成功契约](../qa/acceptance/free-erp-atomic-effects-repaired-fourth-contract.json)逐份核对36成功及19拒绝，源/生成OpenAPI一致、v2门禁通过。debug专项总时间不是Release P95/P99。

首轮API模块可见性编译失败、次轮20通过/2失败及两次契约/文档校验失败均保留。次轮错误为测试预期遗漏夹具真实HOLD库存，以及403已有结构化message字段，修正测试读回后重跑；不能删除日志或把此前失败称为通过。0295仅在127.0.0.1:15438保护QA试运行和应用，1,204份旧原子单据原列保持一致；完整迁移链与生产库未执行。专项输入窗口的4个文件变化（含并行认证/仓储/另登记测试和生成OpenAPI）另核对，本批实现/专项叶保持一致；整体当前源码资格与完整回归、严格Clippy、正式客户端及发布门禁分别验收。

最新完整登记回归实际316通过/53失败/0忽略/0筛除，303.24秒；22个本批单行用例继续全部通过。失败主要是旧库存夹具缺真实session_id/数据库角色，而当前库存命令已要求真实会话与`WMS.INVENTORY.WRITE`，出现实际400“nil command identity”；继续补真实密码登录、角色和会话接线，不放宽守卫换通过。[完整独立读回](../qa/acceptance/free-erp-atomic-effects-full-independent.json)38,185项核对、7,244个既有企业65表全部列保护，单行事实PASS、整体回归FAILED，报告保留PARTIAL及全部53个失败名称。首次完整后读回连接拒绝及空文件保留，原QA随后恢复，重试确认原数据库/角色/127.0.0.1:15438身份后完成快照；没有重建数据库或清理原资料。

严格Clippy仓储lib241/lib test243项失败，API受仓储与四平台适配合计27项诊断阻断；本批atomic新叶没有诊断，材料历史查询已有复杂类型仍待迁移。完整测试窗口6个输入并行变化（认证上下文、库存视图、履约上下文和3个履约测试），当前整体源码仍PARTIAL；源码跟踪、完整回归、普通登录/权限及Release/客户端/正式托管全部门禁没有结项。完整归档与接续见[本批报告](../qa/acceptance/free-erp-atomic-effects-repair.json)。

现 `REPO/shared_contract/material_moves.rs::post_material_move` 已有原工单锁、企业内仓库/SKU、精确库存、原领料余量、质量证明及材料原结果。接续应抽出同一事务内核供现入口与整单入口使用；不要先逐行调用已自行commit的函数，再在外层保存“整单成功”。`source_action`字符串不能替代实际业务动作，`failing_line=true`在handler提前返回也不能证明库存写入后的真实回滚。

现 `API/manufacturing.rs` 的持久任务handler仍调用现 `REPO/shared_contract.rs` 的同名方法。`migrations/0134_durable_tasks.sql`区分task/event/effect及`durable_task_letters`；已有唯一键只证明不会插入同一个文字身份，仍须核对真实事件和业务效果。当前租约owner、失败类别和取消posted等来自请求；正式Worker身份、有效租约代次、已提交效果及实际失败必须由服务端事实确定。

现泛用单据由 `API/manufacturing.rs::{open_versioned_document_route,save_versioned_document_route}` 接 `REPO/shared_contract.rs::{open_versioned_document,save_versioned_document}`，现 `migrations/0135_document_versions.sql`含`versioned_documents/version_conflicts/query_generations`。当前正文为`body_text`，保存冲突分支仍返回HTTP 200与`status=SUCCEEDED/conflict=true`。这一现状不能冒称采购、销售或制造单据已保存；正式业务需合法字段、当前范围及原版本/原冲突结果。外层需要兼容200时，至少明确业务`CONFLICT`，UI不得显示保存成功。

### 制造整单的8个实施任务

| 任务 | 具体功能和输入 | 现入口及规划位置 | 提交规则与验收方向 |
| --- | --- | --- | --- |
| ATX-01 真实原单与明细 | 原材料单ID、工单ID/版本、稳定明细ID、动作、仓库/SKU/基本单位、六位数量；退耗引用原领料，补料有原因 | 现 `API/manufacturing/atomic_effects.rs::commit_atomic_effect_route`、现 `REPO/shared_contract/atomic_effects.rs::commit_atomic_effect_with_actor`只接单行ISSUE；规 `API/manufacturing/material_documents.rs`；规 `DOMAIN/manufacturing/material_document.rs` | 扩展现契约或由它委托正式材料服务，文档/OpenAPI同时标兼容字段与新字段。先读本企业真实材料单，逐行绑定真实工单材料；未知、跨企业、任意动作拒绝。不能创建一张自报“来源”后把它当既有证明 |
| ATX-02 真实认证与仓库范围 | 认证账号/会话、当前角色、当前授权仓库；作业员工是单独业务字段 | 现 `API/auth_layers.rs`、`REPO/data_scopes.rs`，现原handler的`AuthContext`；规材料服务上下文 | 新提交及回查均复核当前权限，禁用账号或撤仓403。身份不由请求owner/员工替代。免费只有一个可登录账号，多端仍有不同会话和原命令 |
| ATX-03 工单状态及冻结需求 | 真工单、提交版本、冻结BOM材料行/单位/批次、已领/返/耗余量、来源预占与质量 | 现 `REPO/shared_contract/material_moves.rs::post_material_move`；现 `REPO/shared_contract.rs::inspection_blocks`；规 `REPO/shared_contract/material_documents.rs::validate_locked_lines` | 新领/补料只允许合法下达工单，暂停是否只退按已有规则；已关闭/取消拒绝。不得用本企业存在任意SKU代替属于该工单的材料。所有行在取得来源锁后核对，预占兑现与实际持料分别记录 |
| ATX-04 六位库存和原来源 | 支持0.000001，批次/库位启用时真实绑定；ISSUE/SUPPLEMENT、RETURN、CONSUME分别实现 | 现 `REPO/inventory_precision.rs`、`REPO/shared_contract/material_moves.rs`；规同叶的`post_material_move_with_tx`供统一内核复用；完整批次沿第十一章 | ISSUE减原仓并加车间持料，CONSUME减持料形成耗用，RETURN按原尚未耗退量返仓。整数投影沿既有精度契约，不能静默截断或再扣原仓；按SKU/仓库/批次分别核对，不能只看工单总量 |
| ATX-05 实际材料价值 | 原库存成本来源ID/版本、精确数量、计价算法/舍入版本、实际转移或耗用金额 | 现 `REPO/shared_contract/atomic_effects.rs`锁读SKU.cost_cent并存精确声明值；规 `REPO/shared_contract/material_documents.rs::value_locked_lines`接原库存成本层/价值转移；接ERP-IMPL-34 | 金额由原成本资料计算。缺原成本明确待核对，合法零成本与未知分开。领料转持料价值，耗用转在制价值，退料沿原计价返仓；ISSUE不自动形成供应商应付。客户端amount_cent只能是待比较的声明，不能直接入账 |
| ATX-06 一次真实提交 | 整单默认最多100行，单行原因≤4096 UTF-8字节，整张请求≤8MiB；每行独立来源及效果ID | 现 `REPO/shared_contract.rs::commit_atomic_effect`保留入口委托；规 `REPO/shared_contract/material_documents.rs::{prepare,commit_with_tx}` | 命令身份→工单/来源→稳定顺序库存/批次→价值。业务单/明细、精确流水、材料动作/余额、价值转移、必需两个日志、必要Outbox、原回执一次commit；任一真实后行/日志/事件失败全回滚。20行/100行分别测，不以prepared_lines代替实际行数 |
| ATX-07 原结果与重复业务动作 | 命令完整指纹、原单/明细及效果ID、原数量/价值/余额、提交版本和原时间 | 现 `command_receipts/atomic_source_documents/atomic_posting_events`；规原表绑定迁移与不可变材料结果；接第十六章只读回查 | 同原ID同语义返回原结果，改任一关键字段409；同真实原材料单重复提交即使换request_id，也不再扣库。旧缺来源/原快照记录返回待核对，不用当前余额补造原结果；具体动作与通用命令共享身份空间 |
| ATX-08 正式录单与清单 | 主表材料单，子表明细，右侧字段；保存草稿、整单确认、原效果查看和失败定位 | 规 `UI/pages/manufacturing/materials.rs`；现 `UI/page_registry.rs`、`UI/http.rs`及Web适配、自研DataGrid；宿主见PAGE-19 | 连续键盘录入/粘贴预检、非法行留在草稿；确认后按真实回执更新。响应丢失只回查原ID，查询取消/切页不能重发写入。桌面/Web分别用真实登录执行领、耗、退及失败恢复，不以GridLab或通用文本单据结项 |

ATX的数据迁移规划：复用现`atomic_source_documents`时需增加明确业务类型、真实工单/版本、认证提交人和可验证状态；原明细、各行物理键/单位/来源、精确数量/金额及效果/日志ID放对应明细或原结果表。新增文件命名按迁移登记分配，不在本文预占可与并行开发冲突的数字。旧行缺这些事实不得批量猜补；数据库约束按企业+真实业务动作唯一，并保留企业+request_id的共享命令唯一。相同`action_no`但不同真实对象是否同意图要按公开规则确定，不能只锁自由文本。

ATX-04/05固定业务样本：原仓1.000000 kg，原价值1200分；领0.250000后原仓0.750000/900分、车间0.250000/300分、实际耗用0。再耗0.125000后原仓不变、车间0.125000/150分、材料在制价值150分；退剩余0.125000后原仓0.875000/1050分、车间0/0分、有效耗用0.125000/150分。实物与价值各自守恒；后续人工/制造费用另计。这是待执行预期，当前单行ISSUE价值事实或cost摘要返回200不算该完整样本通过。

### 持久任务的8个实施任务

| 任务 | 具体功能和输入 | 现入口及规划位置 | 提交规则与验收方向 |
| --- | --- | --- | --- |
| TASK-01 真事件受理 | 任务类型、原事件ID、原业务命令/效果ID、受限执行参数、创建人及授权快照 | 现 `API/manufacturing.rs::enqueue_durable_task_route` →现 `REPO/shared_contract.rs::enqueue_durable_task`；规 `REPO/shared_contract/durable_task_service.rs::enqueue_from_event` | 核对本企业真实Outbox/业务事件和对象；任务唯一依实际事件/执行类别，不凭客户端任意effect_id建任务。受理只表示QUEUED，不表示MRP、导出或库存动作完成；不存在/外企业来源零任务零回执 |
| TASK-02 真实Worker领取 | 执行进程身份、task_id、attempt_id/租约代次、到期、服务端规定的租约时长 | 现 `claim_durable_task`；规 `WORKER/jobs/durable_tasks.rs::claim`接现 `WORKER/runtime.rs`和`WORKER/jobs/mod.rs` | 普通业务账号不凭owner字符串冒充Worker。仅到期且可运行任务可领，事务内领取/增加代次；同代只一个有效执行者。初始候选租约30秒、心跳10秒；最终配置写报告，重领不改变原业务命令 |
| TASK-03 有界心跳及状态读取 | 当前attempt_id、服务端执行阶段、已处理/总条数、心跳和到期 | 现 `heartbeat_durable_task`，规同Worker叶的`heartbeat`和任务查询服务 | 条件更新必须同时匹配执行身份、代次、未到期和可执行状态；旧Worker即使同owner也不能续新租约。progress只描述工作量，100%不证明已提交；不允许跨阶段倒退或客户端设置SUCCEEDED |
| TASK-04 域效果和任务共同完成 | 真业务效果/文件回执/计算结果、原命令、该attempt、实际提交时间 | 规 `REPO/shared_contract/durable_task_service.rs::complete_with_effect`；复用各域with_tx服务；导出沿现 `WORKER/jobs/inventory_export.rs` | 提交前复核租约代次及实际来源；同事务保存该域全部效果与任务完成/原结果，外部文件按第38单元发布协议。数据库已提交但响应丢失后只读回原效果；新Worker不重算或重扣已经完成的命令 |
| TASK-05 真失败与有限重试 | 服务端失败类别/代码、阶段、attempt、是否已提交、下次执行时刻及重试预算 | 现 `fail_durable_task`、`durable_task_letters`；规同服务的`record_failure`和Worker错误分类 | 不信任请求known/transient。已确定暂时性且未提交的失败可退避；未知提交结果进入核对，不能重新发原效果。候选最多5次执行（含首次）、1/2/4/8秒退避，实际配置/version写回；保留每次失败历史，超额DEAD及一条有效死信 |
| TASK-06 取消与真实冲正 | 原任务、取消人、取消意图、实际阶段/已提交效果、按原来源可冲正量 | 现 `request_task_cancel/apply_cancel_checkpoint/reverse_posted_task_issue`及原cancel handler；规同服务的`request_cancel/checkpoint/prepare_reversal` | 前提交取消释放本任务资源；和commit竞争按锁内真实事实决定。已提交保持原SUCCEEDED及效果，另外生成有来源的冲正意图/单据；客户端posted/work_order/qty不决定反向库存。部分已耗/退按原剩余量处理，权限/失败不能留下“已取消但业务已提交”假事实 |
| TASK-07 死信检查及恢复 | 原DEAD任务/死信、真实失败、当前权限、原效果是否已提交、人工恢复原因 | 现 `restore_dead_task`，规同服务的`restore_after_reconciliation` | 已有原成功效果只回查或修复任务投影，不重复执行；无效果且可恢复才新attempt，保留原失败与恢复事件。不能清空error_text充当问题解决；重复恢复不会建立第二个任务或第二份效果 |
| TASK-08 公平准入与任务中心 | 真实队列、租约、并发、任务类型/企业、最老等待时长、拒绝/取消计数 | 现 `admit_tenant_work/read_work_admission/read_durable_task_page`、`work_admissions`，现`GET /tasks`；规 `WORKER/jobs/admission.rs`、`UI/pages/operations/tasks.rs` | 以实际受理/运行/完成释放配额。候选单企业MRP/导出各1个运行，全局重任务2个；重任务每企业排队≤10、全局≤1000，超界429含服务端Retry-After。轮转企业且企业内FIFO，预留普通查询/写入容量；原admission记录不能永远占用已结束任务名额。任务列表授权全集与当前范围先行，计数/页同一快照；未知版本不能硬填1 |

TASK-02至07属于执行服务：公网业务用户只拥有被授权的受理、查询、取消意图和人工恢复权限，执行身份需真实内部凭据及当前权限校验。已有公开URL如何迁移或限制必须写OpenAPI和发布兼容说明，不能只新建安全Worker内核却留下原未受控写入口。任务GET列表已有原入口，当前精确COUNT与分页分别从pool读、count_exact=false也执行COUNT；同快照/授权/计数策略及正式详情继续补齐，六个动作POST和这个列表不冒称已提供完整任务中心。

### 多端单据的4个实施任务

| 任务 | 具体功能和输入 | 现入口及规划位置 | 提交规则与验收方向 |
| --- | --- | --- | --- |
| DOC-01 真实业务类型和正文 | 原对象类型/ID、字段schema版本、合法明细、正文≤64KiB；自由文本备注与业务字段分开 | 现两document handler及`versioned_documents`；规 `API/documents.rs`、`REPO/document_versions.rs`，具体域表与服务沿ERP-IMPL-12至35 | 泛用body_text只代表已保存文本。正式采购、销售、BOM、工单委托原领域校验及当前权限，不能靠JSON字符串绕过数量/来源/财务约束；类型/非法字段/跨企业拒绝，无占位业务记录 |
| DOC-02 原版本、冲突和重试 | expected_version、原命令、实际操作者、当前版本、原提交版本、保留输入/冲突ID | 现 `REPO/shared_contract.rs::save_versioned_document`与`version_conflicts`；规版本服务的`save/verified_result` | 两端同版本竞争恰有一次更新，另一端业务CONFLICT并保留完整草稿。冲突回执绑定原输入/原存储版本，重试不返回后来版本；合法已提交请求重试也返回原版本。数据库提交失败正文/版本/回执/必要日志全回滚 |
| DOC-03 草稿与查询隔离 | 企业/账号/页面实例/对象、基线版本、当前输入、原命令、未决保存；查询有独立代次 | 现 `UI/draft_store.rs`、`UI/leave_guard.rs`、`UI/action_store.rs`和`UI/query.rs`；规对应正式宿主适配 | 页面切换保留草稿，关闭脏页面按现保护规则；只读任务取消不取消已受理写命令。冲突不给成功提示，不静默覆盖、清空或用新ID重复提交；恢复后先回查原结果再确定草稿基线 |
| DOC-04 正常账号双端验收 | 同一免费账号，桌面与Web同时登录；同对象版本竞争、断响应/重启、撤权、非法输入 | 现Native/Web认证和网络适配；规各业务宿主的实际UAT，复用第六章PAGE对应位置 | 按真实界面完成读取→编辑→保存→冲突/原结果回查，分别留下客户端与PG原事实。模拟JWT、HTTP测试、query_generation/restarted客户端标记均不能代替真实登录、进程重启或业务客户端完成 |

DOC-02必须区分“保存请求已处理”和“新正文已提交”。当expected_version=1但原对象已是2，正式业务返回409及`document_version_conflict`、原conflict_id和保留输入；若旧兼容URL仍用200，则业务结果必须为CONFLICT并有原结果，旧客户端纳入升级验收。请求本身收到SUCCEEDED不能触发UI清草稿。本文不把现通用文本改造成所有单据的统一业务数据库。

### 16组可执行验收样本

规新增叶测试 `crates/erp-api/tests/free_erp_acceptance/atomic_effects.rs`、`durable_tasks.rs`、`document_versions.rs`，仅在现 `free_erp_acceptance` 登记后才能运行和计数。保护QA数据库身份、原有企业所有列、各次源码SHA与请求/响应；另一个psql连接读取已提交事实，不只调用同一仓储GET自行证明。

| 样本 | 具体操作 | 独立预期与必须保留的事实 |
| --- | --- | --- |
| DETAIL-U-01 小数领耗退 | 执行本章1kg/1200分样本，另补0.000001数量边界 | 三步数量与价值逐项符合正文；耗用不再减原仓，返仓只能用原领料剩余。台账、明细、工单材料及价值/回执一致 |
| DETAIL-U-02 伪造来源 | 不存在/外企业/非材料来源，客户端另填同名action_no或不同source_action | 404/409/422按公开契约拒绝；零库存、材料、价值、事件、日志和回执效果。不能通过新建atomic_source_documents补造原来源 |
| DETAIL-U-03 工单范围和状态 | 同企业无关SKU/仓、版本过期、PAUSED领料、CLOSED/CANCELLED，以及原请求重放 | 新不合法动作拒绝，实际原版本/库存不变；合法已提交原请求在当前授权下返回原结果，不能因为工单后来关闭再执行或返回当前余额 |
| DETAIL-U-04 金额与质量 | 篡改amount_cent、未知成本、合法零成本、完整未通过质检来源、other批次检验 | 客户端不能决定成本或质量；真实未知拒绝/待核对且不扣库，合法零按公开业务规则通过；别批次证明不放行本批。分别读原成本来源、质量证明和库存 |
| DETAIL-U-05 后行及日志失败 | 20/100行实际材料单，第最后行余量不足；真实事务内日志/Outbox插入注入失败 | 所有前行库存/材料/价值/单据/回执一起回滚；失败地点及PG事实留证。仅failing_line=true的提前400不计本样本 |
| DETAIL-U-06 20路提交竞争 | 20路同原ID/同内容，再用不同ID提交同真实材料单；改变任一SKU/批次/单位/数量/actor | 一次完整业务效果，同原ID其余返回原效果；换ID不重复原业务单。异语义409；每轮独立核对，不用20次顺序调用冒充并发 |
| DETAIL-U-07 实际丢失响应 | 真实commit完成后丢响应，客户端仍持原ID；另一动作先改变当前库存再恢复 | 恢复返回原提交版本/余额/金额/效果而非当前量，库存只执行一次；保留故障点、实际commit与另连读回，不以client_unknown证明已断线 |
| DETAIL-U-08 真任务来源与身份 | 伪event/effect、外企业事件、普通账号声称任意owner，合法真实事件受理 | 非法无任务/回执/业务效果，合法只一个QUEUED；普通用户不能执行Worker变更。真实事件及原效果与任务绑定 |
| DETAIL-U-09 租约竞争与过期代次 | 20路真实执行者领取；旧Worker租约到期新attempt领取后，旧attempt心跳/失败/提交 | 同时只有一份有效租约；旧attempt即使owner相同也拒绝，原业务效果一次。保存实际数据库时刻/代次，不能只改client progress |
| DETAIL-U-10 提交后Worker故障 | 在计算、commit前、commit后/任务响应前分别终止被测QAWorker，再重启恢复 | 未提交可重领，已提交只返回原效果；不存在数据库业务已完成但再次扣库/重建结果。真实进程PID/退出/重启事实分别留证 |
| DETAIL-U-11 取消与提交竞争 | 未运行取消、执行中到checkpoint取消、20路取消/commit竞争、部分已耗后的冲正 | 锁内实际阶段决定最终状态；已提交效果保持，冲正有原来源/剩余量/独立命令。伪posted/work_order/qty不反向入账；取消成功不能掩盖已提交 |
| DETAIL-U-12 失败、死信与恢复 | 暂时失败至5次、永久失败、未知提交、重复恢复、已有效果却DEAD的历史任务 | 有界退避与一份有效死信，全部尝试历史保留；未知先核对；已有原效果不重复执行，恢复事件/新attempt与原失败同时可查 |
| DETAIL-U-13 多租户公平与配额 | 100企业非空队列；一家连续提交重任务至10/1000边界，其他9家真实查询/写入 | 队列/全局2重任务边界不突破，结束/取消释放名额；轮转不饿死其他企业，普通接口继续满足ERP-P02/P04，429包含实际退避 |
| DETAIL-U-14 双端版本冲突 | 正常账号桌面/Web读版本1，分别保存A/B；A成功后B冲突，再保存C使当前版本3 | 只有一个版本2，B原草稿仍在；B原请求重试仍返回原冲突/原stored_version=2，A原请求返回原提交版本2，不混用当前3 |
| DETAIL-U-15 草稿恢复与撤权 | 查询条件连改/IME、保存未决时切页/关闭、重启、真实会话撤销/仓库撤权后回查 | 250ms最终条件防抖与旧查询隔离，草稿/原ID不丢；只读取消不重发写入；当前授权拒绝但原已提交业务事实保留 |
| DETAIL-U-16 大小与精度边界 | 100/101行、请求8MiB/超界、正文65536/65537字节、非UTF-8/坏JSON、i64计算溢出 | 每个边界有明确4xx/预算结果；请求体/文本按实际字节计，超界不留部分效果。不得panic/截断；64KiB正文不能冒称整张8MiB材料单额度 |

实际自动测试建议函数前缀分别为`atomic_effects::`、`durable_tasks::`、`document_versions::`，但函数名/叶文件只是规划，不能写成当前已登记数量。DETAIL-U-10/14/15还需真实进程和双端人工/自动客户端证据；API-only样本通过后这些门禁仍保留未完成。

### 8组专属性能负载与资源上限

固定环境、网络、100企业和计时算法沿第四章。每个动作/规模单独≥1000样本，竞争组100轮，后台完整任务至少30次；Release性能仅通过现有登记`free_erp_bom_performance`及统一构建入口，缺注册先登记，不新建target。所有百分位包含鉴权、连接池等待、锁等待、计算、SQL与编码；返回202只计受理，业务完成计至实际效果和原回执可查。

| 编号 | 实际负载、计时和接线 | 通过指标与资源 | 必须一起通过的正确性 |
| --- | --- | --- | --- |
| DETAIL-P-01 制造整单 | ATX-01至07；10万材料单/100万流水，20/100行分别真实提交，批次/来源/日志启用 | 20行P95≤1s/P99≤2s；100行P95≤2s/P99≤4s；整单请求≤8MiB，100行新增工作内存≤64MiB；API总峰值RSS≤2GiB | 每种材料动作和价值独立复算；后行/日志/事件失败0部分效果，超101行拒绝，不能测只写摘要的原接口 |
| DETAIL-P-02 竞争与原重放 | 同命令、同真实材料单不同命令、取消/commit分别20路×100轮，锁持有及等待有PG证明 | 成功/冲突完成P99≤3s；所有2000次计入，没有死锁残留；重放点查P95≤500ms/P99≤1s | 一次原业务效果、同ID完整原结果、改变负载409；不按失败/超时筛出最快20次 |
| DETAIL-P-03 原结果读取 | 100万原命令，20/100行结果；当前库存另有变化、当前范围收窄；服务器全请求到原响应交付 | 点查P95≤500ms/P99≤1s；100行结果≤1MiB；只取该原对象/有界关联，无全表扫描冒充小样本性能 | 不触碰库存写锁/重新入账，来源/日志/成本证明完整；不能用后来工单/余额凑原快照 |
| DETAIL-P-04 任务领取与心跳 | 100企业各10待执行任务，实际Worker按全局2重任务运行；正常/过期/旧attempt分组 | 受理/领取/心跳P95≤500ms/P99≤1s；DB领取按状态/可执行时刻/稳定task_id有界；一次候选≤100，心跳10s、租约30s为初始配置 | 一个有效attempt，进度100%不等同成功；旧代次不能提交。慢MRP/导出完整时间仍分别满足ERP-P11/P16 |
| DETAIL-P-05 队列与普通业务 | 10企业混合MRP/导出，1家持续满队列，其余9家按第四章50会话/20RPS运行30min，另测100RPS突发60s | 每企业每类运行≤1、全局重任务≤2、每企业队列≤10/全局≤1000；其余查询/写入仍满足ERP-P02/P04，API峰值≤2GiB | 结束/取消名额实际释放；任务无饥饿、普通写入不被重任务占满连接；记录拒绝、各企业等待和调度顺序 |
| DETAIL-P-06 取消与恢复 | 未运行/执行中取消和真实进程故障各30次，commit边界和租约过期实际计时 | 取消意图受理P95≤500ms；可取消阶段checkpoint≤2s；QA恢复就绪后过期租约≤45s被扫描发现。已提交冲正另按P04测 | 不把30s租约等待当业务失败或强制成功取消；不重复已提交效果，错误/恢复历史不删 |
| DETAIL-P-07 版本与草稿 | 10万对象，正文1/16/64KiB各1000请求；20路同版本100轮；双端100个页面实例恢复/切换 | 正常保存P95≤1s/P99≤2s；冲突点查P95≤500ms/P99≤1s；本地输入P95≤50ms，页切换按ERP-P08，正文上限64KiB | 原提交/原冲突版本不随当前变化；草稿隔离、未决原ID保留，UI不显示假保存成功，UTF-8大小边界明确 |
| DETAIL-P-08 证据与运行资源 | 全组记录API/Worker/PG产物与输入SHA、真实CPU/存储/网络、池/队列配置、RSS/连接/临时文件和各次时长 | 同普通组非预期错误率<0.1%，不变量错误0；超时/429/409分别计数；内存1s采样并保留开始/结束与峰值，估算不作实测 | 冻结对应测量输入；若并行源码改变，旧测量只绑定旧SHA。独立PG逐效果/原已有企业保护、文件SHA及真实双端证据齐全才结项 |

上述数值均为待测目标，当前状态**NOT_RUN**。Release负载实现前先盘点现SQL与索引：制造查询用企业+工单/原单及稳定行ID，原效果用企业+原命令；任务待领用状态+可执行时刻+task_id并确保跨企业轮转，原事件/效果建立企业范围唯一；冲突用企业+原对象/原命令。实际EXPLAIN、扫描行数、临时文件、锁等待和池等待一起报告；仅创建索引或debug总秒数不标P01至08通过。

### Grok领取顺序和结项文件

先ATX-01至04、06至07完成真实数量与整单，ATX-05接真实价值并用DETAIL-U-01复算；同期TASK-01至04完成真实执行权和效果，随后TASK-05至08接故障/取消/公平。DOC-01至02完成原版本契约；正式宿主就绪后ATX-08/DOC-03至04执行双端。未知成本、被绕开的原公开入口、部分commit和任务来源伪造都属于未完成，不能通过隐藏菜单或只禁用旧路由将整项打勾。

每批交付明确：任务编号/实际改动文件与函数、迁移/原API兼容、测试实际名称与登记目标、每条请求/响应、真实失败地点、独立数量/价值/原命令事实、源码SHA及并行变化、strict Clippy结果、剩余正式客户端/Release/托管门禁。性能报告沿第二十三节原格式，增加本章场景编号和起止点。保护现有数据，不SQL清除未知命令、不猜补历史成本、不清理构建缓存；未测任务写NOT_RUN，局部事实通过写PARTIAL及具体范围。

## 十八 逐功能交付定位和真实账号专项

### 100项功能的一对一交付定位

本表把原100项功能直接连接到页面、实施单元和量化门禁。点击功能名读完整字段、按钮、接口、表和失败断言；文件简称沿第一章。代码列保留原卡的“现/规”标记，现有源码只是复用入口，仍须逐项盘点业务缺口；规划位置不是已实现文件。页面编号查第六章，ERP-IMPL查第二章，UAT查第八章。所有行的正式双端、托管和性能完成状态均需另附证据，本表不授予PASS。

同一页面承载多个功能时，按功能ID分别验收。F1、V和X行的ERP-IMPL/UAT仅表示可复用基础；报价、委外、联副产、APS、设备、预测和行业等专项必须同时满足原卡的专属样本，不能拿基础剧本通过替代整项。调拨/盘点I03也须执行ERP-IMPL-12/13的守恒和截止点样本，UAT-05只验证共享库存。

| 功能与原优先级 | 页面 / 实施单元 / 基础剧本 | 现有与待补代码位置 | 必须交付的失败验收 | 量化性能门禁（目标） |
| --- | --- | --- | --- | --- |
| [A01 注册和企业开通](FREE_ERP_CHECKLIST.md#a01-注册和企业开通) / F0 | PAGE-01；ERP-IMPL-01；UAT-01 | 规 `UI/pages/account/onboarding.rs`、`API/onboarding/service.rs`、`REPO/onboarding.rs`；现 `REPO/tenants.rs`、`REPO/auth.rs` 接续企业及账号。 | 过期/已用验证凭据拒绝；20路同请求仅一个企业；默认配置写入失败时回滚或进入可恢复阶段。 | ERP-P01/P04：真实登录P95≤1秒/P99≤2秒；同步小规模初始化提交P95≤1秒，批量初始化另测任务受理与完成。 |
| [A02 单账号席位和套餐](FREE_ERP_CHECKLIST.md#a02-单账号席位和套餐) / F0 | PAGE-03；ERP-IMPL-02；UAT-01 | 现 `DOMAIN/access.rs`、`REPO/access_plan.rs`、`REPO/rbac.rs`、`API/rbac_user_handlers.rs`；规 `UI/pages/account/plan.rs`。 | 20路创建/启用第二用户均不能让免费有效登录人数超过1；设备会话增加不改变席位数。 | ERP-P06：20并发×100轮席位准入，判定P95≤1秒；免费有效席位始终≤1，设备会话不增加席位。 |
| [A03 多端登录和会话](FREE_ERP_CHECKLIST.md#a03-多端登录和会话) / F0 | PAGE-03；ERP-IMPL-03；UAT-02 | 现 `API/auth_handlers.rs`、`API/auth_tokens.rs`、`API/auth_layers.rs`、`REPO/access_plan.rs`；规 `UI/pages/account/sessions.rs`，现 `UI/http.rs` 接认证。 | 第9个有效会话按配置拒绝；撤销一个端不误撤其他端；改密后旧凭据和已撤销会话不能发新写入。 | ERP-P01/P06/P07：登录P95≤1秒/P99≤2秒；1/8/9会话边界各100轮，撤销提交后新受保护请求立即拒绝。 |
| [A04 企业及数据隔离](FREE_ERP_CHECKLIST.md#a04-企业及数据隔离) / F0 | PAGE-09/13/18/28；ERP-IMPL-04；UAT-13 | 现 `API/auth_context.rs`、`API/auth_layers.rs`、`REPO/data_scopes.rs`；所有域repo；规租户RLS迁移和对象存储授权适配。 | 替换对象、仓库、任务、附件或下载ID均不能读取他企业内容；拒绝响应不泄露他企业对象名称。 | ERP-P02/P05：授权100行查询P95≤500ms；越权及跨租户漏读/写为0，20并发100轮无授权范围漂移。 |
| [A05 企业配置和业务模式](FREE_ERP_CHECKLIST.md#a05-企业配置和业务模式) / F0 | PAGE-02；ERP-IMPL-01/04；UAT-01 | 现 `API/system_settings_handlers.rs`、`REPO/settings.rs`；规 `UI/pages/account/company_settings.rs`、`DOMAIN/company_settings.rs`。 | 关闭制造菜单不删除工单；两端同时改配置只有一个版本成功；已编号单据不被配置变化重编号。 | ERP-P02/P04：100行列表P95≤500ms；20/100行提交P95≤1/2秒、P99≤2/4秒。 |
| [A06 示例隔离和上手流程](FREE_ERP_CHECKLIST.md#a06-示例隔离和上手流程) / F0 | PAGE-01/08；ERP-IMPL-01/37；UAT-01 | 现 `UI/module_demo/` 保持夹具；规 `UI/pages/account/getting_started.rs`、`API/onboarding/demo.rs`；真实初始化接A01。 | 复制示例或退出示例后正式余额仍不变；清除示例不删除真实单据；不能把示例保存记作正式验收。 | ERP-P01/P04：真实登录P95≤1秒/P99≤2秒；同步小规模初始化提交P95≤1秒，批量初始化另测任务受理与完成。 |
| [A07 登录安全和限流](FREE_ERP_CHECKLIST.md#a07-登录安全和限流) / F0 | PAGE-03/28；ERP-IMPL-03/40；UAT-02/13 | 现 `API/auth_layers.rs`、`API/auth_handlers.rs`；规 `API/middleware/rate_limit.rs`、`REPO/security_events.rs`、`UI/pages/account/security.rs`。 | 连续失败触发可解释拒绝；攻击一个账号不阻断其他企业登录；日志和错误响应均不含密码、令牌或验证码。 | ERP-P07/P18：撤销后新请求立即拒绝；100请求/秒持续60秒突发时限流有界，其他企业查询/保存仍满足P02/P04。 |
| [A08 状态和问题反馈](FREE_ERP_CHECKLIST.md#a08-状态和问题反馈) / F0 | PAGE-28；ERP-IMPL-40；UAT-14 | 现 `API/system_observability_handlers.rs`；规 `API/support/service.rs`、`REPO/support_tickets.rs`、`UI/pages/account/support.rs`。 | 失败保留反馈草稿；其他企业不能读反馈及附件；写入一条状态文字不能证明服务探针正常。 | ERP-P02/P16：100行列表P95≤500ms；10万行导出CSV≤30秒/XLSX≤60秒，脱敏和授权错误为0。 |
| [A09 数据退出和服务约定](FREE_ERP_CHECKLIST.md#a09-数据退出和服务约定) / F0 | PAGE-28；ERP-IMPL-38/40；UAT-14 | 规 `UI/pages/account/data_exit.rs`、`API/tenant_exit/service.rs`、`REPO/tenant_exit.rs`；现 `REPO/export_jobs.rs` 接导出。 | 下载文件行数及校验和匹配清单；过期链接和撤权后链接拒绝；未完成导出不能提示数据已带走。 | ERP-P16/P20：数据交付按10万行CSV≤30秒/XLSX≤60秒；恢复演练RPO≤15分钟/RTO≤2小时，退出保留规则另按公开约定验证。 |
| [A10 资源计量和公平使用](FREE_ERP_CHECKLIST.md#a10-资源计量和公平使用) / F0 | PAGE-28；ERP-IMPL-39/40；UAT-13/14 | 规 `DOMAIN/usage.rs`、`REPO/usage_metering.rs`、`API/usage/service.rs`、`UI/pages/account/usage.rs`；规 `WORKER/jobs/admission.rs`。 | 单企业大量任务时其他9家普通读写仍满足P02/P04；超过资源上限不静默截断或丢已受理任务。 | ERP-P18/P22：重任务拥塞下普通查询P95≤500ms、20行保存P95≤1秒；所有用量可核对，成本预算B须在首发前填写。 |
| [U01 共享工作台和页面状态](FREE_ERP_CHECKLIST.md#u01-共享工作台和页面状态) / F0 | 全部业务页；ERP-IMPL-37；UAT-11 | 现 `UI/workbench/`、`UI/erp_app.rs`、`UI/workspace_shell.rs`、`UI/page_registry.rs`；新页面只接这些入口。 | 两页同一类型的筛选、尺寸和草稿互不覆盖；关闭脏页可取消；写入待回执期间不丢已受理命令。 | ERP-P08/P19：万行×100列滚动帧间隔P95≤32ms；同状态20轮切页后内存≤预热基线+10%+20MiB。 |
| [U02 服务端网格查询和选择](FREE_ERP_CHECKLIST.md#u02-服务端网格查询和选择) / F0 | 全部业务页；ERP-IMPL-37；UAT-11/13 | 现 `CORE/paged_query.rs`、`CORE/remote.rs`、`CORE/matched_selection.rs`、`EGUI/lib.rs`、`UI/remote_selection.rs`；各新域实现查询适配。 | 筛出第501行后仍可正确分页；跨页选中不会遗漏未加载行；旧查询未完成时不能操作旧结果。 | ERP-P02/P08：100行服务端查询P95≤500ms；回包至可操作帧P95≤300ms，迟到回写0次，缓存遵守10块/1000行/8MiB。 |
| [U03 搜索和保存视图](FREE_ERP_CHECKLIST.md#u03-搜索和保存视图) / F0 | 全部业务页；ERP-IMPL-37；UAT-11 | 现 `UI/query.rs`、`UI/named_views.rs`、`CORE/filter.rs`、`CORE/preferences.rs`、`EGUI/query_builder.rs`；各域后端查询翻译。 | 倒置金额范围保留输入并禁止旧结果批量动作；旧响应晚到不覆盖新条件；重置立即清空条件并查询。 | ERP-P09：输入反馈P95≤50ms；250ms防抖、IME组合期间0查询；对象选项P95≤300ms且每次≤100项。 |
| [U04 录入编辑和批量保存](FREE_ERP_CHECKLIST.md#u04-录入编辑和批量保存) / F0 | 全部录单页；ERP-IMPL-37；UAT-04/06/11 | 现 `EGUI/editor.rs`、`CORE/edit_planner.rs`、`UI/shared_edits.rs`、`UI/components.rs`；各新页面编辑适配。 | 100行中第100行无效时整单0效果；独立对象批次可有成功/失败并准确定位；冲突不能清空未保存内容。 | ERP-P04/P09：本地输入反馈P95≤50ms；20/100行提交P95≤1/2秒、P99≤2/4秒，逐行状态与真实效果一致。 |
| [U05 草稿和未知命令恢复](FREE_ERP_CHECKLIST.md#u05-草稿和未知命令恢复) / F0 | 全部录单页；ERP-IMPL-37/39；UAT-11/12 | 现 `UI/draft_store.rs`、`UI/edit_drafts.rs`、`UI/leave_guard.rs`、`UI/action_store.rs` 及其native/web适配；规 `REPO/command_receipts.rs`。 | 保存后丢回包、崩溃、重登均只能产生一次效果；改变服务地址或企业不会恢复错空间草稿。 | ERP-P07/P19：同状态20轮恢复/切页后内存≤基线+10%+20MiB；未决动作重复效果0，撤销后不能新提交。 |
| [U06 中文和平台交互](FREE_ERP_CHECKLIST.md#u06-中文和平台交互) / F0 | 全部业务页；ERP-IMPL-37；UAT-11 | 现 `UI/fonts.rs`、`UI/workbench/theme.rs`、`EGUI/accessibility.rs`、`EGUI/editor.rs`；native/web宿主适配。 | 125%/150%缩放不裁断重要按钮；中文候选期间0查询；ESC取消编辑不会误保存；切主题保留草稿。 | ERP-P08/P09：本地输入反馈P95≤50ms、滚动帧P95≤32ms；真实IME组合期间0查询，缩放与焦点无操作丢失。 |
| [U07 文件导入导出和打印](FREE_ERP_CHECKLIST.md#u07-文件导入导出和打印) / F0 | PAGE-08/27/28；ERP-IMPL-06/38；UAT-10 | 现 `UI/export.rs`、`UI/http/download.rs`、`UI/components.rs`、`API/export_file_stream.rs`、`WORKER/jobs/export_jobs.rs`；规 `API/imports/service.rs`、`WORKER/jobs/import_jobs.rs`。 | 导出文件独立解析正确；危险文本不触发电子表格公式；缺纸或离线不能写PRINTED。 | ERP-P15/P16/P17：1万行导入预检≤30秒/提交≤60秒；10万行CSV≤30秒/XLSX≤60秒；20行A4渲染≤1秒。 |
| [U08 高级组件的业务接入](FREE_ERP_CHECKLIST.md#u08-高级组件的业务接入) / F1 | PAGE-14/27；ERP-IMPL-22/36/37；UAT-07/10 | 现 `CORE/tree.rs`、`CORE/report.rs`、`CORE/formula.rs`、`EGUI/tree_render.rs`、`EGUI/report_chart.rs`；规 `UI/pages/reports/aggregate.rs`。 | 仅当前页100行不能冒充100万流水总计；非法/循环公式定位字段；折叠分组不改变总金额。 | ERP-P08/P14/P16：万行×100列滚动帧P95≤32ms；100万流水完整聚合P95≤2秒；导出按P16核对全范围。 |
| [D01 物料和商品档案](FREE_ERP_CHECKLIST.md#d01-物料和商品档案) / F0 | PAGE-04；ERP-IMPL-05；UAT-03 | 现 `UI/remote_view/product_inspector.rs`、`API/product_handlers.rs`、`REPO/products.rs`、`REPO/skus.rs`；规 `UI/pages/master/items.rs` 作为正式物料页。 | 同编码20路创建只有一次；已引用SKU不能硬删；同物料既转售又自产时仍使用同一库存身份。 | ERP-P02/P04：100行列表P95≤500ms；20/100行提交P95≤1/2秒、P99≤2/4秒。 |
| [D02 单位和小数数量](FREE_ERP_CHECKLIST.md#d02-单位和小数数量) / F0 | PAGE-04；ERP-IMPL-05/09/10；UAT-03 | 现 `DOMAIN/quantity.rs`、`REPO/product_units.rs`、`REPO/product_unit_conversions.rs`、`API/product_handlers.rs`；规 `UI/pages/master/units.rs`。 | 旧1读回1.000000；0.125不变0；非法0分母、超六位精度及溢出在写入前拒绝，草稿保留。 | ERP-P04/P09：20行保存P95≤1秒，输入反馈P95≤50ms；六位小数及往返换算零截断，超精度/溢出明确拒绝。 |
| [D03 客户供应商和结算条件](FREE_ERP_CHECKLIST.md#d03-客户供应商和结算条件) / F0 | PAGE-05；ERP-IMPL-05/35；UAT-04 | 现 `API/crm_handlers.rs`、`API/procurement_handlers.rs`、`REPO/customers.rs`、`REPO/suppliers.rs`；规 `UI/pages/master/partners.rs`。 | 已有应收对象禁用不删账；跨企业合并拒绝；合并失败没有半迁移引用或丢失余额。 | ERP-P02/P04：100行列表P95≤500ms；20/100行提交P95≤1/2秒、P99≤2/4秒。 |
| [D04 仓库和库位](FREE_ERP_CHECKLIST.md#d04-仓库和库位) / F0 | PAGE-06；ERP-IMPL-04/05；UAT-13 | 现 `UI/remote_view/warehouse_tree.rs`、`REPO/warehouse_tree.rs`、`REPO/warehouses.rs`、`REPO/warehouse_locations.rs`；规 `UI/pages/master/warehouses.rs`。 | 有库存库位不能删除；重命名不换库存身份；跨企业仓库或重复库位编号拒绝。 | ERP-P02/P04：100行列表P95≤500ms；20/100行提交P95≤1/2秒、P99≤2/4秒。 |
| [D05 员工班组和组织](FREE_ERP_CHECKLIST.md#d05-员工班组和组织) / F0 | PAGE-07/15；ERP-IMPL-05/24/30；UAT-06 | 规 `DOMAIN/manufacturing/resources.rs`、`REPO/manufacturing/resources.rs`、`API/manufacturing/resources.rs`、`UI/pages/master/personnel.rs`。 | 新增20个员工档案不增加20个免费登录席位；费率变更不改已冻结工单；停用员工保留历史报工。 | ERP-P02/P04：100行列表P95≤500ms；20/100行提交P95≤1/2秒、P99≤2/4秒。 |
| [D06 主数据导入](FREE_ERP_CHECKLIST.md#d06-主数据导入) / F0 | PAGE-08；ERP-IMPL-06；UAT-01/03 | 规 `UI/pages/master/import.rs`、`API/imports/service.rs`、`REPO/import_jobs.rs`、`WORKER/jobs/import_jobs.rs`；复用现对象仓储。 | 1万行中重复编码、空单位、超精度均有行号；重试不重复导入；改文件后不能沿用原预检确认。 | ERP-P15：1万行×20列、≤10MiB，预检≤30秒/确认≤60秒；每任务新增峰值内存≤128MiB，重复建档为0。 |
| [D07 期初启用](FREE_ERP_CHECKLIST.md#d07-期初启用) / F0 | PAGE-08；ERP-IMPL-07/15/35；UAT-03/14 | 规 `UI/pages/master/opening.rs`、`API/opening/service.rs`、`REPO/opening.rs`；现 `REPO/inventory.rs`、`REPO/finance_ar.rs`、`REPO/procurement/repo.rs` 接账本。 | 确认响应丢失后余额不再增加；数量和成本差额未解释时不允许启用；失败中断可原身份恢复。 | ERP-P04/P05/P15：100行确认P95≤2秒/P99≤4秒；大批量按P15，同启用命令100轮重试不叠加余额。 |
| [D08 附件和扩展字段](FREE_ERP_CHECKLIST.md#d08-附件和扩展字段) / F1 | PAGE-04/PAGE-14及附件宿主；ERP-IMPL-08；UAT-13 | 规 `UI/pages/master/attachments.rs`、`API/attachments/service.rs`、`REPO/attachments.rs`；现 `API/system_settings_handlers.rs` 接设置。 | 他企业附件ID、过期链接、未授权对象下载拒绝；附件失败不留不可访问的成功关联；扩展字段不能执行代码。 | ERP-P02/P04/P19：100行附件元数据查询P95≤500ms；元数据提交P95≤1秒；正文流式处理，文件上限在实施前固定并测边界。 |
| [I01 库存账本和余额](FREE_ERP_CHECKLIST.md#i01-库存账本和余额) / F0 | PAGE-09；ERP-IMPL-09/15；UAT-05 | 现 `REPO/inventory.rs`、`REPO/inventory_reads.rs`、`REPO/atp_inventory.rs`、`REPO/inventory_grid_report.rs`、`API/inventory_print_report_handlers/inventory_core.rs`；规 `UI/pages/inventory/balances.rs`。 | 库存1领0.250后只能再卖0.750；1.9-0.9-0.9必须读0.1；余额=期初+有效流水，不一致对象停止准入并给核对状态。 | ERP-P02/P14：100/500行余额或流水P95≤500ms/1秒；100万流水汇总P95≤2秒，独立重建差额为0。 |
| [I02 入出库和预占](FREE_ERP_CHECKLIST.md#i02-入出库和预占) / F0 | PAGE-10/13/19；ERP-IMPL-10/11；UAT-05/12 | 现 `REPO/inventory.rs`、`REPO/inventory_movement_commands.rs`、`REPO/inventory_location_commands.rs`、`REPO/inventory_location_reservations.rs`、`REPO/idempotency.rs`、`API/inventory_print_report_handlers/inventory_core.rs`、`API/inventory_print_report_handlers/inventory_commands.rs`、`API/inventory_print_report_handlers/stock_movement.rs`；规 `UI/pages/inventory/movements.rs`。 | 释放X不会减少Y；重试0新增流水；本笔失败源状态及库存一起回滚；不足量返回冲突而非通用500。 | ERP-P04/P05：100行提交P95≤2秒/P99≤4秒；20路同库存争用×100轮，明确结果P99≤3秒，超占和重复效果0。 |
| [I03 调拨盘点和差异](FREE_ERP_CHECKLIST.md#i03-调拨盘点和差异) / F0 | PAGE-10；ERP-IMPL-12/13；UAT-05 | 现 `REPO/stock_transfers.rs`、`REPO/stock_counts.rs`、`REPO/stock_adjustments.rs`、`API/inventory_print_report_handlers/stock_count.rs`；规 `UI/pages/inventory/documents.rs`。 | 发10收6后在途4；损失1后还在途3；重复收6拒绝超收；盘点期间新增业务不被旧盘点结果覆写。 | ERP-P04/P05：100行提交P95≤2秒/P99≤4秒；20路同库存争用×100轮，明确结果P99≤3秒，超占和重复效果0。 |
| [I04 批次序列及效期](FREE_ERP_CHECKLIST.md#i04-批次序列及效期) / F0 | PAGE-11；ERP-IMPL-14/33；UAT-06 | 现 `REPO/sku_batches.rs`、`REPO/sku_serials.rs`；规 `REPO/manufacturing/genealogy.rs`、`UI/pages/inventory/batches.rs`，API查询接库存及质量域。 | 过期或隔离批次不用于普通销售/领料；批次0.250出库准确；同序列20路出库最多一次。 | ERP-P13：100万关联边中典型≤1000边追溯P95≤2秒，首100项≤500ms；大结果分页且不静默截断。 |
| [I05 库存成本和期间](FREE_ERP_CHECKLIST.md#i05-库存成本和期间) / F0 | PAGE-09/26；ERP-IMPL-15；UAT-04/06 | 现 `REPO/finance_costing.rs`、`REPO/sku_costs.rs`、`API/finance_management.rs`；规 `DOMAIN/costing.rs`、`UI/pages/inventory/costs.rs`。 | 原成本2分/3微单位分三次退回最终恰好2分；零单位成本合法；已关期调整另记，不覆盖原成本。 | ERP-P04/P14：20行确认P95≤1秒；100万流水固定汇总P95≤2秒，重算任务≤60秒；数量金额及残差零不明差额。 |
| [I06 扫码标签及操作打印](FREE_ERP_CHECKLIST.md#i06-扫码标签及操作打印) / F0 | PAGE-09/11/28；ERP-IMPL-38；UAT-10 | 规 `UI/pages/inventory/scan.rs`；现 `REPO/print_jobs.rs`、`REPO/print_templates.rs`、`WORKER/printing.rs`、`crates/erp-print-agent/src/main.rs`。 | 同一输入重复回传不增数；错批次可定位；设备离线不写出纸完成；实际纸上编码可重新扫描识别。 | ERP-P09/P17：扫码本地反馈P95≤50ms；在线Agent受理≤5秒、渲染≤1秒；出纸单独计时，重复业务提交0。 |
| [I07 补货库龄和预警](FREE_ERP_CHECKLIST.md#i07-补货库龄和预警) / F1 | PAGE-09/27；ERP-IMPL-09/36；UAT-10 | 现 `API/planning.rs`、`REPO/planning.rs`、`REPO/stock_count_suggestions.rs`；规 `UI/pages/inventory/alerts.rs`、`WORKER/jobs/inventory_alerts.rs`。 | 一轮重算不生成重复告警；冻结/待检不当可供量；处理告警不直接改变库存或采购事实。 | ERP-P14：100万流水固定报表P95≤2秒；更重任务受理≤500ms/执行≤60秒，完整范围聚合与独立SQL一致。 |
| [I08 批次及库位推荐](FREE_ERP_CHECKLIST.md#i08-批次及库位推荐) / F1 | PAGE-11；ERP-IMPL-14/20/29；UAT-06 | 现 `REPO/sku_batch_strategies.rs`、`REPO/wms.rs`；规 `DOMAIN/inventory_allocation.rs`、`UI/pages/inventory/allocation.rs`。 | 推荐后库存被抢占时提交明确冲突；人工选择不能越权、超占或使用隔离批次；建议保存不扣库存。 | ERP-P09/P05：一次候选≤100项且P95≤300ms；最终分配20并发×100轮不超可用量，推荐与真实扣减分别留证。 |
| [T01 采购订单及修订](FREE_ERP_CHECKLIST.md#t01-采购订单及修订) / F0 | PAGE-12；ERP-IMPL-16；UAT-04 | 现 `API/procurement_handlers.rs`、`REPO/procurement/models.rs`、`REPO/procurement/repo.rs`、`REPO/procurement/store.rs`；规 `UI/pages/trade/purchase_orders.rs`、`API/procurement/service.rs`。 | 订10收6后未收4；修订不得将订单量降到已有效收货以下；100行末行失败整张修订回滚。 | ERP-P02/P03/P04：100行列表/20行详情P95≤500ms；20/100行保存P95≤1/2秒、P99≤2/4秒。 |
| [T02 分批收货入库及采购退货](FREE_ERP_CHECKLIST.md#t02-分批收货入库及采购退货) / F0 | PAGE-12；ERP-IMPL-17/18；UAT-04 | 现 `API/procurement_handlers.rs`、`API/inventory_print_report_handlers/wms_inbound.rs`、`REPO/shared_contract.rs`、`REPO/shared_contract/receipt_values.rs`、`REPO/shared_contract/receipt_offsets.rs`、`REPO/procurement/repo.rs`、`REPO/inbounds.rs`；规 `UI/pages/trade/purchase_receipts.rs`。 | 收6+4累计10；待检6不能销售；退2后应付和库存按原来源冲减；超过原收货余量拒绝且0效果。原成本批次/累计尾差/重放验收见第二十章，后端局部通过不代替正式页面。 | ERP-P04/P05：100行提交P95≤2秒/P99≤4秒；20路同库存争用×100轮，明确结果P99≤3秒，超占和重复效果0。 |
| [T03 销售订单及修订](FREE_ERP_CHECKLIST.md#t03-销售订单及修订) / F0 | PAGE-13；ERP-IMPL-19；UAT-04 | 现 `API/sales_orders.rs`、`REPO/sales_orders.rs`；规 `API/sales_orders/service.rs`、`UI/pages/trade/sales_orders.rs`。 | 没有平台账号仍能录单/确认/交付；交付6后订单量不能降到5；两端修订冲突保留草稿。 | ERP-P02/P03/P04：100行列表/20行详情P95≤500ms；20/100行保存P95≤1/2秒、P99≤2/4秒。 |
| [T04 分批交付和发货](FREE_ERP_CHECKLIST.md#t04-分批交付和发货) / F0 | PAGE-13；ERP-IMPL-20；UAT-04/05 | 现 `API/wms_operation_handlers/outbound_core.rs`、`REPO/wms/repo.rs`、`REPO/sales_orders.rs`；规 `API/sales_orders/deliveries.rs`、`UI/pages/trade/deliveries.rs`。 | 订单10可交6+4；重复交付ID无第二次扣减；物流超时显示待核对；取消余量只释放本订单未交预占。 | ERP-P04/P05：100行提交P95≤2秒/P99≤4秒；20路同库存争用×100轮，明确结果P99≤3秒，超占和重复效果0。 |
| [T05 销售退货和账款冲减](FREE_ERP_CHECKLIST.md#t05-销售退货和账款冲减) / F0 | PAGE-13；ERP-IMPL-21；UAT-04 | 现 `API/sales_orders.rs::create_sales_return` 已接POST退货；现 `REPO/after_sales.rs`、`REPO/finance_ar.rs` 提供复用依据；规 `API/sales_returns/service.rs`、`REPO/sales_returns.rs`、`UI/pages/trade/sales_returns.rs`。 | 卖6退2后有效交付4；第三笔不能超原6；改SKU/批次/他企业原交付拒绝；重试不重复退款或入库。 | ERP-P04/P05：100行提交P95≤2秒/P99≤4秒；20路同库存争用×100轮，明确结果P99≤3秒，超占和重复效果0。 |
| [T06 单据动作及历史](FREE_ERP_CHECKLIST.md#t06-单据动作及历史) / F0 | PAGE-12/13；ERP-IMPL-16/19/39；UAT-04/12 | 现 `API/procurement_handlers.rs`、`API/sales_orders.rs` 及各repo；规 `DOMAIN/trade_documents.rs`、`REPO/document_revisions.rs`、`UI/pages/trade/document_history.rs`。 | 已产生库存及款项的单据不能直接删除；非法状态动作0效果；历史记录和独立账本能相互定位。 | ERP-P04/P05：20行动作P95≤1秒/P99≤2秒；同版本20并发×100轮最多一个有效修订，重复动作不重复反向入账。 |
| [T07 单据和对账打印](FREE_ERP_CHECKLIST.md#t07-单据和对账打印) / F0 | PAGE-12/13/25；ERP-IMPL-38；UAT-10 | 现 `REPO/print_templates.rs`、`REPO/print_mappings.rs`、`API/inventory_print_report_handlers/printing.rs`；规 `UI/pages/trade/print_preview.rs`。 | 修改模板不改旧打印快照；重印不重复出入库；缺纸状态真实；PDF与单据合计独立核对一致。 | ERP-P15/P16/P17：1万行导入预检≤30秒/提交≤60秒；10万行CSV≤30秒/XLSX≤60秒；20行A4渲染≤1秒。 |
| [T08 报价价表和信用](FREE_ERP_CHECKLIST.md#t08-报价价表和信用) / F1 | PAGE-12/PAGE-13及报价信用扩展；ERP-IMPL-16/19；UAT-04 | 现 `REPO/marketing_price_lists.rs`、`REPO/planning.rs`；规 `REPO/trade_quotes.rs`、`DOMAIN/credit_policy.rs`、`API/trade_quotes/service.rs`、`UI/pages/trade/quotes.rs`。 | 报价过期不能直接转单；同报价20路转单不重复；已逾期客户按配置拒绝而不是悄悄建单。 | ERP-P02/P03/P04：100行列表/20行详情P95≤500ms；20/100行保存P95≤1/2秒、P99≤2/4秒。 |
| [T09 直送寄售和套装](FREE_ERP_CHECKLIST.md#t09-直送寄售和套装) / F1 | PAGE-12/PAGE-13及货权扩展；ERP-IMPL-16/20；UAT-04 | 现 `REPO/finance_consignment.rs`、`REPO/kits.rs`；规 `DOMAIN/trade_ownership.rs`、`API/trade_fulfillment/service.rs`、`UI/pages/trade/special_fulfillment.rs`。 | 寄售发10卖3时我方剩余货权7；退回不重复形成采购；套装配置变化不改已确认订单组成。 | ERP-P04/P05：100行提交P95≤2秒/P99≤4秒；20路同库存争用×100轮，明确结果P99≤3秒，超占和重复效果0。 |
| [M01 多级制造 BOM](FREE_ERP_CHECKLIST.md#m01-多级制造-bom) / F0 | PAGE-14；ERP-IMPL-22；UAT-07 | 规 `UI/pages/manufacturing/bom.rs`、`DOMAIN/manufacturing/bom.rs`、`API/manufacturing/bom.rs`、`REPO/manufacturing/bom.rs`；现 `REPO/kits.rs` 只保留贸易套装。 | P→S→B多级展开保留路径和共用物料汇总；环、零基本产量、缺单位及超32层拒绝，不能输出半个成功结果。 | ERP-P10：1万物料/5万边/深度≤32，从快照加载到结果提交≤5秒，新增峰值内存≤256MiB；环和预算超限明确拒绝。 |
| [M02 BOM 发布和版本](FREE_ERP_CHECKLIST.md#m02-bom-发布和版本) / F0 | PAGE-14；ERP-IMPL-23；UAT-07 | 规 `DOMAIN/manufacturing/bom.rs`、`REPO/manufacturing/bom.rs`、`API/manufacturing/bom.rs`、`UI/pages/manufacturing/bom_versions.rs`。 | 相邻区间可衔接，重叠默认版本拒绝；发布后不可原地改组件；老工单仍读原快照而不是最新版本。 | ERP-P04/P05：100行版本提交P95≤2秒/P99≤4秒；20并发×100轮不覆盖已发布内容或突破有效区间，原工单快照不变。 |
| [M03 工序和工艺路线](FREE_ERP_CHECKLIST.md#m03-工序和工艺路线) / F0 | PAGE-15；ERP-IMPL-24；UAT-07 | 规 `DOMAIN/manufacturing/routing.rs`、`REPO/manufacturing/routing.rs`、`API/manufacturing/routing.rs`、`UI/pages/manufacturing/routings.rs`。 | 形成工序环、负工时或未知工作中心拒绝；更改展示顺序不改前置关系；已下达工单路线不随新配置改变。 | ERP-P03/P04/P12：100工序版本保存P95≤2秒/P99≤4秒；20行详情P95≤500ms，检环和工序校验零漏判。 |
| [M04 工作中心日历和负荷](FREE_ERP_CHECKLIST.md#m04-工作中心日历和负荷) / F0 | PAGE-15；ERP-IMPL-24；UAT-07/08 | 规 `DOMAIN/manufacturing/resources.rs`、`REPO/manufacturing/resources.rs`、`API/manufacturing/resources.rs`、`UI/pages/manufacturing/work_centers.rs`。 | 休息日可用工时0；2台×480分钟能力960分钟；排1000分钟明确超载40，不能显示已自动优化。 | ERP-P14：100万流水固定报表P95≤2秒；更重任务受理≤500ms/执行≤60秒，完整范围聚合与独立SQL一致。 |
| [M05 需求和主生产计划](FREE_ERP_CHECKLIST.md#m05-需求和主生产计划) / F0 | PAGE-16；ERP-IMPL-25；UAT-08 | 规 `DOMAIN/manufacturing/demand.rs`、`REPO/manufacturing/demand.rs`、`API/manufacturing/planning.rs`、`UI/pages/manufacturing/production_plans.rs`；现 `REPO/sales_orders.rs` 提供来源。 | 同销售行不重复引入需求；销售已交4的10件订单只保留剩余6；取消只取消未执行计划，不抹工单事实。 | ERP-P02/P04：100行列表P95≤500ms；20/100行提交P95≤1/2秒、P99≤2/4秒。 |
| [M06 基础 MRP](FREE_ERP_CHECKLIST.md#m06-基础-mrp) / F0 | PAGE-17；ERP-IMPL-26；UAT-08 | 规 `DOMAIN/manufacturing/mrp.rs` 纯算法、`REPO/manufacturing/mrp.rs` 输入输出、`API/manufacturing/mrp.rs` 接口、`WORKER/jobs/mrp_jobs.rs` 执行、`UI/pages/manufacturing/mrp.rs`。 | 后到货不能抵早缺料；S现存抵用后仅展开净制造量；共用B先合并后批量取整；试算不写采购、工单或库存。 | ERP-P11：1万物料/5万边/1000需求/90天，受理P95≤500ms、执行至全部结果提交≤30秒、新增内存≤512MiB；取消按P11。 |
| [M07 MRP 回查和建议转单](FREE_ERP_CHECKLIST.md#m07-mrp-回查和建议转单) / F0 | PAGE-17；ERP-IMPL-27；UAT-09 | 规 `REPO/manufacturing/mrp.rs`、`API/manufacturing/service.rs`、`UI/pages/manufacturing/mrp_results.rs`；采购和工单仓储复用。 | 建议60先转20后余40；再转41拒绝；同身份返回原目标单；过期运行按明确重算/确认策略处理。 | ERP-P04/P05/P11：100行转单P95≤2秒/P99≤4秒；同建议20并发×100轮不超建议量，MRP结果查询和取消按P11。 |
| [M08 生产订单状态和下达](FREE_ERP_CHECKLIST.md#m08-生产订单状态和下达) / F0 | PAGE-18；ERP-IMPL-28；UAT-06/07 | 规 `DOMAIN/manufacturing/production.rs`、`API/manufacturing/service.rs`、`REPO/manufacturing/production.rs`、`UI/pages/manufacturing/work_orders.rs`。 | 下达末行缺料则工单仍原状态、前面预占0新增；20路下达一次；已领料取消必须有余料及成本处置。 | ERP-P12/P05：100行生产动作P95≤2秒/P99≤4秒；20并发×100轮无超报、超领或重复入库，同库存争用结果P99≤3秒。 |
| [M09 领补退料和耗用](FREE_ERP_CHECKLIST.md#m09-领补退料和耗用) / F0 | PAGE-19；ERP-IMPL-29；UAT-06/12 | 规 `DOMAIN/manufacturing/materials.rs`、`REPO/manufacturing/materials.rs`、`API/manufacturing/materials.rs`、`UI/pages/manufacturing/materials.rs`；现 `REPO/inventory.rs` 接库存。 | 领0.250、耗0.100、退0.125后车间余0.025；原仓0.875；错SKU/来源或累计超原领料拒绝，重试回原余额快照。 | ERP-P12/P05：100行领/退/耗分别P95≤2秒/P99≤4秒；20路争用×100轮，材料/批次/原领料余量零超用，明确结果P99≤3秒。 |
| [M10 工序报工](FREE_ERP_CHECKLIST.md#m10-工序报工) / F0 | PAGE-20；ERP-IMPL-30；UAT-06 | 规 `DOMAIN/manufacturing/reporting.rs`、`REPO/manufacturing/reporting.rs`、`API/manufacturing/reporting.rs`、`UI/pages/manufacturing/reporting.rs`。 | 前序10件后序累计不能报11；同序20路不能超报；负工时拒绝；冲正保留原报工、反向量及认证提交人。 | ERP-P12/P05：100行生产动作P95≤2秒/P99≤4秒；20并发×100轮无超报、超领或重复入库，同库存争用结果P99≤3秒。 |
| [M11 在制半成品和完工入库](FREE_ERP_CHECKLIST.md#m11-在制半成品和完工入库) / F0 | PAGE-21；ERP-IMPL-31；UAT-06 | 规 `REPO/manufacturing/production.rs`、`REPO/manufacturing/receipts.rs`、`API/manufacturing/receipts.rs`、`UI/pages/manufacturing/receipts.rs`；现库存仓储及规质量服务。 | 有效合格10只能分入6+4；待检10销售可用0，放行6后可用6；重复入库不加产量，半成品投入可追原工单。 | ERP-P12/P05：100行生产动作P95≤2秒/P99≤4秒；20并发×100轮无超报、超领或重复入库，同库存争用结果P99≤3秒。 |
| [M12 工单和在制成本](FREE_ERP_CHECKLIST.md#m12-工单和在制成本) / F0 | PAGE-24；ERP-IMPL-34；UAT-06 | 规 `DOMAIN/manufacturing/costing.rs`、`REPO/manufacturing/costing.rs`、`API/manufacturing/costing.rs`、`WORKER/jobs/costing_jobs.rs`、`UI/pages/manufacturing/costs.rs`；现 `REPO/sku_costs.rs` 接成品成本。 | 300+100+100=500元，完工10件单位50元；部分完工保留在制；晚到费用正式调整；缺费用不能显示确定利润。 | ERP-P04/P14：20行确认P95≤1秒；100万流水固定汇总P95≤2秒，重算任务≤60秒；数量金额及残差零不明差额。 |
| [M13 缺料报废及返工](FREE_ERP_CHECKLIST.md#m13-缺料报废及返工) / F0 | PAGE-18/20/24；ERP-IMPL-28/30/34；UAT-06 | 规 `DOMAIN/manufacturing/exceptions.rs`、`REPO/manufacturing/exceptions.rs`、`API/manufacturing/exceptions.rs`、`UI/pages/manufacturing/exceptions.rs`。 | 9合格+1不良必须保留不良去向；返工额外领料计额外消耗；未处置余料或成本时关闭拒绝。 | ERP-P12/P05：100行生产动作P95≤2秒/P99≤4秒；20并发×100轮无超报、超领或重复入库，同库存争用结果P99≤3秒。 |
| [M14 基础委外](FREE_ERP_CHECKLIST.md#m14-基础委外) / F1 | 委外页（规划扩展）；ERP-IMPL-16/17/28；UAT-06 | 规 `DOMAIN/manufacturing/subcontract.rs`、`REPO/manufacturing/subcontract.rs`、`API/manufacturing/subcontract.rs`、`UI/pages/manufacturing/subcontract.rs`；现采购和库存服务接续。 | 送10收6后在外4；收回6不把同一原料再耗一次；加工费可按原工单核对；超收或他供应商来源拒绝。 | ERP-P02/P03/P04：100行列表/20行详情P95≤500ms；20/100行保存P95≤1/2秒、P99≤2/4秒。 |
| [M15 替代料和工程变更](FREE_ERP_CHECKLIST.md#m15-替代料和工程变更) / F1 | PAGE-14/PAGE-18及工程变更扩展；ERP-IMPL-23/28；UAT-07 | 规 `DOMAIN/manufacturing/engineering.rs`、`REPO/manufacturing/engineering.rs`、`API/manufacturing/engineering.rs`、`UI/pages/manufacturing/engineering_changes.rs`。 | 已领/耗/报事实不能随变更删除；不兼容单位替代拒绝；新BOM不自动改老工单；可反查所有受影响单。 | ERP-P04/P05：100行版本提交P95≤2秒/P99≤4秒；20并发×100轮不覆盖已发布内容或突破有效区间，原工单快照不变。 |
| [M16 联副产品拆解和倒冲](FREE_ERP_CHECKLIST.md#m16-联副产品拆解和倒冲) / F1 | PAGE-18/PAGE-19/PAGE-21及联副产扩展；ERP-IMPL-29/31/34；UAT-06 | 规 `DOMAIN/manufacturing/co_products.rs`、`REPO/manufacturing/co_products.rs`、`API/manufacturing/co_products.rs`、`UI/pages/manufacturing/co_products.rs`。 | 联产品分摊合计严格等于原成本；已手耗0.4、应耗1只倒冲0.6；重复拆解不能再回收同一成品。 | ERP-P12/P05：100行生产动作P95≤2秒/P99≤4秒；20并发×100轮无超报、超领或重复入库，同库存争用结果P99≤3秒。 |
| [M17 生产看板和交期估算](FREE_ERP_CHECKLIST.md#m17-生产看板和交期估算) / F1 | PAGE-18/27；ERP-IMPL-36；UAT-10 | 规 `REPO/manufacturing/reports.rs`、`API/manufacturing/reports.rs`、`UI/pages/manufacturing/dashboard.rs`；现图表组件复用。 | 已暂停或缺料工单不显示正常准时；旧刷新不能覆盖新版本；未知工时明确未知，不填0造出确定交期。 | ERP-P14：100万流水固定报表P95≤2秒；更重任务受理≤500ms/执行≤60秒，完整范围聚合与独立SQL一致。 |
| [Q01 检验标准和放行](FREE_ERP_CHECKLIST.md#q01-检验标准和放行) / F0 | PAGE-22；ERP-IMPL-32；UAT-06 | 现 `REPO/inbounds.rs`、`REPO/wms_qc_sampling_rules.rs` 提供接续；规 `REPO/quality/inspections.rs`、`API/quality/service.rs`、`UI/pages/quality/inspections.rs`。 | 漏必检项不能放行；旧检验使用旧标准；待检10放6只增加可用6；20路放行不会增加两次可用量。 | ERP-P04/P05：100项检验/处置提交P95≤2秒/P99≤4秒；20路竞争放行不超可处置量，隔离料误销售/领用为0。 |
| [Q02 不良隔离和处置](FREE_ERP_CHECKLIST.md#q02-不良隔离和处置) / F0 | PAGE-22；ERP-IMPL-32；UAT-06 | 规 `DOMAIN/quality/dispositions.rs`、`REPO/quality/dispositions.rs`、`API/quality/service.rs`、`UI/pages/quality/dispositions.rs`；现库存事务接状态转换。 | 隔离5不能处置6；让步放行需合法权限；报废扣对应实物与成本；处置失败保持原不可用量。 | ERP-P04/P05：100项检验/处置提交P95≤2秒/P99≤4秒；20路竞争放行不超可处置量，隔离料误销售/领用为0。 |
| [Q03 双向批次追溯](FREE_ERP_CHECKLIST.md#q03-双向批次追溯) / F0 | PAGE-23；ERP-IMPL-33；UAT-06 | 规 `REPO/manufacturing/genealogy.rs`、`API/quality/trace.rs`、`UI/pages/quality/trace.rs`；现 `REPO/sku_batches.rs`、`REPO/sku_serials.rs` 接批次对象。 | 原料→半成品→成品→交付与反向都能回原单；分页无重复遗漏；跨租户边拒绝；超范围不悄悄截断。 | ERP-P13：100万关联边中典型≤1000边追溯P95≤2秒，首100项≤500ms；大结果分页且不静默截断。 |
| [Q04 抽检趋势和召回](FREE_ERP_CHECKLIST.md#q04-抽检趋势和召回) / F1 | PAGE-22/PAGE-23/PAGE-27及召回扩展；ERP-IMPL-32/33/36；UAT-06 | 现 `REPO/wms_qc_sampling_rules.rs`；规 `REPO/quality/recalls.rs`、`API/quality/recalls.rs`、`UI/pages/quality/recalls.rs`。 | 模板变化不改历史抽检判定；召回覆盖全部匹配范围；任务重试不重复冻结；不能只写召回完成文字。 | ERP-P13：100万关联边中典型≤1000边追溯P95≤2秒，首100项≤500ms；大结果分页且不静默截断。 |
| [F01 应收应付及期初](FREE_ERP_CHECKLIST.md#f01-应收应付及期初) / F0 | PAGE-25；ERP-IMPL-07/35；UAT-04 | 现 `API/finance_handlers.rs`、`API/finance_management.rs`、`REPO/finance_ar.rs`、`REPO/procurement/repo.rs`；规 `UI/pages/finance/receivables_payables.rs`。 | 销售600元只形成一次600应收；收400后余200；期初与新增单据能区分；未确认草稿不计往来。 | ERP-P02/P04/P05：100行账款列表P95≤500ms；20行登记P95≤1秒/P99≤2秒；20并发核销×100轮不超未结余额。 |
| [F02 收付款和核销](FREE_ERP_CHECKLIST.md#f02-收付款和核销) / F0 | PAGE-25；ERP-IMPL-35；UAT-04 | 规 `DOMAIN/settlement.rs`、`REPO/payment_entries.rs`、`API/finance/payments.rs`、`UI/pages/finance/payments.rs`；现AR/AP仓储接核销。 | 一笔700可核销400+300；再分配1拒绝；应付1000付700余300；重复请求不再扣账户或核销。 | ERP-P02/P04/P05：100行账款列表P95≤500ms；20行登记P95≤1秒/P99≤2秒；20并发核销×100轮不超未结余额。 |
| [F03 费用和附加成本](FREE_ERP_CHECKLIST.md#f03-费用和附加成本) / F0 | PAGE-26；ERP-IMPL-34/35；UAT-06 | 现 `REPO/finance_expenses.rs`、`REPO/finance_costing.rs`、`API/finance_management.rs`；规 `DOMAIN/cost_allocation.rs`、`UI/pages/finance/expenses.rs`。 | 费用100元分3行总和仍100；零/负权重按规则拒绝；未审批费用不算确定成本；费用重复引用不重复入账。 | ERP-P04/P14：20行确认P95≤1秒；100万流水固定汇总P95≤2秒，重算任务≤60秒；数量金额及残差零不明差额。 |
| [F04 业务成本和期间对账](FREE_ERP_CHECKLIST.md#f04-业务成本和期间对账) / F0 | PAGE-24/26；ERP-IMPL-15/34/35；UAT-04/06 | 现 `REPO/inventory_reconciliation_center.rs`、`REPO/finance_costing.rs`；规 `REPO/manufacturing/costing.rs`、`API/finance/reconciliation.rs`、`UI/pages/finance/reconciliation.rs`。 | 库存期初+入-出=期末；在制+消耗-结转保持去向；未解决差异不能伪装0；关期后原事实不被覆盖。 | ERP-P04/P14：20行确认P95≤1秒；100万流水固定汇总P95≤2秒，重算任务≤60秒；数量金额及残差零不明差额。 |
| [F05 发票记录和账龄](FREE_ERP_CHECKLIST.md#f05-发票记录和账龄) / F1 | PAGE-25及发票扩展；ERP-IMPL-35；UAT-04 | 现 `REPO/finance_tax.rs`、`REPO/finance_ar.rs`；规 `API/finance/invoices.rs`、`UI/pages/finance/invoices.rs`。 | 未结200按真实到期日进入唯一账龄桶；重复发票号按已定范围拒绝；核销后总账龄同步减少而非新增快照冒充余额。 | ERP-P14：100万流水固定报表P95≤2秒；更重任务受理≤500ms/执行≤60秒，完整范围聚合与独立SQL一致。 |
| [F06 标准实际差异和分摊增强](FREE_ERP_CHECKLIST.md#f06-标准实际差异和分摊增强) / F1 | PAGE-24/PAGE-26及分摊扩展；ERP-IMPL-34/35；UAT-06 | 规 `DOMAIN/manufacturing/costing.rs`、`REPO/manufacturing/costing.rs`、`API/manufacturing/costing.rs`、`UI/pages/finance/cost_variances.rs`。 | 数量差+价格差按冻结公式复算；缺实际费用显示暂估；重算不覆盖已确认历史；合计与工单成本一致。 | ERP-P04/P14：20行确认P95≤1秒；100万流水固定汇总P95≤2秒，重算任务≤60秒；数量金额及残差零不明差额。 |
| [R01 经营报表](FREE_ERP_CHECKLIST.md#r01-经营报表) / F0 | PAGE-27；ERP-IMPL-36；UAT-10 | 现 `REPO/reports.rs`、`REPO/inventory_grid_report.rs`、`API/inventory_print_report_handlers/reports_exports.rs`；规 `UI/pages/reports/business.rs`。 | 100万流水SQL复算一致；页100行不影响总计；未知成本销售不显示确定毛利；不同币种不直接相加。 | ERP-P14：100万流水固定报表P95≤2秒；更重任务受理≤500ms/执行≤60秒，完整范围聚合与独立SQL一致。 |
| [R02 制造报表](FREE_ERP_CHECKLIST.md#r02-制造报表) / F0 | PAGE-27；ERP-IMPL-36；UAT-10 | 规 `REPO/manufacturing/reports.rs`、`API/manufacturing/reports.rs`、`UI/pages/reports/manufacturing.rs`；复用现图表和DataGrid。 | 报工冲正后产出正确减少；部分入库不清空全部在制；报表余额能与工单/库存逐项核对。 | ERP-P14：100万流水固定报表P95≤2秒；更重任务受理≤500ms/执行≤60秒，完整范围聚合与独立SQL一致。 |
| [R03 导出任务和下载](FREE_ERP_CHECKLIST.md#r03-导出任务和下载) / F0 | PAGE-28；ERP-IMPL-38；UAT-10 | 现 `REPO/export_jobs.rs`、`REPO/inventory_export_jobs.rs`、`REPO/inventory_export_cursor.rs`、`WORKER/jobs/export_jobs.rs`、`WORKER/jobs/inventory_export.rs`、`API/inventory_export_access.rs`、`API/export_file_stream.rs`、`UI/remote_view/exports.rs`；规各新报表导出适配。库存CSV输入、回执、权限和性能接线详见 [ERP-IMPL-38](FREE_ERP_DEVELOPMENT_SPEC.md#库存后台-csv-的实际接线)。 | 10万行完整输出；文件SHA及行数可独立核对；改筛选不改变已受理导出；他企业任务或撤权后下载拒绝。 | ERP-P16：10万行×20列、≤50MiB，CSV≤30秒/XLSX≤60秒，新增内存≤256MiB；计时至可下载，下载另计。 |
| [R04 异常及任务中心](FREE_ERP_CHECKLIST.md#r04-异常及任务中心) / F0 | PAGE-28；ERP-IMPL-39；UAT-12 | 现 `REPO/dead_letters.rs`、`API/system_observability_handlers.rs`；规 `API/operations/tasks.rs`、`UI/pages/reports/exceptions.rs`。 | 死信重放最多一次有效入账；已提交任务不能显示取消成功；权限已撤销不能借重试继续操作。 | ERP-P02/P18/P19：100行任务查询P95≤500ms；单企业MRP/导出各≤1并行、全局重任务先≤2；重启与24小时运行无丢任务或重复效果。 |
| [R05 保存报表和趋势](FREE_ERP_CHECKLIST.md#r05-保存报表和趋势) / F1 | PAGE-27；ERP-IMPL-36/37；UAT-10 | 现 `UI/named_views.rs`、`CORE/report.rs`、`EGUI/report_chart.rs`；规 `REPO/report_views.rs`、`API/reports/views.rs`、`UI/pages/reports/saved.rs`。 | 字段删除不能默默丢条件生成另一报表；趋势各桶口径一致；视图仅保存当前用户允许配置。 | ERP-P14：100万流水固定报表P95≤2秒；更重任务受理≤500ms/执行≤60秒，完整范围聚合与独立SQL一致。 |
| [O01 原子提交和业务幂等](FREE_ERP_CHECKLIST.md#o01-原子提交和业务幂等) / F0 | 所有正式写入口；ERP-IMPL-10/20/29/35/39；UAT-12 | 现 `API/middleware/idempotency.rs`、`REPO/idempotency.rs`、`REPO/outbox.rs`、`REPO/inventory.rs`；各域应用服务接线，制造新增规 `API/manufacturing/service.rs`、`REPO/manufacturing/production.rs`。 | 前99行成功准备、末行失败时整单0效果；同身份不同负载409；100轮竞争中重复效果0。 | ERP-P05：20路争同库存/版本×100轮，完成或明确冲突P99≤3秒；余额、台账、回执、Outbox部分提交和重复效果为0。 |
| [O02 命令回执和未知结果核对](FREE_ERP_CHECKLIST.md#o02-命令回执和未知结果核对) / F0 | 所有正式写入口及28；ERP-IMPL-39；UAT-12 | 现 `UI/action_store.rs`、`UI/action_store/`、`UI/leave_guard.rs`；规 `API/command_handlers.rs`、`REPO/command_receipts.rs`、`UI/pages/reports/commands.rs`；接入现 `API/router.rs`。 | 提交丢回包后查询得到原effect_id；重登或切端不新发命令；未找到不能立即判断失败并自动再次扣账。 | ERP-P02/P05：100行回执查询P95≤500ms；原命令丢响应后100轮回查/重试效果始终一次，查询不触发业务重执行。 |
| [O03 任务重试 死信和取消](FREE_ERP_CHECKLIST.md#o03-任务重试-死信和取消) / F0 | PAGE-28及Worker；ERP-IMPL-39；UAT-12 | 现 `WORKER/runtime.rs`、`WORKER/event_router.rs`、`WORKER/inbox.rs`、`WORKER/dlq.rs`、`REPO/inbox.rs`、`REPO/worker_effects.rs`、`REPO/dead_letters.rs`；新增任务接现 `WORKER/jobs/mod.rs`。 | Worker崩溃恢复不重复效果；取消提交后2秒内给真实阶段；正在提交不能假报取消已完成。 | ERP-P02/P18/P19：100行任务查询P95≤500ms；单企业MRP/导出各≤1并行、全局重任务先≤2；重启与24小时运行无丢任务或重复效果。 |
| [O04 多端版本和请求代次](FREE_ERP_CHECKLIST.md#o04-多端版本和请求代次) / F0 | 全部录单页；ERP-IMPL-37/39；UAT-11 | 现 `UI/query.rs`、`UI/edit_drafts.rs`、`UI/shared_edits.rs`、`CORE/remote.rs`、`CORE/edit_planner.rs`；版本校验接各域 API/service 与 REPO 更新事务。 | 两端同版本提交只有一端成功；较早查询最后返回也不能覆盖新筛选；只读取消不取消已受理写事务。 | ERP-P05/P08：20路同版本×100轮无丢更新；回包到可操作帧P95≤300ms，旧代次覆盖新结果0次。 |
| [O05 授权 审计和脱敏](FREE_ERP_CHECKLIST.md#o05-授权-审计和脱敏) / F0 | 全部受保护入口及28；ERP-IMPL-04/40；UAT-13 | 现 `API/auth_context.rs`、`API/auth_layers.rs`、`API/audit_helpers.rs`、`REPO/permissions.rs`、`REPO/data_scopes.rs`、`REPO/audit_logs.rs`；规 `API/attachments.rs`；Worker 的任务来源继续绑定租户和授权范围。 | 无权限直接API拒绝；跨企业回执不能读；导出撤权及时生效；员工填报名不得替代认证提交人。 | ERP-P02/P05：授权100行查询P95≤500ms；越权及跨租户漏读/写为0，20并发100轮无授权范围漂移。 |
| [O06 备份 恢复和租户误操作](FREE_ERP_CHECKLIST.md#o06-备份-恢复和租户误操作) / F0 | 运维入口；ERP-IMPL-40；UAT-14 | 现 `scripts/backup_postgres.sh`、`docs/OPERATIONS.md`；规 `scripts/Restore-ErpBackup.ps1`、`qa/acceptance/free-erp/recovery/`；恢复步骤补入现运维文档，归档监控接现 `infra/observability/`。 | 连续写入后3次恢复均保持截点余额/台账/回执一致；记录RPO/RTO；写RECOVERED标记不算恢复成功。 | ERP-P20：至少3次真实独立恢复，数据库RPO≤15分钟、服务RTO≤2小时；截点内账实/附件可核对，恢复后不重复扣库存。 |
| [O07 迁移 兼容和发布回退](FREE_ERP_CHECKLIST.md#o07-迁移-兼容和发布回退) / F0 | 运维入口；ERP-IMPL-40；UAT-14 | 现 `migrations/`、`scripts/release_gate_backend.mjs`、`scripts/Invoke-ErpBuild.ps1`、`scripts/Build-ErpWeb.ps1`；规 `API/version_handlers.rs`、`UI/version_check.rs`；发布与回退步骤补入现 `docs/OPERATIONS.md`。 | 升级后旧整数1仍为1个单位；已有小数事实不能回退截成整数；重复迁移编号不能直接改已执行迁移。 | ERP-P20/P21：带历史业务与在制数据升级，数据损坏/丢失0；回退恢复RTO≤2小时，升级后冷/热启动按P21重新测。 |
| [O08 健康 可观测性和公平调度](FREE_ERP_CHECKLIST.md#o08-健康-可观测性和公平调度) / F0 | PAGE-28及运维入口；ERP-IMPL-39/40；UAT-14 | 现 `API/auth_layers.rs` 的健康检查、`API/system_observability_handlers.rs`、`WORKER/runtime.rs`、`infra/observability/prometheus/`、`infra/observability/grafana/`；规 `WORKER/jobs/admission.rs`。 | 数据库断连时readiness失败；重任务压满不饿死普通查询；任务/连接池队列有界；指标采集失败不伪造0。 | ERP-P18/P19/P22：50会话/20请求每秒稳态30分钟及24小时长跑；正常错误率<0.1%、不变量失败0，普通响应满足P02/P04。 |
| [O09 公网与浏览器安全](FREE_ERP_CHECKLIST.md#o09-公网与浏览器安全) / F0 | 公网入口；ERP-IMPL-40；UAT-13 | 现 `API/main.rs`、`API/auth_layers.rs`、`API/auth_tokens.rs`、`API/export_file_stream.rs`、`UI/http.rs`、`UI/http/web.rs`；规 `API/middleware/web_security.rs`；安全配置补入现 `docs/OPERATIONS.md`，反向代理配置随部署交付。 | 未授权跨域/敏感写请求拒绝；用户输入不能注入查询或导出公式；日志不暴露凭据；TLS失效不假报已连接。 | ERP-P07/P18：撤销后新请求立即拒绝；100请求/秒持续60秒突发时限流有界，其他企业查询/保存仍满足P02/P04。 |
| [O10 双端发布和平台适配](FREE_ERP_CHECKLIST.md#o10-双端发布和平台适配) / F0 | 全部业务页及发布入口；ERP-IMPL-37/40；UAT-11/14 | 现 `UI/main.rs`、`UI/lib.rs`、`UI/fonts.rs`、`UI/http/`、`UI/draft_store.rs`、`UI/action_store/`、`scripts/Build-ErpWeb.ps1`；真实页面接现 `UI/page_registry.rs` 与 `UI/erp_app.rs`。 | 两端各重登读回同一事实；只读Web预览不能算完成；平台不支持设备操作时返回真实结果。 | ERP-P21：至少30次，桌面首交互≤5秒，Web冷载≤15秒/热载≤3秒，登录后首业务页≤2秒；两端操作分别验收。 |
| [O11 发布门禁和依赖许可证](FREE_ERP_CHECKLIST.md#o11-发布门禁和依赖许可证) / F0 | CI及发布入口；ERP-IMPL-40；UAT-14 | 现 `scripts/verify_repo_baseline.mjs`、`scripts/verify_rust_baseline.mjs`、`scripts/verify_no_ag_grid.mjs`、`scripts/verify_openapi_gate_v2.mjs`、`.github/workflows/ci.yml`；规 `scripts/verify_dependency_licenses.mjs`；检查与许可证记录入口补入现 `docs/OPERATIONS.md`。 | 严格Clippy失败不写全后端PASS；缺集成目标先登记；不因锁等待另建target；文档改动不启动编译。 | ERP-P08至P21按实际发布范围执行；报告须有Release产物摘要、固定数据和原始样本，缺样本/FAIL/NOT_RUN不得写性能PASS。 |
| [O12 持续演练和容量扩展](FREE_ERP_CHECKLIST.md#o12-持续演练和容量扩展) / F1 | 运维入口；ERP-IMPL-39/40；UAT-14 | 现 `docs/OPERATIONS.md` 增补巡检和容量步骤，规 `qa/acceptance/free-erp/operations/` 留证；扩展现 `infra/observability/`、`qa/performance/` 和既有运维入口，不建立平行发布系统。 | DB/Worker/文件服务故障都记录实际恢复；24小时无无法解释增长；突发限流后普通流量恢复，已入账不丢。 | ERP-P18/P19/P22：50会话/20请求每秒稳态30分钟及24小时长跑；正常错误率<0.1%、不变量失败0，普通响应满足P02/P04。 |
| [V01 多人席位 权限和协作](FREE_ERP_CHECKLIST.md#v01-多人席位-权限和协作) / V | 成员与审批（规划）；ERP-IMPL-02/03/04；UAT-01/02/13 | 现 `DOMAIN/access.rs`、`REPO/access_plan.rs`、`REPO/rbac.rs`、`REPO/data_scopes.rs`、`API/rbac_user_handlers.rs`、`API/rbac_role_handlers.rs`；规 `DOMAIN/approval.rs`、`REPO/approval_flows.rs`、`API/approval_handlers.rs`、`UI/pages/account/members.rs`。 | 多人独立user/session审计可区分；降级明确保留账号并撤销其余权限；历史单据和已受理命令仍可核对。 | ERP-P06/P07：20并发×100轮不超所购席位；撤权后新受保护请求立即拒绝，免费降级席位≤1且历史记录保留。 |
| [V02 集团 多法人和多工厂](FREE_ERP_CHECKLIST.md#v02-集团-多法人和多工厂) / V | 集团与工厂（规划）；ERP-IMPL-05/13/35；UAT-04 | 规 `DOMAIN/group.rs`、`REPO/group_companies.rs`、`REPO/intercompany.rs`、`API/group_handlers.rs`、`UI/pages/master/group.rs`、`UI/pages/finance/intercompany.rs`；复用现 `REPO/tenants.rs`、`REPO/warehouses.rs` 及贸易域服务。 | 甲公司不能借组织树读乙未授权账；内部交易不虚增合并收入；跨法人调货金额及往来可追溯。 | ERP-P02/P04/P14：普通查询/保存按500ms/1秒基线；100万流水固定汇总P95≤2秒；跨组织或专业核算大样本另定发布门禁。 |
| [V03 有限产能 APS](FREE_ERP_CHECKLIST.md#v03-有限产能-aps) / V | 有限产能排程（规划）；ERP-IMPL-24/25/26；UAT-08 | 规 `DOMAIN/manufacturing/aps.rs`、`REPO/manufacturing/schedule_runs.rs`、`API/manufacturing/aps.rs`、`WORKER/jobs/aps_jobs.rs`、`UI/pages/manufacturing/schedule.rs`；复用 M03/M04/M06 的快照和资源。 | 设备不可同时分给重叠工序；缺料/休息日约束有效；优化算法规模和时限实施前另定，不能冒用P11通过。 | ERP-P18保障其他企业普通查询/保存；实施前另固定工序数、资源数、计划窗口、求解截止时长和可行性/目标值标准，不用MRP的30秒充作APS指标。 |
| [V04 设备 MES OEE和专业质量](FREE_ERP_CHECKLIST.md#v04-设备-mes-oee和专业质量) / V | 设备与车间（规划）；ERP-IMPL-30/32；UAT-06 | 规 `DOMAIN/manufacturing/mes.rs`、`REPO/manufacturing/device_events.rs`、`API/device_event_handlers.rs`、`WORKER/jobs/device_event_jobs.rs`、`UI/pages/manufacturing/mes.rs`、`UI/pages/quality/advanced.rs`。 | 重复/乱序事件不双记产量；无运行时长不能假算OEE；设备断流有状态；收费MES不阻塞免费手工报工。 | ERP-P18/P19：24小时采集不挤占普通业务、设备事件不重复；实施前按设备频率固定事件量、丢包/补传及设备到页面延迟指标。 |
| [V05 专业系统和平台连接器](FREE_ERP_CHECKLIST.md#v05-专业系统和平台连接器) / V | 连接器（规划）；ERP-IMPL-39/40；UAT-12/13 | 复用现 `crates/erp-adapter/`、`crates/adapter-taobao/`、`crates/adapter-jd/`、`crates/adapter-pdd/`、`crates/adapter-douyin/` 和现 `WORKER/sync_jobs.rs`；新专业接口按同一边界扩展，规 `API/connector_handlers.rs`、`UI/pages/account/connectors.rs`。 | 授权过期可恢复；Webhook重复和补偿轮询去重；外部超时不再次出入库；连接器未启用不阻塞贸易单据。 | ERP-P18保障普通业务，外部限流不堵库存事务；实施前固定每通道并发、重试预算、同步规模/耗时和未知结果回查，按真实授权环境测试。 |
| [V06 高级分析 预测和订阅](FREE_ERP_CHECKLIST.md#v06-高级分析-预测和订阅) / V | 高级分析（规划）；ERP-IMPL-36/39；UAT-10 | 现 `crates/erp-analytics/`、`REPO/reports.rs` 及网格报表组件；规 `DOMAIN/analytics.rs`、`REPO/report_subscriptions.rs`、`API/analytics_handlers.rs`、`WORKER/jobs/analytics_jobs.rs`、`UI/pages/reports/analytics.rs`。 | 回测按相同截止训练避免未来泄漏；数据不足显示未知；发送及外部通道单独验收，不用写一条订阅记录充数。 | ERP-P14：固定报表P95≤2秒；模型训练/预测另固定数据量、基线误差、预测范围和资源预算，无样本不能宣称预测有效。 |
| [V07 专属资源和实施服务](FREE_ERP_CHECKLIST.md#v07-专属资源和实施服务) / V | 托管控制面（规划）；ERP-IMPL-40；UAT-14 | 规 `REPO/service_contracts.rs`、`API/service_contract_handlers.rs`、`UI/pages/account/service_contract.rs`；专属部署步骤补入现 `docs/OPERATIONS.md`，复用现部署产物、迁移、备份和可观测性配置。 | 资源购买成功不等于实例已可用；实际恢复、性能和隔离按约定实测；迁移客户数据有校验和及读回。 | ERP-P20/P22：默认演练RPO≤15分钟/RTO≤2小时并核对独立成本；更高约定指标须在该实例容量下另测并写入服务约定。 |
| [V08 支付 账单和权益](FREE_ERP_CHECKLIST.md#v08-支付-账单和权益) / V | 账单与权益（规划）；ERP-IMPL-02；UAT-01 | 现 `DOMAIN/access.rs`、`REPO/access_plan.rs`；规 `DOMAIN/billing.rs`、`REPO/billing_orders.rs`、`REPO/billing_events.rs`、`API/billing_handlers.rs`、`WORKER/jobs/billing_jobs.rs`、`UI/pages/account/billing.rs`。 | 20次重复回调只授予一次；前端伪造成功不生效；退款及降级不删历史账；支付通道阈值另定。 | ERP-P04/P06/P07：权益提交P95≤1秒；20并发×100轮不超已购席位，重复/乱序回调权益效果一次；支付通道耗时单独报告。 |
| [V09 完整会计 税务和多币种](FREE_ERP_CHECKLIST.md#v09-完整会计-税务和多币种) / V | 会计与税务（规划）；ERP-IMPL-35；UAT-04 | 现 `REPO/finance.rs`、`REPO/finance_ar.rs`、`REPO/finance_costing.rs`、`API/finance_handlers.rs`；规 `DOMAIN/accounting.rs`、`REPO/finance_gl.rs`、`API/accounting_handlers.rs`、`UI/pages/finance/general_ledger.rs`。 | 每张凭证借贷平衡；不同币种不能直接相加；已关期修正用合法调整；记录发票不冒称官方开票成功。 | ERP-P02/P04/P14：普通查询/保存按500ms/1秒基线；100万流水固定汇总P95≤2秒；跨组织或专业核算大样本另定发布门禁。 |
| [X01 配方和流程制造](FREE_ERP_CHECKLIST.md#x01-配方和流程制造) / X | 配方与批次生产（规划）；ERP-IMPL-22/24/29/34；UAT-06 | 规 `DOMAIN/industry/process_manufacturing.rs`、`REPO/industry/process_batches.rs`、`API/industry/process_handlers.rs`、`UI/pages/manufacturing/process_batches.rs`；复用 M02/M09/M12/M16 和质量域。 | 质量/体积不能凭比例直接互换；每批输入输出损耗可解释；行业算法和设备阈值另定，普通BOM通过不算流程制造完成。 | 普通查询/100行提交按ERP-P02/P04的500ms/2秒基线；行业计算、外部流程另固定规模与阈值后验收，行业不变量零失败。 |
| [X02 深度机加工和工序委外](FREE_ERP_CHECKLIST.md#x02-深度机加工和工序委外) / X | 机加工与委外（规划）；ERP-IMPL-24/28/30；UAT-06 | 规 `DOMAIN/industry/machining.rs`、`REPO/industry/tooling.rs`、`REPO/industry/operation_wip.rs`、`API/industry/machining_handlers.rs`、`UI/pages/manufacturing/machining.rs`；复用 M03/M04/M10/M14/M15。 | 改图不覆写已开工事实；工序外协分送分收可核；计件费用按有效报工核算；未知能力不伪装自动排程。 | 普通查询/100行提交按ERP-P02/P04的500ms/2秒基线；行业计算、外部流程另固定规模与阈值后验收，行业不变量零失败。 |
| [X03 序列化装配 返修和保修](FREE_ERP_CHECKLIST.md#x03-序列化装配-返修和保修) / X | 序列装配与售后（规划）；ERP-IMPL-14/30/33；UAT-06 | 现 `REPO/sku_serials.rs`、`REPO/after_sales.rs`；规 `DOMAIN/industry/serialized_assembly.rs`、`REPO/industry/assembly_links.rs`、`API/industry/assembly_handlers.rs`、`UI/pages/manufacturing/assembly.rs`、`UI/pages/quality/service_history.rs`。 | 同关键件不能同时装在两台有效成品；拆换保留旧件去向；保修换件不重复形成销售或新生产产量。 | ERP-P13：100万关联边中典型单台追溯P95≤2秒、首100项≤500ms；维修/换件100行提交按P04，序列重用或错配为0。 |
| [X04 跨境和复杂贸易](FREE_ERP_CHECKLIST.md#x04-跨境和复杂贸易) / X | 跨境贸易（规划）；ERP-IMPL-16/19/35；UAT-04 | 规 `DOMAIN/industry/cross_border.rs`、`REPO/industry/trade_terms.rs`、`API/industry/cross_border_handlers.rs`、`UI/pages/trade/cross_border.rs`；复用 T09、F03、V09 及现 `REPO/finance_consignment.rs`。 | 不同币种总计分组；跨境在途不能冒称现货；费用晚到有调整来源；国内贸易验收不等于跨境合规已通过。 | 普通查询/100行提交按ERP-P02/P04的500ms/2秒基线；行业计算、外部流程另固定规模与阈值后验收，行业不变量零失败。 |

### 正式账号和多端的当前接线

免费边界仍为一个业务登录账号、同账号多端在线；当前会话默认上限8及期限3600秒是现有安全配置，不增加免费席位。下面给Grok的任务补足A02/A03/A04/A07、O01/O02/O04/O05；收费多人审批不作为其前置。

| 当前位置 / HTTP入口 | 已核对的接线 | 仍须完成的验收 |
| --- | --- | --- |
| 现 `API/router.rs`；POST `/auth/login`，现 `API/auth_handlers.rs::login`；现 `REPO/auth.rs`、`REPO/access_plan.rs::open_session` | 密码查验及数据库角色取权限；真实`auth_sessions`，服务器签发令牌绑定sid/cep。登录入参有tenant_id/email/password/client_kind/client_label；响应有access_token/token_type/expires_in/session_id/session_policy | 普通非管理员账号真登录后完成库存、采购、销售、制造动作；自行签发Claims的测试不能作此证据 |
| 现 GET `/me/sessions`、DELETE `/me/sessions/{session_id}`，现 `API/auth_handlers.rs::{list_own_sessions,revoke_own_session}` | 按当前企业/账号读取和撤销真实会话 | 列表中当前端、到期/最近活动是真实事实，不含令牌；撤A保留B/C，撤销后A不能发新写入 |
| 现 PUT `/me/password`，现 `API/auth_handlers.rs::update_current_user_password`；现 `REPO/access_plan.rs::bump_credentials_epoch_in_tx` | 密码hash替换和凭据代次在同事务提交，不能继续描述为分开调用 | 真实旧会话/旧密码拒绝、新密码可登录；代次更新SQL失败回滚密码，不保存“改密成功”假结果；必要安全审计另核对提交方式 |
| 现 `API/router.rs` 的 `/auth/sessions/renew`、`/auth/password/recover` → `API/auth_handlers.rs::{renew_own_session,recover_own_password}`；`REPO/access_plan.rs::{renew_session,recover_formal_password}` | 当前正式续期绑定同用户/sid/代次并重新签发令牌；正式找回按企业、账号、令牌hash消费一次凭据并替换实际密码/代次。原`shared_contract`的login-client契约为历史记录入口，不能再描述为当前正式接线 | 真实旧/新密码、过期/重复找回、到期/撤销/跨端续期和当前角色需核对；找回凭据的真实送达、服务故障/竞争和双端另验。仅有路由或契约记录仍不足以完成A03 |
| 现 `REPO/inventory_movement_commands.rs::{execute,payload_hash,snapshot_payload_hash,verified_snapshot}` | 新快照v2：业务摘要移除session_id，真实提交会话留在原快照及两个必需日志的submitter_session_id；回查核对该出处。旧v1按原版本/摘要和原效果核对，不重写回执；当前执行仍要求真实session_id、角色和仓库授权 | 修前真实换端重试实测409；修后同业务换端返回原效果和5.9原余额，当前6.9不重记账；真实修前v1也已换端读取/重放。锁等待撤权、全部写入口及双端实际恢复仍按IAM-04/05/09单独验收 |
| 现 `REPO/access_plan.rs::open_session` | 同设备复用先于新增会话准入；复用只取当前凭据代次，真实重新登录延长该会话有效期。旧代次会话保留为REVOKED并退出当前名额，不复活旧sid | 原改密后新密码误复用epoch0会话并实测401；修后新sid/epoch1可完成实际入库。8会话同设备复用通过；20路边界、8个旧代次后的补名额及登录/改密在途竞争仍须独立证据 |
| 现 `API/auth_layers.rs::auth_middleware`、`API/rbac_user_handlers.rs::update_user_roles`、`API/rbac_role_handlers.rs::update_role_permissions` | 普通正式登录含sid/cep；部分旧测试使用不带会话的合成令牌 | 逐入口证明当前角色、用户状态、仓库范围和会话权威；角色检查后等待库存锁的竞争需验证共同授权顺序，不能仅看JWT权限或一次提前COUNT |

### 10项真实账号实施任务

这些是接线及验收任务，初始要求为NOT_RUN，当前仍按完整结束条件逐项结项；局部实际结果见下方证据，不把一个用例通过写成整个IAM任务完成。新增测试已接现登记`free_erp_acceptance`，按统一构建入口执行。不得为让旧夹具通过而删除会话/角色校验。

| 任务 | 具体交付与实现位置 | 数据、拒绝与结束条件 |
| --- | --- | --- |
| IAM-01 真登录夹具 | 现 `crates/erp-api/tests/free_erp_acceptance/account_login.rs`统一构建本批新企业、唯一用户、正式角色/权限、仓库范围；复用现 `API/auth_handlers.rs::login`、`REPO/auth.rs`；已接两个库存夹具和四个请求助手，其余业务夹具继续迁移 | 有效Argon2密码hash，实际POST取得服务端令牌；每夹具有界复用会话，不每请求开新会话。跨操作者负例使用明确付费QA上下文中的第二真实登录，不绕过免费席位规则；不在日志输出密码/令牌，独立核对users/user_roles/role_permissions/auth_sessions |
| IAM-02 唯一席位 | 现 `API/rbac_user_handlers.rs::{create_user,update_user}`、`REPO/access_plan.rs::admit_new_login_seat`、`REPO/rbac.rs`；规邀请/恢复/升级服务及 `UI/pages/account/plan.rs` | 所有启用入口同企业事务锁内复核；已有U时20路启用其他用户均拒绝，停用后明确选择继任用户；员工/设备档案不算第二席位，付费仅按有效权益扩席 |
| IAM-03 正式会话生命周期 | 现 `API/auth_handlers.rs`、`API/auth_layers.rs`、`REPO/access_plan.rs`；规 `API/auth/session_service.rs`统一实际续期、退出、撤销与到期处理 | 真实auth_sessions是业务登录会话权威。续期与login_clients记录接续，响应必须含可用令牌；达到8/9边界、到期/撤销不静默续命，不自动重发未决业务 |
| IAM-04 跨端原业务重放 | 现 `REPO/inventory_movement_commands.rs::{payload_hash,execute,verified_snapshot}`；规各正式域的规范化业务指纹 | 指纹包含企业/业务操作者/动作/对象/数量/版本等业务字段，sid/令牌/设备只作当前授权和原提交出处。A与B是同用户不同有效会话时，同request_id同业务回同效果/原快照；改变业务内容409，当前角色和范围仍重新检查 |
| IAM-05 撤权与写入竞争 | 现 `API/rbac_user_handlers.rs`、`API/rbac_role_handlers.rs`、`REPO/rbac.rs`、`REPO/access_plan.rs`及各域写事务；规统一授权锁/代次服务 | 角色变更、禁用、改密、撤会话和受保护写入建立明确授权序列。变更commit先于最终授权点则拒绝；写入已先取得授权保护时变更等待，记录实际顺序。不得检查一次权限后无保护地长等库存锁再提交；所有结果无越权效果/死锁残留 |
| IAM-06 改密和找回 | 现 `API/auth_handlers.rs::update_current_user_password`、`REPO/access_plan.rs::bump_credentials_epoch_in_tx`；规 `API/auth/password_service.rs`接真实一次性找回凭据 | 密码、代次、必要安全记录的失败/恢复有一致结果；真实旧密码与旧会话拒绝。找回凭据过期、二次消费、改企业/账号都拒绝；契约表成功不代表实际密码更新 |
| IAM-07 正式业务接线 | 现 `API/auth_context.rs`、`API/inventory_views.rs`、`API/sales_orders.rs`、`API/procurement_handlers.rs`、`API/manufacturing.rs`及当前仓储叶模块 | 真实会话/用户/角色/授权仓库进入权威事务；覆盖批次、库位、来源、制造与原回执。正常调用真登录，伪造令牌/无sid单列反例；不能改为is_admin让普通权限门禁失真 |
| IAM-08 双端会话与草稿 | 规 `UI/pages/account/sessions.rs`，接现 `UI/http.rs`、`UI/action_store.rs`、`UI/draft_store.rs`、`UI/leave_guard.rs` | 登录保存身份、退出/到期停止新动作；草稿及UNKNOWN原request_id不丢。重新登录另一会话后先回查原结果；不显示假成功，不把旧待写请求悄悄改ID发送；native/WASM各真实验收 |
| IAM-09 历史回执兼容 | 现 `REPO/inventory_movement_commands.rs::verified_snapshot`、`command_receipts`；规显式指纹版本及兼容读取 | 原业务快照、session出处、原摘要与效果保留；旧/新摘要各按原版本验证。不得删旧sid字段后重算覆盖历史摘要、重写台账或新发命令“修复”历史。无法证明原始身份/内容返回需核对，兼容读仍查当前授权 |
| IAM-10 回归与独立证据 | 现 `crates/erp-api/tests/free_erp_acceptance.rs`及库存/批次/库位子模块；规 `qa/acceptance/free-erp-real-account-*.json` | 保留旧失败报告，记录真实HTTP、原请求/效果和DB只读前后对比，至少保护原企业全部事实；所有新IAM样本和受影响回归实际执行。正常登录/双端/Release未执行分别NOT_RUN，不沿用旧源码通过 |

### 12组账号多端验收样本

所有正常样本使用一个正式业务用户U及真实POST登录签发的令牌。权限、会话、席位及业务事实从独立数据库连接读回；以下是可复算预期值，尚未执行。

| 样本 | 前提和操作 | 独立预期 / 必须保存的证据 |
| --- | --- | --- |
| IAM-U01 一账号三端 | 免费企业仅U，桌面A、浏览器B/C各真实登录并访问授权列表 | 活跃席位1、三个不同sid；均能真实查询，设备列表不含令牌。用户/会话/角色原事实、登录/查询状态留证 |
| IAM-U02 会话8/9边界 | 已7个有效会话，20路同时真登录；另测到期及撤销后补名额 | 最多补1个有效会话，其余明确session_limit；不超过配置8。过期/撤销不计有效名额，不把浏览器标签直接算新席位 |
| IAM-U03 席位并发 | U仍ACTIVE，20路创建或激活其他可登录账号；另用员工档案录入 | 免费有效登录席位仍1，其他用户启用拒绝；员工建档可以成功但不能自行登录。实际users状态与企业套餐核对 |
| IAM-U04 两端原结果 | 无预占/冻结，实物4.900000；A命令X入0.125000；另一命令Y入0.500000；B同X同业务重试 | X原余额5.025000、当前余额5.525000；B返回X原效果/台账/日志/快照。X新增效果1，X+Y共两次入库；B的sid不改变X业务身份 |
| IAM-U05 改业务内容 | IAM-U04后，B同X数量改0.125001，或换仓库/SKU/动作/原来源 | 409且零新增业务效果，X摘要/快照和当前5.525000不变；同用户新会话不能让不同负载冒充原成功 |
| IAM-U06 单端撤销 | B撤销A，A发新受保护写入，B/C继续各发授权查询 | A身份失效，写入0效果；B/C仍有效。旧已提交X保留，A的撤销不删除原回执或误踢其他端 |
| IAM-U07 角色撤权 | 保留U登录，数据库真实角色撤去WMS.INVENTORY.WRITE，旧令牌仍含该权限；再调用库存写入 | 按当前角色403、0新增库存/台账/业务日志/Outbox；恢复权限后合法新命令可执行。令牌内容不能替代role_permissions |
| IAM-U08 锁等待竞争 | 独立连接持有目标库存锁，真实请求等待；另一连接真实撤角色/禁用U/撤会话/改密，分组释放锁 | 各组记录阻塞关系、变更commit和写入最终授权/commit顺序；变更先提交则写入拒绝，反序则授权保护必须使变更等待。不能仅比较HTTP回包时刻；无越权效果/死锁 |
| IAM-U09 改密故障 | 正常PUT改密，尝试旧/新密码及旧会话；另在代次更新SQL注入失败 | 正常组旧密码/旧会话拒绝，新密码可登录；失败组原hash/代次均不变，旧有效密码可用，没有部分改密或假成功 |
| IAM-U10 企业和仓库 | U的令牌访问非空另一企业的对象、仓库、原回执、导出和下载；另撤本企业仓库授权后重放X | 按公开401/403/404策略无数据泄漏或业务效果；原命令读/重放重新检查当前仓库范围。被拒绝对象的事实前后不变 |
| IAM-U11 授权事实不可读 | 只在受控QA使会话/角色查询实际失败，发送有效令牌的读/写请求；恢复后再操作 | 明确服务不可用或已定义内部故障，拒绝0业务效果；不能当无权限悄悄降级、跳过校验或自动重发新ID。留实际SQL故障/HTTP/恢复事实 |
| IAM-U12 丢响应后换端 | A实际commit命令X后丢HTTP响应；客户端保留UNKNOWN，撤A，U在B真登录，按原X回查/合法重试 | X始终一次效果，原提交会话出处仍A，B回原快照；UNKNOWN正确恢复，草稿/request_id不丢，新登录不新建X业务。真实桌面/Web各50轮单独记录 |

### 6组真实账号性能与资源负载

沿第四/九章固定硬件、80ms RTT和原P95/P99计法；全部为Release目标，初始NOT_RUN。实际密码查验成本包含在登录耗时，不能换弱hash。会话8上限按真实登录后撤销/到期管理，不为压测关闭席位限制。

| 组别 / 对应ERP-P | 固定负载和起止点 | 通过目标与正确性 |
| --- | --- | --- |
| IAM-P01 / P01 登录 | 100个非空企业各唯一U，正常真密码登录至少1000次；从客户端发出到可访问正式受保护业务，冷/热分组 | 登录P95≤1s/P99≤2s，错误率<0.1%；真实auth_sessions和sid/cep，不泄漏密码/令牌，过期会话有界清理 |
| IAM-P02 / P06 准入 | 席位、已有7会话补1、已有8会话拒9分别20路×100轮；完整所有2000请求分组 | 判定P95≤1s；有效席位≤1、会话≤8或实际配置，成功/拒绝分布分别公开；SQL/池/锁等待均计入 |
| IAM-P03 / P07 撤销传播 | 撤会话、撤角色、禁用、改密各1000轮；变更事务确认commit后立即发新的受保护请求 | 后续新请求立即按当前事实拒绝；业务不变量错误0。拒绝接口耗时仍按P02/P03/P04预算；在途请求另按IAM-U08明确授权顺序 |
| IAM-P04 / P05 跨端竞争 | 同U至少两个真实sid，100轮每轮20路同业务原ID；另组同ID不同业务；独立读原结果 | 全部请求明确完成/冲突P99≤3s，原结果点查P95≤500ms/P99≤1s；每轮同语义只有1个效果，业务变更409，设备改变不误冲突 |
| IAM-P05 / P08/P09/P21 真实双端 | 桌面/Web各30次冷/热启动和各50次IAM-U12，真实服务及80ms RTT；登录与绘制分段 | 桌面启动≤5s，Web冷≤15s/热≤3s；登录后首业务可操作≤2s，收到原回执后状态应用P95≤100ms，本地输入P95≤50ms；身份/原ID/草稿丢失0 |
| IAM-P06 / P02/P04/P18/P19 混合资源 | 50真实会话、20RPS持续30min，100RPS突发60s另组；100非空企业，登录占5%，并行MRP/导出有界 | 普通100行列表P95≤500ms/P99≤1s，20行写P95≤1s/P99≤2s；非预期失败<0.1%、越权/重复效果0；API峰值RSS≤2GiB，全局重任务≤2，按P19保留24h趋势 |

### 本批真实账号修复证据

以下为2026-10-04实际执行记录，与上面的目标剧本和Release性能任务分别记录。环境仅为受保护本机PostgreSQL 17 QA（`erp_shared_contract_test`、`erp_grid_test`、`127.0.0.1:15438`）及正式HTTP处理器的进程内调用；没有完成真实网络、桌面、浏览器、官方托管或发布性能验收。

| 阶段 / 独立证据 | 实际观察与范围 | 剩余边界 |
| --- | --- | --- |
| [修前3个真实密码用例](../qa/acceptance/free-erp-real-login-original-pre-cases-second-independent.json) | 1通过/2失败，9.12秒。已有8会话的同设备复用通过；改密后新密码登录误复用旧epoch0会话，实际库存请求401；同用户A/B不同真实sid重放原命令实际409。没有用自行签发的Claims代替登录 | 最早整批快照另发现一个既有`a03k-limit`企业的会话在用例开始前被并行测试改变，原FAIL及[时间窗口定位](../qa/acceptance/free-erp-real-login-original-protected-drift.json)保留；不能宣称最早快照至修前结束全部旧数据不变 |
| [修后3用例及真实历史v1](../qa/acceptance/free-erp-real-login-repaired-independent.json) | 3通过/0失败，9.65秒。改密后新sid/epoch1可真实入库；B重放X回X原余额5.9，当前余额仍6.9、X一次效果、共两回执。另以新密码登录读取/重放真实修前v1，原摘要/快照/台账未重写。独立38,385项核对、7,651个既有企业69表保护；历史企业仅增加明确指定的一条登录会话，全部旧行不变 | 测试窗口`access_plan.rs`、`login_sessions.rs`发生并行修改，整体PARTIAL；9.65秒是三个功能用例总耗时，不是登录或重放的P95/P99 |
| v2原提交出处 | 原快照、operation_logs、audit_logs共同保留A的原sid；B GET/POST仍返回同效果/原快照。实际尝试改写已提交回执被0284既有不可变触发器拒绝，随后独立核对三个出处和一次流水；不能关闭触发器造假破坏 | 首个测试原以为可直接篡改快照，实被既有守卫拦截，失败日志保留；修正测试以实际不可变契约为结束条件，完整回归中该项通过 |
| [首轮完整登记回归](../qa/acceptance/free-erp-real-login-full-independent.json) | 375通过/5失败/0忽略/0筛除，548.86秒；四个真实账号/原出处核心用例通过。独立39,218项核对、7,662个既有企业69表未变化，source窗口仍有`access_plan.rs`漂移 | 五失败为两处夹具改名后的真实付费第二用户准备、两处当前认证提前拒绝后的断言、到期竞争前实际剩两个名额的测试准备；修正后重跑另存证据。首轮不能改写为通过 |
| [复核完整登记回归](../qa/acceptance/free-erp-real-login-full-recheck-independent.json) | 381通过/0失败/0忽略/0筛除，512.60秒；五处失败均已修正，包含并行增加的登记用例。独立41,370项核对、8,094个既有企业69表未变化，v2原快照/操作/审计三处sid仍为A，库存只有一次原效果 | `auth_handlers.rs`、`login_sessions.rs`在测试窗口并行变化，整体源码认证仍PARTIAL。12个受影响叶文件经统一入口格式复查通过；首次格式失败及定向修正记录保留。普通正常库存夹具真登录不代表所有域已迁移 |

本批新提交使用指纹版本2：业务摘要不包含sid，原提交出处仍是原sid；历史版本1按原摘要核对。真实执行仍复核当前会话、凭据代次、数据库角色及仓库授权。`open_session`只复用当前代次会话，旧代次保留为REVOKED并退出名额；复用真实重新登录后的有效期不缩短。普通库存、批次及库位夹具已改用真实密码登录，第二操作者负例只在明确付费的本批QA企业内准备，不能以此增加正式免费席位。

当前严格Clippy实际FAIL：[仓储检查](../qa/acceptance/free-erp-real-login-repo-clippy-current.log)报告lib 241/lib test 243；[API范围检查](../qa/acceptance/free-erp-real-login-api-clippy-current.log)因仓储241及四适配器合计27项失败，未完成API包验收。本批库存命令和会话仓储文件没有出现在这些诊断位置，不等于包级通过。统一入口、固定缓存和锁均保留，没有为通过而压制warnings、删除校验或清理别的构建。

IAM-01至10不据上述局部通过整项结项：其余业务正常夹具、全域授权在途竞争、实际续期/找回、双端UNKNOWN恢复、完整迁移、严格Clippy、源输入一致性及全部Release负载仍逐门禁留证。会话边界的低成本QA密码hash只验证并发正确性，不能代替生产密码成本或登录性能。源码、功能断言、独立数据库读回和发布资格分别报告；免费版保持一个业务账号、多端在线。

### 登录连接池 密码计算和准入竞态

2026-10-07接续IAM-03/IAM-06的实际修复。正式POST `/auth/login`复用现`API/auth_handlers.rs::login`，不新增客户端业务字段。现`REPO/access_plan.rs::open_session`接收服务端已验证的`verified_password_hash`，该值来自真实数据库用户，不允许客户端提供。

| 功能 / 实际位置 | 实现与验收方向 | 当前证据 / 性能接续 |
| --- | --- | --- |
| 单连接会话准入；`REPO/access_plan.rs::open_session` | 同一事务连接完成租户行锁、当前用户核对、旧代次处理、命名会话复用、名额计数和提交；保留原8会话上限及已有历史。禁止先占一条连接持有会话级锁，再从同池取另一条连接 | 真密码+单连接池应200，新增一次真实会话；按IAM-P01/P02/P06另测池等待/峰值连接，不能用扩池掩盖双连接占用 |
| 取消与锁释放；同仓储函数、`API/auth_handlers.rs` | 使用租户行锁及已有事务级提交锁；请求取消/失败依靠事务回滚释放，没有长期留在池内的session advisory lock。SQL 10秒/锁5秒是保护上限，非性能SLA | 真实PostgreSQL阻塞PID被观察后取消请求，释放自有阻塞事务；连接全部回到idle、锁0、会话新增0。取消已提交后的登录恢复另验，不删除已提交会话 |
| 旧验证结果与新凭据竞争；`open_session`及`API/auth_handlers.rs::login` | 密码在事务外真实校验；取得租户提交序列后按READ COMMITTED再读当前用户状态/hash。改密/禁用已提交则401、零新会话，不把旧验证结果绑定到最新epoch；不对users反向加锁造成用户→租户锁顺序死锁 | 本机实际隔离级别已核对；测试使用正式RbacRepo更新用户与AccessPlanRepo代次的同事务、真实登录和真实锁等待，不宣称整个找回/改密HTTP及所有角色竞争已完成 |
| 后台密码验证及资源界限；`API/auth_handlers.rs::{verify_password_off_executor,PasswordVerificationBudget}` | 每进程密码验证最多32个已准入任务，其中2个运行Argon2；计算用spawn_blocking，工作持有票据至实际结束。等待者取消归还自己的票据；第33个拒绝，不无限增加密码任务或占用数据库连接 | 库级真实盐/hash用例检查队满、31等待者取消、剩余合法密码实际通过、错误密码仍拒绝、最终32/2容量恢复。已运行作业取消的完整压力/RSS另测；不把本用例1.44秒视为P95 |
| 对外繁忙错误；现`openapi/src/paths.yaml`及生成包`openapi/erp_api_v1.yaml` | 验证容量不足返回503 JSON，code=`login_busy`、message为暂不可用；不签令牌或打开会话。前端保留登录输入，不能把该结果显示为密码错误 | [契约核对](../qa/acceptance/free-erp-session-pool-oct7-openapi.json)仅确认源/生成包/处理器映射；503端到端过载、双端提示及Release仍按IAM-P01/P06验收 |

新增真实五场景位于现`crates/erp-api/tests/free_erp_acceptance/session_pool.rs`，共用真实账号夹具的`password_login_payload`，不输出密码/令牌。修前五项全部失败（7.40秒）：单连接500，双请求200/500，取消后锁1，改密/禁用提交后仍200并签发epoch1。首次修后4通过/1失败（7.82秒）保留；该历史窗口另有789个旧企业的outbox_events新增四列，行数和原字段多集无变化，严格全列保护仍FAIL，见[历史字段变化](../qa/acceptance/free-erp-session-pool-repaired-first-protected-drift.json)。不白名单或回滚这些旧记录来美化结果。

[2026-10-07五场景独立读回](../qa/acceptance/free-erp-session-pool-oct7-five-independent.json)为PASS：五项全通过、0忽略，7.54秒；44,140项核对、8,807个既有企业69表未变化，960项源输入窗口无漂移。当前单/双连接分别200及200/200，取消锁0；改密/禁用竞争401、原会话各列未变化、零新会话。密码资源库级用例另1通过/0失败/0忽略。整体ERP发布、全域权限等待竞争、当前续期/找回全流程、正式双端及全部Release目标仍独立结项，不能据本批通过标记全部IAM完成。

完整登记首轮实际484通过/4失败/0忽略/0筛除、652.30秒，[首轮独立读回](../qa/acceptance/free-erp-session-pool-oct7-full-independent.json)45,394项/8,812个既有企业69表保护和960项源输入一致；整体因四失败保持PARTIAL。两登录测试先验错误密码后立即用正确密码，现按服务端真实retry_after等待再试，不删除失败记录或关闭退避。盘点样本改为真实入库commit后从数据库clock_timestamp取得截点，保存原10,000,000微量、销售1,000,000微量及原单据，不用已过期的日历常量。

普通导出链接的实际修复在现`API/export_job_views.rs::visible_output_url`，由现`API/inventory_print_report_handlers/reports_exports.rs::get_export_job`调用：只有数据库存储的exit_manifests绑定文件才应用退出DELIVERED门禁；普通DONE订单/商品导出保留本企业授权链接，未完成普通任务不显示完成链接。查询失败明确500；退出manifest、当前权限及实际文件下载的既有检查继续生效。这四项独立定向各1通过；初次格式入口因导出宿主含外部子模块被拒，实际逻辑接到现有叶模块后仍使用统一入口，未绕过格式边界。

第二轮完整记录在254个已完成用例后中断，没有terminal result，见[中断记录](../qa/acceptance/free-erp-session-pool-oct7-interrupted.json)。实际检查时无验收/Cargo进程，QA集群也停止；不把部分日志认作通过。接续使用现`qa/acceptance/free-erp-session-pool-full-run.ps1`独立后台编排，固定保护库、固定构建入口/缓存/锁；pid/阶段/实际exit、前后快照及独立事实各保存，不初始化新集群或覆盖旧日志。

后台完整回归实际488通过/0失败/0忽略/0筛除，654.09秒；[独立事实](../qa/acceptance/free-erp-session-pool-oct7-detached-independent.json)PASS，49,984项核对、9,730个既有企业69表未变化及960项源输入一致。编排器末尾匹配表达式的反斜线曾丢失，误报控制器FAILED；[结果核对](../qa/acceptance/free-erp-session-pool-oct7-detached-reconciled.json)以唯一完整终态、实际exit=0和独立快照确认通过，原控制器FAIL保留，检测表达式已修正并由PowerShell实际验证，不重跑业务来替换该记录。

六个当前叶文件格式复查通过；[API包整体格式检查](../qa/acceptance/free-erp-session-pool-oct7-api-package-format.log)有25个文件差异并以`-1073740791`退出，仍FAIL，没有全包写入或压制检查。当前[仓储](../qa/acceptance/free-erp-session-pool-oct7-repo-clippy.log)/[API](../qa/acceptance/free-erp-session-pool-oct7-api-clippy.log)严格Clippy都被`erp-domain/src/trade.rs`四个过长参数列表阻断，未完成各包资格；领域输入结构/调用方及整体格式另批修复。功能通过不解除正式双端、公网、完整迁移及Release/RSS门禁，整体仍PARTIAL。

### Grok逐项结项记录

每次只领取明确功能ID及IAM任务；交付单至少逐格记录：字段/按钮、实际文件/函数、真实HTTP与数据库迁移、正常/边界/失败/并发/重试/越权独立事实、native/WASM真实操作、Release负载和资源、源码/产物SHA、PASS/FAIL/NOT_RUN与剩余项。数据库回读要包含正式角色/会话，不能只保护库存表；原命令和原提交出处不得覆盖。文档本次补细及引用校验不运行构建、不修改运行数据，也不把上表性能目标写成实测值。

## 十九 贸易原事实读回与具名判定输入

2026-10-07接续T02/I02/F03。原trade四个长参数列表阻断Clippy；收货判定还只接受6件/2件和固定金额，仓储汇总整企业同SKU，并按请求编号后缀推断拒绝。原7到货/5放行/2退货/应付3000分的闭合样本实测0通过/1失败，日志为qa/acceptance/free-erp-trade-facts-original-test.log。现以来源守恒判定，不能仅改参数结构保留样本常量。

### 功能和实际代码位置

| 具体功能 | 实际位置 | 验收方向 |
| --- | --- | --- |
| 具名事实 | DOMAIN/trade.rs的ReceiptOffsetFacts、ShortSuggestionFacts、DatedSuggestionFacts、ManualChoiceFacts及原四函数；REPO/shared_contract.rs六个分配调用 | 微量、分金额、前后库存和行数按字段构造；后三规则保持原约束。收货支持非样本量、部分放行、合法领用和已知零成本，不压制lint |
| 原来源与同快照 | REPO/shared_contract/receipt_offsets.rs::read；SharedContractRepo::read_purchase_receipt_offset委托；PurchaseReceiptOffset输出 | 企业/receipt_id/po_id/line_no与原数量绑定；单SQL同一快照读质检、退货、源库存和AP，不拼不同提交时刻 |
| 来源库存 | 同SQL按quality_id的source_line及仓库/库位/SKU/批次过滤 | held=arrived-released，on_hand=released-returned-issued；同SKU另一收货不混入。issue_rows为本来源负流水减真实退货行，越退按累计原退货查，不看request_id后缀 |
| 实际金额 | 原receipt/supplier/CNY/AP的PURCHASE_RELEASE/PURCHASE_RETURN及shared_supplier_returns | payable=真实放行入账-实际退货成本，金额/行数对应；保留原分次舍入，无入账不能当已知零。其他币种、成本层及累计舍入写入另验 |
| 当前授权与错误 | ReceiptOffsetActor从API/onboarding.rs原GET传user_id/sid，同SQL查用户/会话/代次、数据库角色、仓库授权；ReceiptOffsetError明确分类 | 400身份、401缺会话、403跨企业、404缺失/当前范围不可读、409源事实需核对、500实际SQL失败；旧令牌撤权拒绝，读取零写入，无空/零替代 |
| 数据契约 | openapi/src/components.yaml、paths.yaml与生成包 | query只有quality_id和路径企业，不收客户端数量成本；新增issued_qty/released_cost_cent，status为OFFSET/NO_OFFSET；Rust13字段、源/生成包及错误范围分别核对 |
| 查询索引 | migrations/0296_receipt_offset_read_indexes.sql；复用已有库存source_line索引 | 退货按tenant/receipt/created_at/return_id包含量/成本；AP按tenant/source/kind/currency/partner/source_type包含金额。仅保护QA已应用/读回，生产与全链迁移未验 |

### 可复算样本与剩余边界

现crates/erp-api/tests/free_erp_acceptance/trade_facts.rs以真实密码角色会话调用正式GET；本批UUID夹具直接构造已存原事实，不据此宣称完整采购录单/质检放行写流程完成。

| 样本 | 实际输入与操作 | 独立预期 |
| --- | --- | --- |
| TF-U01 非样本/舍入 | 7到货、5两次放行，单价1599分；每次入账3998，共7996。退1.25/成本1999，领0.5，同SKU另11件收货 | 首笔held2/on_hand3.25/应付5997分/领用行1/越退0；第二笔11独立。不将合计5件重算7995抹平原7996；另企业403，零读取写入 |
| TF-U02 已知零 | 2到货/1放行/0成本，本源真实AP一行0；另建缺AP源 | 有记录200/NO_OFFSET/0分，缺证据409，不删除第一笔真实零凭据 |
| TF-U03 错金额 | 7/5/2原源，放行5000/退2000，只将本批夹具应付改3001 | GET409，不返回假OFFSET或让前端修补，零新库存/回执/金额 |
| TF-U04 撤权 | 真账号先200，撤本批仓库；恢复后撤数据库角色，同令牌读 | 两次404，原业务不变；受控QA授权表变更，RBAC写HTTP和完整在途写竞争仍另验 |
| TF-U05 SQL故障 | 本批qa_receipt_fault_UUID schema/view只对其企业/quality_id真实抛异常，正式身份仍有效 | 500存储失败、零业务写入，不改public触发器或停他人；自有schema留作排障工件 |

修前领域反例FAIL保留；修后领域84项/严格领域Clippy PASS。首三个真实读取通过，51,802项核对/10,349个既有企业73表保护及源窗口一致；补授权/存储错误后五项通过，51,850项/10,353个既有企业73表保护及源窗口一致，见[实际读回](../qa/acceptance/free-erp-trade-facts-authority-independent.json)。原6/2经典HTTP仍1通过。[契约核对](../qa/acceptance/free-erp-trade-facts-openapi.json)PASS；首脚本遗漏schemas根层的错误保留，修正未改业务。完整493项与仓储/API严格门禁另按终态登记。

### 性能与发布目标

沿ERP-P02/P06/P12/P18，Release、4vCPU/8GiB服务基线及80ms RTT，下列NOT_RUN。10万收货/质检、100万库存/AP背景；每源1/10/1000个退货或放行入账分别固定。正常/零/错金额/无权/SQL故障分组，至少1000个成功原样本、20并发乘100轮及20RPS30分钟，另有有界MRP/导出。负载不是新增免费单据量限制。

来源点查P95不超过500ms/P99不超过1秒，明确拒绝同预算；非预期失败低于0.1%，来源混入/越权/读写入为0。计入HTTP、会话/权限、池等待、单SQL、复算/序列化，保存源索引EXPLAIN ANALYZE/实际扫描/返回字节、连接峰值和RSS（API整体不超过2GiB），源窗口/数据摘要完整。4.38/6.74秒为三/五样本debug总耗时，0.46秒为原单样本，不证明分位数、全迁移或正式native/WASM通过。

### 当前整批验收

本批完整登记回归493通过/0失败/0忽略/0筛除，710.41秒；后台控制器实际functional_exit=0及independent_exit=0，见[整批独立读回](../qa/acceptance/free-erp-trade-facts-full-independent.json)：53,128项核对、10,361个既有企业73表不变与963项源输入一致均PASS。新的五个业务/授权/故障样本及原经典样本均实际运行，没有据单项绿色替代完整基线。

五个受影响叶文件格式PASS；领域严格Clippy再次PASS。仓储/API严格Clippy继续FAIL：仓储lib 299/lib test 301，API被仓储299及四适配器合计27项阻断；新收货叶没有本批诊断不等于包级通过。原失败、样本常量反例、首契约脚本失败均保留。正式双端、生产迁移/恢复、全域写权限竞争、完整收货/放行/退货写入及Release成本/性能仍按各自门禁结项，整体PARTIAL。

## 二十 收货写入的原成本与命令身份

2026-10-07接续T02、ERP-IMPL-17/18及第十九章。第十九章的493项证据保留；本章补放行、退货、领用的现有后端事务，业务UI和官方在线发布仍按各自门禁验收。

### 原问题和本次行为

原收货1件实际入账1分，分两次各退0.5件，旧实现每次按单价舍入1分，真实POST共冲回2分，库存0，应付/AP均为-1分。同一request_id改退货数量还返回200/replay。完整原反例日志为qa/acceptance/free-erp-receipt-values-original-readback-test.log；首次权限未齐和首次独立SQL括号错误日志均另保留，不充作业务反例通过。

现按实际放行成本建立不可改写批次，退货和领用按批次先入先出共同消耗。每批本次金额为round(原批成本×累计消耗量/原批数量)减已消耗金额；数量为微量、金额为整数分，宽整数计算，尾差归最后一次消耗。后续放行新增批次，旧批成本、分摊和回执不重算。

### 功能、实现位置和逐项验收

| 具体功能 | 现有实现位置 | 验收方向 |
| --- | --- | --- |
| 一个请求一个采购动作 | API/procurement_handlers.rs::save_purchase_revision_request | 到货/放行/退货/领用/原收货/修订只能选一项；混合返回400，0业务效果。沿用既有POST purchase-orders入口，正式单据页面另验 |
| 请求内容身份 | REPO/shared_contract/receipt_values.rs::Input::identity/post；原三个SharedContractRepo方法委托 | SHA256绑定版本、企业、quality_id、动作、数量及原payload；同请求改数量/动作/原来源409，无第二次效果。设备/sid不作为业务效果身份 |
| 并发与提交 | 同叶post；先请求锁后原收货锁，锁住质检行；复用command_receipts命名空间 | 放行/退货/领用共用原来源序列；数量、库存流水、AP、退货、成本及回执同事务。成功同时有真实效果和原回执；存储失败500且所有业务效果回滚 |
| 实际放行批次 | 同叶post；shared_receipt_cost_lots的RELEASE行 | 每次保留实际数量和AP入账金额；两次0.5件各入1分保留2分，不能重算整件1分。存stock/AP/command关联，数量超过到货或金额溢出拒绝 |
| 累计原成本冲回 | 同叶allocate；DOMAIN/trade.rs::CostReturnInput/cumulative_return_cost | 每批用实际原金额，不逐次重算单价；1件/1分退两半得到1分、0分；最后剩余为0，已冲回金额不得超过源成本。跨批先入先出，耗尽旧批尾差后再用新批 |
| 领用占用成本 | 同一shared_receipt_cost_allocations，action为ISSUE | 领用和退货共享批次余量；领0.5/成本1分，再退0.5/成本0分，库存0、应付1分。只领用不冲AP，不借退货拿回已领用成本 |
| 已存旧来源接续 | 同叶retain_legacy；LEGACY_BALANCE及legacy_facts | 先独立核对原收货、源库存、实际AP与退货守恒；无历史领用时冻结实际剩余量/金额及来源指纹。有旧领用但没有原成本分摊、错金额、缺AP或超回原成本时409，历史行不改，不填假0 |
| 旧弱hash回放 | 同叶legacy_replay | 原SUCCEEDED回执以实际请求库存及唯一原AP/退货时间配对证明动作和数量；相同内容返回原结果且0新写，改数量/来源/动作或无法唯一证明时拒绝。保持旧hash/原请求号/原金额，不新增独立回执命名空间 |
| 读回复算 | REPO/shared_contract/receipt_offsets.rs::read；migrations中的receipt_value_integrity | 新退货以实际批次分摊和来源关系判断，合法0分尾差能读；旧单价规则只用于未登记旧退货。原13字段接口保持；新命令源库存/AP/原回执或批次分摊不符时409 |
| 不可改写与提交约束 | migrations/0297_purchase_receipt_value_sources.sql | 新命令、批次、分摊拒绝update/delete；跨企业外键，原库存/AP/退货关系，批次累计数量/成本与完整消耗尾差在提交检查中核对；不把触发器替代当前用户授权 |
| 指纹时区稳定 | migrations/0298_purchase_receipt_fingerprint_timezone.sql | 原sqlx来源指纹按UTC；固定函数时区，UTC/上海会话复算一致，保留旧指纹/旧流水/金额。指纹用于变化检测，业务请求身份另用SHA256 |
| 拒绝与故障留证 | API上述分支；qa/acceptance/free-erp-receipt-values-run.ps1及verify.py | 正式路由真实密码身份，独立数据库读回，记录原失败、确实通过的命名用例、实际退出和源输入；SQL错误500不伪装业务冲突或成功。后台验收要求PowerShell7，仍走统一入口/固定缓存/锁 |

新表：shared_receipt_value_commands保存动作、数量、金额、原结果/hash及stock/held/balance/return关联；shared_receipt_cost_lots保存冻结批次及旧来源基线；shared_receipt_cost_allocations保存逐批消耗的数量、成本和序号。按tenant/quality及lot建立查询索引，迁移仅保护QA应用/读回，生产应用、备份/恢复与退出导出须包含三表后另验。

旧响应的amount在放行表示剩余待检微量、退货表示冲减分；领用left表示剩余微量。Grok接页面时须按动作明确单位，不能把放行amount当金额。拟UI/pages/trade/purchase_receipts.rs复用DataGrid、稳定quality_id/lot_id及已有查询/草稿契约；这是待接页面位置，本批未建页面。

### 可复算验收样本

| 编号 | 具体操作 | 独立结果 |
| --- | --- | --- |
| RV-U01 | 原1件/实际1分，各退0.5两次 | 两笔1分/0分；累计退1件/1分，库存/AP/应付0，两个真实原回执 |
| RV-U02 | 同请求先退0.25，再改0.5 | 首次200；重试409；库存、AP、退货、命令、批次、分摊全列不变 |
| RV-U03 | 原1件/1分退半件；后放行1件/1分；重放原退货，再退剩余1.5 | 原重放仍1分/0新效果；最后冲1分，旧半件0分尾差保留；总实际放行/冲回2分，应付0 |
| RV-U04 | 待检1件分两次放行0.5，两次各入1分，再分别退半件 | 两个放行批次各1分、两笔退货各1分；实际总2分不改写成1分，库存/应付0 |
| RV-U05 | 原1件/1分领半件，再退半件；重放领用并改成退货 | 原领用成本1分，退货0分，库存0、应付1分；原领用left不变，改动作409且0写 |
| RV-U06 | 显式原成本0、实际AP0的源退两半；另源有旧领用但无原领用成本 | 第一源两笔0分合法；第二源409，0成本批次/分摊/新效果，不把未知当0 |
| RV-U07 | 20个不同请求同时各退半件，原余量1件/1分 | 2成功、18冲突；共退1件/1分，2命令，库存/AP/应付0。只是单轮功能竞争，尚非P99负载验收 |
| RV-U08 | 保存真实旧弱hash/半件退货源；原内容重试、改成四分之一，再退尾部 | 原内容200/原1分/0写；改数量409；尾部实际0分。原hash/回执/金额保留 |
| RV-U09 | 自有企业专属QA触发器在库存/AP/退货之后、命令入库前故障；移除自有故障后同ID重试 | 首次500，全业务表零提交；同ID可200且仅一次1分；不改其他企业或停别的进程 |
| RV-U10 | 同ID换同企业另一原收货；混合放行/退货；20个相同ID各退半件 | 换来源409，混合400，0写；并发仅1存储/19原回放，不重复冲回原成本 |

实际用例在API/tests/free_erp_acceptance/receipt_values.rs、receipt_value_edges.rs。直接构造自有旧原事实，后用正式密码身份调用现有放行/退货/领用及GET路由；不以夹具证明采购单录入、原价格准入、双端、公网或完整库存/总账闭环已完成。

### 性能与尚未完成的准入

沿ERP-P04/P05/P06/P12。以下为Release目标，当前NOT_RUN；10万收货/质检、100万库存/AP/成本背景，单源1/10/1000个放行批次及消耗记录分别跑，4vCPU/8GiB、80ms RTT，包含真实密码会话、当前权限、池/锁等待、所有SQL/触发器、提交和序列化。

| 负载 | 指标 | 证据要求 |
| --- | --- | --- |
| 单项20/100行单据 | 提交P95不超过1/2秒，P99不超过2/4秒 | 完整单据服务及主从UI接好后跑；当前单原收货接口的10用例不得代替100行单据 |
| 同库存/同原来源20并发乘100轮 | 每请求明确结果P99不超过3秒；越量、负应付、超原成本、重复效果为0 | 成功/业务冲突/同ID回放/存储失败分组；实际源读回、开始/提交时间、连接峰值、锁等待及FIFO尾差 |
| 正常/已知零/错金额/旧来源/无权/故障各组 | 点查P95不超过500ms，P99不超过1秒；意外失败低于0.1% | 20RPS持续30分钟及至少1000成功样本；source索引EXPLAIN ANALYZE、扫描行/返回字节、API RSS不超过2GiB，不借本地切片或debug总秒数通过 |

继续实施顺序：原到货价格绑定已准入采购来源并区分缺失/显式0；当前actor/sid/代次/角色/实际来源仓库在写事务内核对及撤权竞争；新三表进入备份/恢复/退出导出；整体库存/成本/总账来源接通；双端真实录单/保存/错误路径；再做Release负载和生产迁移。已有正例权限不代表全域在途写授权完成，领用冻结成本不等于已入总账。

### 当前证据与交付状态

10个正式路由写入样本全部通过，13.48秒；[独立读回](../qa/acceptance/free-erp-receipt-values-edges-pwsh-independent.json)55,295项核对、10,994个既有企业76表全列不变、968项源输入窗口一致PASS。分摊按独立Python整数模型和原库存/AP/退货重建；旧来源指纹另按UTC序列化复算。

六个受影响叶文件格式PASS。严格仓储Clippy lib 297/lib test 299，API被仓储297及四适配器27项阻断仍FAIL；新收货叶没有本批诊断不等于包级通过。首次PowerShell5控制器中断/缺验证脚本的实际记录保留；PowerShell7登记入口10项和独立核对已实际完成，不清缓存、不绕锁。

完整登记回归503通过/0失败/0忽略/0筛除，556.83秒；PowerShell7控制器实际functional_exit=0及independent_exit=0。[完整独立复核](../qa/acceptance/free-erp-receipt-values-full-independent.json)56,595项，11,004个既有企业76表全列不变、968项源输入一致PASS。整批日志和来源摘要保留，调试总耗时不作性能分位数。

本批交付汇总为[收货写入修复记录](../qa/acceptance/free-erp-receipt-values-repair.json)。整体免费ERP、正式双端、公网、原价格准入、全域写权限、生产迁移/恢复、新三表的备份/退出导出与Release性能仍PARTIAL。

## 二十一 收货当前授权与操作者原证据

2026-10-07接续T02/C03及第二十章。原收货事务只用令牌权限和来源锁：撤仓库/角色后，放行、退货、领用三个动作仍200并入账；六个真实锁等待请求在已提交仓库撤权、角色撤权、会话撤销、代次变化、账号禁用或过期后仍200。原4用例0通过/4失败、9.40秒；[独立原反例](../qa/acceptance/free-erp-receipt-authority-original-independent.json)35,015项确认实际未授权效果，11,639个既有企业76表不变，969项源输入一致。原失败不覆盖。

### 功能、具体位置与提交边界

| 实施项 | 当前位置 | 验收方向 |
| --- | --- | --- |
| 必须有真实操作者 | REPO/shared_contract/receipt_authority.rs::ReceiptValueActor；三个SharedContractRepo收货动作的必填参数 | authentication_tenant、user_id、session_id、credential_epoch、warehouse_id都具名；不提供无会话/管理员/测试默认构造绕过。内部直接调用也用实际账号、服务端会话、角色和仓库授权 |
| 经验证令牌代次 | API/auth_context.rs::VerifiedCredentialEpoch；API/auth_layers.rs验证令牌后写Extension；API/procurement_handlers.rs::create_purchase_order/save_purchase_revision_request | 代次只由已解码验证令牌传入；不读业务body。无sid/无代次401；管理员标记不能代替业务会话，跨认证企业403 |
| 当前角色即时生效 | REPO上述叶lock；API收货三个动作交给数据库守卫 | 实际user_roles/role_permissions匹配purchase_orders.write或PROCUREMENT.ORDER.WRITE；旧令牌缓存不能延续撤权，也不能拦截数据库新授予的权限。其他采购分支继续原权限契约，不据此宣称全域RBAC完成 |
| 锁等待后再准入 | REPO/shared_contract/receipt_values.rs::post；receipt_authority.rs::lock | 先原请求/收货序列，之后读当前身份；用户→企业→会话→角色/权限→仓库共享行锁，原质检来源锁定。遵循现有改密写入用户后推进企业代次的顺序，不反向等待 |
| 账号及凭据 | 同叶lock/recheck；users、tenants、auth_sessions | 当前账号ACTIVE；会话归属同企业/用户、ACTIVE、未到期；令牌/会话/企业代次一致。禁用、撤会话、过期、代次不符401，不用过时验证结果补入账 |
| 实际原来源仓库 | 同叶lock；receipt_values.rs原来源查询/锁；user_warehouse_access及warehouses | 真实仓库归属及实际授权必须存在；请求warehouse_id必须匹配quality_id原仓库。授权不足或来源不匹配404/0效果，不能忽略仓库选择或按管理员扩范围 |
| 提交前有效期 | receipt_authority.rs::Permit::retain/recheck；0299的延迟提交约束 | 时间不会被行锁冻结；库存/AP/退货/成本/回执SQL之后再核对有效期，提交末约束再查。PR401明确映射401，全事务回滚；不把时间过期当SQL成功或补假Actor |
| 原操作者保留 | migrations/0299_purchase_receipt_current_actor.sql的shared_receipt_value_actors；Permit::retain | 原request关联认证企业、用户、sid、实际仓库、代次、匹配角色/权限、策略版本及授权时间；不可改写/删除。旧元数据不回填假身份，用户/会话/角色后续生命周期不抹掉原快照 |
| 多端/旧来源回放 | receipt_values.rs的原命令身份/legacy_replay，回放前后均当前准入 | 业务hash继续绑定原企业/来源/动作/数量，不加入sid或把会话刷新当新效果。设备B有当前权限可回放设备A原结果；A被撤销后拒绝A，原Actor与回执保留，0新业务写 |
| HTTP错误 | API/procurement_handlers.rs::receipt_value_error；ReceiptValueAuthorizationError | 401当前身份、403当前角色/认证企业、404实际来源范围、409业务内容/数量/成本冲突、500真实存储失败；保留草稿及原请求号，不按错误文案或后缀推断成功 |

新Actor表为第77张保护表；它关联不可变业务命令，不把可撤销会话/角色做成阻止正常生命周期的历史外键。0299仅保护QA应用/触发器定义读回，0旧身份伪回填。备份、恢复及退出导出需包含当前四张收货成本/身份表，生产迁移和全链恢复另验。

### 可复算用例与实际证据

源用例：API/tests/free_erp_acceptance/receipt_authority.rs。13个自有真实密码账号夹具对应8个登记用例；成功身份不自行签Claims。两个管理员/错误代次令牌仅用于负向边界。等待不是睡眠猜测：按目标收货的pg_locks/pg_stat_activity确认真实写请求已经等待，才提交撤权并释放门闩。晚到期用例在实际Actor插入前的自有故障触发器持有专用原生锁，独立观察到它后才认可SQL后过期路径。

| 编号 | 实际动作 | 独立预期 |
| --- | --- | --- |
| RA-U01 仓库撤权 | 登录后删除本企业仓库授权，分别放行/退货/领用 | 三个404，各次业务源、库存/AP、成本及回执/Actor全列0变化 |
| RA-U02 角色撤权 | 真令牌仍带旧权限，数据库撤采购写权限 | 三个403，0业务效果；不凭令牌继续入账 |
| RA-U03 仓库不匹配 | 同账号有两个真实仓库授权，请求仓库B但原质检来源在A | 三个404，不能按A静默执行B操作，0效果 |
| RA-U04 实际在途撤销 | 原源锁挡住真实POST，确认等待后分别撤仓库/角色/会话、推进epoch、禁用、设过期 | 404/403/401/401/401/401，六组全0效果；没有关闭其他服务或伪造回执 |
| RA-U05 双设备原结果 | 同账号真实桌面A/Web B，A成功，随后撤A，B重放同ID，A再重放 | B200/原金额/原Actor A/0新写；A401；账号仍一个、SID真实不同 |
| RA-U06 新权限 | 真Web令牌原来没有采购写权限，随后数据库授予规范大写权限码 | 同旧令牌200；记录实际匹配权限PROCUREMENT.ORDER.WRITE，不以缓存拒绝当前合法权限 |
| RA-U07 管理员与代次 | 自有负向签名令牌分别无sid/cep、sid合法但cep错误，带管理员标记 | 两次401，0效果；不据合成负例替代真登录正例 |
| RA-U08 晚到期回滚 | 有效会话已完成库存/AP/成本/回执SQL；自有Actor插入延迟锁被独立观察，期间自然到期 | 401；源数量/金额、回执、批次、分摊及Actor全0提交，故障仅该企业且自有触发器/函数移除 |

8项正式路由专项全通过，17.96秒；[首轮独立复核](../qa/acceptance/free-erp-receipt-authority-first-independent.json)58,529项、11,648个既有企业77表不变及972项源输入窗口一致PASS。完整登记回归511通过/0失败/0忽略/0筛除，726.59秒；[整批独立复核](../qa/acceptance/free-erp-receipt-authority-full-independent.json)59,866项、11,661个既有企业77表不变、972项源输入一致PASS，控制器实际两项退出0。该整批输入为旧采购契约夹具修正之前的确切版本，生产守卫代码与后续夹具修正分别留证。

登记shared_contract采购契约首轮13通过/5失败。三个直接调用的自有辅助函数把inet_server_addr的CIDR文本误作主机地址；两个HTTP正例使用无会话管理员Claims，实际401。地址改host(inet_server_addr())仍限定数据库/角色/IP/端口；正例迁到实际账号/密码/服务端SID，原采购、并发和独立数据库断言保留。修后18通过/0失败/0忽略/382筛除，14.04秒；[采购契约独立复核](../qa/acceptance/free-erp-receipt-authority-contract-fixed-independent.json)36,994项，12,310个既有企业77表不变与972项源输入一致PASS。控制器因拒绝覆盖既有格式日志而原FAILED保留；实际test退出0、另做7叶格式PASS及补全after快照分别核对，不重跑或隐藏原失败。

### 性能、依赖与剩余结项

沿ERP-P04/P05/P12，第20章负载与4vCPU/8GiB、80ms RTT继续生效：20/100行提交P95不超过1/2秒、P99不超过2/4秒；20路同来源竞争乘100轮明确结果P99不超过3秒，越权/超量/超原成本/重复效果0。新增授权、当前角色/仓库SQL、行锁等待、Actor保存、提交约束及错误序列化均计入，不只测业务核算。正常、撤权等待、令牌缺凭据、过期、合法重放、新授予权限分别至少1000样本和20RPS持续30分钟；记录连接/等待峰值、EXPLAIN/扫描量/字节、CPU及API RSS不超过2GiB。Release仍NOT_RUN；17.96/726.59秒均为debug总耗时，不是分位数。

七个受影响叶文件格式PASS。当前严格仓储Clippy lib 297/lib test 299与API依赖仍FAIL。进一步登记直接工作流0通过/1失败，0.80秒，在本批两个追加Actor辅助调用之前终止：待检收货的mfg_qty_ledgers记录实物6件，旧断言要求0；实际mfg_receipts的HOLD及available_micros=0独立读回，不能将实物流水与可用量混同，也不能仅放宽断言通过。原失败为qa/acceptance/free-erp-receipt-authority-contract-workflow-direct-purchase.log，后续应补同一来源的实物/冻结/可用及放行前后守恒验收。

完整511项与修后18项采购契约的生产输入相同；两项测试身份文件修正后，18项按最新972输入另验，未隐去全仓输入变化。交付[当前授权修复记录](../qa/acceptance/free-erp-receipt-authority-repair.json)。下一步仍是直接工作流语义核对、原到货价格准入及缺失/显式0区分、其他业务在途写授权、四表备份/恢复/退出导出、库存/成本/总账来源闭环、双端、公网及Release。该三动作当前授权和操作者修复不把整个免费ERP或全面IAM标成完成，整体PARTIAL。

## 二十二 采购到货成本、当前版本与原结果核对

2026-10-08接续T02、I02、O01及第二十一章。正式密码、服务器签发会话和存储角色调用既有采购POST，原接口在缺失成本、把200分伪改0/999分、最新版本关闭后回退旧OPEN、同ID改数量五个场景都返回200；独立库存/原收货/价格读回确认前三种存入错误成本。合法明确零价和已提交原结果重放另列正例。首轮夹具误把共享版本分支的百万分数量写成20微单位，未到目标成本准入；原3通过/4失败和PARTIAL保留。修正为20,000,000微单位后2通过/5失败，[独立原反例](../qa/acceptance/free-erp-workflow-oct8-arrival-original-units-independent.json)61,692项核对、12,329既有QA企业80表不变、974项输入窗口一致，状态CONFIRMED_UNSAFE。

### 代码位置与实际行为

| 位置 | 当前行为 | 尚未结项 |
| --- | --- | --- |
| `crates/erp-api/src/procurement_handlers.rs::save_purchase_revision_request` | 到货成本必须显式非负；缺失/null不默认0；来源价不符或最新单关闭409，存储故障500；原结果缺证明返回409、`purchase_arrival_reconcile_required`及明确核对标记 | 到货/单纯接收/版本写入的当前在途actor、仓库/库位与货品来源权限仍需接线 |
| `crates/erp-repo/src/shared_contract/receipt_arrivals.rs` | 先锁原采购，再锁原行，与修订/关闭/接收串行；只取最新OPEN行；数量、原单价、HELD流水和命令回执同事务；数量加法检查溢出 | 采购货品/仓库/库位实体绑定、不可变原版本快照、权威精确库存与必需审计仍有独立任务 |
| `SharedContractRepo::{arrive_purchase_receipt,receive_purchase_line}` | 到货复用具名输入；纯接收共用采购→行的锁序和最新版本判定，不复用历史OPEN写新数量 | 纯接收的完整业务指纹与可核对原接收映射仍待补，不能扩称全部采购命令身份已完成 |
| 到货`purchase-arrival-v2`身份 | 指纹取企业/原采购/行/仓库/库位/SKU/批次/微量/原单位成本；不取客户端自报hash或SID。新旧重放都核对原收货与真实HELD效果；已关闭仍可读取已证明原结果 | 新指纹不等于物理效果证明；其他共享命令/消费者仍独立核对 |
| `openapi/src/{components,paths}.yaml`及生成包 | 明确条件成本、数量单位、七种既有POST成功形态、409核对状态及422/500；保留现有重载分支，不无声改单位 | 当前`revision_action`的`items[].qty`为百万分量，普通录入为整单位；正式客户端须明确适配，后续独立治理重载契约 |

旧弱hash只按实际原收货、原来源行与原HELD流水校验，重放保留原hash/ID/结果，未改成v2。扩展实测发现：复制正确v2意图hash但引用不存在的原效果，仍被旧实现当作200重放；[原证明反例](../qa/acceptance/free-erp-workflow-oct8-arrival-proof-before-fix-independent.json)9通过/1失败及独立61,790项保留。修后无论hash版本都需要真实原效果；缺证据保持原命令和业务事实，返回明确需要核对，不制造成功效果或清除历史。

### 实际验收与余下门禁

- 十项专项10通过/0失败/0忽略/511筛除，9.74秒；[专项读回](../qa/acceptance/free-erp-workflow-oct8-arrival-proof-fixed-independent.json)61,839项、12,353既有QA企业80选定表全列保护、975项输入窗口一致PASS。
- 登记`free_erp_acceptance`完整521通过/0失败/0忽略/0筛除，573.64秒；[整批独立读回](../qa/acceptance/free-erp-workflow-oct8-arrival-proof-full-independent.json)63,186项、12,363既有QA企业80选定表全列不变、975项输入一致PASS。复核流式保留本批事实及全部旧企业摘要，不减少全列比对范围，见[流式再核对](../qa/acceptance/free-erp-workflow-oct8-arrival-proof-full-recheck-independent.json)。
- 追加登记`shared_contract`采购目标18通过/0失败/0忽略/382筛除，10.97秒，实际日志[另留证](../qa/acceptance/free-erp-workflow-oct8-purchase-contract.log)；它是局部契约结果，不扩称全部shared_contract通过或其后全部数据库再读回。
- 四涉及叶模块格式检查通过；OpenAPI源/生成包和v2门禁通过。19成功、6冲突、1校验拒绝，共26份实际采购响应符合新契约，[运行响应复核](../qa/acceptance/free-erp-workflow-oct8-openapi-runtime-recheck.json)PASS；最初误设30响应阈值的FAIL和验收脚本缩进检查失败保留。
- 严格Clippy仓储lib297/lib test299仍FAIL；API被仓储297及四适配器7/6/5/9项阻断。没有屏蔽warnings、整体格式迁移、清缓存、新target或生产部署。

直接整链夹具已补实际不良报工1件后报废1件/1000分，验证重复不增、超不良及缺报工拒绝且0回执；质检改为三项有原来源的冻结标准/有效读数，审批重放一份原证明，审批后实物仍HOLD/可用0，必须另做实际库存放行。[冻结质检局部事实](../qa/acceptance/free-erp-workflow-oct8-frozen-quality-independent.json)61,620项及12,321原企业78表保护通过；整链随后仍在旧不良处置缺真实精确库存效果处失败，未放宽该守卫。

继续推进真实不良处置/库存效果、源货品/仓库/库位和在途授权、原版本快照/日志及库存成本闭合，再接正式双端、托管和Release门禁。本批为受保护本地QA/Axum正式处理器的功能与数据证据；调试秒数不是P95/P99，整套免费ERP继续PARTIAL。

## 二十三 原收货质量处置与权威库存接续

2026-10-08接续Q02/M13/I02，实际源码为`shared_contract/receipt_dispositions.rs`与API `manufacturing/receipt_dispositions.rs`。原入口只按`mfg_quality_holds`计数写独立数量，不能证明原实物；原整链的精确库存守卫失败保留，未撤销守卫。

现POST `/quality/dispositions`明确`receipt_id`，可选原`parent_disposition_id`仅引用同收货ISOLATE分配；仓库/库位/SKU必须匹配原收货、原有效报工、待检记录、数量/共享投影及权威库存证据。工作单→收货→账号/会话/当前角色/仓库→库存统一锁序，免费同账号跨端的业务指纹不含SID。处置结果/原快照不可变；原结果重放不重入账，也不改原收货事实。GET `/quality/receipt-dispositions/{receipt_id}?warehouse_id=...`返回当前待检/隔离/可用/报废/返工数量，校验总量守恒，原到货HOLD标签不冒充当前冻结量。

| 动作 | 实际效果 | 边界 |
| --- | --- | --- |
| ACCEPT | 从原待检或隔离分配解冻，权威实存不净增加 | 必需标准/实测及当前检验结果仍须通过 |
| ISOLATE | 实存/冻结总量不变，保留明确隔离分配；可继续处理其余量 | 不是报废或默认终态；深度最多32 |
| SCRAP | 扣原实存和冻结，按冻结的已确认原成本累计分摊尾差 | 缺成本拒绝；明确0价与缺值分开，未宣称总账过账已完成 |
| REWORK | 原冻结实物减少，真实目标领料/持料及原价值对应记录 | 目标需已下达、有容量、引用原工单；同时要求生产写权限；后续耗退及价值总账仍待接 |
| CONCESSION | 当前独立QUALITY.CONCESSION授权及已测量的完整标准证据，记录授权/失败读数/原因后解冻 | 缺测不能借权限跳过；普通放行仍拒绝失败读数 |
| RETURN_SUPPLIER | 制造收货不编造供应商与原应付，明确要求引用真实采购退货协议 | 制造/委外与采购来源统一处置仍是后续任务 |

库存服务同事务写真实权威效果及制造来源投影，保留原库存守卫；命令、处置、原成本快照、返工领料、必需操作/审计和Outbox失败共同回滚。处置后旧整笔生产放行拒绝二次释放，必须按真实来源余量接续。0300新增两张不可变来源表，只在受保护QA应用，无旧行回填或生产迁移。

实际用例在`tests/free_erp_acceptance/source_dispositions.rs`：部分0.5/0.75放行与原结果、隔离1后处理0.75及超量、1件/1分两半报废只共1分、缺成本/撤权零效果、真实返工0.5及100分、20路争用10成功/10冲突、存储故障全回滚/同ID再试、已观察来源锁等待后撤权、失败实测/单独授权/缺测让步拒绝。底仓原有0.9冻结量完整保留；成本来源/工单原始准备为自有QA夹具，不据此宣称完整生产成本和录单界面验收。

首轮1通过/4失败为冻结底仓断言及成本单ID夹具错误；下一轮6通过/2失败为第6绑定参数遗漏，且读取接口/格式化并行使三个源输入漂移，原日志和PARTIAL不覆盖。稳定8专项通过，65,341项保护核对、13,040既有QA企业85表不变；让步扩展9通过、65,392核对/13,048既有企业保护，源码窗口均一致。

当前完整登记`free_erp_acceptance`530通过/0失败/0忽略/0筛除，588.28秒；[独立复核](../qa/acceptance/free-erp-disposition-oct8-full-recheck-independent.json)66,753项、13,057既有QA企业85选定表全列不变、979输入一致PASS。读文件未关闭的原ResourceWarning保留；流式明确关闭句柄后同范围再次通过。OpenAPI源/生成及门禁通过，五涉及叶固定格式通过。仓储严格Clippy297/299、API被仓储297及四适配器27项阻断仍FAIL。

交付[质量来源处置修复记录](../qa/acceptance/free-erp-disposition-oct8-repair.json)。旧直接hold/处置调用方尚须迁移；原质检多样本/公差/附件、原成本生产者、采购/委外来源退回、返工耗退/价值总账、完整迁移备份导出、真实桌面/Web、官方托管及Release性能继续独立结项，整个免费ERP仍PARTIAL。调试秒数不作P95/P99或峰值成绩。


## 二十四 返工耗退的原价值与再处置

2026-10-08接续M10/M13/Q02/I02。`shared_contract/rework_material.rs`承接上一章真实REWORK领料的原处置数量和冻结成本；既有材料POST现在传真实SID、凭据代次和当前角色/仓库，仓储在同一事务重新检查。原来源工单及目标工单按UUID排序锁定，再锁账号/会话/角色和库存；质量REWORK分支的生产权限检查已移到工作单锁之后，避免同一链路反向持锁。没有当前业务actor的旧直接入口不能继续处理新增返工来源；其他内部、Worker及整单调用仍须独立迁移。

耗用引用原领料，减少车间持料，不再次扣仓库；退料把实物与冻结同步增加，恢复原库位/批次的HOLD。权威库存流水、制造投影与共享库位流水绑定，保留数据库精确库存守卫；领料的耗用和退回合计不能超过原量。0301的不可变`mfg_rework_material_values`记录实际材料move、原处置/入库、微量、分值及当前actor，真实操作/审计和制造事件共同提交；中途失败整批回滚。

分值按原REWORK分配成本与原量累计分摊，尾差只分一次。质量POST新增可选`returned_source_move_id`，明确引用本收货的真实RETURN，不能与隔离父来源同时指定；返回量可接收、隔离、再报废或再次返工。隔离子链继承对应退料分值，不能再按原整批单价重复计费。旧请求没有该字段时不加入v1业务指纹，原身份保留。GET另列累计`rework_returned_micros`供解释；它已纳入当前隔离/后续分类，不参与总量求和。

本批14专项全部通过、0失败/0忽略/521筛除，20.52秒；包括部分耗用0.25/退回0.5、当前撤权、原结果不重增、超余量、1分返工全部退回后两半报废合计1分、退回隔离后的尾差及伪退料来源拒绝、故障回滚/同ID再试。完整原成本和目标工单准备仍为明确自有QA夹具，不是成本生产者、真实录单页面或总账验收。

[专项独立再核对](../qa/acceptance/free-erp-rework-oct8-fixed-recheck-independent.json)68,950项，13,737既有QA企业86选定表全列不变、981输入一致PASS。原首轮10通过/3失败为退料投影缺权威绑定；两个原独立汇总报告误计原入库投影，失败原样保留，按真实receipt/move绑定修正读取后对同一快照再验，不修改业务数据。OpenAPI源/生成门禁通过；涉及叶格式通过。原Clippy新增1处参数数量告警已修正，重新严格仓储Clippy仍lib297/lib test299 FAIL。

首个完整登记522通过/13失败/0忽略/0筛除，598.85秒；13失败来自正常材料测试使用没有真实会话的令牌。[整批独立读回](../qa/acceptance/free-erp-rework-oct8-full-independent.json)70,336项、13,751既有QA企业86表不变、981源输入一致，因登记失败保持PARTIAL。正常材料身份及共享制造HTTP夹具已改为密码登录、真实角色和仓库授权；越权仓库预期为403，原来源/数量/结果断言保留。正在按修后夹具重新运行完整登记，不能把14专项扩称535登记或整体免费ERP通过。[本批阶段记录](../qa/acceptance/free-erp-rework-oct8-repair.json)保留PARTIAL。原成本确认/批处理的取值、倒冲/Worker及整单消费者、价值总账、完整迁移备份导出、正式双端、公网与Release仍待结项，调试总秒数不是P95/P99成绩。


## 二十五 返工原分值进入成本生产者

2026-10-08接续M12与第二十四章。`shared_contract/material_cost_basis.rs`由已有`confirm_work_order_cost`、`save_missing_fee_estimate`、`complete_cost_batch`共同调用。它核对原REWORK处置及真实CONSUME、原领料、数量/工作单和不可变分值映射；缺映射或源预算不可信要求核对，不按客户端物料价补认。已记录的小数耗用直接取原分值，RETURN不计入本次材料费用；返工来源从旧数量乘单价分支排除，避免重复计费。最多读取10,001条、超10,000明确拒绝，不截断成功。普通耗料仍保留既有单物料取价/整单位限制，依据明确标为`legacy_single_item_price`，未宣称逐物料原成本已完成。

迁移0302的`mfg_material_cost_bases`保存对应真实成本run、材料总分值、返工分值、旧未来源计价数量和冻结取值依据，追加后禁止更新/删除；与成本run/原命令同事务。没有原耗用分值的现POST返回409、`material_cost_reconcile_required`和核对标记；存储失败返回500，整笔回滚。已提交原命令仍返回原结果，不给旧run补写新依据。代码分别位于仓储新叶、原共享仓储三方法、API `manufacturing/cost_response.rs`与两条现成本/估算路由；新增表只在已核对身份的QA应用。

实际源质量/耗退/成本19专项通过、0失败/0忽略/521筛除，27.94秒。原0.25件耗用50分/0.5件退回100分，只把50分进入确认；客户端未登记的物料代码不替代已证明返工价值。1件/1分的两半耗用仍合计1分；缺耗用价值映射409/零新run；估算保留未知费用，手工准备租约后的仓储批完成取同样原50分；注入成本依据写失败，成本run与回执共同回滚，同ID随后成功。批完成用例只证明仓储完成契约，不是实际Worker进程、执行主体或租约有效性验收。

[扩展独立读回](../qa/acceptance/free-erp-rework-cost-oct8-expanded-independent.json)72,629项，14,448既有QA企业87选定表全列不变、984源输入一致PASS；前17项与72,484独立核对另留证。现有正常材料身份夹具恢复遗漏的Utc导入后7项通过；原第二次完整登记的编译失败和72,119保护读回保留，没有执行用例或伪记完整通过。本阶段完整540通过/0失败/0忽略/0筛除，645.97秒；[整批独立读回](../qa/acceptance/free-erp-rework-cost-oct8-full-independent.json)74,045项、14,468既有QA企业87表不变、984输入一致PASS。[成本阶段记录](../qa/acceptance/free-erp-rework-cost-oct8-repair.json)保持PARTIAL，须接续真实终态及独立读回。

涉及叶格式和OpenAPI源/生成门禁通过。严格仓储Clippy仍lib297/lib test299 FAIL；API严格门禁另按同一锁入口执行，不由本项通过推定成功。普通物料按真实SKU/原批成本计价、实际产出/费率/费用准入、成本的当前在途actor与必需日志、真实Worker、价值总账、完整升级/备份/退出导出、双端、官方托管、Release分位数/内存/长稳仍待补；不把原分值接线或debug秒数扩称整个M12、制造及免费上线已完成。


## 二十六 普通材料的原来源价值与成本账衔接

2026-10-08接续M10/M12及上一章。新增`shared_contract/material_values.rs`，原材料内核在真实SKU成本账的既有锁下取得移动平均位置；领料按账面总值与数量分出原金额，不用截断后的单价计算。源工作单→当前actor→成本期间/SKU/成本项→物理库存保持同一锁序；退料在库存锁前取得原成本项锁。领料扣减成本账的数量与价值，退料按原分配量/分值回填。工作单、库存、原值、成本账、操作/审计与事件和回执共同提交，消费不重复扣仓库。

0303仅在保护QA新增不可变`mfg_material_issue_values`和`mfg_material_value_allocations`。新原领料记录实际工作单/仓库/SKU/微量、成本账位置/版本与原金额；每次耗退共享原量余量，按累计比例分摊分值尾差。明确零值仍为VALUED，未覆盖数量/估计成本/缺成本记录为UNVALUED及NULL，不把它转0。后补或改价不会重写原领料；历史已耗退但缺原价值分配，继续保留未知，不拿当前价格补认。已证明的原atomic文档另通过其原库存/台账/日志/事件证明读取，来源类型保留，不冒充新SKU成本账扣减。

原直接材料路径和共享领退耗接新增价源/分配，成本三生产者仅汇总真实CONSUME的不可变分值，RETURN不计费用。旧按客户端单一物料码选价及按请求名`use-%`猜耗用的价格替代已移除；旧已提交run与回执不回填、不重算。新依据schema2列普通/返工耗用与各自分值，旧价字段为NULL。真实账号调用记录USER；没有人类actor的内部内核记录INTERNAL，不伪造操作者。部分旧共享/整单/倒冲入口及历史迁移仍须独立验收，不据静态接线宣称全路径完成。

24专项通过/0失败/0忽略/521筛除，36.78秒：两SKU按原100/300分价领料，改价后两次0.25耗用仍共100分；其余退回原值；缺价后补价仍409核对；明确0和1分钱两半耗用分开；成本账10件/1分的余值不会因单价0而丢失，5件退回恢复1分；材料审计注入失败时库存/成本账/价源共同回滚，同ID随后一次成功。生产成本总量另含夹具1分钟/1分，材料100分与总101分分别断言。成本账及报告原始准备为明确自有QA，不是完整采购/仓库计价或真实生产录单验收。

[扩展独立读回](../qa/acceptance/free-erp-material-values-oct8-expanded-independent.json)76,357项、15,170既有QA企业90表全列不变、986输入一致PASS；新增原SKU账面扣减、原价退回、真实分配与必需日志/事件绑定核对。初22专项/76,189读回另保留，首严格APIClippy含2处新增可合并if告警原样保留；修正后仓储仍lib297/lib test299、API依赖仍FAIL，未压制warnings。涉及叶格式/OpenAPI源及生成门禁通过，本阶段完整545通过/0失败/0忽略/0筛除；整批独立77,803项/15,196既有QA企业90表不变/986输入一致PASS，仍不是全部免费ERP验收。[阶段记录](../qa/acceptance/free-erp-material-values-oct8-repair.json)保持PARTIAL。

上一阶段真实540登记与74,045独立核对已终态PASS，属于其984输入历史证据；不能作为本批新价格/日志语义的完整通过证据。当前成本在途授权、实际产出/费率/费用准入、全部历史/共享/整单/倒冲与真实Worker、全采购仓库计价/总账、完整迁移备份导出、正式双端、公网及Release继续未结项，整体免费ERP未完成。


## 二十七 成本当前资格与已记录产出接续

2026-10-08接续M12/O01及第二十六章。原确认/缺费用估算两POST要求真实SID与服务端已验证凭据代次，传具名`WorkOrderCostInput`及`CostValueActor`，没有用户/会话身份不能按旧无会话令牌继续写成本。仓储工单锁后检查当前账号状态、企业代次、ACTIVE/未过期会话及存储MFG写角色；按真实材料来源和输出来源导出的全部仓库逐项检查当前范围，最多1000仓库，超预算明确核对，不取客户端自选仓库来缩窄资格。Role/范围在事务持锁，提交前重新检查时间性会话；同ID原结果也要当前资格。

`shared_contract/cost_commit.rs`按冻结工作单工序的末序号取唯一末工序，没有冻结工序时仅接受唯一已记录工序；多工序无法确定末工序则要求核对，不按最新日期猜。末工序已记录报工的净合格/不良与工序在制对应请求数量，计划量仅作上限，不把未开始的计划量当作在制。旧mfg_wip完成标记与工序在制不一致保持原记录、要求核对，不清零求通过。直接确认、缺费用估算及仓储批完成都接数量核对；产量不符、负值或超过原计划不写成本单。原回执重放不按后来的报工重算原价。

0304不可变`mfg_cost_commit_authorities`记录实际成本run/原命令、用户/SID/凭据代次、已核对仓库与角色、末工序量证明。成本run、材料依据、资格、原回执与必需操作/审计/Outbox共同提交；故障共同回滚。角色错误403、会话错误401、来源仓库不可见404、数量/原来源核对409、存储500。没有人类actor的现内部入口和Worker主体仍另验，不据公开两POST通过宣称全部内核IAM已完成。只在保护QA应用迁移，无回填/生产部署。

当前28专项通过/0失败/0忽略/521筛除，74.11秒；原/新成本请求在撤权后均403/零新效果；实际观察工单锁等待再撤权也403；CUT与FINAL各10件不相加成20，成本20件拒绝、10件按末工序存证明；资格写入故障500，成本run/依据/回执及日志共同回滚，同ID再试一次成功。修正返工夹具先用正常密码会话调用正式报工接口记录1件/0分钟，再确认来源成本；旧普通夹具实际记录10件，成本数量也用10件，不仅放宽断言或绕过新检查。

[专项独立读回](../qa/acceptance/free-erp-cost-commit-oct8-focused-independent.json)80,002项、15,880既有QA企业94选定表全列不变、988源输入一致PASS，核对实际资格/cost run/原命令/操作者及日志事件共同提交。涉及叶格式/OpenAPI源及生成门禁通过，严格仓储Clippy297/299继续FAIL；API另按原入口尝试。本阶段完整549通过/0失败/0忽略/0筛除，1177.43秒，独立81,468项/15,910既有QA企业94表不变/988输入一致PASS；[阶段记录](../qa/acceptance/free-erp-cost-commit-oct8-repair.json)保持PARTIAL。

完整工序报工/冲正与工序在制生产者自身的身份/数量证据、部分产出成本分配、费率日期来源与制造费用准入、真实Worker租约/主体、完整总账、全贸易制造/升级备份退出、正式双端、公网/免费托管及Release仍须独立完成，不把已记录数量核对或debug总耗时扩称全部M12及免费ERP上线。上一阶段545和77,803独立核对保持其986输入历史资格，不替代本批988完整基线。


## 二十八 成品、在制和不良的一池成本分配

2026-10-08接续M12/F03。静态核对原三生产者：整池材料/人工/费用作为成品单位成本，而另一函数又把材料/人工整池分给在制/不良；直接和批完成在已知费用时的去向也不一致。原函数说明及已确认旧run保留历史，不据它继续为新run重复分配金额。本批新内核`DOMAIN/manufacturing/cost_allocation.rs`与仓储`shared_contract/cost_allocation.rs`按明确的`QUANTITY_SHARE_V1`数量规则处理一池材料、实际人工与已知制造费用，成品→在制→不良按累计整数分摊，尾差只属于最后非空去向；三个金额之和等于原总池。

这是基础数量分配规则，方法版本随结果冻结，未把它当作所有行业的等价工时/异常损耗会计政策。实际材料仍按上一章原值，数量仍按已记录末工序和工序在制。本次不改价源、报工或历史成本；未覆盖的行业等价产量、成本政策配置与页面需独立实现。新成品单位值只取成品分配额，支持六位小数成品量；精确单位分值以百万分刻度保存，旧整数单位字段为可识别投影。无成品时单位字段为NULL，不用0冒充已知零价；旧正产量run原单位值及原回执不重算。

直接确认、缺费用估算、仓储批完成共同使用新分配。估算的在制/不良从记录来源取得，缺费用仍NULL及estimated=true，不把请求所无的字段默认0而抹掉来源在制。0305仅保护QA新增不可变`mfg_cost_allocations`及nullable单位列，保存真实run、规则、三数量/金额、单位投影/精确刻度和原快照，数据库检查三金额守恒，与run、材料依据、资格/回执及日志共同事务。历史run不回填。公开成本确认/估算响应单位字段相应nullable，制造到货原价读取排除没有成品单位值的run。

纯内核4测试通过，包括100分按1成品/1在制分50/50、1分小数去向尾差、0成品时空单位、全部小数量/金额组合守恒；受影响domain严格Clippy PASS。API 32专项通过/0失败/0忽略/521筛除，87.17秒：部分50/50、一半成品单位200分、0.25/0.25/0.5份1分钱尾差、仅在制无单位值、缺费用估算保留未知与原在制、原回执重放不增。独立[83,942项核对](../qa/acceptance/free-erp-cost-allocation-oct8-focused-independent.json)16,633既有QA企业95选定表全列不变、991输入一致PASS，验证实际run/不可变分配/NULL单位与总池守恒。

首smoke编译的3处新增不用变量警告已消除，首次严格仓储1处新增冗余字段写法已修，原日志保留，无lint压制；最新API依赖仍被仓储297及既有适配器阻断，未报全包严格通过。涉及叶格式和OpenAPI源/生成门禁通过；本阶段完整553通过/0失败/0忽略/0筛除，1174.66秒，整批独立85,433项/16,668既有QA企业95表不变/991输入一致PASS；[阶段记录](../qa/acceptance/free-erp-cost-allocation-oct8-repair.json)保持PARTIAL。上一阶段549/81,468读回PASS属于其988历史输入，不能充当本批991完整通过证据。

费率日期与制造费用权威来源、行业分配策略与界面、全部旧调用/真实Worker主体、后到调整/生产收货成本尾差及完整总账、全迁移/备份退出、真实双端、公网免费托管和Release分位数/内存/长稳仍未验，整体ERP未完成。小样本秒数不是性能上线资格。


## 二十九 旧成本调用方的空单位类型与两项并发接续

2026-10-08接续上一章。仅修改`crates/erp-api/tests/shared_contract.rs`中受影响测试调用方，不改生产计算/接口或数据。四项正产量成本tuple和一项并发Stored结果匹配改为Some(原单位分值)，保留原总额和并发断言；原无成品NULL不得回退0。首编译还发现遗漏的并发match E0308，实际原日志保留，修正后旧400项测试二进制编译通过。

最早筛选`cost_complete_race_read_back_body`只是辅助函数，实际0通过/400筛除，不能验收。改用登记包装测试`cost_complete_race_read_back`后实际0通过/1失败/399筛除：随机仓库/SKU没有对应企业实体，被真实库存守卫拒绝。此为旧夹具问题，未撤回生产守卫。修后在本企业创建租户/仓库/SKU，并以规范精确库存服务建立期初，再用post_material_move按原issue身份耗用、SKU原成本冻结；工单、报工、费率和已租用任务仍按原业务准备，保留两次并发只存1成本run、1原完成回执及DONE读回。

修后`cost_complete_race_read_back`1通过/0失败/0忽略/399筛除，0.93秒；[独立原值读回](../qa/acceptance/free-erp-cost-callers-oct8-complete-independent.json)验证真实材料3000/人工10000/费用10000分→23000分、单位2300分、一个原完成命令、DONE任务与仓库实物0。旧`late_cost_race_read_back`同样改规范实体/库存/原领料耗用，不改原业务断言；1通过/0失败/0忽略/399筛除，0.45秒，[独立读回](../qa/acceptance/free-erp-cost-callers-oct8-late-independent.json)保留原材料30000/人工10000/费用10000→50000分、单位5000分，一次2000分晚到调整及一个原调整回执，原run/分配不改。

这些是两个实际登记旧调用契约及其自有QA读回，不是400旧契约全部运行、真实Worker进程、全部晚到费用分配或总账验收。当前无生产源码改动，原测试root含外部模块与既有全仓格式债务，未以包级写格式扩大任务；只核对选中修改和编译/对应契约。上一阶段553完整及85,433独立读回已终态PASS，保持991源输入资格；随后测试源码变化另记录，不伪称两者输入完全相同。见[调用方接续记录](../qa/acceptance/free-erp-cost-callers-oct8-repair.json)。

其余旧工作流、价格/数量来源、当前HTTP角色/会话夹具继续逐实际调用迁移；不得以包编译或这两项通过扩称全部制造/贸易、当前权限或免费发布资格。行业策略、费率/费用来源、财务调整分配/总账、真实Worker、全迁移备份退出、正式双端/托管/Release及严格仓储/API门禁继续未结项，整体PARTIAL。


## 三十 工单中心绑定与逐报工作业日期费率

2026-10-08接续M12。现三生产者原来用成本请求center/as_of取一个费率，乘全部报工分钟；一份请求可以跨日期改变已记录作业的人工。新`shared_contract/cost_labor.rs`读取已记录每份报工的作业日和原分钟，对应冻结工序的工作中心；无冻结工序中心时使用明确的工单默认中心。按各日有效费率版本分别计值，冻结报告、作业日/中心/费率ID与原行、捕获边界及人工金额，源数量不重复改变。成本as_of仅是不能早于已记录作业日的截止检查，不再作为人工选价日期，成本body中心也要匹配已记录默认绑定。

新POST `/manufacturing/work-orders/{id}/cost-center-binding`在工单锁下使用当前SID/凭据、存储MFG权限和全部导出仓库，显式绑定企业内中心及理由；原同中心返回原快照，另中心拒绝替换。0306仅保护QA新增不可变`mfg_cost_center_bindings`、`mfg_cost_labor_bases`。初次绑定早于报告时以报告创建为费率版本捕获上界；绑定晚于已有报告时以上界绑定时刻显式配置，不能把它宣称为未记录的原历史中心/费率。已存费用run原结果/人工快照不重算；以后新增费率版本不自动覆盖原捕获范围。

有reverses_report_id的负分钟必须找到本工单原报告，继承原报告日期、中心和捕获边界，用原分钟的反向；未引用原来源的负人工或反向分钟不匹配保持记录、要求核对。三生产者均改用已证明的人工分值，分配内核接直接labor_cent，不假造1分钟×总金额。制造费用仍按基础规格允许有权限操作者显式手工确认；在当前资格/数量证据中另留输入金额和确认来源，未知为NULL，不变确定0。本项不是外部费用凭据、已过账费用或全部财务来源验收。

当前36专项通过/0失败/0忽略/521筛除；初35smoke亦通过，原日志保留。未来截止日期2026-11-01不选100分新费率，原10月8日报工仍1分；另廉价中心/重绑及早于作业日的截止均拒绝；10月1/2日报工各10分钟分别用2/3分费率，共50分，不能以最后3分费率算60；冲正即使写11月作业日也继承原10月8日1分费率，净人工0、无成品单位为NULL。自有成本账/报工的原始准备依然是明确QA夹具，不是工序生产者全验或真实企业录单。

[专项独立读回](../qa/acceptance/free-erp-cost-labor-oct8-focused-independent.json)87,916项/17,401既有QA企业99选定表全列不变/994输入一致PASS，逐报告绑定实际mfg_reports、率版本、日期区间、分值和实际run。涉及叶格式/OpenAPI源与生成门禁通过；仓储Clippy297/299、API依赖仍FAIL，没有warnings压制、缓存绕过或自动清理。557完整登记正在执行，[阶段记录](../qa/acceptance/free-erp-cost-labor-oct8-repair.json)保持PARTIAL。

旧未绑定中心的内部调用/Worker需先显式配置，不以body当前中心冒充原来源；上一章旧两并发契约通过仅保留原991阶段输入资格，不能充当新994来源协议的运行验证。完整报告/工序生产者与冲正身份、初次历史绑定的人工核对、费用凭据与总账、行业政策/收货尾差、全迁移备份退出、真实Worker/双端、官方托管与Release继续未验，整套免费ERP未完成。

## 三十一 原生产成本与来源依据查询

2026-10-08接续M12/F04与上一章。新增`GET /v1/tenants/{tenant_id}/manufacturing/work-orders/{work_order_id}/costs/{cost_run_id}`，由`manufacturing/cost_read.rs`与`shared_contract/cost_read.rs`接原成本run、材料原值、逐报工人工依据、三去向分摊、原操作者授权及工单中心绑定。按工单先锁、当前会话/凭据代次、数据库制造读权限与全部真实仓库范围检查；不借用制造写权限作为查询准入。撤仓库范围沿既有不可见404，撤读权限403，撤会话401；企业、工单、run必须共同匹配。

返回`original`、`evidence`及独立的`work_order_adjustments`。原完成量和精细单位成本使用十进制字符串，无完工量的单位成本保持NULL。后续费用仅为工单层调整，不在本查询擅自重算原单价或分配到此run。历史缺材料/人工/分摊/授权/中心绑定保留NULL及具名`evidence_issues`；记录总额、来源金额、分摊及快照不一致明确要求核对。当前GET提供原记录与内部一致性，不宣称独立审计完全部原库存/总账。每种依据最多1行、每份snapshot最多4 MiB，调整最多1000行，超限409，不能截断返回成功。0307仅补两个原表查询索引，保护身份的私有QA已应用并独立读回；没有生产DDL、回填或外部发布。

上一章557登记已终结：557通过/0失败/0忽略/0筛除，1178.68秒；独立89427项PASS，17440个原QA企业99表完整选定事实不变，994输入前后无漂移。见[原输入窗口完成记录](../qa/acceptance/free-erp-cost-labor-oct8-full-completion.json)。这只证明该次保存的源码窗口，不认证随后新增GET。新增专项首次3通过/2失败/557筛除，12.30秒；一处旧QA表没有调整ID默认值，改用实际迟到费用POST生成原ID；另一处仓库撤权实际404与测试403预期不符，保留不可见契约并修正预期。首次日志不改写。修后五专项正在受保护100表及当前源码清单窗口验收，结果另见阶段报告。

选定叶固定入口格式与API范围编译通过。OpenAPI首次Windows写入错误保留，重试打包及v2门禁通过。严格Clippy另有实际执行记录，不能由编译结果推定通过。正式DataGrid生产成本页、完整列表/查询集、实际双端操作、Worker全部身份、来源生产者全不可变契约、完整迁移/恢复、Release与官方免费托管仍待完成。

本章新增GET修后5专项通过/0失败/0忽略/557筛除，12.87秒；独立重核90906项PASS，18143个原QA企业100张选定表完整事实不变，6个本批企业已观测，997输入无漂移。核对器首次扩表误把费率字典键改成元组，失败脚本及原阶段PARTIAL保留；修复后使用同一组完整快照和原测试日志重核，未减少业务断言、重跑或改写历史事实。另保留首次重核参数误用单横线的失败日志。选定叶格式检查、API范围编译及OpenAPI重试门禁PASS；实际严格Clippy仓储297项、四适配器27项阻断，API lint未到。具体记录见[本批交付](../qa/acceptance/free-erp-cost-read-oct8-repair.json)和[独立重核](../qa/acceptance/free-erp-cost-read-oct8-focused-recheck-independent.json)。同文件直接契约回归正在追加，不将此前557原窗口冒称当前562登记全部通过。

本批收尾：当前同文件41直接契约全部通过/0失败/0忽略/521筛除，116.07秒，包含五项新增GET及原成本写入、材料/人工、授权撤销和质量来源契约；997源码输入与受保护窗口一致。该41项的通过不扩大五专项100表独立窗口，不冒称当前562全登记回归完成。整体免费ERP仍PARTIAL，下一阶段接正式贸易操作页面与真实双端闭环。

## 三十二 贸易录单的多行精确数量接续

2026-10-08接续T03/ERP-IMPL-19。核对正式宿主时发现`POST /sales-orders`带request_id后转到单行分支，revision_action分支仅取items.first，且旧qty字段只有i32。当前新增`API/sales_orders/quantity.rs`与`revision_lines.rs`，修订分支接全部1..1000行；稳定line_no必须为正且唯一。新文本qty按基准单位读取，最多六位小数；旧数字修订qty保留已有微单位含义，不能静默乘一百万。另一个单行精确销售分支的数字qty仍为基准单位；旧普通整数单据入口明确拒绝小数，不能截断。逐行税额、折扣、交期、地址与RESALE/MAKE均进入原修订；多行非零表头税拒绝，避免重复计税。旧普通行金额默认计算改为checked运算，不以饱和值保存溢出金额。

修订POST补orders.write检查，对全部有效行、客户、实际动作及期望版本生成SHA256指纹，命令ID不再兼任内容指纹。`SharedContractRepo::save_sales_revision`先锁原命令，再分配订单身份及取得原订单锁，10路同原ID只一份修订、一份原回执、两条明细；不同第二行价格的同ID重试409。已有历史请求ID字面指纹不被改写或补认；无法证明同内容的旧请求保持冲突。相关锁参数必须绑定文本租户ID。

首专项1通过/3失败/0忽略/562筛除，4.89秒；新增锁误将UUID二进制绑定到文本参数，三次正常保存及同命令竞争均在提交前被数据库拒绝。原日志保留，修后4专项全部通过/0失败/0忽略/562筛除，5.09秒；涵盖两行1.25/2.000001、逐行5/7分税额、第二行变价重试拒绝、末行七位小数/重复行身份/表头税拒绝后0行/0回执，以及10路全部原结果。独立psql56项PASS，读取4个自有QA企业的修订、明细、原回执与需求；同一测试窗口1000源码输入无漂移。该独立范围不是全部既有企业保护扫描、正式页面或Release。三个选定叶格式检查PASS；新增公开POST契约打包/v2门禁PASS，契约随后变动另有清单，未冒称此前测试认证新文档运行版本。

本批实际修订仍写shared_sales_revisions/shared_sales_lines，普通sales_orders列表/详情与其没有共同原事实，因此正式录单页面尚未接入。继续完成统一真实单据列表/详情及原命令回查、单头/币种与客户/SKU/仓库绑定、事务内当前会话/角色审核、修订删除行的交付/取消约束、DataGrid录单和持久草稿、多端冲突/断线恢复。源码中的REVISE守卫目前只遍历新行，整行删除所关联的已交付/取消量需补实际反例与守卫；不能以本批四专项冒称上述边界或完整贸易闭环通过。整体免费ERP保持PARTIAL。

本章收尾：严格Clippy实际FAIL（{'erp-adapter-pdd': '6', 'erp-adapter-taobao': '7', 'erp-adapter-jd': '5', 'erp-adapter-douyin': '9', 'erp-repo': '297'}），API lint未到；运行后仅两份OpenAPI源码/生成输入改变并单独通过门禁，六个本批Rust源码保持SHA一致。详细范围见[本批记录](../qa/acceptance/free-erp-trade-lines-oct8-repair.json)及[独立事实](../qa/acceptance/free-erp-trade-lines-oct8-independent.json)。正式页面及免费首发仍未结项。

## 三十三 完整贸易单据单头 绑定和原命令读取

2026-10-08接续T03/O01/O02/O04/O05与上一章。新增`POST/GET /trade-sales-orders`、`GET /trade-sales-orders/{so_id}`和独立`GET /trade-sales-order-commands/{request_id}`。新DTO严格拒绝未知字段，数量必须为六位精度内的正基准单位文本；客户必须实际ACTIVE客户，SKU按本企业真实代码绑定不可变UUID。单据号和明确CNY/USD/EUR/HKD币种保持稳定，金额在checked/i128中间量后回到i64分值。列表、详情、原命令读取使用实际shared_sales_revisions/shared_sales_lines及同事务的新单头/绑定；较晚版本不替代原命令结果。列表按完整授权范围应用当前仓库谓词，再按单据号游标分页；页尺寸100/200/300/500/800/1000/2000，默认500，UNKNOWN总数不伪造。列表使用repeatable-read并锁定当前授权行，列表不复制可能很大的历史authority数组，详情保留其证据。

保存及回查使用真实用户/SID/凭据代次、数据库orders.write/orders.read及绑定仓库授权；原结果分支仍复核当前权限，不依赖旧JWT权限宣告。单头、原修订/明细、真实客户/SKU/可选仓库绑定、原回执、USER操作/审计和Outbox一起提交；写入恢复可只提供原请求号，不依赖已收到生成的单据ID。所有来源和声明数量/金额不被当前资料重算。非SUBMIT必须有订单ID与正期望版本；缺SKU的末行、旧版本、越权和整行遗漏已有交付/取消量均不留半单。旧入口不能修订完整贸易单据或绕其原结果权限；已有无单头/币种的旧身份明确要求核对，不补造历史，也不猜建关联。

0308新三表、归属索引和不可变守卫；第一轮FK因旧SKU/仓库仅有全局UUID主键失败，事务完整回滚，新表未留下。追加企业/对象组合唯一索引后保护QA应用成功，未改变旧业务行。0309增加完整修订的新增明细/孤立版本/原命令改写拒绝，以及提交时真实原成功回执、行绑定及总额完整性。原DDL失败源码/日志保留。全部DDL仅私有QA，不代表全升级链或生产迁移。合并和历史原身份保留政策继续生效。

初5专项通过，扩展7通过/0失败/0忽略/566筛除，9.84秒；实际涵盖保存/列表/详情同事实、537分到538分修订、原请求仍回1版、仓库/写权限/SID撤销、末SKU失败后同ID恢复、企业边界、旧入口拒绝、10路同ID一单一日志/审计/事件、四次数据库改写/孤立版本拒绝及本企业历史PLANNED一微量所在整行不能删除。该历史计划夹具不表示真实出库通过。独立296项PASS，8个新本批QA企业的实际主从/绑定/原回执/金额/操作者/日志/事件关系及成功HTTP对象与数据库一致，1005运行输入无漂移；未扩称全部原企业保护。选定三叶格式PASS；公開契约打包/v2门禁PASS，后续只有两份OpenAPI输入改变，另行绑定，未将此前运行扩称新文档运行认证。严格Clippy仍仓储297及适配器27阻断，API lint未到。

追加旧trade_lines四项回归被统一入口准入拒绝：工具链卷须至少10 GiB，当前C约9.88 GiB。没有清理、改缓存或绕过门禁，旧回归NOT_RUN而非PASS。详见[本批记录](../qa/acceptance/free-erp-trade-docs-oct8-repair.json)及[独立读回](../qa/acceptance/free-erp-trade-docs-oct8-independent.json)。正式DataGrid录单、持久原命令/草稿、多端实际编辑和恢复、筛选/分页大样本、单位换算、更广币种、完整交退/账款、全迁移/备份恢复/Release及公网仍待完成；免费ERP整体PARTIAL，不将七专项冒称573全登记通过。

## 三十四 正式销售录单共享页面源码接续

2026-10-08接续上一章。本轮在共享Rust宿主注册`Page::TradeSales`“销售/贸易/销售录单”，复用现有登录与服务端核实的actor；`remote_view/trade_sales.rs`绘制实际DataGrid明细，`trade_sales/model.rs`记录单据/稳定行身份、精确基准数量和两位金额。查询使用新增`GET /trade-sales-order-lookup?document_no=`，该路由在当前身份审核后解析真实业务单号，再调用上一章相同详情/仓库审核；未用当前页筛选假冒全量查询，也未新建并行订单表。明细搜索仅作用于已完整读取的当前单据（最多1000行），250ms/IME提交后触发，本地排序使用字段类型；不宣称服务端完整贸易列表/条件查询已接UI。

草稿与原请求按真实服务/企业/actor隔离，用已有SQLite/IndexedDB CAS存储的独立trade-sales-v1命名空间。发送前必须收到持久写入完成；令牌和密码仅留在HTTP请求内存，不进入checkpoint。源代码保留原请求号、so_id和冻结字段，超时查询原命令，不把未知直接当失败，不用新身份重复请求。已确认响应逐项匹配原字段/版本，再持久记录完成；另一窗口存储冲突保留输入。页签、关闭、刷新和整应用离开接未完成保护，清草稿等待本地存储确认。状态显示“已下达/执行中/已取消”，未把共有FULFILLED标志当真实库存/款项完成。现有不确定4xx统一保留原请求；可修正的确定校验失败回到编辑仍需正式协议/运行验收，不能宣称完整失败路径已交付。

源码新增3个计划单元测试，覆盖1.2500规范为1.25、分值精度、错误原结果版本/数量、不同账号及被改写的持久请求；均NOT_RUN。静态UTF8、括号配对、菜单/路由引用和不含凭据的checkpoint检查PASS，仅为文本检查，不是Rust语法/类型/运行认证。新增公开业务号查单契约打包/v2门禁PASS。格式调用首次因含外部mod的非叶被入口拒绝，调整模型模块声明位置为宿主声明后，选定叶fmt仍被C卷准入拒绝。native及web Scope grid check也在编译前拒绝：C约9.61GiB，要求至少10GiB。未执行raw cargo/rustfmt、改缓存、清理或终止其他进程，不能报告编译、Clippy、样式或界面通过。

本轮所有UI/持久恢复行为目前仅为源码实现，正式页面实机、客户/SKU/仓库选择、跨窗CAS/断线与存储故障、中文IME、桌面/WASM及整个贸易闭环仍待C盘门禁恢复后验证。新增查单API也未运行验证，不沿用上一章七项通过作新接口证明。详见[源码交付边界](../qa/acceptance/free-erp-trade-ui-oct8-source-review.json)。免费ERP目标保持完整且未完成。

### 2026-10-08 销售页面离开状态复核与构建阻碍

上一源码批次后复核整应用放弃流程：新增独立discard_waiting状态，销售本地清除未确认时不冒称远端保存失败，回执后才处理其余页面及关闭。清除回执只在编辑代次相同时标记干净，清除过程中冻结编辑；原结果额外按六位数量和分值重新核对总额，编码前也拒绝负净值/溢出，原请求拒绝nil UUID。计划测试补返回合计不一致断言，未实际运行。

当前选定文件UTF8及括号结构检查通过，仅为文本检查。固定入口native Scope grid再次在编译前被C卷10GiB门槛拒绝，C约9.40GiB；没有正在运行的cargo/rustc/link进程等待，没有绕过、改target或清理。相同磁盘条件已在连续三个目标轮次中确认。当前已有未经编译的页面、恢复与查单接线，继续叠加未验证功能不宜作为完成依据；需恢复磁盘准入后进行格式、native/WASM编译、计划测试和实际双端验收。详见[状态修复记录](../qa/acceptance/free-erp-trade-ui-oct8-state-repair.json)。整体目标未完成，资源阻碍待用户或宿主外部状态变更解除。

### 2026-10-08 用户指定G盘构建与门禁修正

用户明确“不在C盘构建，在G盘构建”。现统一入口复用原`G:/erp/build/cargo`，Cargo下载/索引/解包缓存固定`build/cargo-home`，编译临时文件固定`build/tmp`；三处同级固定用途、同一构建锁、合计80GiB准入和所在卷至少20GiB空闲，不按任务/PID新建target。首次只读复用全局已下载包/索引，不复制凭据或用户config，不移动或删除任何目录。C上的已安装固定1.98.1编译器只读执行，不再以用户目录C卷10GiB空闲作为构建门槛；调用指定安装的工具链程序，临时环境仅本进程设置，结束后恢复。其他项目及全局工具链配置保持原有设置。

实际选定叶格式PASS，native Scope grid check PASS，wasm Scope grid check PASS；子编译器out-dir实际位于G盘。首次页面模型相对路径错误与egui输入法事件匹配/弃用告警已修；日志保留。环境恢复初次将未设置变量变为空字符串，改为真正删除缺省变量后，成功fmt和失败fmt两条路径七变量全部回到原状态。初失败fmt随后仅格式化涉及叶，不全仓迁移。原目标缓存继续复用；当次盘点target约70.52GiB、依赖0.75GiB、临时0，低于合计80GiB准入。

具体证据见[构建存储验证](../qa/acceptance/free-erp-g-build-oct8-validation.json)。此前C盘造成的准入阻碍已解除；编译通过不代表销售页面实机或真实双端恢复通过，计划测试、交互/存储异常、原订单/生产完整流程与免费公网仍需接续。免费ERP整体未完成。

## 三十五 确定提交前拒绝与销售持久恢复接续

2026-10-08接续正式销售页面。原路径所有HTTP错误都保留原请求，但无法修改确定错误的客户/SKU；新增可选`rejection_envelope=true`回复协议。SharedRepo仅把核心事务返回的非SQLx、非授权逻辑错误标为SalesDocumentRejected；COMMIT/传输数据库错误、当前权限/SID撤销、同ID变意图和提交后的读回错误仍走普通错误/未知路径。原客户端未请求该协议时状态不变。协议只含固定code、committed=false、原request_id、解码原字段和原因；客户端必须逐项匹配原请求，不能把任意409/404或消息文字当已回滚。

确认拒绝后先将完整原请求和拒绝原因写入本账号checkpoint，原请求仍受保护；SQLite/IndexedDB写入回执成功后才清pending并保留草稿为可编辑。校验字段再次保存生成新命令，原so_id不变，最近一次确定拒绝字段不丢；超时、存储失败、授权拒绝和不匹配响应继续保持原ID/冻结字段，不能弃单或自动换身份重发。恢复记录增加可选rejected字段，已有无此字段schema1可读取。后台没有新增并行失败账本或改变原command_receipts；该声明仅证实该次逻辑拒绝之前未发COMMIT，不表示任意历史未决结果完成。

当前9实际密码/SID API专项全部通过/0失败/0忽略/566筛除，11.24秒；新增无客户的逻辑拒绝无单据/原回执，修正用新请求成功，orders.write撤销仍403且无拒绝协议。10个新本批QA企业独立329项PASS，原修订/头/绑定/回执/日志/事件/成功HTTP对象和拒绝对象关系一致，运行1007输入无漂移；不是所有旧企业保护或575全登记回归。客户端最终8通过/0失败/0忽略：原字段/总额/版本精度、不匹配拒绝不解锁、真实自有临时SQLite重开原字段、不同actor隔离、两窗口CAS拒绝且先写不变、确定拒绝等待真实存储确认才恢复编辑。SQLite夹具不是IndexedDB或真实两个应用窗口验收。

状态方法提取首次未写入，新增测试编译因缺方法失败；修后8专项通过，原件保留。严格native客户端Clippy初9库/15库测试错误，包含本次条件和测试初始化；逐项修，无allow，曾因let-chain OR需括号被fmt拒绝，修后native/wasm Scope grid Clippy均PASS，WASM check PASS，受影响叶格式PASS，OpenAPI静态门禁PASS。API运行后的变动仅后续客户端测试/lint/公开契约等另留输入清单，四个关键API源码仍SHA一致；不以该运行认证后来的UI字节。整体后端此前297/27 lint问题不因客户端门禁通过而被视为解决。

全部构建沿G盘固定target/依赖/临时和锁预算，无清理或新target。实机桌面与浏览器登录、真实编辑/保存/字段失败、IndexedDB与完整窗间恢复、选择器/分页查询UI、真实交退/往来、制造全链、迁移恢复/Release及公网仍待完成。详见[本批交付](../qa/acceptance/free-erp-trade-rejection-oct8-repair.json)及[独立读回](../qa/acceptance/free-erp-trade-rejection-oct8-independent.json)，免费ERP整体PARTIAL。

## 三十六 正式 Web 销售录单与 IndexedDB 实际验收

2026-10-08接续上一章。通过 `Build-ErpWeb.ps1` 调用统一入口在G盘固定存储构建/打包当前debug WASM，原生 `Scope api -Action build` 构建实际API。独立回环地址静态服务15440与API15441连接保护QA库15438；新增一个自己的QA企业与普通账号，客户/SKU为真实本企业资料，旧企业数据未主动修改。没有连接真实NATS/Redis/Worker；debug编译时长不是Release或性能成绩。

内置浏览器工具初始化与reset重试均报kernel assets路径缺失，改用项目既有Playwright CLI和实际Edge154.0.4258.62、1280×800 canvas。全部录入和提交经过Rust页面：密码登录；销售菜单进入录单；单号、错误客户、SKU、数量1.25、单价2.35及地址；拒绝响应绑定原字段且数据库单据/修订/回执均0。实际IndexedDB保存原拒绝请求和可编辑草稿，刷新遇到未保存离开保护，确认后完整重载、重新登录，原输入恢复。修正客户使用新request_id但保持同so_id，真实提交第1版金额293分。

修订数量1.250001后，Playwright `route.fetch` 将原请求送至实际API并取得真实第2版提交结果，再 `route.abort` 截断送回Rust客户端的响应。页面进入待确认并冻结编辑，IndexedDB保存完全相同的原请求、期望版本1及六位数量。再次完整刷新、密码登录后客户端GET原命令号，收到真实原第2版而非新POST；原响应和独立数据库一致，持久pending清除、草稿clean。新建后通过业务单号查询实际第2版，未依赖本地筛选。此为可控响应丢失，不冒称操作系统崩溃、服务器崩溃或公网断网验收。

独立psql核对59项全部通过：一单、两修订/主从绑定/成功原回执、各两条操作/审计/Outbox；原拒绝没有成功回执，真实客户/SKU、数量、原字段、293分及实际actor一致。浏览器记录、截图、三阶段IndexedDB和原HTTP对象位于 `output/playwright/trade-web`；[本批证据](../qa/acceptance/free-erp-trade-web-oct8-repair.json)及[独立事实](../qa/acceptance/free-erp-trade-web-oct8-independent.json)限定一个新企业。首次核对器错误假设audit指向so_id且有request_id，实际契约指向revision_id和完整snapshot；修正关系后复核，原失败日志保留。运行后记录源码/二进制/WASM SHA，不宣称完整源窗口或全旧企业保护认证。

已观察且未解决：默认货品页在保护QA库查询/options返回500，因为旧商品表缺少status字段。初QA权限名导致403已仅为自己的夹具补实际skus.read；初public模式拒绝HTTP回环CORS，改为明确本地开发模式，未放宽公网HTTPS策略。因此仅销售录单所列Web路径通过，不宣称正式应用所有页、完整迁移、公网或整套ERP通过。后续先建立商品真实页面所需的完整QA结构基线，避免用局部制造/贸易保护库冒称全部应用验收；再推进真实桌面、选择器/查询分页、两个真实窗口CAS、存储故障/中文IME、完整交退往来/制造、后端严格门禁、恢复与Release、官方托管。免费ERP整体保持PARTIAL。

## 三十七 完整新库安装与正式商品保存

2026-10-08已独立核对：原 `erp_shared_contract_test` 商品结构仅局部兼容，原 `erp_grid_acceptance` 账到0101，无当前auth_sessions；不能补一列status就当完整应用升级。新增固定用途回环15438 `erp_free_ui_acceptance`，所有权comment、库名/角色/地址/端口同时审核。`free-erp-ui-migrate.py` 按当前318个完整SQL文件名排序（保留重复数字前缀），每文件事务、成功文件SHA不可改写、输入原件及日志留证；advisory锁有60秒等待上限，每事务锁/语句上限10/120秒。新空库318/318通过，再次执行按同SHA跳过已成功文件，仍318/318通过，不drop、不重建、不伪造旧库进度。[首次安装](../qa/acceptance/free-erp-ui-migration-first-install.json)与[续跑](../qa/acceptance/free-erp-ui-migration-status.json)仅证明新安装/当前同源续跑，已有经营数据升级、SQL对象完整行为与备份恢复独立待验。

`free-erp-product-web-fixture.py` 只在此新库创建一个自己的普通FREE账号、两产品及实际关联SKU，不重置历史seed。真实密码登录、产品/选项查询、点击父行查SKU、名称修改并按原updated_at保存、刷新查回均实际200，主/子SKU230分与SQL一致。初SKU等待使用启动工具条变化前的行坐标超时，刷新截图后真实点击通过，失败捕获保留。

发现 `remote_view.rs` 普通单元格编辑强制展开视图、导出、查找、复制工具，表格纵向被挤动。现高级工具仅显式打开；`ProductActions::needs_attention` 让未决/存储/账号异常控件独立可见，`edit_tools` 的原版本冲突处理不依赖高级栏。两个相关Rust文件精确接续，不全仓格式迁移；叶fmt初次拒绝长链，固定入口WriteFormat修正。实际更新WASM后普通编辑保持行位置；第二次保存采用真实route.fetch提交，再abort回传，页面仍出现“核实上次操作”。按钮GET同operation_id取得真实原Applied回执，指纹/版本/字段完全相同，IndexedDB Complete保存原命令；未重复POST，没有改成新操作号。

原商品运行窗和工具修后运行窗分别1007源码/产物SHA稳定。独立51项核对通过，两正式操作、各两USER操作/审计，只有目标产品name/updated_at改变，另一产品和SKU全列不变。native/WASM Scope grid严格Clippy与四编辑契约回归通过；debug/API/WASM构建沿G固定目录，增量session finalize拒绝访问note保留、不清理缓存、不改权限。公开契约未变，不启动额外OpenAPI或全后端重复测试。证据见[交付](../qa/acceptance/free-erp-product-web-oct8-repair.json)、[独立核对](../qa/acceptance/free-erp-product-web-oct8-independent-final.json)、`output/playwright/product-web`；QA脚本语法/JSON和文档引用另检。

剩余明确：未决阶段工具条高度变化及暂态编辑文字叠加仍需修，未将普通编辑修复扩称完整紧凑布局；四mock编辑契约不是两个真实浏览器写者。原保护库/旧grid库未升级，新完整库上的制造/贸易全登记仍未跑。真实桌面、选择器/分页/中文IME、全进销存/生产/往来、全后端严格门禁、已有数据迁移/灾备、Release性能和公网托管仍待完成。测试服务仅本轮自有进程被关闭，库、样本、恢复和原失败证据保留，整体免费ERP仍PARTIAL。

## 三十八 未决单元格独占绘制与版本复核

2026-10-08接续上章。商品 `remote_view.rs` 的草稿操作行原按actions.available隐藏，未决恢复又新增一行，导致可见网格上下变动。现按真实actor确认后保留草稿操作行并禁用未准备好的操作，恢复行替换普通编辑商品行；权限/原命令/草稿状态不放宽。共享 `erp-ui-egui/src/lib.rs` 原先底层画旧值、awaiting编辑器又用Label居中画原草稿，形成双文字；可见编辑单元格现由编辑器独占，awaiting草稿使用网格同一文字绘制/语义颜色/对齐/裁剪，仍标记editor_drawn且不产生新编辑/票号。离屏窗口原编辑器另保留，不把可见结果扩称离屏范围完整验收。

新增 `editor_tests` 绘制形状回归检查旧字形为0、冻结草稿为1、未决不能再次准备保存、原草稿/票号保留。原Renderer9通过/1失败，修后10通过/0失败/0忽略；最初未清TexturesDelta导致失败断言展开时析构再panic，原进程退出和诊断日志保留，修测试清理后得到明确旧值断言失败。入口新增ShowTestOutput仅允许命名库TestFilter，以原锁/缓存/范围执行nocapture；静态AST、正常库调用和拒绝集成/无库过滤调用通过，不让诊断开关放宽白名单。首字节脚本中文bytes与CRLF假设错误均在写入前停止，原错误留证，Rust叶固定格式、WASM打包、native/WASM Scope grid严格Clippy及4宿主编辑回归通过。

完整安装QA库内复用本批自有普通账号、两商品/SKU，实际Edge页面录入 `QA UI pending verified`，真正API提交后route.abort仅截断回传。实际IndexedDB记录Unknown原命令，页面原结果按钮GET查同操作并恢复，未重发。六张正常/编辑/未决/未决确认/恢复/最终保存图按商品编码字形扫描第一行y=239；扫描210—300覆盖两行、避免把上移后的第二行误认首行，旧参考待核图首行y=211。当前1280×800浅色该剧本稳定，画面原草稿无双值，非完整布局/性能认证。

随后前台未提交 `QA UI reviewed draft`，同一允许账号另以真实WEB密码会话/独立session更新 `QA UI peer update`。前台原版本返回持久Rejected/VERSION_CONFLICT；实际最新查询、显式采用新版本仍保留输入，按新的operation_id/最新updated_at保存成功。API peer不是第二GUI，未扩称真实两窗口验收。独立52项PASS，运行窗1007输入SHA一致，四新增原命令对应三次业务效果与一次拒绝；三次操作/审计，拒绝无业务效果，历史两命令、另一商品和SKU全列不变。IndexedDB原未决/完成、HTTP原字段/指纹/回执、SQL和截图见[交付](../qa/acceptance/free-erp-pending-ui-oct8-repair.json)、[独立核对](../qa/acceptance/free-erp-pending-ui-oct8-independent.json)及 `output/playwright/pending-ui`。Peer工具在成功捕获后补了未来发送前固定原命令留存，未再新写一遍，不以该后续工具源码冒称已验超时恢复。

当前修复不代表深色/窄屏/长错误、所有业务宿主/离屏、中文IME、两个GUI写者或真实桌面已验；版本错误与明确复核信息仍按必要操作信息显示。完整新库上的贸易/制造全回归、旧业务升级/恢复、后端严格门禁、Release与官方托管继续待完成。没有清理G缓存、改变文件权限或接入真实Worker，只关闭自有浏览器/API/静态服务，QA数据及失败证据保留，免费ERP整体PARTIAL。

## 三十九 正式完整结构与恢复令牌迁移

2026-10-08在原登记目标内新增测试准入叶qa_database：只允许旧保护库/自有完整库，固定角色/回环IP/15438，完整库核comment及当前所有迁移文件SHA，按文件名身份保留同数字前缀文件。一次目标进程中完整SHA只读取一次，实际每次pool仍核身份/comment。测试不自动创建缺列或放宽FK。新增masters叶以同事务显式创建真实产品和所属SKU；贸易两SKU、材料身份和共用库存/生产夹具接续该入口，原数量/成本/错误/幂等断言不删。

原贸易9项全部失败，正式0004要求skus.product_id非空而旧局部夹具未填；修后9通过，1003源码窗一致，独立718核对及10原QA企业589表110行哈希不变。第一份名为before的快照实际是在原九失败运行之后取得，不能当其运行前保护证明；后续fixed-before/after有完整窗口。格式调用初位置参数错误留证，同叶随后固定入口检查PASS。

随后完整576登记131通过/445失败/0忽略，521.34秒；原输出不删，不将专项通过替换完整失败。分类包括354个SKU父产品非空失败、88个旧库名断言、打印设备device_fingerprint必填及未分类失败。原20企业全行哈希不变、1003源码一致，测试另建password_reset_tokens表，说明正式源码迁移少一个实际依赖。[全基线](../qa/acceptance/free-erp-complete-schema-oct8-full-original-status.json)保持PARTIAL。

0310_formal_password_recovery.sql将正式恢复令牌表纳入安装：原tenant/user/hash/expiry/used字段、所属用户组合索引、FK及未用令牌索引，历史导入不改写，FK NOT VALID但新写入必须属于同企业用户。移除恢复测试的临时CREATE。自有完整QA在318基础追加319文件及成功SHA；当前令牌孤儿数0，不追认为旧约束已验证。独立回滚schema实际跑新建表分支、合法所属写入/跨企业所属23503拒绝并确认trial对象不留；不是完整319全空库安装或生产升级。

88个冗余DB断言接受与上级同一两个准入名，原业务断言保持。材料身份7、生产放行24、BOM2、正式密码恢复1、URL准入1在319结构下全部通过；生产首次23通过/1失败来自另一个旧SKU，还原该未知实物剧本后补父产品，24通过，原失败保留。独立735项，538原企业590表10027行全字段摘要不变，1004输入一致；放行原量/命令/两真实效果、库存净转移/冻结减少、实际原HTTP回执和真实SKU归属另复核。数据摘要不导出账号密码值。具体见[交付](../qa/acceptance/free-erp-complete-schema-oct8-repair.json)、[贸易核对](../qa/acceptance/free-erp-complete-schema-oct8-trade-independent.json)、[最新核对](../qa/acceptance/free-erp-complete-schema-oct8-focused-independent.json)、[0310新建试验](../qa/acceptance/free-erp-password-recovery-migration-trial.json)。

最新API严格Clippy因四渠道适配器6/5/7/9共27项失败，API本体lint未到；所选叶fmt PASS，未全格式重写巨大旧测试文件。全576修后未重跑，其他SKU/打印夹具和显露的真实部署缺口仍待补；这些后端API/仓储证据不能当正式双端、真实Worker、全链成本往来、已有数据迁移/恢复、Release或官方免费上线。所有Rust沿G固定入口，不清理缓存或旧QA事实，整体保持PARTIAL。

## 四十 正式SKU 店铺与打印夹具完整运行

2026-10-08完成26个剩余SKU插入的显式原子产品归属，包括1001组件上限。原values表达式、绑定顺序、SKU种类/成本、库存量/批次/失败断言保留，未降低not-null/FK，不将无实物历史补0。打印边界案例补企业唯一实际device_fingerprint，同步原兼容字段；创建真实店铺/模板并写入要求的template_id，移除整个测试临时DDL块。第一次设备修复仍因template_id非空失败，原日志留证；后续本企业非完成输出仍409，另一企业设备不得完成/泄露，真实密码普通账号路径和独立读回PASS。

完整登记576实际553通过/23失败/0忽略/0筛除，1205.97秒。577旧QA企业原590表全行摘要不变，1004输入一致，所有源及原失败保存；不是576全通过或Release计时。测试新增12张仓储实际引用表，清单见[安装缺口审计](../qa/acceptance/free-erp-formal-fixtures-oct8-schema-gaps.json)。来源预占等23失败集中映射缺shop_id；7处以原子CTE创建真实自有店铺和映射，SKU/status不变，没有静默删映射渠道/Outbox剧本。组件快照15及其余10个原失败精确函数均实际通过，25个用例覆盖全部23原失败；未将分窗口25+旧553合成一次576绿灯。

映射后独立641项PASS：1284旧企业602表全部行摘要不变，1004输入同SHA，新映射的店铺/SKU/所属产品都属本企业。源代码在每个运行窗内冻结，不因等待另建target或重启同目标；仅G固定入口，旧QA及新失败样本不清理。证据见[本轮交付](../qa/acceptance/free-erp-formal-fixtures-oct8-repair.json)、[完整553/23](../qa/acceptance/free-erp-formal-fixtures-oct8-independent.json)、[全部原失败专项](../qa/acceptance/free-erp-formal-fixtures-oct8-mapping-independent.json)，两个编辑JSON保留原SHA和语句数量，所选叶格式通过，巨大旧文件只精确改动。

另从真实回传与数据库发现五个分配响应的创建时间初次/重放不同：`fulfillment/store.rs` 初次OmsShipment用Utc::now构造，实际库存/发货身份一致不代表时间字段真实。独立[时间观察](../qa/acceptance/free-erp-formal-fixtures-oct8-observations.json)绑定原shipment_id和实存created_at，验证期间未改业务源码；下一步接实存返回时间。12表正式迁移覆盖、后续修后完整576、适配器/后端严格门禁、完整双端/实际Worker/进销存生产往来、旧数据迁移灾备、Release及官方托管继续待完成，免费ERP整体PARTIAL。

## 四十一 正式运行依赖 实存时间与公平隔离

2026-10-09（文件保留本批oct8固定用途名）新增0311_runtime_contract_tables.sql：attachment下载权限、公式故障、两工资费率表、期初差异、余额核对、批次/序列/普通发料拒绝和来源、调拨拒绝、纸面码共12实际依赖，已逐一盘点原字段/主键、源码引用和唯一一致的测试定义；15个重复测试CREATE移除。原行不重写，企业FK NOT VALID保护历史并约束新写，查询索引非唯一避免猜补历史归属。原QA以320成功SHA接续；迁移器--fresh-install只允许另一个固定空库erp_free_install_acceptance、专用comment及日志，不给用户任意库名/drop。真实全320空库执行通过、独立12表存在，12表实际合法/错误企业FK写入及回滚试验通过，非旧经营库升级/旧FK验证/恢复认证。

OMS分配INSERT原写now但响应另Utc::now；新增HTTP/独立SQL纳秒精度回归先实际失败。fetch_one RETURNING两个实存时间给初始Dto，原发货身份/库存/日志/Outbox事务保留，当前19个atomicity回归通过，首次与原重放完整shipments对象相同且只一单。完整577通过/0失败/0忽略/0筛除、962.50秒为debug功能时间；69次分配创建回传均等于独立实存created_at。运行窗1005源一致，不能由这些结果推定UI/Worker/性能完成。

原数据保护另发现真实失败：公平调度测试准备阶段全局UPDATE关闭所有旧HEAVY记录，改动两条此前企业行。原686检查只该表差异，其余原行保留；通过虚拟将closed_at置NULL后全字段MD5匹配旧记录，证实唯一字段变化，没有实际UPDATE恢复历史。[两行证据](../qa/acceptance/free-erp-runtime-tables-oct8-old-admission-change.json)保留，不静默剔表/减少断言或改保护快照来造PASS。

现去掉全局更新，qa_database::isolated_admission_pool创建真实自有队列表（LIKE原正式定义），只为该用例的数据库连接设置search_path；认证/企业/命令回执仍真实。同一admit_work全局/企业预算逻辑与两企业重洪泛/订单/跨企业拒绝原断言均保留，隔离不是公网容量或真实Worker验收。该1目标通过，直接SQL五命令为两QUEUED、两429、一ORDER ACCEPTED；2057原企业602表全部原行摘要不变，1005源码一致。[修后定向保护](../qa/acceptance/free-erp-runtime-tables-oct8-admission-independent.json)是新独立窗口，不覆盖原577的保护FAIL。核对器首猜计数错误，读回定义的五命令后修正，没有减少业务断言。

仓储严格Clippy最新lib297/lib-test299 FAIL，原件留证、无lint压制。所选Rust叶固定格式通过，巨大旧测试只精确改动。对应[交付](../qa/acceptance/free-erp-runtime-tables-oct8-repair.json)、[原完整功能与保护](../qa/acceptance/free-erp-runtime-tables-oct8-independent.json)、[空库320](../qa/acceptance/free-erp-install-migration-status.json)和安装/FK读回分别绑定。后续先在公平修复后的源码下重采完整目标与原数据保护，再完成严格门禁、更多实际双端/Worker/经营闭环、旧数据升级灾备、Release和官方免费服务；当前整体仍PARTIAL。

## 四十二 适配器门禁 数据保护重采与确定工作日

2026-10-09本次先恢复中断的protected-full：工具句柄丢失不是单独重启理由，独立确认原host16788及所有test/cargo进程不存在、日志停在18个结果且无终态；QA15438也无服务。核G:/erp/qa/postgres-grid原postmaster参数/目录/pg_ctl状态后，仅启动既有集群，实际PostgreSQL完成中断恢复并核库/角色/IP/320迁移。pid/log/库不删除、不init/reset，不启动主5432。恢复后原protected前后原行和源码一致，状态另留INTERRUPTED_NO_PROCESS，不能报告该次完整通过。

JsonDiagnostics由统一入口序列化原Clippy诊断，限定Action clippy；新Scope adapters只含PDD/JD/Taobao/Douyin且native-only，同锁/80GiB/G缓存不变。四文件仅27机器可用条件合并，逐条验证诊断原行、范围/文件，不混合重叠建议：先26、重新strict诊断后1，原文件各轮保存。四包严格all-targets Clippy与8/10/14/9共41库测试通过，无lint压制，不等于签名集成/真实平台/API/仓储完整门禁。对应JSON、byte/原SHA和[独立验证](../qa/acceptance/free-erp-adapter-lint-oct9-validation.json)保留。

新完整窗口577实际567通过/10失败/0忽略/0筛除，704.56秒，原数据保护及创建回传SQL检查通过：686核对唯一失败仍是完整全绿条件，不能改核对器期望567来造PASS。十失败均在成本/劳动日期：as_of/期望费率日固定2026-10-08，普通/分配/CUT报告三个INSERT只声明数量而未录真实started_at，当前created_at进入10月9日触发正确reconcile_required，劳动日也确认为9而非8。三个夹具显式started_at=10月8日，保留原截止、日报费率、材料数量/金额及反转断言，没有调业务规则/当前时间或跳过测试。

41来源处置目标全部通过，覆盖十原失败；直接SQL无缺失工作开始时间。日期修后2795原企业602表127246行全字段摘要不变，1005源输入一致，前/后原窗和修后窗独立，41不是新577全量结果。[完整567/10](../qa/acceptance/free-erp-resumed-full-oct9-independent.json)、[日期核对](../qa/acceptance/free-erp-deterministic-dates-oct9-independent.json)、[当前交付](../qa/acceptance/free-erp-quality-gates-oct9-repair.json)和原断言/中断日志/源快照不覆盖。后续先重建确定日期后的完整目标与保护，再修仓储lib297/lib-test299，全部经营流程/实际双端/Worker、已有数据恢复/Release和官方免费托管仍须验证，不把底层库/局部测试门禁扩称整套ERP完成。

## 四十三 仓储局部机器修复与完整保护窗口

2026-10-09固定入口Scope repo/JsonDiagnostics实际诊断，27适配器修后仓储仍lib297/lib-test299，主要215个长参数函数以及复杂类型。仅选collapsible_if/borrow_deref_ref/needless_borrow/needless_option_as_deref的MachineApplicable组，验证文件/原行/字节范围、去重Lib/LibTest同建议、重叠分轮。原源文件存repo-lint-source-before，先63后1共64组17文件；根shared_contract三条件布局留待接口实际重构，借用替换保持原结构。其他所选叶统一格式，未全仓格式化/未添加allow/expect，数量、身份、权限、原回执、未知状态以及SQL数据形状不改。

实际第三次严格门禁仍lib233/lib-test235 FAIL，原件及[唯一剩余源位置](../qa/acceptance/free-erp-repo-lint-oct9-remaining.json)保留，未将错误减少当通过；类型复杂/长参数等须建立适合的输入契约，不通过机械元组/批量局部例外规避。对应[本批交付](../qa/acceptance/free-erp-repo-lint-oct9-repair.json)绑定原SHA、各机器建议及新SHA。

随后当前完整577/0/0/0通过，724.09秒为debug功能时长；[独立686核对](../qa/acceptance/free-erp-repo-lint-full-oct9-independent.json)全部PASS，2840既有企业原602表全字段摘要不变、1005输入SHA同源，所有记录到的分配创建返回时间等于实存行。该次包含日期修复和公平调度隔离后的完整窗口，不借之前567/10或577保护失败窗口合成结果；所有失败/中断/历史记录保留。真实320 SQL安装仍沿安装专属证据。

这里只完成登记后端完整功能与保护，100卡及22门槛、全量原生/Web UI/剪贴板文件打印/离开失败、真正Worker/平台请求及制造进销存往来、生产已有数据升级恢复/降级、Release分位数/内存长稳、公网免费托管仍未结项；仓储严格门禁也未解除。继续按接口契约逐步重构和实际运行验收，不修改完整目标为已完成子集，整体PARTIAL。


## 四十四 材料成本具名输入与当前回归窗口

2026-10-09将收料、发料、原成本退回、调整和关期调整改为五个具名Rust输入，内部成本写入亦用具名输入；新增shared_contract/item_cost_input.rs并在原根导出。没有改变HTTP DTO、数据库结构、原请求身份、数量/金额精度及成本验证规则；不添加lint压制。六文件63处调用逐项核对原表达式，存原调用源和映射；旧共享集成目标实际编译、QA地址拒绝测试通过，不修改旧QA身份或运行其全部业务用例。

当前材料成本9项真实完整QA专项全部通过，568筛除；包括三微量分两分的累计尾差、原发出成本、20路独立退回及同命令竞争、未知价值、溢出、关期锁与实际效果顺序。独立756项PASS：63调用参数/调用外代码（仅忽略格式空白与尾逗号）、3558既有QA企业原602表167087行摘要保持、1006源输入一致，22实际成本行的数量金额独立读回。原源快照覆盖本批调用转换前的阶段，不宣称全部成本模块最初字节均有备份。[专项核对](../qa/acceptance/free-erp-cost-input-oct9-independent.json)与[调用映射](../qa/acceptance/free-erp-cost-input-oct9-calls.json)保留。

严格仓储实际229 lib/231 lib-test FAIL，比233/235各少4；本成本模块无诊断不等于依赖链通过。剩余211个长参数接口及复杂类型等继续按实际契约重构，不用元组或allow规避。当前完整577登记实际577通过/0失败/0忽略/0筛除，1134.35秒为debug功能时长；[独立686项核对](../qa/acceptance/free-erp-cost-input-full-oct9-independent.json)全部PASS，3567既有企业原602表167146行摘要不变、1006源输入SHA一致，69个分配响应创建时间与实存行一致。该完整窗口包含本组成本接口，未沿用上一版结果或把9专项补算为全量。[进度证据](../qa/acceptance/free-erp-cost-input-oct9-repair.json)与[严格诊断](../qa/acceptance/free-erp-cost-input-oct9-lint-results.json)分层记录。所有构建沿G盘统一入口固定target/依赖/临时和锁，不新建缓存或清理。

该批尚不满足100功能卡/22性能门槛、全部原生/Web正式页面、真实Worker/设备、已有经营数据升级恢复及官方免费托管。整体目标仍PARTIAL；本组完整功能/数据保护已通过，继续其余严格门禁与业务运行。


## 四十五 贸易与往来款具名输入成组重构

2026-10-09成组迁移26个原SharedContractRepo入口：采购/销售修订、交付计划/推进、报价及转换、原普通贸易采购/销售、小数销售、往来对象/联系人，以及应收应付/期初/预收预付、票据、收付款冲抵、费用分摊和期间对账调整。新增trade_document_input.rs与finance_input.rs共25个具名Rust输入，原引用发票和迟到发票共享同一字段契约。263处调用按原参数名明确赋值，HTTP DTO和命令指纹/数量金额/来源规则不变；不新增数据库结构或lint例外。

保留11个原源文件的完整字节备份与263调用原表达式；静态903项最终PASS，覆盖输入类型、每项原表达式、调用外代码和26方法体。逆向去掉本批输入接线与原已无用途的PartnerBalanceKind裸导入后，整个仓储根方法体/SQL与原字节完全一致。初核对FAIL是核对器将当前read_text的CRLF规范化为LF，而原备份用decode保留CRLF，导致旧测试原raw SQL字符串误差；改成两侧均按字节解码，业务文件未因该误差重写。原核对器/失败报告保留，[最终903项](../qa/acceptance/free-erp-trade-finance-input-oct9-wiring-final.json)与[原失败](../qa/acceptance/free-erp-trade-finance-input-oct9-wiring.json)分别记录。

首次API check通过，类型迁出后的未使用导入已移除，新叶按固定入口格式化，巨大模块只精确接线及排布本批新增块。仓储严格Clippy由229/231降至203 lib/205 lib-test，仍FAIL；185个长参数接口、13复杂类型及其他诊断继续待修，无压制。旧共享目标实际编译并运行QA地址保护用例1/0/0通过，399筛除；没有运行旧库全部业务。新trade-finance-full窗口完整577实际577通过/0失败/0忽略/0筛除，1066.72秒仅为debug功能时长；[独立686核对](../qa/acceptance/free-erp-trade-finance-input-full-oct9-independent.json)全部PASS，4285既有QA企业原602表204472行摘要不变、1008源输入SHA一致，69个分配响应创建时间与实际SQL一致。本结果绑定当前26入口及全部登记目标，没有将903静态或上一版577合并为当前通过；不宣称每一旧共享业务用例均运行。[本批进度](../qa/acceptance/free-erp-trade-finance-input-oct9-repair.json)与[严格诊断](../qa/acceptance/free-erp-trade-finance-input-oct9-lint-results.json)保留分层状态。

所有构建继续固定G盘统一入口、缓存/依赖/临时与单锁，不清理或新建target。完整原生/Web贸易/制造、真正Worker/设备、经营旧数据升级与备份恢复、22组Release资源/性能及官方免费托管仍未完成，整体PARTIAL。


## 四十六 真实材料单草稿与原保存结果

2026-10-09先落地ATX-01/02的正式材料单基础，未把整单确认缩为草稿保存。新增0312三张正式表，头关联本企业真实工单、保存版本/实际工单来源字段；当前明细有稳定UUID/顺序、四动作、仓库/SKU/可选批次及原领料、单位与六位数量。修订和原请求/认证提交人不可变，头当前版本与原修订、修订与共享回执使用延迟FK共同提交。禁止删除单据身份或修改已POSTED事实；当前API只创建DRAFT、posted=false，不开放确认或客户端自报posted。

新POST为work-orders/{work_order_id}/material-documents；新GET在其后/document_id，可带原request_id。保存服务核对实际账号ACTIVE、SID/凭据代次、当前制造写权限和全部明细仓库；查询复核当前读权限。新单expected_version=0，编辑版本竞争只允许一方成功；工单数量/BOM等已捕获来源变化拒绝MATERIAL_SOURCE_CHANGED，不能默默采用。原请求重放/GET返回不可变v1而不是最新v2；未完成原回执明确要求核对。来源审核/adoption与确认时冻结BOM/真实需求/原成本及100行同一事务仍待接续，草稿层不证明这些事实已放行。

先前完整QA原320正式文件保留SHA，0312仅追加本自有erp_free_ui_acceptance，第321文件实存登记；不是新的321空库安装、生产已有数据升级或恢复。两叶API/Repo、测试叶使用固定G盘入口格式；首次API check通过。OpenAPI源/生成及v2门禁通过，数量微量为十进制字符串，返回真实原saved_at，明细1..100及原因4096 UTF-8字节/单据预算规范保留。

首轮五专项4通过/1失败，测试读取撤仓文本404却仅按JSON解码；原日志/源保留，修读取后新增不完整回执反例，六专项6/0/0通过，8.84秒debug；[独立719核对](../qa/acceptance/free-erp-material-documents-oct9-fixed-independent.json)PASS，5008旧QA企业605表全行摘要不变、1012源输入一致，实际头/明细/原修订/回执和HTTP结果一致，无材料/库存成本/价值过账，六真实期初余额不变。真实审计INSERT故障在头/明细/修订/回执准备后发生，全回滚，恢复后原ID可保存；20同命令仅一次保存、两同版本写者一200一409、撤权/错误来源/伪posted拒绝。原失败源备份位于material-documents-oct9-first-tests.rs.txt。该六专项使用两行夹具；当前完整583登记实际583通过/0失败/0忽略/0筛除，1131.98秒为debug功能时长。100真实行的v1/v2保存、原重放与GET在此窗口通过；[独立720项核对](../qa/acceptance/free-erp-material-documents-full-oct9-independent.json)PASS，5014既有QA企业605表242098行摘要不变、1012源输入一致，真实头/当前100行及原修订/回执/成功HTTP结果对实存，草稿无库存/材料/价值过账、六实际期初余额不变。未把719专项或旧577合并认证当前版本。

严格仓储203/205仍FAIL，新材料单叶无诊断不代表API依赖链通过。[本批进度](../qa/acceptance/free-erp-material-documents-oct9-repair.json)与[321追加](../qa/acceptance/free-erp-material-documents-oct9-migration.json)分层留证。接续材料单查询/明确来源采用/整单确认及真实原生/Web录单恢复，真正Worker、已有数据升级恢复、Release和官方免费托管仍待完成，整体PARTIAL。


## 四十七 真实材料单整单确认与恢复接续

2026-10-10接续ATX后端确认：新增0313不可变材料单明细/实际move效果绑定，新POST material-documents/{document_id}/confirm只接原request_id和已保存expected_version，不接受客户端再报数量/价格/posted。读取真实当前材料单、原修订及实际明细，三者不符要求核对；当前SID/代次/角色/全部仓库与原工单来源重新检查。读回兼容原schema1草稿及schema2确认：原保存请求仍返回原DRAFT，原确认请求/当前GET返回原POSTED效果，不取当前库存、价格或工单状态重算旧结果。换新请求重复确认同单拒绝，不重复扣库。

同一事务调用带真实actor的原材料内核，复用精确库存、来源预占、普通材料价值/原退耗分配及日志/Outbox；补料原因、原领料匹配与剩余量沿原规则。ISSUE/SUPPLEMENT按工单绑定的实际BOM版本/组件/单位/需求余量检查；RETURN/CONSUME需要原单位及原价值证明，不补造历史。原来源→成本期间/稳定代码→原预占→稳定库存键顺序，全部真实行、价值、不可变效果、两个单据日志、必要Outbox和共享回执一次提交；后行或实际日志/事件INSERT失败全回滚。原结果核对完整move身份、数量/原成本、库存流水、实存日志与事件payload。

首轮1通过/5失败是本批误将Outbox表写为outbox，实际outbox_events，原日志/源码保留。修后六专项6/0/0/583筛除，13.35秒；10月10日恢复G盘后才读到真实终态，原测试不再运行，QA也停止。从既有qa/postgres-grid启动同15438并实际崩溃恢复，未init/reset/删PID/动5432；1015源码输入与原窗一致。补采旧行和当前真实效果，[1046独立核对](../qa/acceptance/free-erp-material-confirm-oct9-fixed-independent.json)PASS，原失败与未完成现场不改写为通过。

新边界专项8/0/0/583筛除通过，16.65秒；[1084独立核对](../qa/acceptance/free-erp-material-confirm-oct10-edge-independent.json)PASS，旧606表全部原行/1015源输入一致。固定1200分样本真实领0.25转持料300分、耗0.125转耗用150分、退0.125返仓150分；独立SQL仓0.875和账面1050分，与原分录相符，耗用未再扣原仓。另有补料/100实际行、真实末行缺货、审计/Outbox后序故障、原ID恢复、20同请求一实际效果、撤权/错误单位/未知成本拒绝；合法零成本真实VALUED/0分。不同工单的A/B与B/A两SKU写者实际同时到达同一PG成本锁等待点，释放后两方成功且实际余量/账面一致，非模拟排序验收。

0313仅追加完整自有QA为322正式SQL，原321文件SHA保留，不是新322空库/已有生产数据升级/灾备。OpenAPI打包及v2门禁通过，严格仓储203/205仍FAIL，本次确认叶未观察到诊断不代表依赖链通过。当前完整591实际591通过/0失败/0忽略/0筛除，731.94秒为debug功能时长；[完整1084独立核对](../qa/acceptance/free-erp-material-confirm-full-oct10-independent.json)PASS，5765既有QA企业606表283942行摘要不变、1015源输入一致，12真实确认单/114实际明细效果与原HTTP/回执/完整日志事件payload相符。当前版本八确认用例均在全登记窗口内，不合并专项或旧583冒认全量。详见[本批进度](../qa/acceptance/free-erp-material-confirm-oct10-repair.json)。

本批不结项所有ATX：旧缺原单位/价值需核对；已存在返工专属价值、库位维度、明确来源采用、正式清单与原生/Web材料录单/未知恢复、Worker及Release/免费托管仍有独立工作。目标保持PARTIAL，继续补这些真实经营缺口。


## 四十八 来源复核明确采用与全授权范围清单

2026-10-10解决工单来源改变后材料草稿无法继续编辑的接续。新GET source-review读取DRAFT实际保存来源/当前可编辑工单来源与真实版本，展示changed及两份来源；SHA256令牌绑定当前业务账号、企业、单据、工单、保存版本和两份真实来源。它是明确采用意图/并发检查，不是授权凭据；确认采用时仍读取最新数据库权限和事实。新POST source-adoptions只接request_id/expected_version/review_token，不接受替换明细、伪posted或客户端来源对象。当前工单再变化、跨单据或错误版本令牌拒绝，当前撤权拒绝；明确采用经同一草稿保存内核递增版本并保存新实际来源，当前真实明细不变，原修订及原回执保留，新增采用操作/审计和回执共同提交，不生成材料/库存/价值。

采用快照仍为schema1 DRAFT，新增可选source_adoption（原来源与复核令牌）；原保存schema1及确认schema2协议保留。GET原采用请求及同命令重放在后续v3编辑后仍返回原v2，不取最新数据冒充原结果；旧v1来源继续可查。工单来源复核不是BOM/库存/原成本放行，确认仍执行原591窗口的真实约束。

原材料单集合路由补GET offset/limit/status。查询在同一repeatable-read范围校验当前SID/代次/制造读权限，锁定当前仓库授权；所有实际明细仓库都授权的单据才进入范围，状态筛选与总数先作用于全部范围，然后SQL仅取所需页。按稳定document_id排序，offset≤1000000、limit1..2000，最多10000当前仓库授权，超限显式拒绝不截断；页面只载固定宽度头摘要，不批量加载100行明细再切片。不支持已取消/未知假菜单，未建设业务UI。

当前仓储/API check、所选叶格式与OpenAPI打包/v2门禁通过；四真实密码/SID HTTP专项4/0/0/591筛除，8.71秒debug，[独立680核对](../qa/acceptance/free-erp-material-source-oct10-independent.json)PASS，6500旧QA企业606表全部原行及1017源输入不变。实际两份采用修订和成功HTTP/回执对实存；工单10M→20M复核后再变30M，旧令牌拒绝，重新复核明确采用v2，正常编辑v3，原采用v2仍可查；实际采用审计INSERT故障全回滚并用原ID恢复。8真实单据撤另一仓库后实际完整授权6条，两个SQL页各3且稳定ID无遗漏/重复，POSTED筛选0；跨单据/旧版本/撤权均拒绝。没有库存/材料/成本/价值过账，四原期初真实余额不变。

严格仓储203/205仍FAIL，新来源叶无诊断不等于依赖链已解除。当前完整595实际595通过/0失败/0忽略/0筛除，717.66秒为debug功能时长；[完整680项独立核对](../qa/acceptance/free-erp-material-source-full-oct10-independent.json)PASS，6504既有QA企业606表323234行摘要不变、1017源输入一致。真实原采用/保存HTTP对实存修订与共享回执，完整授权6条先计总数再分页的实际页与范围符合，采用不产生材料库存成本价值；新四来源/页用例在当前全登记窗内，不借专项或旧591认证；详见[本批进度](../qa/acceptance/free-erp-material-source-oct10-repair.json)。下一步正式材料页面及桌面/Web查询/编辑/来源采用/确认/未知保存恢复，库位/返工、完整安装升级恢复、Worker/Release及官方托管仍未完成，整体PARTIAL。


## 四十九 真实工单业务目录与材料上下文

2026-10-10核对正式材料UI前置发现：原GET manufacturing/work-orders只接work_order_id读放行结果，不能让普通用户按工单号选择。新增work-order-options读真实本企业工单号/产品/状态/BOM及精确数量；当前真实SID/代次/制造read权限在DB事务重新核验，不凭前端菜单或合成身份。单据规划读取是租户授权域，仓库选择则按当前账号实际仓库授予；两域明确分开，不把仓库数量当工单席位或任意扩权。

目录query在work_order_no/product_code进行字面contains，转义%/_/反斜杠；open_only默认true指PLANNED/RELEASED/PAUSED，历史状态可明确查询。repeatable-read内先筛完整租户授权范围/状态并计数，再SQL仅取offset页，work_order_id稳定排序，offset≤1000000/limit1..2000。全部订单/已领/耗用数量返回微量十进制字符串，超过JS安全整数亦不浮点化，不冒称发布/下达完成。

work-orders/{work_order_id}/material-context读实际工单绑定版本及最多1000完整BOM行，左连接本企业实际active SKU。未映射或停用SKU保留原component_code/单位/每件量/行号并返回sku_id=null；不silently丢料行。原非直系父行保留direct_component=false，缺版本/映射/单位或未支持直接消费的层级标needs_reconciliation；目录不是多级展开/MRP或来源放行凭据。仓库选项只给当前账号真实仓库业务编号与名称；超过1000材料或仓库显式拒绝，不取前1000冒充完整。只读查询不保存命令、库存、材料或价值。

当前API check、叶格式和OpenAPI打包/v2门禁通过。四真实密码/SID专项4/0/0/595筛除，5.57秒debug，[独立639核对](../qa/acceptance/free-erp-manufacturing-catalog-oct10-independent.json)PASS；7243旧QA企业606表全部原行保持、1020源输入一致。实际字面WO_%_搜索在全部范围精确得到3条开放工单并分2+1页，关闭可选得到4；巨量9007199254740993以上保持字符串。真实BOM未映射行保留、撤仓后选项为空、未知工单404、read角色和会话撤销403/401，1001原BOM行不截断成功。业务工单/全部BOM行/SKU映射/仓库名称均对独立SQL，未有材料/价值写入。

严格仓储203/205仍FAIL，新目录叶无诊断不代表依赖门禁解除。当前完整599实际599通过/0失败/0忽略/0筛除，730.01秒为debug功能时长；[完整639项独立核对](../qa/acceptance/free-erp-manufacturing-catalog-full-oct10-independent.json)PASS，7248既有QA企业606表363631行摘要不变、1020源输入一致。实际业务工单/BOM/SKU/仓库字段、全范围查询/分页对SQL；新四目录用例在当前全登记窗内，未借专项或旧595认证；见[本批进度](../qa/acceptance/free-erp-manufacturing-catalog-oct10-repair.json)。这一批只补真实页面所需查询，未新增空业务菜单或把只读选择数据称为正式材料页面完成。下一步在共享Rust页面用实际业务编号/对象选择与DataGrid接草稿、来源复核采用、确认和未决恢复，再分别验收原生/Web；库存库位/返工、安装恢复、Worker/Release/免费托管继续PARTIAL。


## 五十 材料写入的确定拒绝与未决恢复边界

2026-10-10接续正式材料页面前置。现HTTP适配器对非成功状态只保留状态码，不能把409/422/500直接视为“本请求未提交”并解锁草稿。材料保存、来源采用和确认三个POST新增可选`rejection_envelope=true`，默认不启用，原成功结果和原错误状态保留。新契约见[OpenAPI](../openapi/src/paths.yaml)，仓储[新命令拒绝分类](../crates/erp-repo/src/shared_contract/material_rejections.rs)、[草稿事务](../crates/erp-repo/src/shared_contract/material_documents.rs)、[确认事务](../crates/erp-repo/src/shared_contract/material_document_commit.rs)与[HTTP声明](../crates/erp-api/src/manufacturing/material_documents.rs)共同生产证据。

只有经过原请求准入、确认不存在同身份既有结果的**新命令**，在调用COMMIT前遇到确定业务拒绝，才返回HTTP 200、`code=material_document_rejected_before_commit`、`committed=false`。正文绑定实际请求ID、反序列化后DTO字段及真实企业/认证使用者/工单/材料单上下文；省略的可选明细字段在DTO中表示为显式null，客户端必须用同一表示比较。该响应是拒绝声明，不是保存或过账成功。数据库错误、COMMIT失败、当前身份/权限拒绝、原请求内容冲突及原结果核对异常均保留普通错误，不能产生安全解锁声明。保存/确认的旧版本和来源变化保留现有草稿与行ID；来源采用读取实际保存明细，过期复核令牌不能自动采用。

客户端后续必须先冻结全部输入并持久化原请求，再发送写入；待定行持续可见且禁用编辑。仅当声明与原请求ID、DTO字段、企业/账号/工单/材料单全部匹配，且匹配拒绝已按本地存储CAS持久化确认，才允许编辑并分配**新**请求ID。存储失败、提交结果未知或拒绝声明丢失仍保留Unknown，原GET只回查该原请求的实际成功结果，不能把404、当前余额、新工单来源或当前版本当作原请求未提交的证明。本轮未新增持久化服务端拒绝回执、自动重发或材料页面；真实网络提交断线、原生/Web的持久恢复仍待验收。

首轮编译缺辅助函数上下文参数，业务执行为0，原日志保留。修后4专项实际4/0/0，独立645项PASS；增加旧版本确认/默认错误兼容及过期来源采用后，5专项实际5/0/0，独立659项PASS，7996既有QA企业606表404156行摘要不变、1022源输入一致。四份匹配拒绝对实际草稿/认证提交人/工单/原命令无存储记录逐项读回；五企业库存均1000000微量、真实期初价值1200分，末行失败无材料/价值效果；改正来源后新ID仅保存v2草稿500000微量。见[专项独立核对](../qa/acceptance/free-erp-material-rejection-oct10-edge-independent.json)。这是Axum真实路由与正式QA PostgreSQL验证，不包含客户端实机或真实网络断线测试。

OpenAPI打包/门禁通过。当前严格仓储203/205仍FAIL，本次三材料叶无新增诊断，API依赖门禁未解除；见[当前严格结果](../qa/acceptance/free-erp-material-rejection-oct10-strict.json)。当前完整登记604实际604通过/0失败/0忽略/0筛除，700.35秒为debug功能时长；[完整661项独立核对](../qa/acceptance/free-erp-material-rejection-oct10-full-independent.json)PASS，8001既有QA企业606表404315行摘要不变、1022源输入一致。四份匹配拒绝和五企业实际库存/价值、改正后新ID草稿对SQL，未借专项或旧599认证。见[本批交付](../qa/acceptance/free-erp-material-rejection-oct10-repair.json)。完整窗口的OpenAPI原件/哈希保留；窗口后仅把拒绝字段描述改为精确DTO回显、允许携带被拒绝的业务值，避免套用成功输入的约束；最终OpenAPI门禁另外通过，Rust输入保持，文档更改不启动编译。

当前免费贸易/基础离散制造的主要差距按产品使用顺序为：正式采购/材料/生产/往来款等经营页面及双端闭环；库位/返工与原数量成本的完整接续；真实Worker执行/中断恢复；完整322迁移的空库安装与经营数据升级/备份恢复；严格工程门禁与Release资源负载；官方托管的注册、实际免费账号及公网运行验收。现正式页面注册表只接货品查询、订单查询、销售录单，其余为开发预览。已有后端函数、备份脚本和局部QA不替代这些经营与运行验收，免费ERP整体继续PARTIAL。


## 五十一 正式材料页面与真实Web原请求恢复

2026-10-10共享Rust正式入口新增“生产→材料录单”。页面注册表同时生成模块菜单，避免注册材料页却被旧硬编码三模块隐藏；货品/销售旧顺序保留。实际工单业务号、BOM/SKU映射及当前授权仓库查询接自研DataGrid，材料单主表/明细/原领料选择共用稳定ID；数量六位文本、原金额（分）及原库存结果取实际结果，未知显示—。新材料、动作、仓库通过实际对象选择，原领料来自本工单实际POSTED效果，不让用户手填技术UUID。

[模型](../apps/erp-ui/src/remote_view/material_model.rs)、[宿主状态](../apps/erp-ui/src/remote_view/material_documents.rs)、[绘制](../apps/erp-ui/src/remote_view/material_documents_ui.rs)和[账号/离开接线](../apps/erp-ui/src/remote_view/material_host.rs)接保存、来源复核/明确采用、确认、原结果回查及最新版本明确采用。固定恢复分区material-documents-v1绑定服务/企业/使用者；SQLite/IndexedDB的CAS真实提交后才POST，结果和精确拒绝也先持久确认再解锁。未决行保留可见且禁用修改，重载仅GET原请求，不换号重发或以404当作未提交。来源采用验证服务器实际已保存行，但本地未保存修改继续保留；明确采用最新版本后用新ID保存，不默默覆盖输入。材料待定状态接标签刷新/关闭、退出和账号切换，存储失败不清空恢复文件。

八材料契约/绘制专项通过；当前Scope grid完整397/0/0/0（grid-core87、ERP UI238、egui72），原生及Web严格Clippy、Web check/打包通过，155最终UI输入保持。首轮RowId/Uuid类型错误、三Clippy诊断、旧HTTP代理环境导致的不完整回归及旧复制测试未先展开更多操作均留原件；只终止已核身份的本轮等待测试PID17684，重跑给回环NO_PROXY，不改变固定G盘缓存和预算。复制用例通过真实指针展开更多操作，原一次POST/检查源与持久回执断言保留。真实浏览器另发现模块菜单硬编码、原领料确认按钮被挤出、清单/提示残留，修复并留截图；窗口四项UI源变动单列，最终编译输入与门禁另外绑定，未把原窗口说成源始终不变。

实际Edge/WASM通过真实密码登录隔离API/PG：选择业务工单，领0.25/耗0.125/退0.125，各原价值300/150/150分，仓库原结果0.75/0.75/0.875，最终账面1050分。保存成功后由Playwright丢弃响应，页面原行禁用；IndexedDB重载登录后GET同原ID，只有一次原POST。来源1M→2M拒绝保存时本地0.5保留；明确采用v2只保留服务端原1.0，随后新ID存v3的0.5。真实API并发写v4的0.6，本地0.25旧版拒绝，明确采用最新基线后新ID存v5的0.25；这是并发保存模拟，未宣称第二浏览器/设备完整验收。

[独立640项核对](../qa/acceptance/free-erp-material-web-oct10-independent.json)PASS，8750既有QA企业606表444877行摘要不变；四实际单据、三实际材料效果、八不可变修订及原回执对SQL。新QA公司使用真实父商品/用户/角色/仓库/工单/BOM，并经真实库存调整和成本收货API建1.0/1200分期初；只修正该新公司的WMS权限，不写旧公司。见[本批交付](../qa/acceptance/free-erp-material-ui-oct10-repair.json)与[实际页面](../output/playwright/material-documents/return-confirmed.png)。私有API仅回环，NATS未接、开发CORS非公网规则；浏览器故障注入在服务端提交之后，并非物理断电或正式灾备。

本批只完成这些实际Web操作及共享代码/离线原生门禁。原生桌面认证交互、UUID批次选择、库位/返工、完整业务排序/筛选/面板与导出、规模性能及更多多端故障仍待验；库存/生产工单/BOM/MRP/报工/采购/往来款完整页面、真实Worker、322安装升级恢复、Release及官方免费公网继续未完成。后端仓储严格203/205仍FAIL，UI范围严格通过不解除它。免费ERP整体保持PARTIAL。


## 五十二 材料真实批次查询与页面选择

2026-10-10，材料工单增加 `material-batch-options`，真实工单/当前制造读权限、SID/代次及仓库授权在 repeatable-read 事务复核。完整仓库/SKU批次最多1000，1001明确拒绝；先验证精确实物/预占、整数投影、真实批次父记录和仓库数量守恒，再字面搜索及分页。未知或矛盾使整个上下文进入待核对，可用量不猜零；真实过期/空批次保留并禁止选择。不存在的production_date不作为查询字段。

共享Rust正式材料页用自研DataGrid显示批号、效期、实物/预占/可用及状态，250ms防抖/IME/取消旧任务；实际UUID绑定草稿与原命令，名称仅为独立显示缓存。领/补可选；耗/退沿原领料锁定批次。保存、未决恢复、明确来源/版本采用继续复用原持久契约。

当前5个真实API/PG专项及完整609/0/0/0通过，717.98秒为debug功能时长；完整636独立核对保持8761旧企业606表449397行及1031源输入。UI九材料专项、Scope grid完整398/0/0/0、原生/Web严格Clippy及Web check/打包通过。仓储严格仍203 lib/205 lib-test，共408诊断实例，本批次叶0新增；不得称全后端严格通过。

真实Edge→WASM→隔离API→PG选择QA-RAW-LOT-001，领.25实际300分，仓库及该批次都.75，剩余账面900分；过期零库存批次可见，选择后采用仍禁用。[浏览器618独立核对](../qa/acceptance/free-erp-material-batch-web-oct10-independent.json)PASS，9515旧企业606表492156行与1031源输入不变。批次耗/退由实际API/PG验证，本浏览器仅领料及过期禁用；第二QA本地草稿仍持久保存。未知之后的总量矛盾用例尚不构成健康上下文独立矛盾验收。原失败与修复留证，详见[交付记录](../qa/acceptance/free-erp-material-batch-oct10-repair.json)、[确认页面](../output/playwright/material-documents/batch-issue-confirmed.png)、[过期禁用](../output/playwright/material-documents/batch-expired-disabled.png)。原生/Ubuntu实机、库位/返工、完整查询/面板/导出及其他经营页、Worker、322空库/经营升级/恢复、Release负载、官方免费公网仍未完成，整体PARTIAL。


## 五十三 工序报工身份、数量与原结果

2026-10-10，现reports POST要求真实认证SID/用户/凭据代次及当前存储制造写权限，先工单锁、再身份/角色锁，提交前复查时效。新报工只用实际正工单量或冻结前序净合格产出作上限；previous_good_qty仅保留请求兼容，不授予额度。冻结工序不存在须核对；RFC3339起止非法/倒置、非RELEASED状态及数量越界拒绝。冲正追加原同工单/同工序非冲正记录的精确反向量/工时，已有正量收货引用或冲正后不足有效后序产出则拒绝。

0314新增不可变mfg_operation_report_outcomes，真实用户/工单/报工/原命令FK，报工行、回执、原结果同事务。POST重放不取后来累计量；GET reports/commands/{request_id}给原字段、当次累计量、冻结来源与提交身份。读取和重放校验实际原回执及整条报工记录，时间戳按实际时刻比较；源变动返回report_reconciliation_required。GET仍要求当前制造读权限/有效SID，但同一业务用户可用新的当前SID读取旧SID原结果。历史记录不补造原结果，404不证明丢失提交未入账。旧内部兼容方法尚未全部迁移。

原五专项、扩大九专项保留；原时区文本核对失败保留，修正版仅按时刻核对，不修改业务行。SQL优先级问题在只读PG检查发现，终止仅本聊天编译且0业务执行留证。第一完整619实测606通过/13失败，都是返工夹具的虚构工作站SID及缺冻结工序；改为新QA企业真实登录SID和实际路线/步骤父记录，不放宽库存/成本/撤权/回滚断言，相关41专项通过。当前修后完整619/0/0/0通过，750.69秒为debug功能时长；十个报工专项包含实际并发上限、跨单/工序、重复冲正、HOLD收货引用、有效后序、原结果恢复、授权撤销/代次/过期、实际存储失败回滚及跨时区读取。

[当前完整独立核对](../qa/acceptance/free-erp-operation-report-oct10-recovered-full-independent-recheck.json) 779项PASS，10342旧企业607表538115行摘要和1035输入不变，16个实际原结果/报工/回执及精确量对SQL；一个声明的新QA源破坏保留并拒绝原回查，不自动修复。真实回环API密码登录得不同当前SID，同一业务用户GET原结果完全一致，六业务表不变，无报工POST重试，见[网络证据](../qa/acceptance/free-erp-operation-report-oct10-network.json)。这不是两台设备GUI或物理故障验收。叶格式及OpenAPI门禁通过；仓储严格203/205仍FAIL，本报工叶0新诊断，API严格未因依赖失败而通过。见[交付记录](../qa/acceptance/free-erp-operation-report-oct10-repair.json)。

正式报工页/持久未决与确定拒绝绑定、员工班组/主数据真实引用、返工产出来源、完整集合和新报工准入的全源核对、完工SKU/产品归属及原内部调用仍须接续。不得把原请求回查校验扩称全部累计来源已核对，或把这批当成完整M10/M11。323仅隔离库迁移试验/追加安装，未验323空库、经营升级/恢复；原生实机、Worker/Release、严格后端与官方免费公网继续未完成，整体PARTIAL。构建固定复用G盘原入口，不在C盘建缓存。


## 五十四 完整报工来源与页面上下文

2026-10-10，新报工准入及现单工序累计GET先验证同工单全部报工行与原结果/回执，真实报工行、原身份/效果/负载及创建时刻必须一致；无原结果的历史行或已改变报工/回执要求核对，不按局部工序筛选绕过。原请求独立GET/重放继续只核对原结果，在别的历史行待核对时仍可恢复正确原命令。

reporting-context提供实际业务工单号、冻结工序/前序/路线版本、工作中心、当前真实认证actor/SID、观察时刻及微量/分钟字符串。实际零仅来自验证后的空报工集。冻结路线的真实父定义/版本/步骤、正且唯一序号及前序低序号检查；报工与冻结工序分别完整1000预算，1001拒绝，不返回截断数量。上限取存储工单或实际前序；余量由已验证真实合格/不良累计扣除，quote不是提交资格。

6专项6/0/0通过、10.62秒为debug功能时长，包含另一工序未知历史行的全范围拒绝、数量/回执破坏不能授予新前序产能、原正确单请求恢复、1001报工预算、缺父/循环/重复序/缺步骤及当前角色/外工单范围。[622独立核对](../qa/acceptance/free-erp-report-source-oct10-independent.json)PASS，11108旧企业607表581347行和1037输入一致；真实empty/posted余量按观察时刻对实存，1007报工行及5真实原结果保留。叶格式与OpenAPI门禁通过；不沿用旧619与严格结果认证当前版本。

当前G盘build/cargo、cargo-home、tmp合计80.0077GiB，超过80GiB入口预算；后续严格入口在执行前明确拒绝，当前完整625及严格NOT_RUN。已只读登记两个本聊天旧编译失败、无占用、无重解析点的working目录，共0.696GiB，预计清后79.3118GiB；有效增量和其余数据不列入清单。用户AGENTS要求先核准，清理尚未批准且0删除，见[审计清单](../qa/acceptance/free-erp-report-source-oct10-cache-cleanup.json)。

正式报工页、持久未决/确定拒绝绑定、员工班组/单位/主数据真实引用及返工产出仍未接完；当前上下文分工序读取累计量，批量聚合和Release规模待补，冻结变更原授权史仍须核对。323空库/经营升级/恢复、实际Worker、严格后端、Release及免费公网继续未完成，整体PARTIAL，见[阶段交付](../qa/acceptance/free-erp-report-source-oct10-repair.json)。


五十四章接续：2026-10-10完整验证行单次汇总改动使用i128中间值，最终转非负i64；上下文/累计与准入复用同一验证数据。只读SQL/等价算术一致，新增边界测试未运行。固定缓存80.0077GiB仍超准入，叶格式/编译/当前625/严格NOT_RUN，旧6专项/622不认证新源。清理仍待人类批准，0删除。见[单次汇总阶段记录](../qa/acceptance/free-erp-report-aggregate-oct10-repair.json)，整体PARTIAL。


## 五十五 200GiB授权维护与单次报工汇总验证

2026-10-10用户明确批准200G构建及自动清理，统一入口固定G盘缓存预算改为200GiB；原target/cargo-home/tmp、build.lock、至少20GiB空闲及固定工具链继续保持。ErpCacheMaintenance仅处理登记清单内编译失败证据明确的增量working目录，绝对边界/目录形状/重解析点/当前Rust工具/逐文件独占打开检查后才清理；预算超限时可自动登记同类编译失败目录。不能恢复准入仍拒绝，未授权其他缓存类别和进程终止。实际删除已批准两目录747293780字节，约0.696GiB，完整路径与UTC时间留证，旧待批准记录保留为历史。边界、增量根/完成目录拒绝探针与脚本语法检查通过。

报工真实验证行一次按工序汇总，i128中间值后转换最终非负i64，避免报告/冲正UUID顺序造成中途溢出误拒；上下文/累计与新报工的本序/前序/后序保护复用同一验证结果，去掉循环SUM往返。两库边界测试、6真实API/PG源专项通过；当前完整625/0/0/0实际通过，771.9秒为debug功能时长。[622独立核对](../qa/acceptance/free-erp-report-aggregate-oct10-full-independent.json)PASS，11128旧企业607表583977行和1038输入一致。仓储严格仍FAIL、408诊断实例，本两报工叶0新增；不称API/发布严格通过。

见[维护及汇总交付](../qa/acceptance/free-erp-200g-report-aggregate-oct10-delivery.json)与[实际清理](../qa/acceptance/free-erp-200g-oct10-authorized-cleanup.json)。80GiB阻塞与先前NOT_RUN是历史，当前授权与验证已解除该存储条件。正式报工页、员工/班组/单位业务引用、持久未决/确定拒绝绑定、GUI、严格后端、Worker、323安装恢复、Release规模及免费公网仍未完成，整体PARTIAL。


五十五章接续：2026-10-10现有仓储命名行、动作元数据和测试布局修复，未扩业务API/页；完整627与616独立核对通过，可比严格203/205降至197/198仍FAIL，记录实际内存/QA恢复/测试期望错误。200GiB固定G缓存保持，统一入口默认单作业；快照全范围分批且同一只读一致性事务。详细限定与证据见[仓储质量交付](../qa/acceptance/free-erp-quality-rows-oct10-delivery.json)，整体PARTIAL。


五十五章接续：2026-10-10现有网格查询/选择仓储改具名输入，同步全部14调用，身份/原SQL/锁/校验/回执不变；API协议专项和共享契约编译通过，完整627及619独立核对通过，12694旧企业607表673351行和1042源码输入一致。严格同口径195/196仍FAIL，总诊断391，减少4实例但183长参数函数仍待修。协议记录/回读不等于真实1000订单服务器分页或双端操作；未扩业务API/页面，200GiB固定G缓存及授权清理边界保持。详见[网格入参质量交付](../qa/acceptance/free-erp-grid-input-oct10-delivery.json)，整体PARTIAL。


### 2026-10-10 当前323迁移空库安装与独立备份恢复QA

当前323份正式SQL在新建的固定用途安装QA库实际从0应用完成，文件SHA逐项绑定；重复入口323已安装/0重新应用。单库事务、advisory锁及每迁移身份/归属标记保护；原私有UI库未作为写入目标。新安装库加入16条直接SQL恢复样本，包含真实企业/账号/店铺/商品/SKU/仓库父项、250001微量库存、中文/空值/二进制、HASH分区、材料原版本和报工原回执。它们是SQL恢复样本，不是经营API流程证明。

自定义备份约1.8MB保留完整owner/ACL信息；以单事务恢复至另一个新空库，无clean/旧备份删除。源及恢复716关系、1842约束、1456索引、87触发器、87函数、612根表全部列/339行、4序列分别读回比较；339为323迁移记录+16合成ERP行。补充6178真实表/视图/分区列ACL/注释、schema/default ACL及locale一致，完整行再次独立SHA256比较。密码哈希验证通过；不可变原报工/材料版本实际返回P0001及精确正文，指定缺失材料行FK返回23503/原约束名；错误库/归属标记实际拒绝，探针后整个恢复库事实不变。

原始失败全部留证：一项CHECK的嵌套AND括号经PostgreSQL原样重解析双方一致，未删除表达式；首不可变探针误断言55000，核实际函数后改P0001+正文而未改业务SQL。补充查询误包含索引内部属性，kit_boms_pkey历史kit_sku_id与新构索引bundle_sku_id单独留证；实际索引定义/有效性与所有真实列一致。数据与其他元数据没有忽略差异。

旧完整QA保护13474企业/607表/718018行和1053源输入前后全部一致。仅新增QA脚本/样本并登记两保护阶段，没有业务UI/API或迁移文件变化，没有Rust构建、数据库/备份删除或终止其他进程。G盘构建200GiB与自动缓存清理边界保持。

本门禁仅同一Windows PostgreSQL17.10集群及既有QA角色；旧经营数据版本升级、跨机/异地灾备、RPO/RTO、Release规模、真实业务双端/原生/Ubuntu、Worker、后端严格、生产迁移/备份调度及官方免费公网继续未完成，整体PARTIAL。见[完整交付/SHA](../qa/acceptance/free-erp-install-restore-oct10-delivery.json)、[恢复与约束实测](../qa/acceptance/install-restore-v323/09-comparison-PASS.json)、[旧库保护](../qa/acceptance/free-erp-install-restore-oct10-protected.json)。


### 2026-10-10 320结构副本带旧样本升级至323

只读核实原安装QA库320迁移、0企业/账号/回执；不把空基线冒称旧经营数据。原库完整对象/行SHA256/序列快照和自定义备份保留，复制到新的固定用途升级库；复制CHECK文本差异只在副本中由PostgreSQL重解析核对，原库无写入。副本先在320结构建立13条直接SQL旧样本（实际父项、中文/空值/二进制、精确库存、报工及2原回执、密码哈希），再逐事务追加0312/0313/0314，保源码SHA。

320→323实际通过，607根表/333行变为612根表/336行，增加仅3迁移记录，所有旧完整行SHA256、旧目录对象、回执/账号/序列保留；5新业务表为空，不给历史数据编造材料版本或报工原结果。重复0应用且全量快照相同；新323目录与独立新安装恢复库精确一致，6178真实列权限/注释/schema/default ACL/locale复核通过。

失败验证：事务内建表+改旧商品后故意触发迁移登记PK23505，连接退出全回滚，表不存在且全量快照相同。在升级副本故意改变历史校验和，真实入口在执行前拒绝；finally以预期错误值CAS恢复，全部事实重新核对不变。错误库实际只读连接被拒。原320源库前后全部对象/行/序列一致；旧完整UI QA13474企业/607表/718018行、1053源输入不变。Python故障SQL的无效转义警告以raw f-string修正，整个可执行AST相同，保原诊断。

没有改业务API/UI或迁移SQL、没有Rust构建/数据库或备份清理/终止其他进程，G盘200GiB和自动缓存清理边界保持。仅同一Windows PostgreSQL17.10集群、QA角色和合成旧样本，真实经营库升级、跨机/异地恢复、RPO/RTO/大规模与生产部署权限、真实业务双端/Ubuntu/Worker、后端严格/Release/官方免费公网仍未完成，整体PARTIAL。见[交付/SHA](../qa/acceptance/free-erp-upgrade-v323-oct10-delivery.json)、[升级原样保留](../qa/acceptance/upgrade-v323/07-upgrade-PASS.json)、[实际失败门禁](../qa/acceptance/upgrade-v323/11-operational-probes-PASS.json)。
