"""Add a source-bound delivery index to the existing ERP specifications only."""

from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / "qa/acceptance"
prefix = "free-erp-doc-detail-matrix"
report_path = base / f"{prefix}-edit.json"
if report_path.exists():
    raise SystemExit("Refusing to overwrite an earlier document edit record")

names = [
    "docs/FREE_ERP_DEVELOPMENT_SPEC.md", "docs/FREE_ERP_CHECKLIST.md",
    "docs/README.md", "docs/ROADMAP.md",
]
original = {name: (root / name).read_bytes() for name in names}
texts = {name: raw.decode("utf-8").replace("\r\n", "\n") for name, raw in original.items()}
spec_name, checklist_name, index_name, roadmap_name = names
spec, checklist = texts[spec_name], texts[checklist_name]
heading = "## 十八 逐功能交付定位和真实账号专项"
if heading in spec:
    raise SystemExit("The delivery index already exists; review rather than append it twice")

# Page, implementation and business-script IDs are authored independently of
# the field/path text copied from the authoritative feature cards.
mapping = {
    "A01": ("01", "01", "01"),
    "A02": ("03", "02", "01"),
    "A03": ("03", "03", "02"),
    "A04": ("09/13/18/28", "04", "13"),
    "A05": ("02", "01/04", "01"),
    "A06": ("01/08", "01/37", "01"),
    "A07": ("03/28", "03/40", "02/13"),
    "A08": ("28", "40", "14"),
    "A09": ("28", "38/40", "14"),
    "A10": ("28", "39/40", "13/14"),
    "U01": ("全部业务页", "37", "11"),
    "U02": ("全部业务页", "37", "11/13"),
    "U03": ("全部业务页", "37", "11"),
    "U04": ("全部录单页", "37", "04/06/11"),
    "U05": ("全部录单页", "37/39", "11/12"),
    "U06": ("全部业务页", "37", "11"),
    "U07": ("08/27/28", "06/38", "10"),
    "U08": ("14/27", "22/36/37", "07/10"),
    "D01": ("04", "05", "03"),
    "D02": ("04", "05/09/10", "03"),
    "D03": ("05", "05/35", "04"),
    "D04": ("06", "04/05", "13"),
    "D05": ("07/15", "05/24/30", "06"),
    "D06": ("08", "06", "01/03"),
    "D07": ("08", "07/15/35", "03/14"),
    "D08": ("04/14及附件宿主", "08", "13"),
    "I01": ("09", "09/15", "05"),
    "I02": ("10/13/19", "10/11", "05/12"),
    "I03": ("10", "12/13", "05"),
    "I04": ("11", "14/33", "06"),
    "I05": ("09/26", "15", "04/06"),
    "I06": ("09/11/28", "38", "10"),
    "I07": ("09/27", "09/36", "10"),
    "I08": ("11", "14/20/29", "06"),
    "T01": ("12", "16", "04"),
    "T02": ("12", "17/18", "04"),
    "T03": ("13", "19", "04"),
    "T04": ("13", "20", "04/05"),
    "T05": ("13", "21", "04"),
    "T06": ("12/13", "16/19/39", "04/12"),
    "T07": ("12/13/25", "38", "10"),
    "T08": ("12/13及报价信用扩展", "16/19", "04"),
    "T09": ("12/13及货权扩展", "16/20", "04"),
    "M01": ("14", "22", "07"),
    "M02": ("14", "23", "07"),
    "M03": ("15", "24", "07"),
    "M04": ("15", "24", "07/08"),
    "M05": ("16", "25", "08"),
    "M06": ("17", "26", "08"),
    "M07": ("17", "27", "09"),
    "M08": ("18", "28", "06/07"),
    "M09": ("19", "29", "06/12"),
    "M10": ("20", "30", "06"),
    "M11": ("21", "31", "06"),
    "M12": ("24", "34", "06"),
    "M13": ("18/20/24", "28/30/34", "06"),
    "M14": ("委外页（规划扩展）", "16/17/28", "06"),
    "M15": ("14/18及工程变更扩展", "23/28", "07"),
    "M16": ("18/19/21及联副产扩展", "29/31/34", "06"),
    "M17": ("18/27", "36", "10"),
    "Q01": ("22", "32", "06"),
    "Q02": ("22", "32", "06"),
    "Q03": ("23", "33", "06"),
    "Q04": ("22/23/27及召回扩展", "32/33/36", "06"),
    "F01": ("25", "07/35", "04"),
    "F02": ("25", "35", "04"),
    "F03": ("26", "34/35", "06"),
    "F04": ("24/26", "15/34/35", "04/06"),
    "F05": ("25及发票扩展", "35", "04"),
    "F06": ("24/26及分摊扩展", "34/35", "06"),
    "R01": ("27", "36", "10"),
    "R02": ("27", "36", "10"),
    "R03": ("28", "38", "10"),
    "R04": ("28", "39", "12"),
    "R05": ("27", "36/37", "10"),
    "O01": ("所有正式写入口", "10/20/29/35/39", "12"),
    "O02": ("所有正式写入口及28", "39", "12"),
    "O03": ("28及Worker", "39", "12"),
    "O04": ("全部录单页", "37/39", "11"),
    "O05": ("全部受保护入口及28", "04/40", "13"),
    "O06": ("运维入口", "40", "14"),
    "O07": ("运维入口", "40", "14"),
    "O08": ("28及运维入口", "39/40", "14"),
    "O09": ("公网入口", "40", "13"),
    "O10": ("全部业务页及发布入口", "37/40", "11/14"),
    "O11": ("CI及发布入口", "40", "14"),
    "O12": ("运维入口", "39/40", "14"),
    "V01": ("成员与审批（规划）", "02/03/04", "01/02/13"),
    "V02": ("集团与工厂（规划）", "05/13/35", "04"),
    "V03": ("有限产能排程（规划）", "24/25/26", "08"),
    "V04": ("设备与车间（规划）", "30/32", "06"),
    "V05": ("连接器（规划）", "39/40", "12/13"),
    "V06": ("高级分析（规划）", "36/39", "10"),
    "V07": ("托管控制面（规划）", "40", "14"),
    "V08": ("账单与权益（规划）", "02", "01"),
    "V09": ("会计与税务（规划）", "35", "04"),
    "X01": ("配方与批次生产（规划）", "22/24/29/34", "06"),
    "X02": ("机加工与委外（规划）", "24/28/30", "06"),
    "X03": ("序列装配与售后（规划）", "14/30/33", "06"),
    "X04": ("跨境贸易（规划）", "16/19/35", "04"),
}

cards = list(re.finditer(
    r"^#### ([A-Z]\d{2}) ([^\n]+)\n(.*?)(?=^#### |^### |^## |\Z)",
    checklist, re.M | re.S,
))
priority_rows = dict(re.findall(r"^\| \[[ x]\] ([A-Z]\d{2}) \| ([^|]+?) \|", checklist, re.M))
priorities = {
    feature: feature[0] if feature[0] in {"V", "X"} else value.strip()
    for feature, value in priority_rows.items()
}
if len(cards) != 100 or set(mapping) != {match.group(1) for match in cards}:
    raise SystemExit("The authored mapping must cover exactly the current 100 cards")
if set(priorities) != set(mapping):
    raise SystemExit("Priority rows and feature cards disagree")

def required(body, label):
    match = re.search(r"^- " + re.escape(label) + r"：(.*)$", body, re.M)
    if not match:
        raise SystemExit(f"Missing source card field: {label}")
    return match.group(1).strip().replace("|", "\\|")

matrix = [
    heading, "",
    "### 100项功能的一对一交付定位", "",
    "本表把原100项功能直接连接到页面、实施单元和量化门禁。点击功能名读完整字段、按钮、接口、表和失败断言；文件简称沿第一章。代码列保留原卡的“现/规”标记，现有源码只是复用入口，仍须逐项盘点业务缺口；规划位置不是已实现文件。页面编号查第六章，ERP-IMPL查第二章，UAT查第八章。所有行的正式双端、托管和性能完成状态均需另附证据，本表不授予PASS。", "",
    "同一页面承载多个功能时，按功能ID分别验收。F1、V和X行的ERP-IMPL/UAT仅表示可复用基础；报价、委外、联副产、APS、设备、预测和行业等专项必须同时满足原卡的专属样本，不能拿基础剧本通过替代整项。调拨/盘点I03也须执行ERP-IMPL-12/13的守恒和截止点样本，UAT-05只验证共享库存。", "",
    "| 功能与原优先级 | 页面 / 实施单元 / 基础剧本 | 现有与待补代码位置 | 必须交付的失败验收 | 量化性能门禁（目标） |",
    "| --- | --- | --- | --- | --- |",
]
for match in cards:
    feature, title, body = match.groups()
    pages, units, scripts = mapping[feature]
    if re.fullmatch(r"\d{2}(?:/\d{2})*", pages):
        pages = "PAGE-" + pages
    else:
        pages = re.sub(r"\b(\d{2})(?=/|及|$)", r"PAGE-\1", pages)
    anchor = re.sub(r"[^\w -]", "", f"{feature} {title}".lower()).strip().replace(" ", "-")
    location = required(body, "位置")
    failure = required(body, "失败断言")
    performance = required(body, "量化门禁")
    priority = priorities[feature].strip()
    matrix.append(
        f"| [{feature} {title}](FREE_ERP_CHECKLIST.md#{anchor}) / {priority} "
        f"| {pages}；ERP-IMPL-{units}；UAT-{scripts} "
        f"| {location} | {failure} | {performance} |"
    )

matrix.extend(["", "### 正式账号和多端的当前接线", "", """免费边界仍为一个业务登录账号、同账号多端在线；当前会话默认上限8及期限3600秒是现有安全配置，不增加免费席位。下面给Grok的任务补足A02/A03/A04/A07、O01/O02/O04/O05；收费多人审批不作为其前置。

| 当前位置 / HTTP入口 | 已核对的接线 | 仍须完成的验收 |
| --- | --- | --- |
| 现 `API/router.rs`；POST `/auth/login`，现 `API/auth_handlers.rs::login`；现 `REPO/auth.rs`、`REPO/access_plan.rs::open_session` | 密码查验及数据库角色取权限；真实`auth_sessions`，服务器签发令牌绑定sid/cep。登录入参有tenant_id/email/password/client_kind/client_label；响应有access_token/token_type/expires_in/session_id/session_policy | 普通非管理员账号真登录后完成库存、采购、销售、制造动作；自行签发Claims的测试不能作此证据 |
| 现 GET `/me/sessions`、DELETE `/me/sessions/{session_id}`，现 `API/auth_handlers.rs::{list_own_sessions,revoke_own_session}` | 按当前企业/账号读取和撤销真实会话 | 列表中当前端、到期/最近活动是真实事实，不含令牌；撤A保留B/C，撤销后A不能发新写入 |
| 现 PUT `/me/password`，现 `API/auth_handlers.rs::update_current_user_password`；现 `REPO/access_plan.rs::bump_credentials_epoch_in_tx` | 密码hash替换和凭据代次在同事务提交，不能继续描述为分开调用 | 真实旧会话/旧密码拒绝、新密码可登录；代次更新SQL失败回滚密码，不保存“改密成功”假结果；必要安全审计另核对提交方式 |
| 现 `API/onboarding.rs` 的 `/auth/sessions/renew`、`/auth/password/recover`；现 `REPO/shared_contract.rs` 的login-client契约 | 路由和契约记录已经存在；属于需要核对的接续入口 | 续期须产生可用于正式业务的服务器令牌/真实会话；找回须验证真实一次性凭据并替换正式密码。只更新login_clients/password_recoveries记录不足以完成A03 |
| 现 `REPO/inventory_movement_commands.rs::{execute,payload_hash,verified_snapshot}` | 原库存命令要求session_id；先读当前数据库角色与会话。当前v1摘要序列化整个input，input含session_id | 本轮静态发现同用户换设备重试可能产生不同摘要；真实两会话反例尚未执行，不写为已修复。授权会话与业务语义指纹分开，按IAM-04/09接续 |
| 现 `API/auth_layers.rs::auth_middleware`、`API/rbac_user_handlers.rs::update_user_roles`、`API/rbac_role_handlers.rs::update_role_permissions` | 普通正式登录含sid/cep；部分旧测试使用不带会话的合成令牌 | 逐入口证明当前角色、用户状态、仓库范围和会话权威；角色检查后等待库存锁的竞争需验证共同授权顺序，不能仅看JWT权限或一次提前COUNT |

### 10项真实账号实施任务

这些是待补接线及验收任务，初始状态NOT_RUN；规划测试文件须先接入现登记`free_erp_acceptance`，按统一构建入口执行。不得为让旧夹具通过而删除会话/角色校验。

| 任务 | 具体交付与实现位置 | 数据、拒绝与结束条件 |
| --- | --- | --- |
| IAM-01 真登录夹具 | 规 `crates/erp-api/tests/free_erp_acceptance/account_login.rs`统一构建本批新企业、唯一用户、正式角色/权限、仓库范围；复用现 `API/auth_handlers.rs::login`、`REPO/auth.rs` | 有效Argon2密码hash，实际POST取得服务端令牌；每夹具有界复用会话，不每请求开新会话。不在日志输出密码/令牌；独立核对users/user_roles/role_permissions/auth_sessions |
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

### Grok逐项结项记录

每次只领取明确功能ID及IAM任务；交付单至少逐格记录：字段/按钮、实际文件/函数、真实HTTP与数据库迁移、正常/边界/失败/并发/重试/越权独立事实、native/WASM真实操作、Release负载和资源、源码/产物SHA、PASS/FAIL/NOT_RUN与剩余项。数据库回读要包含正式角色/会话，不能只保护库存表；原命令和原提交出处不得覆盖。文档本次补细及引用校验不运行构建、不修改运行数据，也不把上表性能目标写成实测值。
"""])

def replace_once(text, old, new):
    if text.count(old) != 1:
        raise SystemExit(f"Expected one source fragment, found {text.count(old)}: {old[:90]}")
    return text.replace(old, new, 1)

page_corrections = {
    "06": ("ERP-IMPL-05/08", "ERP-IMPL-04/05"),
    "09": ("ERP-IMPL-08/12/15", "ERP-IMPL-09/11/14/15"),
    "10": ("ERP-IMPL-09/10/11；P04/P05/P12", "ERP-IMPL-10/12/13；P04/P05"),
    "11": ("ERP-IMPL-12/13/33", "ERP-IMPL-14/32/33"),
    "12": ("ERP-IMPL-16/17/35", "ERP-IMPL-16/17/18/35"),
    "13": ("ERP-IMPL-18/19/20/35", "ERP-IMPL-19/20/21/35"),
}
for number, (old, new) in page_corrections.items():
    matches = list(re.finditer(r"^\| PAGE-" + number + r" [^\n]+$", spec, re.M))
    if len(matches) != 1:
        raise SystemExit(f"Expected one page row: {number}")
    row = matches[0].group(0)
    spec = replace_once(spec, row, replace_once(row, old, new))

spec = replace_once(
    spec,
    "`REPO/access_plan.rs::{open_session,active_session,bump_credentials_epoch}`。",
    "`REPO/access_plan.rs::{open_session,active_session,bump_credentials_epoch_in_tx}`。",
)
spec = replace_once(
    spec,
    "将改密、禁用与凭据代次更新接入一致的安全事务，核对现有分开调用；桌面/Web网络适配分别接真实身份。",
    "现PUT改密已将正式密码与代次更新放在同一事务；核对禁用、找回、真实续期及必要安全记录接线；桌面/Web网络适配分别接真实身份。",
)
spec = replace_once(
    spec,
    "| 确定当前该交付什么以及如何结项 | [批次和交付记录](#五-开发批次和交付记录) |",
    "| 逐功能定位页面、代码、验收和性能；补真实账号接线 | [100行交付矩阵、10账号任务、12样本及6组负载](#十八-逐功能交付定位和真实账号专项) |\n| 确定当前该交付什么以及如何结项 | [批次和交付记录](#五-开发批次和交付记录) |",
)
spec += "\n" + "\n".join(matrix).rstrip() + "\n"

checklist = replace_once(
    checklist,
    "- 契约：现登录及 `/me/sessions`；规续期和找回接口，复用 `auth_sessions`、凭证代次及用户状态，更新会话不能消耗第二席位。",
    "- 契约：现正式登录及 `/me/sessions`，复用 `auth_sessions`、凭证代次及用户状态；现 `/auth/sessions/renew`和`/auth/password/recover`已有login-client契约路由，待核对其正式令牌/真实会话/密码接线，更新会话不能消耗第二席位。",
)
checklist = replace_once(
    checklist,
    "- 实施接线：现 POST /auth/login、GET /me/sessions、DELETE /me/sessions/{session_id}、PUT /me/password；找回密码和续期另补正式路由。",
    "- 实施接线：现 POST /auth/login、GET /me/sessions、DELETE /me/sessions/{session_id}、PUT /me/password；找回与续期从现有契约路由接真实验证凭据、正式密码/会话及可用令牌，详见[IAM-01至10](FREE_ERP_DEVELOPMENT_SPEC.md#十八-逐功能交付定位和真实账号专项)。",
)
checklist = replace_once(
    checklist,
    "| 分批领取工作 | [开发任务包](#二十四-grok-开发任务包和具体交付示例) | 16个任务包的依赖、改动范围与交付结果 |",
    "| 逐项定位实现和验收 | [100行功能交付矩阵](FREE_ERP_DEVELOPMENT_SPEC.md#十八-逐功能交付定位和真实账号专项) | 每项原优先级、页面/实施单元/剧本、现有与待补代码、失败验收及量化性能；另附真实账号10任务/12样本/6负载 |\n| 分批领取工作 | [开发任务包](#二十四-grok-开发任务包和具体交付示例) | 16个任务包的依赖、改动范围与交付结果 |",
)
texts[spec_name], texts[checklist_name] = spec, checklist
texts[index_name] = replace_once(
    texts[index_name],
    "## 推荐阅读顺序\n",
    "## 推荐阅读顺序\n\n逐项交给Grok开发时先看 [100行功能交付矩阵](FREE_ERP_DEVELOPMENT_SPEC.md#十八-逐功能交付定位和真实账号专项)：功能ID/原优先级连接页面、实施单元、代码、失败样本和量化性能；点击功能名读完整字段及操作。新增10项真实账号接线、12个可复算多端样本和6组Release目标，覆盖唯一席位、正式登录/续期、撤权竞争、跨端同请求重放、历史回执及双端恢复。标明源码已接线与仍待验收的边界；性能目标不算实测。\n",
)
texts[roadmap_name] += """

## 2026-10-04 功能位置、真实账号与性能细化文档

- 按用户“具体哪些功能、实现在什么位置、性能指标”的要求，在[实施规格第十八章](FREE_ERP_DEVELOPMENT_SPEC.md#十八-逐功能交付定位和真实账号专项)补100行一对一交付定位；沿原72项F0、15项F1及13项收费/行业边界，串页面、实施单元、基础剧本、现有/规划文件、失败断言及量化门禁。
- 修正6个页面与实施单元的对应关系：仓库资料、库存查询、库存作业、批次、采购收退、销售交退；不新增业务菜单或实现代码。
- 补10项真实账号任务、12组具体样本和6组Release负载；实查正式登录/会话/改密及已存在的续期找回契约入口。现改密已同事务，正式续期/找回和全域真实会话仍需验收。
- 当前库存命令v1指纹序列化包含session_id的整个输入，换端同业务可能冲突。此为静态风险及待复现/修复任务，未运行真实两会话反例；原摘要版本和提交出处应保留，不能重写历史回执。
- 本批仅文档及引用、一致性检查，无Rust构建、业务测试、数据库写入、部署或性能测量；当前功能和免费发布资格继续沿各自实际报告，不能由文档补细标为已完成。
"""

new_bytes = {}
for name, value in texts.items():
    newline = "\r\n" if b"\r\n" in original[name] else "\n"
    new_bytes[name] = value.replace("\n", newline).encode("utf-8")
for name, raw in original.items():
    if (root / name).read_bytes() != raw:
        raise SystemExit(f"Concurrent edit; no documents written: {name}")
for name, raw in original.items():
    backup = base / f"{prefix}-before-{name.replace('/', '_')}.txt"
    with backup.open("xb") as handle:
        handle.write(raw)
for name, raw in new_bytes.items():
    if (root / name).read_bytes() != original[name]:
        raise SystemExit(f"Concurrent edit before writing; stop without overwriting: {name}")
    (root / name).write_bytes(raw)

record = {
    "updated_at": datetime.now(timezone.utc).isoformat(),
    "scope": "Documentation only; not business, build or performance acceptance",
    "feature_mapping": {feature: list(value) for feature, value in mapping.items()},
    "page_unit_corrections": page_corrections,
    "before_sha256": {name: hashlib.sha256(raw).hexdigest() for name, raw in original.items()},
    "after_sha256": {name: hashlib.sha256(raw).hexdigest() for name, raw in new_bytes.items()},
    "source_feature_cards": 100,
    "account_tasks": 10, "account_cases": 12, "account_performance_groups": 6,
    "business_tests_executed": 0, "rust_builds_executed": 0,
    "database_mutations": 0,
}
with report_path.open("x", encoding="utf-8") as handle:
    json.dump(record, handle, ensure_ascii=False, indent=2)
    handle.write("\n")
print(json.dumps({"status": "EDITED", "documents": names, "feature_rows": 100,
                  "tasks": 10, "cases": 12, "performance_groups": 6}, ensure_ascii=False))
