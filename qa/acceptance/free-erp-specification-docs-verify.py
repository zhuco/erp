"""Verify the ERP checklist structure and the current referenced declarations."""

from datetime import datetime, timezone
from decimal import Decimal
import argparse
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / "qa/acceptance"
report_file = base / "free-erp-specification-docs.json"
previous_bytes = report_file.read_bytes()
previous = json.loads(previous_bytes.decode("utf-8-sig"))
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--output", help="Write a new report under qa/acceptance without replacing prior evidence")
args = parser.parse_args()
if args.output:
    candidate = (root / args.output).resolve()
    if not candidate.is_relative_to(base.resolve()) or candidate.suffix != ".json":
        raise SystemExit("Output must be a JSON file under qa/acceptance")
    if candidate.exists():
        raise SystemExit("Refusing to overwrite an existing verification report")
    report_file = candidate
documents = [
    "docs/FREE_ERP_CHECKLIST.md", "docs/PRODUCT_SCOPE.md", "docs/ROADMAP.md",
    "docs/README.md", "docs/FREE_ERP_DEVELOPMENT_SPEC.md",
    "docs/UI_COMPONENT_LIBRARY.md",
]
texts = {}
document_hashes = {}
errors = []
for name in documents:
    raw = (root / name).read_bytes()
    try:
        value = raw.decode("utf-8")
    except UnicodeDecodeError as error:
        errors.append(f"{name}: {error}")
        continue
    if value.startswith("\ufeff") or "\ufffd" in value or "\x00" in value:
        errors.append(f"{name}: invalid document encoding marker")
    texts[name] = value
    document_hashes[name] = hashlib.sha256(raw).hexdigest()

local_links = 0
internal_heading_links = 0
for name, value in texts.items():
    heading_slugs = set()
    for heading in re.findall(r"^#{1,6} (.+?)\s*$", value, re.M):
        slug = re.sub(r"[^\w -]", "", heading.lower()).strip().replace(" ", "-")
        heading_slugs.add(slug)
    for target in re.findall(r"\[[^\]\n]+\]\(([^)\n]+)\)", value):
        if target.startswith("#"):
            internal_heading_links += 1
            if target[1:] not in heading_slugs:
                errors.append(f"{name}: missing heading link {target}")
            continue
        if re.match(r"^[A-Za-z]+://", target):
            continue
        path = target.split("#", 1)[0].split(":", 1)[0].strip("<>")
        if path and not ((root / name).parent / path).is_file():
            errors.append(f"{name}: missing local link {target}")
        local_links += 1

checklist = texts[documents[0]]
development_spec = texts["docs/FREE_ERP_DEVELOPMENT_SPEC.md"]
cards = re.findall(r"^#### ([A-Z][0-9]{2})\s+", checklist, re.M)
field_lists = len(re.findall(r"^- 最小字段与操作：", checklist, re.M))
performance_gates = len(re.findall(r"^- 量化门禁：", checklist, re.M))
page_operations = len(re.findall(r"^- 页面操作：", checklist, re.M))
integration_instructions = len(re.findall(r"^- 实施接线：", checklist, re.M))
failure_assertions = len(re.findall(r"^- 失败断言：", checklist, re.M))
metric_ids = sorted(set(re.findall(r"ERP-P([0-9]{2})", checklist)))
task_ids = sorted(set(re.findall(r"\| DEV-([0-9]{2})\s", checklist)))
case_ids = re.findall(r"^\| ERP-CASE-([0-9]{2}) /", checklist, re.M)
inventory_section = checklist.split("## 二十五", 1)[1].split("## 二十六", 1)[0]
inventory_rows = [line for line in inventory_section.splitlines() if line.startswith("|")]
inventory_count = len(inventory_rows) - 2
json_examples = re.findall(r"```json\s*\n(.*?)\n```", checklist, re.S)
for example in json_examples:
    try:
        json.loads(example)
    except json.JSONDecodeError as error:
        errors.append(f"JSON example: {error}")

table_count = 0
for name, value in texts.items():
    block = []
    for line in value.splitlines() + [""]:
        if line.startswith("|"):
            block.append(line)
            continue
        if not block:
            continue
        widths = [len(re.split(r"(?<!\\)\|", row)) for row in block]
        if len(block) < 2 or len(set(widths)) != 1:
            errors.append(f"{name}: inconsistent table widths {widths}")
        table_count += 1
        block = []

for condition, label in [
    (len(cards) == 100 and len(set(cards)) == 100, "100 unique feature cards"),
    (field_lists == 100 and performance_gates == 100, "100 field lists and performance gates"),
    (page_operations == integration_instructions == failure_assertions == 100,
     "100 page-operation lists, integration instructions and failure assertions"),
    (metric_ids == [f"{i:02}" for i in range(1, 23)], "22 metric IDs"),
    (task_ids == [f"{i:02}" for i in range(1, 17)], "16 task package IDs"),
    (case_ids == [f"{i:02}" for i in range(1, 17)], "16 acceptance case IDs"),
    (inventory_count == 14, "14 actual inventory entry rows"),
    (len(json_examples) == 2, "two valid JSON examples"),
]:
    if not condition:
        errors.append(f"Failed structure requirement: {label}")

# Explicit declaration move; preserve the previous report as historical evidence.
symbols = []
for entry in previous["referenced_source_symbols"]:
    if entry["file"] == "crates/erp-api/src/manufacturing.rs" and entry["symbol"] in {"AtomicEffectRequest", "commit_atomic_effect_route"}:
        entry = {**entry, "file": "crates/erp-api/src/manufacturing/atomic_effects.rs"}
    if entry["file"] == "crates/erp-repo/src/inventory_batch_consumption.rs" and entry["symbol"] == "InventoryBatchLimit":
        entry = {**entry, "file": "crates/erp-repo/src/inventory_batch_effects.rs"}
    if entry not in symbols:
        symbols.append(entry)
# Four public read methods moved to the owned leaf; get_balance and writers
# retain their original declarations. Keep historical validation files intact.
for entry in symbols:
    if entry["file"] == "crates/erp-repo/src/inventory.rs" and entry["symbol"] in {
        "list_inventory", "list_inventory_summary", "list_inventory_costs", "list_ledgers",
    }:
        entry["file"] = "crates/erp-repo/src/inventory_reads.rs"
prefixes = {
    "UI": "apps/erp-ui/src", "API": "crates/erp-api/src", "REPO": "crates/erp-repo/src",
    "DOMAIN": "crates/erp-domain/src", "WORKER": "crates/erp-worker/src",
    "CORE": "crates/erp-grid-core/src", "EGUI": "crates/erp-ui-egui/src",
}
referenced_rust_locations = {}
for line_number, line in enumerate(checklist.splitlines(), 1):
    for match in re.finditer(r"`((?:UI|API|REPO|DOMAIN|WORKER|CORE|EGUI)/[^`]+)`", line):
        token = match.group(1).split("::", 1)[0]
        if any(char in token for char in "{}*, "):
            continue
        prefix, leaf = token.split("/", 1)
        name = prefixes[prefix] + "/" + leaf
        exists = (root / name).exists()
        referenced_rust_locations[name] = exists
        markers = list(re.finditer(r"(?:现|规)\s*(?=`|$)", line[:match.start()]))
        declared = markers[-1].group(0).strip() if markers else None
        if declared == "现" and not exists:
            errors.append(f"Missing declared current Rust location at line {line_number}: {name}")
    if re.search(r"`[A-Za-z]+`(?:UI|API|REPO|DOMAIN|WORKER|CORE|EGUI)/", line):
        errors.append(f"Malformed Rust path code span at line {line_number}")


def add_symbol(name, symbol):
    if name == "crates/erp-api/src/manufacturing.rs" and symbol in {"AtomicEffectRequest", "commit_atomic_effect_route"}:
        name = "crates/erp-api/src/manufacturing/atomic_effects.rs"
    entry = {"file": name, "symbol": symbol}
    if entry not in symbols:
        symbols.append(entry)


implementation_units = re.findall(
    r"^### ERP-IMPL-([0-9]{2}) [^\n]+\n(.*?)(?=^### |^## |\Z)",
    development_spec, re.M | re.S,
)
if [number for number, _ in implementation_units] != [f"{i:02}" for i in range(1, 41)]:
    errors.append("Expected 40 unique ordered concrete implementation units")
implementation_current_locations = set()
implementation_current_declarations = []
for number, body in implementation_units:
    for label in ["功能与字段", "现有接续", "补齐位置", "事务与拒绝", "验收与性能"]:
        if not re.search(r"^- " + label + r"：\S", body, re.M):
            errors.append(f"ERP-IMPL-{number}: missing {label}")
    current = re.search(r"^- 现有接续：(.*)", body, re.M)
    if not current:
        continue
    current_line = current.group(1)
    for token in re.findall(r"`([^`]+)`", current_line):
        match = re.fullmatch(
            r"(UI|API|REPO|DOMAIN|WORKER)/([^:]+\.rs)(?:::(.+))?", token,
        )
        if match:
            prefix, leaf, declaration = match.groups()
            name = prefixes[prefix] + "/" + leaf
        elif re.fullmatch(r"(?:apps|crates)/[A-Za-z0-9_/-]+\.rs", token):
            name, declaration = token, None
        else:
            continue
        implementation_current_locations.add(name)
        if not (root / name).is_file():
            errors.append(f"ERP-IMPL-{number}: missing current file {name}")
        if declaration:
            for symbol in declaration.strip("{}").split(","):
                symbol = symbol.strip()
                if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", symbol):
                    errors.append(f"ERP-IMPL-{number}: invalid current declaration {token}")
                    continue
                add_symbol(name, symbol)
                implementation_current_declarations.append({"file": name, "symbol": symbol})
    if "REPO/shared_contract.rs` 的 " in current_line:
        tail = current_line.split("REPO/shared_contract.rs` 的 ", 1)[1]
        for symbol in re.findall(r"`([a-z][a-z0-9_]+)`", tail):
            name = "crates/erp-repo/src/shared_contract.rs"
            add_symbol(name, symbol)
            implementation_current_declarations.append({"file": name, "symbol": symbol})

implementation_metrics = re.findall(r"^\| ERP-P([0-9]{2}) \|", development_spec, re.M)
if implementation_metrics != [f"{i:02}" for i in range(1, 23)]:
    errors.append("Expected 22 unique ordered performance execution tasks")
for metric, cells in re.findall(r"^\| ERP-P([0-9]{2}) \| (.+)$", development_spec, re.M):
    if len(cells.split("|")) != 4 or any(not cell.strip() for cell in cells.split("|")[:3]):
        errors.append(f"ERP-P{metric}: incomplete load, measurement or acceptance details")
for feature in set(re.findall(r"(?<![A-Za-z0-9_-])\b([AUDITMQFROVX][0-9]{2})\b", development_spec)):
    if feature not in cards:
        errors.append(f"Development specification references unknown feature {feature}")
for token in [
    "## 三 关键动作的事务顺序", "### 工单下达", "### 材料动作与完工放行",
    "### 应收核销", "### 当前实现与验收边界", "### 一项任务的验收单",
    "-Release -RunPerformance", "--test-threads=1",
]:
    if token not in development_spec:
        errors.append(f"Missing implementation or measurement requirement: {token}")
development_json_examples = re.findall(r"```json\s*\n(.*?)\n```", development_spec, re.S)
if len(development_json_examples) != 4:
    errors.append("Expected two planned business examples plus actual inventory export and reservation request shapes")
for example in development_json_examples:
    try:
        json.loads(example)
    except json.JSONDecodeError as error:
        errors.append(f"Implementation JSON example: {error}")
example_arithmetic_checks = 0
if len(development_json_examples) == 4:
    material, settlement, reservation, inventory_export = [json.loads(value) for value in development_json_examples]
    reservation_fields = {"request_id", "warehouse_id", "sku_id", "location_id", "batch_code", "quality_status", "reference_id", "quantity"}
    if (set(reservation) != reservation_fields or reservation["quality_status"] != "GOOD"
        or type(reservation["quantity"]) is not int or reservation["quantity"] != 2
        or reservation["quantity"] * 1_000_000 != 2_000_000
        or not re.fullmatch(r"[!-~]{1,128}", reservation["request_id"])
        or not reservation["reference_id"].strip() or not reservation["batch_code"].strip()
        or any(not re.fullmatch(r"[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}", reservation[key])
               for key in ["warehouse_id", "sku_id", "location_id"])):
        errors.append("Reservation example must match the strict source/location whole-unit request and exact micro-unit conversion")
    else:
        example_arithmetic_checks += 1
    material_lines = material["lines"]
    source_keys = [
        tuple(line[key] for key in ["warehouse_id", "sku_id", "batch_id", "original_issue_id"])
        for line in material_lines
    ]
    if len(set(source_keys)) != 1 or {line["kind"] for line in material_lines} != {"RETURN", "CONSUME"}:
        errors.append("Material example must use the same physical original source")
    else:
        example_arithmetic_checks += 1
    params = inventory_export.get("params", {})
    query = params.get("query", {})
    if (inventory_export.get("type") != "INVENTORY_EXPORT"
        or set(params) != {"request_id", "query"}
        or set(query) != {"warehouse_id", "available_min", "available_max"}
        or not Decimal(query["available_min"]) <= Decimal(query["available_max"])):
        errors.append("Inventory export example must use the actual strict request and exact inclusive range")
    else:
        example_arithmetic_checks += 1
    material_remaining = Decimal("1.000000") - sum(Decimal(line["quantity"]) for line in material_lines)
    if material_remaining != Decimal("0.200000"):
        errors.append("Material example does not conserve the stated remaining quantity")
    else:
        example_arithmetic_checks += 1
    allocated_minor = sum(line["amount_minor"] for line in settlement["allocations"])
    if allocated_minor != 60000 or 100000 - allocated_minor != 40000:
        errors.append("Settlement example does not conserve the stated payment and receivable")
    else:
        example_arithmetic_checks += 1
report_section = development_spec.split("### 五种固定报表的负载口径", 1)[1].split(
    "### 实施位置和原始证据", 1,
)[0]
fixed_reports = len([line for line in report_section.splitlines() if line.startswith("|")]) - 2
if fixed_reports != 5:
    errors.append("Expected five specified report workloads")

page_section = development_spec.split("### 菜单 操作和页面文件", 1)[1].split(
    "### 页面接入必须改哪些现有文件", 1,
)[0]
page_rows = re.findall(r"^\| PAGE-([0-9]{2}) ([^\n]+)$", page_section, re.M)
if [number for number, _ in page_rows] != [f"{i:02}" for i in range(1, 29)]:
    errors.append("Expected 28 ordered planned page hosts")
page_scopes = []
for number, cells in page_rows:
    if len(cells.split("|")) != 5 or any(not cell.strip() for cell in cells.split("|")[:4]):
        errors.append(f"PAGE-{number}: missing actions, UI location or backend/performance")
    scopes = re.findall(
        r"`((?:account|master|inventory|trade|manufacturing|quality|finance|reports|system)"
        r"\.[a-z_]+)`", cells,
    )
    if len(scopes) != 1 or "UI/pages/" not in cells or "ERP-IMPL-" not in cells:
        errors.append(f"PAGE-{number}: missing planned scope, UI file or implementation unit")
    page_scopes.extend(scopes)
if len(set(page_scopes)) != 28:
    errors.append("Planned page scopes must have 28 unique identities")
uat_ids = re.findall(r"^### UAT-([0-9]{2}) ", development_spec, re.M)
if uat_ids != [f"{i:02}" for i in range(1, 15)]:
    errors.append("Expected 14 ordered executable business acceptance scripts")
priority_counts = {}
for priority in re.findall(r"^\| \[[ x]\] [A-Z][0-9]{2} \| (F[01]) \|", checklist, re.M):
    priority_counts[priority] = priority_counts.get(priority, 0) + 1
if priority_counts != {"F0": 72, "F1": 15}:
    errors.append("Refresh the implementation specification's free-feature counts")

delivery_heading = "## 十八 逐功能交付定位和真实账号专项"
delivery_section = development_spec.split(delivery_heading, 1)[-1]
if delivery_heading not in development_spec:
    errors.append("Missing per-feature delivery index and real-account requirements")
delivery_matrix_section = delivery_section.split("### 正式账号和多端的当前接线", 1)[0]
delivery_rows = re.findall(
    r"^\| \[([A-Z]\d{2}) ([^\]]+)\]\(FREE_ERP_CHECKLIST\.md#([^)]+)\) / "
    r"(F[01]|V|X) \| ([^\n]+)$", delivery_matrix_section, re.M,
)
card_details = {
    match.group(1): (match.group(2), match.group(3))
    for match in re.finditer(
        r"^#### ([A-Z]\d{2}) ([^\n]+)\n(.*?)(?=^#### |^### |^## |\Z)",
        checklist, re.M | re.S,
    )
}
source_priority_rows = dict(re.findall(
    r"^\| \[[ x]\] ([A-Z]\d{2}) \| ([^|]+?) \|", checklist, re.M,
))
if [row[0] for row in delivery_rows] != list(card_details) or len(delivery_rows) != 100:
    errors.append("Delivery index must cover the same 100 feature IDs once and in order")
delivery_priority_counts = {}
for feature, title, anchor, priority, tail in delivery_rows:
    expected_title, body = card_details[feature]
    expected_anchor = re.sub(r"[^\w -]", "", f"{feature} {expected_title}".lower()).strip().replace(" ", "-")
    expected_priority = feature[0] if feature[0] in {"V", "X"} else source_priority_rows[feature].strip()
    if (title, anchor, priority) != (expected_title, expected_anchor, expected_priority):
        errors.append(f"{feature}: delivery title, target heading or product priority drifted")
    delivery_priority_counts[priority] = delivery_priority_counts.get(priority, 0) + 1
    fields = [field.strip() for field in re.split(r"(?<!\\)\|", tail)]
    if len(fields) != 5 or fields[-1]:
        errors.append(f"{feature}: expected page/unit, code, failure and performance fields")
        continue
    binding, code, failure, performance = fields[:4]
    for label, actual in [("位置", code), ("失败断言", failure), ("量化门禁", performance)]:
        original_field = re.search(r"^- " + label + r"：(.*)$", body, re.M)
        if not original_field or actual != original_field.group(1).strip().replace("|", "\\|"):
            errors.append(f"{feature}: delivery {label} is not bound to the authoritative card")
    for prefix, maximum in [("ERP-IMPL", 40), ("UAT", 14)]:
        references = re.findall(prefix + r"-(\d{2}(?:/\d{2})*)", binding)
        if len(references) != 1 or any(
            not 1 <= int(number) <= maximum for reference in references for number in reference.split("/")
        ):
            errors.append(f"{feature}: invalid {prefix} delivery binding")
    page_binding = binding.split("；", 1)[0]
    if any(not 1 <= int(number) <= 28 for number in re.findall(r"\b\d{2}\b", page_binding)):
        errors.append(f"{feature}: invalid planned page reference")
if delivery_priority_counts != {"F0": 72, "F1": 15, "V": 9, "X": 4}:
    errors.append("Delivery index changed the free, paid or industry scope")

# These domain requirements catch valid-looking IDs assigned to the wrong page.
page_required_domains = {
    "06": {"企业和仓库数据范围", "共享主数据和单位"},
    "09": {"精确库存查询和汇总", "来源预占与释放", "批次序列和效期"},
    "10": {"所有库存写入口使用一个服务", "盘点和差异", "调拨与在途"},
    "11": {"批次序列和效期", "检验 放行和不良处置", "双向实际批次追溯"},
    "12": {"采购录单与修订", "采购到货与放行", "采购退货"},
    "13": {"销售录单与修订", "销售交付", "销售退货"},
}
implementation_titles = dict(re.findall(r"^### ERP-IMPL-(\d{2}) ([^\n]+)$", development_spec, re.M))
page_domain_bindings = {}
for number, row in page_rows:
    if number not in page_required_domains:
        continue
    ids = re.findall(r"ERP-IMPL-(\d{2}(?:/\d{2})*)", row)
    titles = {implementation_titles.get(part) for item in ids for part in item.split("/")}
    page_domain_bindings[number] = sorted(title for title in titles if title)
    if not page_required_domains[number].issubset(titles):
        errors.append(f"PAGE-{number}: actions are not connected to their required business domains")

iam_row_counts = {}
for prefix, expected in [("IAM", 10), ("IAM-U", 12), ("IAM-P", 6)]:
    pattern = r"^\| " + prefix + (r"-(\d{2})\b" if prefix == "IAM" else r"(\d{2})\b")
    ids = re.findall(pattern, delivery_section, re.M)
    iam_row_counts[prefix] = len(ids)
    if ids != [f"{index:02}" for index in range(1, expected + 1)]:
        errors.append(f"Expected {expected} ordered {prefix} real-account requirements")
iam_arithmetic = {
    "original_receipt_balance": Decimal("4.900000") + Decimal("0.125000") == Decimal("5.025000"),
    "later_balance_not_original": Decimal("5.025000") + Decimal("0.500000") == Decimal("5.525000"),
    "one_remaining_session": 8 - 7 == 1,
}
if not all(iam_arithmetic.values()) or not all(
    value in delivery_section for value in ["4.900000", "0.125000", "5.025000", "0.500000", "5.525000"]
):
    errors.append("Real-account original/current balance or session-boundary sample is inconsistent")
iam_current_declarations = {
    "crates/erp-api/src/auth_handlers.rs": ["LoginRequest", "LoginResponse", "login", "update_current_user_password"],
    "crates/erp-api/src/onboarding.rs": ["renew_login_client_route", "recover_login_password_route"],
    "crates/erp-repo/src/auth.rs": ["find_user_by_email", "list_permissions"],
    "crates/erp-repo/src/access_plan.rs": ["open_session", "bump_credentials_epoch_in_tx"],
    "crates/erp-repo/src/inventory_movement_commands.rs": ["InventoryMovementCommandInput", "payload_hash", "execute", "verified_snapshot"],
    "crates/erp-api/src/rbac_user_handlers.rs": ["update_user_roles"],
    "crates/erp-api/src/rbac_role_handlers.rs": ["update_role_permissions"],
    "crates/erp-domain/src/access.rs": ["FREE_LOGIN_SEAT_LIMIT", "FREE_MAX_ACTIVE_SESSIONS", "SESSION_TTL_SECONDS"],
}
for name, declarations in iam_current_declarations.items():
    for declaration in declarations:
        add_symbol(name, declaration)
for heading in [
    "## 六 正式页面的入口和实现位置", "## 七 页面状态和操作结果的具体规则",
    "## 八 可直接执行的业务验收剧本", "## 九 性能埋点和资源超限的实施规则",
    "## 十 交给 Grok 的单批开发输入",
]:
    if heading not in development_spec:
        errors.append(f"Missing detailed page, acceptance or performance requirement: {heading}")


current_feature_sources = {}
for match in re.finditer(r"^#### ([A-Z]\d{2}) [^\n]+\n(.*?)(?=^#### |^### |^## |\Z)",
                         checklist, re.M | re.S):
    feature_id, body = match.groups()
    for label in ["最小字段与操作", "量化门禁", "页面操作", "实施接线", "失败断言"]:
        if not re.search(r"^- " + label + r"：\S", body, re.M):
            errors.append(f"{feature_id}: missing detail {label}")
    continuation = re.search(r"^- 现有接续：(.*)", body, re.M)
    if not continuation:
        errors.append(f"{feature_id}: missing current continuation")
        continue
    source_part = continuation.group(1).split("现 `migrations/", 1)[0]
    rust_sources = re.findall(r"`(REPO/[^`]+?\.rs)(?:::(\w+))?`", source_part)
    declarations = []
    for path, explicit in rust_sources:
        prefix, leaf = path.split("/", 1)
        name = prefixes[prefix] + "/" + leaf
        if explicit:
            declarations.append({"file": name, "symbol": explicit})
    if "的 " in source_part and rust_sources:
        prefix, leaf = rust_sources[0][0].split("/", 1)
        name = prefixes[prefix] + "/" + leaf
        for symbol in re.findall(r"`([a-z][a-z0-9_]+)`", source_part.split("的 ", 1)[1]):
            declarations.append({"file": name, "symbol": symbol})
    if feature_id == "M09":
        declarations.append({"file": "crates/erp-repo/src/shared_contract/material_moves.rs",
                             "symbol": "MaterialMoveInput"})
    if not declarations:
        errors.append(f"{feature_id}: no concrete current declaration")
    current_feature_sources[feature_id] = declarations
    for entry in declarations:
        add_symbol(entry["file"], entry["symbol"])

handler_names = [
    "router", "save_bom", "save_bom_structure_route", "publish_bom_version_route",
    "save_routing_route", "save_operation_route_handler", "save_work_center_handler",
    "save_production_plan_handler", "save_bucket_mrp_handler", "plan_mrp_run",
    "save_execution_route", "save_work_order_route", "transition_work_order_route",
    "save_material_route", "BomStructureRequest", "BomVersionRequest",
    "OperationRouteRequest", "WorkCenterRequest", "WorkOrderSaveRequest",
    "WorkOrderActionRequest", "MaterialMoveRequest",
]
for symbol in handler_names:
    add_symbol("crates/erp-api/src/manufacturing.rs", symbol)
for symbol in ["InventoryQuantityKey", "InventoryPreciseDelta", "InventoryPreciseBalance",
               "get_precise_balance", "apply_precise_with_tx", "lock_precise_balance"]:
    add_symbol("crates/erp-repo/src/inventory_precision.rs", symbol)
add_symbol("crates/erp-repo/src/shared_contract/material_moves.rs", "post_manufacturing_reservation")
add_symbol("crates/erp-api/src/router.rs", "build_app")
add_symbol("apps/erp-ui/src/page_registry.rs", "PageSpec")
sales_handlers = [
    "router", "list_sales_orders", "create_sales_order", "get_sales_order",
    "update_sales_order", "update_sales_order_status", "create_sales_delivery",
    "create_sales_return", "read_sales_return", "list_sales_actions", "create_sales_action",
    "save_scaled_trade_sale", "SalesOrderItemRequest",
]
for symbol in sales_handlers:
    add_symbol("crates/erp-api/src/sales_orders.rs", symbol)
for symbol in [
    "admit_new_login_seat", "open_session", "active_session", "list_own_sessions",
    "revoke_own_session", "bump_credentials_epoch",
]:
    add_symbol("crates/erp-repo/src/access_plan.rs", symbol)
for symbol in ["login", "update_current_user_password", "list_own_sessions", "revoke_own_session"]:
    add_symbol("crates/erp-api/src/auth_handlers.rs", symbol)
for symbol in ["create_user", "update_user"]:
    add_symbol("crates/erp-api/src/rbac_user_handlers.rs", symbol)
add_symbol("crates/erp-api/src/auth_layers.rs", "auth_middleware")
for symbol in ["BucketMrpRequest", "MrpPlanRequest"]:
    add_symbol("crates/erp-api/src/manufacturing.rs", symbol)
add_symbol("crates/erp-domain/src/manufacturing.rs", "plan_bucket_mrp")
for symbol in ["reserve_for_order_item_with_audit", "release_reservation_with_audit",
               "begin_audited_reservation", "write_required_logs"]:
    add_symbol("crates/erp-repo/src/inventory_reservation_audit.rs", symbol)
for symbol in ["reserve_for_order_item_with_tx", "update_reservation_with_tx",
               "lock_reservation_keys", "validate_reservation_input", "InventoryReservationOutcome"]:
    add_symbol("crates/erp-repo/src/inventory.rs", symbol)
for symbol in ["legacy_reservation_source_id", "reserve_inventory", "release_inventory"]:
    add_symbol("crates/erp-api/src/inventory_print_report_handlers/stock_movement.rs", symbol)
for symbol in ["InventoryRepo", "InventoryRepoDb"]:
    add_symbol("crates/erp-api/src/repo_ports.rs", symbol)
for symbol in ["InventoryReservationRequest", "InventoryReleaseRequest"]:
    add_symbol("crates/erp-api/src/lib.rs", symbol)
add_symbol("crates/erp-api/src/inventory_views.rs", "inventory_error_response")
for symbol in ["FulfillmentCostError", "insert_shipment_costs"]:
    add_symbol("crates/erp-repo/src/fulfillment/costs.rs", symbol)
add_symbol("crates/erp-repo/src/inventory_batch_consumption.rs", "consume")
add_symbol("crates/erp-repo/src/inventory_batch_effects.rs", "InventoryBatchLimit")
add_symbol("crates/erp-repo/src/fulfillment/store.rs", "ship_oms_shipment")
add_symbol("crates/erp-repo/src/inventory.rs", "consume_batches_by_fefo")
add_symbol("crates/erp-repo/src/shared_contract/item_cost.rs", "lock_cost_command")
for symbol in ["resolve_order_reservation_items", "refuse_unverified_legacy_children",
               "FrozenReservationItems", "MAX_ORDER_RESERVATION_COMPONENTS"]:
    add_symbol("crates/erp-repo/src/inventory_order_reservations.rs", symbol)
for symbol in ["ReservationItem", "ReservationExpansion", "expand_order_item", "component_order_item_id"]:
    add_symbol("crates/erp-repo/src/inventory.rs", symbol)

batch_heading = "## 十一 批次库存逐动作实施规格"
batch_section = (development_spec.split(batch_heading, 1)[1].split(
    "## 十二 生产收货放行逐动作规格", 1)[0] if batch_heading in development_spec else "")
if not batch_section:
    errors.append("Missing detailed batch-inventory action specification")
batch_row_counts = {}
for prefix, expected in [("BATCH-DEV", 8), ("BATCH-CASE", 16), ("BATCH-PERF", 8)]:
    rows = re.findall(r"^\| (" + prefix + r"-([0-9]{2})) ([^\n]+)$", batch_section, re.M)
    if [number for _, number, _ in rows] != [f"{i:02}" for i in range(1, expected + 1)]:
        errors.append(f"Expected {expected} unique ordered {prefix} rows")
    required_columns = 4 if prefix == "BATCH-DEV" else 3
    for identity, _, cells in rows:
        columns = [cell.strip() for cell in cells.rstrip("\r\n |").split("|")]
        if len(columns) != required_columns or not all(columns):
            errors.append(f"{identity}: incomplete implementation, input or acceptance columns")
        if prefix == "BATCH-PERF":
            metric = re.search(r"ERP-P([0-9]{2})", columns[0])
            if not metric or metric.group(1) not in metric_ids:
                errors.append(f"{identity}: missing existing performance-metric binding")
    batch_row_counts[prefix] = len(rows)

batch_current_locations = set()
batch_current_declarations = []
for token in re.findall(r"现\s*`((?:API|REPO)/[^`]+)`", batch_section):
    match = re.fullmatch(r"(API|REPO)/([^:]+\.rs)(?:::(.+))?", token)
    if not match:
        errors.append(f"Invalid current batch implementation location: {token}")
        continue
    prefix, leaf, declaration = match.groups()
    name = prefixes[prefix] + "/" + leaf
    batch_current_locations.add(name)
    if not (root / name).is_file():
        errors.append(f"Missing current batch implementation file: {name}")
    if declaration:
        for symbol in declaration.strip("{}").split(","):
            symbol = symbol.strip()
            if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", symbol):
                errors.append(f"Invalid current batch implementation declaration: {token}")
                continue
            add_symbol(name, symbol)
            entry = {"file": name, "symbol": symbol}
            if entry not in batch_current_declarations:
                batch_current_declarations.append(entry)
for token in [
    "inventory_batch_movements", "inventory_ledger_id", "before_on_hand_micros",
    "after_on_hand_micros", "batch_metadata_snapshot", "实际INSERT RETURNING",
    "不按整数自行激活批次", "仓库变化为0", "旧请求摘要", "NOT_RUN",
    "同一SKU流水/批次", "逐来源子明细", "源码及产物SHA",
]:
    if token not in batch_section:
        errors.append(f"Missing batch action, provenance or acceptance requirement: {token}")

quality_source_heading = "## 十三 质量来源与冻结标准逐动作规格"
outcome_heading = "## 十四 实测读数、复检和审核逐动作规格"
quality_source_section = (development_spec.split(quality_source_heading, 1)[1].split(
    outcome_heading, 1)[0] if quality_source_heading in development_spec else "")
quality_source_rows = {}
for prefix, count in [("QSOURCE", 9), ("QSOURCE-CASE", 16), ("QSOURCE-PERF", 7)]:
    numbers = re.findall(r"^\| " + prefix + r"-([0-9]{2}) ", quality_source_section, re.M)
    quality_source_rows[prefix] = len(numbers)
    if numbers != [f"{i:02}" for i in range(1, count + 1)]:
        errors.append(f"Expected {count} ordered quality source {prefix} rows")
for name, declarations in {
    "crates/erp-repo/src/shared_contract/inspection_sources.rs": ["SourceKind", "LockedSource", "parse", "as_str",
        "require_item", "lock_source", "insert_frozen_lines_and_binding"],
    "crates/erp-api/src/manufacturing.rs": ["inspection_write_error"],
    "crates/erp-repo/src/shared_contract.rs": ["release_after_quality", "advance_sales_delivery", "save_sales_revision"],
}.items():
    for declaration in declarations:
        add_symbol(name, declaration)

gate_heading = "## 十五 消费者质量闸门与精确贸易库存"
outcome_section = (development_spec.split(outcome_heading, 1)[1].split(gate_heading, 1)[0]
                   if outcome_heading in development_spec else "")
outcome_rows = {}
for prefix, count in [("QREAD", 12), ("QREAD-CASE", 12), ("QREAD-PERF", 8)]:
    numbers = re.findall(r"^\| " + prefix + r"-([0-9]{2}) ", outcome_section, re.M)
    outcome_rows[prefix] = len(numbers)
    if numbers != [f"{i:02}" for i in range(1, count + 1)]:
        errors.append(f"Expected {count} ordered inspection outcome {prefix} rows")
for token in ["NOT_RUN", "FAILED", "MISSING", "previous_item_id", "declared_reviewer", "quality.read",
              "quality.write", "1000", "20", "3项原反例", "真实密码", "多样本", "同账号"]:
    if token not in outcome_section:
        errors.append(f"Missing inspection outcome contract or evidence boundary: {token}")
for name, declarations in {
    "crates/erp-domain/src/manufacturing/inspection_outcomes.rs": ["InspectionVerdict", "InspectionOutcome", "record_measurement"],
    "crates/erp-api/src/manufacturing/inspection_outcomes.rs": ["actor", "quality_tenant", "require_quality", "outcome_response",
        "InspectionReinspectionRequest", "InspectionReadingReviewRequest", "InspectionReadingHistoryQuery",
        "reinspect_route", "review_route", "history_route"],
    "crates/erp-repo/src/shared_contract/inspection_outcomes.rs": ["InspectionActor", "InspectionOutcomeError",
        "InspectionOutcomeWrite", "InspectionReadingInput", "InspectionReviewInput", "require_active", "lock_request",
        "save_inspection_outcome", "review_inspection_outcome", "read_inspection_history", "effective_readings", "log_effect"],
}.items():
    for declaration in declarations:
        add_symbol(name, declaration)

gate_section = development_spec.split(gate_heading, 1)[1] if gate_heading in development_spec else ""
gate_rows = {}
for prefix, count in [("QGATE", 8), ("QGATE-CASE", 14), ("QGATE-PERF", 8)]:
    numbers = re.findall(r"^\| " + prefix + r"-([0-9]{2}) ", gate_section, re.M)
    gate_rows[prefix] = len(numbers)
    if numbers != [f"{i:02}" for i in range(1, count + 1)]:
        errors.append(f"Expected {count} ordered consumer quality gate {prefix} rows")
for name, declarations in {
    "crates/erp-repo/src/shared_contract/production_receipt_release.rs": ["source_inspection_proof", "scoped_inspection_proof"],
    "crates/erp-repo/src/shared_contract/trade_stock.rs": ["StockPosting", "post"],
    "crates/erp-repo/src/shared_contract.rs": ["append_stock_ledger", "reserve_stock_line", "release_stock_line", "inspection_source_required", "inspection_blocks"],
    "crates/erp-api/tests/free_erp_acceptance/inspection_gates.rs": ["facts", "gate_http", "delivery_config"],
}.items():
    for declaration in declarations:
        add_symbol(name, declaration)
for token in ["NOT_RUN", "PARTIAL", "4MiB", "8MiB", "1001", "20路", "正常密码", "单账号多端", "UNAVAILABLE/HELD/WIP"]:
    if token not in gate_section:
        errors.append(f"Missing consumer quality gate boundary: {token}")

detail_heading = "## 十七 制造整单任务执行和多端单据开发细目"
detail_section = development_spec.split(detail_heading, 1)[1] if detail_heading in development_spec else ""
detail_rows = {}
for prefix, count in [("ATX", 8), ("TASK", 8), ("DOC", 4), ("DETAIL-U", 16), ("DETAIL-P", 8)]:
    numbers = re.findall(r"^\| " + prefix + r"-([0-9]{2}) ", detail_section, re.M)
    detail_rows[prefix] = len(numbers)
    if numbers != [f"{i:02}" for i in range(1, count + 1)]:
        errors.append(f"Expected {count} ordered implementation-detail {prefix} rows")
detail_current_declarations = {
    "crates/erp-api/src/manufacturing.rs": [
        "AtomicEffectRequest", "commit_atomic_effect_route", "VersionedDocumentOpenRequest",
        "VersionedDocumentSaveRequest", "open_versioned_document_route", "save_versioned_document_route",
        "enqueue_durable_task_route", "claim_durable_task_route", "heartbeat_durable_task_route",
        "fail_durable_task_route", "cancel_durable_task_route", "restore_dead_task_route",
        "DurableTaskPageQuery", "read_durable_task_page_route",
    ],
    "crates/erp-repo/src/shared_contract.rs": [
        "commit_atomic_effect", "enqueue_durable_task", "claim_durable_task", "heartbeat_durable_task",
        "fail_durable_task", "request_task_cancel", "apply_cancel_checkpoint", "reverse_posted_task_issue",
        "restore_dead_task", "open_versioned_document", "save_versioned_document", "admit_tenant_work",
        "read_work_admission",
        "read_durable_task_page",
    ],
    "crates/erp-repo/src/shared_contract/material_moves.rs": ["post_material_move"],
    "crates/erp-repo/src/shared_contract/atomic_effects.rs": ["AtomicEffectError", "AtomicEffectInput", "AtomicEffectWrite", "commit_atomic_effect_with_actor", "material_source", "verify_original"],
    "crates/erp-domain/src/manufacturing.rs": ["AtomicEffect", "admit_atomic_effect"],
}
for name, declarations in detail_current_declarations.items():
    for declaration in declarations:
        add_symbol(name, declaration)
for token in ["NOT_RUN", "PARTIAL", "1.000000", "0.875000/1050", "0.125000/150",
              "PAGE-19", "expected_version", "原冲突", "attempt_id", "真实进程", "正常账号",
              "65536/65537", "20路", "1s采样", "全局≤1000", "不新增收费限制"]:
    if token not in detail_section:
        errors.append(f"Missing implementation-detail requirement: {token}")
detail_arithmetic = {
    "issued_value_cent": Decimal("0.250000") * Decimal("1200") == Decimal("300"),
    "consumed_value_cent": Decimal("0.125000") * Decimal("1200") == Decimal("150"),
    "remaining_warehouse_qty": Decimal("1.000000") - Decimal("0.250000") + Decimal("0.125000") == Decimal("0.875000"),
    "remaining_warehouse_value_cent": Decimal("1200") - Decimal("300") + Decimal("150") == Decimal("1050"),
    "physical_conservation": Decimal("0.875000") + Decimal("0.125000") == Decimal("1.000000"),
    "value_conservation": Decimal("1050") + Decimal("150") == Decimal("1200"),
}
for label, valid in detail_arithmetic.items():
    if not valid:
        errors.append(f"Implementation-detail sample arithmetic failed: {label}")

source_names = sorted(set(previous["referenced_source_sha256"]) | implementation_current_locations
                      | batch_current_locations
                      | {entry["file"] for entry in symbols}
                        | {"apps/erp-ui/src/http.rs", "apps/erp-ui/src/http/web.rs",
                           "crates/erp-common/src/telemetry.rs", "crates/erp-grid-core/src/remote.rs"}
                        | {"migrations/0282_material_identity.sql", "migrations/0283_inventory_precision.sql",
                           "migrations/0288_inventory_batch_movements.sql", "migrations/0289_inventory_batch_movement_integrity.sql",
                           "migrations/0290_production_receipt_release_sources.sql",
                           "migrations/0291_inspection_source_bindings.sql",
                           "migrations/0292_inspection_command_identity.sql",
                           "migrations/0293_inspection_bounded_command_keys.sql",
                           "migrations/0294_inspection_reading_outcomes.sql",
                           "crates/erp-api/tests/free_erp_acceptance/inspection_outcomes.rs",
                           "crates/erp-api/tests/free_erp_acceptance/inspection_outcome_edges.rs",
                           "crates/erp-api/tests/free_erp_acceptance/inspection_outcome_commands.rs",
                           "crates/erp-api/tests/free_erp_acceptance/inspection_outcome_sales.rs",
                           "crates/erp-api/tests/free_erp_acceptance.rs",
                           "crates/erp-api/tests/free_erp_acceptance/production_quality_sources.rs",
                           "crates/erp-api/tests/free_erp_acceptance/production_receipt_release.rs"})
source_hashes = {}
source_names = sorted(set(source_names).union({
    "crates/erp-repo/src/shared_contract/command_readback.rs",
    "crates/erp-api/src/manufacturing/command_readback.rs",
    "crates/erp-api/tests/free_erp_acceptance/command_readback.rs",
    "migrations/0132_atomic_effects.sql", "migrations/0134_durable_tasks.sql",
    "migrations/0135_document_versions.sql", "migrations/0139_health_signals.sql",
    "migrations/0295_atomic_material_sources.sql", "crates/erp-api/tests/free_erp_acceptance/atomic_effects.rs",
    "crates/erp-worker/src/runtime.rs", "crates/erp-worker/src/jobs/mod.rs",
}))
source_texts = {}
for name in source_names:
    path = root / name
    if not path.is_file():
        errors.append(f"Missing current source file: {name}")
        continue
    raw = path.read_bytes()
    source_hashes[name] = hashlib.sha256(raw).hexdigest()
    source_texts[name] = raw.decode("utf-8-sig")
for entry in symbols:
    name = entry["symbol"]
    if not re.search(r"\b(?:fn|struct|enum|type|trait|const)\s+" + re.escape(name) + r"\b", source_texts.get(entry["file"], "")):
        errors.append(f"Missing current declaration: {entry['file']}::{name}")

auth_source = source_texts["crates/erp-api/src/auth_handlers.rs"]
password_source = auth_source.split("async fn update_current_user_password(", 1)[-1]
password_sequence = [
    password_source.find("pool().begin().await"),
    password_source.find(".update_user_in_tx("),
    password_source.find(".bump_credentials_epoch_in_tx("),
    password_source.find("tx.commit().await"),
]
inventory_auth_source = source_texts["crates/erp-repo/src/inventory_movement_commands.rs"]
onboarding_source = source_texts["crates/erp-api/src/onboarding.rs"]
iam_source_facts = {
    "login_is_actual_password_and_database_roles": all(value in auth_source for value in [
        ".verify_password(", ".list_permissions(", ".open_session(", "is_admin: false",
        "sid: Some(session.session_id.to_string())", "cep: Some(session.credentials_epoch)",
    ]),
    "password_and_epoch_commit_same_transaction": all(value >= 0 for value in password_sequence)
        and password_sequence == sorted(password_sequence),
    "renewal_and_recovery_routes_already_exist": all(value in (root / "crates/erp-api/src/router.rs").read_text(encoding="utf-8") for value in [
        '"/auth/sessions/renew", post(renew_own_session)', '"/auth/password/recover", post(recover_own_password)',
    ]) and all(value in auth_source for value in [
        "async fn renew_own_session(", "async fn recover_own_password(", ".renew_session(", ".recover_formal_password(",
    ]),
    "inventory_command_requires_session": "self.session_id.is_none_or(|id| id.is_nil())" in inventory_auth_source,
    "current_v2_business_fingerprint_excludes_session": all(value in inventory_auth_source for value in [
        "const RESULT_VERSION: u8 = 2;", "business.session_id = None;",
        'serde_json::to_vec(&("inventory-movement-v2", business,))',
    ]),
    "legacy_v1_hash_keeps_original_session_and_version": all(value in inventory_auth_source for value in [
        "const LEGACY_RESULT_VERSION: u8 = 1;", 'serde_json::to_vec(&("inventory-movement-v1", input,))',
        "snapshot_payload_hash(&input, snapshot.version)",
    ]),
    "v2_original_session_has_mandatory_log_anchor": all(value in inventory_auth_source for value in [
        'metadata["submitter_session_id"] = json!(input.session_id);',
        "movement_metadata(&input, &hash, &balance, RESULT_VERSION)",
        "snapshot.version,",
    ]),
    "stale_login_sessions_excluded_from_reuse_and_quota": all(value in source_texts["crates/erp-repo/src/access_plan.rs"] for value in [
        "revoke_reason = 'CREDENTIALS_CHANGED'", "credentials_epoch <> $3",
        "and s.credentials_epoch = $6", ".bind(stored.credentials_epoch)",
    ]),
    "inventory_reads_current_role_and_auth_session": all(value in inventory_auth_source for value in [
        "from users u", "join user_roles ur", "join role_permissions rp",
        "WMS.INVENTORY.WRITE", "from auth_sessions s", "for update of s",
    ]),
    "formal_login_passes_the_server_verified_hash_to_admission": all(value in source_texts["crates/erp-api/src/auth_handlers.rs"] for value in [
        "verify_password_off_executor(req.password.clone(), user.password_hash.clone()).await",
        "&user.password_hash,",
    ]) and all(value in source_texts["crates/erp-repo/src/access_plan.rs"] for value in [
        "verified_password_hash: &str", "select status,password_hash from users where tenant_id=$1 and user_id=$2",
        'status == "ACTIVE" && hash == verified_password_hash',
    ]),
    "formal_login_has_no_connection_held_for_a_session_level_guard":
        "open_session_once" not in source_texts["crates/erp-repo/src/access_plan.rs"]
        and "pg_advisory_lock(" not in source_texts["crates/erp-repo/src/access_plan.rs"]
        and "Exactly one pooled connection." in source_texts["crates/erp-repo/src/access_plan.rs"],
    "password_worker_and_waiting_budget_are_bounded": all(value in source_texts["crates/erp-api/src/auth_handlers.rs"] for value in [
        "struct PasswordVerificationBudget", "tokio::sync::Semaphore::new(32)",
        "tokio::sync::Semaphore::new(2)", ".try_acquire_owned()", "tokio::task::spawn_blocking(move ||",
        "let _admitted = admitted;", "let _running = running;", '"code":"login_busy"',
    ]),
    "real_credential_session_defaults": bool(re.search(
        r"FREE_LOGIN_SEAT_LIMIT: i32 = 1;", source_texts["crates/erp-domain/src/access.rs"],
    )) and bool(re.search(
        r"FREE_MAX_ACTIVE_SESSIONS: i32 = 8;", source_texts["crates/erp-domain/src/access.rs"],
    )) and bool(re.search(
        r"SESSION_TTL_SECONDS: i64 = 60 \* 60;", source_texts["crates/erp-domain/src/access.rs"],
    )),
}
for fact, found in iam_source_facts.items():
    if not found:
        errors.append(f"Refresh real-account current source finding: {fact}")

batch_static_facts = {}
batch_input_source = source_texts.get("crates/erp-repo/src/inventory_movement_commands.rs", "")
batch_input_match = re.search(r"pub struct InventoryMovementCommandInput\s*\{(.*?)^\}", batch_input_source, re.M | re.S)
batch_static_facts["original_input_has_optional_uuid_batch"] = bool(
    batch_input_match and 'skip_serializing_if = "Option::is_none"' in batch_input_match.group(1)
    and "pub batch_id: Option<Uuid>" in batch_input_match.group(1)
)
batch_static_facts["original_snapshot_saves_batch_effects"] = (
    "pub batch_effects: Vec<crate::inventory_batch_effects::InventoryBatchEffect>" in batch_input_source
    and "inventory_batch_effects::verify(" in batch_input_source
)
batch_stock_source = source_texts.get("crates/erp-api/src/inventory_print_report_handlers/stock_movement.rs", "")
batch_core_source = source_texts.get("crates/erp-api/src/inventory_print_report_handlers/inventory_core.rs", "")
batch_static_facts["three_original_handlers_retain_uuid_batch"] = (
    "does not accept batch_id yet" not in batch_stock_source + batch_core_source
    and len(re.findall(r"^\s+batch_id,\r?$", batch_stock_source, re.M)) >= 2
    and "quantity_delta: req.quantity_delta" in batch_core_source
)
batch_static_facts["legacy_adjustment_delegates_precise_batch_kernel"] = (
    "inventory_batch_effects::apply(tx, &stock, batch_id)" in source_texts.get("crates/erp-repo/src/inventory.rs", "")
    and "on_hand = inventory_batch_balances.on_hand +" not in source_texts.get("crates/erp-repo/src/inventory.rs", "")
)
batch_material_source = source_texts.get("crates/erp-repo/src/shared_contract/material_moves.rs", "")
batch_static_facts["manufacturing_uses_actual_effect_without_null_guess"] = (
    "inventory_batch_effects::apply(tx, &stock_effect, input.batch_id)" in batch_material_source
    and "None => ScaledQty::from_legacy_whole_units(row.on_hand)?" not in batch_material_source
    and "None => ScaledQty::from_legacy_whole_units(row.reserved)?" not in batch_material_source
)
batch_static_facts["fefo_uses_same_effect_kernel"] = (
    "inventory_batch_effects::apply(tx, effect, None)" in source_texts.get("crates/erp-repo/src/inventory_batch_consumption.rs", "")
)
batch_static_facts["precise_ledger_returns_actual_id"] = bool(re.search(
    r'let ledger_id: Uuid = sqlx::query_scalar\("insert into inventory_ledgers[^\n]*returning ledger_id',
    source_texts.get("crates/erp-repo/src/inventory_precision.rs", ""), re.I,
))
batch_kernel_source = source_texts.get("crates/erp-repo/src/inventory_batch_effects.rs", "")
batch_static_facts["batch_plan_has_bound_and_bulk_affected_row_checks"] = all(
    token in batch_kernel_source for token in ["limit 1001 for update", "MAX_BATCHES", "push_values", "rows_affected()", "effects.len() != changes.len()"]
)
batch_static_facts["decision_uses_current_database_clock_after_metadata_lock"] = (
    "order by s.batch_id for share" in batch_kernel_source and "select clock_timestamp()" in batch_kernel_source
    and batch_kernel_source.index("order by s.batch_id for share") < batch_kernel_source.index("select clock_timestamp()")
)
batch_static_facts["exact_effect_quantities_serialize_as_decimal_strings"] = (
    batch_kernel_source.count('#[serde(with = "decimal_integer")]') == 6
    and "serializer.serialize_str(&value.to_string())" in batch_kernel_source
)
batch_schema = source_texts.get("migrations/0288_inventory_batch_movements.sql", "")
batch_integrity_schema = source_texts.get("migrations/0289_inventory_batch_movement_integrity.sql", "")
batch_static_facts["real_owned_ledger_children_are_immutable_and_conserved"] = all(
    token in batch_schema for token in ["inventory_batch_movements_immutable", "inventory_batch_movements_sum",
        "references inventory_ledgers(tenant_id,warehouse_id,sku_id,ledger_id)", "deferrable initially deferred"]
)
batch_static_facts["metadata_identity_and_parent_source_are_guarded"] = all(
    token in batch_integrity_schema for token in ["inventory_batch_movements_metadata_identity", "inventory_ledger_batch_source_immutable"]
)
for fact, found in batch_static_facts.items():
    if not found:
        errors.append(f"Refresh the batch specification's current-source fact: {fact}")
for token in ["mfg_material_identity_insert", "validate_new_material_identity", "original_issue_id"]:
    if token not in source_texts["migrations/0282_material_identity.sql"]:
        errors.append(f"Missing material migration contract: {token}")
for token in ["inventory_exact_balance_guard", "inventory_exact_ledger_guard",
              "inventory_ledger_delete_guard",
              "inventory_exact_batch_guard", "inventory_exact_reservation_guard",
              "inventory_quantity_activations", "inventory_quantity_source_maps"]:
    if token not in source_texts.get("migrations/0283_inventory_precision.sql", ""):
        errors.append(f"Missing precision migration contract: {token}")

route_section = checklist.split("### 当前制造写入口的准确位置", 1)[1].split(
    "### 正式查询的拟交付接口", 1)[0]
query_section = checklist.split("### 正式查询的拟交付接口", 1)[1].split(
    "### 最小错误响应和客户端动作", 1)[0]
document_routes = re.findall(r"^\| M[^|]+\| `([^`]+)`", route_section, re.M)
proposed_query_groups = len(re.findall(r"^\| M[^|]+ GET ", query_section, re.M))
manufacturing_router = source_texts.get("crates/erp-api/src/manufacturing.rs", "").split(
    "#[derive", 1)[0]
registered_routes = []
registered_manufacturing_operations = []
registered_quality_operations = []
registered_finance_operations = []
registered_export_operations = []
registered_task_report_operations = []
for route in re.finditer(r'\.route\(\s*"([^"]+)"\s*,(.*?)\)\s*,?\s*\)',
                        manufacturing_router, re.S):
    # axum MethodRouter supports both post(handler).get(handler) and
    # get(handler).post(handler); the POST contract does not depend on order.
    full_path = re.sub(r":([A-Za-z_]+)", r"{\1}", route.group(1))
    if "/quality/" in full_path:
        for method, handler in re.findall(r"\b(get|post)\(([A-Za-z_][A-Za-z0-9_:]*)", route.group(2)):
            registered_quality_operations.append((method.upper(), full_path, handler))
        continue
    if "/finance/" in full_path:
        for method, handler in re.findall(r"\b(get|post)\(([A-Za-z_][A-Za-z0-9_:]*)", route.group(2)):
            registered_finance_operations.append((method.upper(), full_path, handler))
        continue
    if "/exports/" in full_path:
        for method, handler in re.findall(r"\b(get|post)\(([A-Za-z_][A-Za-z0-9_:]*)", route.group(2)):
            registered_export_operations.append((method.upper(), full_path, handler))
        continue
    if full_path in {"/v1/tenants/{tenant_id}/tasks", "/v1/tenants/{tenant_id}/work/admissions", "/v1/tenants/{tenant_id}/documents"} or "/tasks/" in full_path or "/reports/" in full_path or "/commands" in full_path or "/documents/" in full_path or "/access/" in full_path or "/health/" in full_path:
        for method, handler in re.findall(r"\b(get|post)\(([A-Za-z_][A-Za-z0-9_:]*)", route.group(2)):
            registered_task_report_operations.append((method.upper(), full_path, handler))
        continue
    if "/manufacturing" not in full_path:
        errors.append(f"Unclassified route in manufacturing router: {full_path}")
        continue
    if re.search(r'\bpost\(\w+', route.group(2)):
        registered_routes.append(route.group(1))
    path = full_path.split("/manufacturing", 1)[1]
    for method, handler in re.findall(r"\b(get|post)\(([A-Za-z_][A-Za-z0-9_:]*)", route.group(2)):
        registered_manufacturing_operations.append((method.upper(), path, handler))
normalized_routes = [re.sub(r":([A-Za-z_]+)", r"{\1}", path).split("/manufacturing", 1)[1]
                     for path in registered_routes]
document_manufacturing_posts = [
    ("POST", path, handler) for path, handler in re.findall(
        r"^\| M[^|]+\| `([^`]+)` \| `(\w+)`", route_section, re.M,
    )
]
document_manufacturing_gets = [
    ("GET", path, handler) for path, handler in re.findall(
        r"^\| GET `([^`]+)` \| `(\w+)`\s*\|", route_section, re.M,
    )
]
if (len(document_routes) != 37 or sorted(document_routes) != sorted(normalized_routes)
    or sorted(document_manufacturing_posts)
       != sorted(item for item in registered_manufacturing_operations if item[0] == "POST")):
    errors.append("Documented manufacturing POST routes differ from current registration")
if (len(document_manufacturing_gets) != 11
    or sorted(document_manufacturing_gets)
       != sorted(item for item in registered_manufacturing_operations if item[0] == "GET")):
    errors.append("Documented manufacturing GET routes differ from current registration")
manufacturing_get_query_fields = {}
manufacturing_source = source_texts.get("crates/erp-api/src/manufacturing.rs", "")
manufacturing_source += "\n" + source_texts.get("crates/erp-api/src/manufacturing/atomic_effects.rs", "")
task_report_rows = re.findall(
    r"^\| (GET|POST) `([^`]+)` \| `(\w+)` / `(\w+)` \| ([^|]+) \|", gate_section, re.M)
document_task_report_operations = [(method,path,handler) for method,path,handler,_,_ in task_report_rows]
if len(task_report_rows) != len(registered_task_report_operations) or sorted(document_task_report_operations) != sorted(registered_task_report_operations):
    errors.append("Task/report route source inventory differs from current registration")
task_report_fields = {}
for method, path, handler, request_type, field_cell in task_report_rows:
    signature = re.search(r"async fn " + re.escape(handler) + r"\s*\((.*?)\)\s*->",manufacturing_source,re.S)
    fields = []
    documented = sorted({field for value in re.findall(r"`([^`]+)`",field_cell) for field in value.split("/")})
    if request_type == "PathOnly":
        valid = (method == "GET" and signature is not None and "Query<" not in signature.group(1)
                 and "Json<" not in signature.group(1) and documented == [])
    else:
        body = re.search(r"struct " + re.escape(request_type) + r"\s*\{(.*?)^\}",manufacturing_source,re.M | re.S)
        fields = sorted(re.findall(r"^\s*(\w+)\s*:\s*",body.group(1),re.M)) if body else []
        extractor = "Query" if method == "GET" else "Json"
        valid = (signature is not None and f"{extractor}<{request_type}>" in signature.group(1) and fields == documented)
    if not valid:
        errors.append(f"Task/report fields differ from current declaration: {handler}")
    task_report_fields[f"{method} {path}"] = {"handler":handler,"type":request_type,"fields":fields}
for path, handler, field_cell in re.findall(
    r"^\| GET `([^`]+)` \| `(\w+)` \| ([^|]+) \|", route_section, re.M,
):
    signature = re.search(
        r"async fn " + re.escape(handler) + r"\s*\((.*?)\)\s*->", manufacturing_source, re.S,
    )
    query_type = re.search(r"Query<(\w+)>", signature.group(1)) if signature else None
    query_body = re.search(
        r"struct " + re.escape(query_type.group(1)) + r"\s*\{(.*?)^\}",
        manufacturing_source, re.M | re.S,
    ) if query_type else None
    current_fields = sorted(re.findall(r"^\s*(\w+)\s*:\s*", query_body.group(1), re.M)) if query_body else []
    document_fields = sorted({field for value in re.findall(r"`([^`]+)`", field_cell)
                              for field in value.split("/")})
    if not current_fields or document_fields != current_fields:
        errors.append(f"Documented manufacturing GET parameters differ from current declaration: {path}")
    manufacturing_get_query_fields[path] = {"handler": handler,
                                          "query_type": query_type.group(1) if query_type else None,
                                          "fields": current_fields}
if proposed_query_groups != 14:
    errors.append(f"Expected 14 proposed query groups, found {proposed_query_groups}")

manufacturing_new_request_fields = {}
new_request_section = checklist.split("#### 新增制造写入的真实请求字段", 1)[1].split(
    "#### 已注册的制造读取和真实参数", 1)[0]
for handler, request_type, field_cell in re.findall(
    r"^\| `(\w+)` / `(\w+)` \| ([^|]+) \|", new_request_section, re.M,
):
    signature = re.search(r"async fn " + re.escape(handler) + r"\s*\((.*?)\)\s*->",
                          manufacturing_source, re.S)
    body = re.search(r"struct " + re.escape(request_type) + r"\s*\{(.*?)^\}",
                     manufacturing_source, re.M | re.S)
    fields = sorted(re.findall(r"^\s*(\w+)\s*:\s*", body.group(1), re.M)) if body else []
    documented = sorted({field for value in re.findall(r"`([^`]+)`", field_cell) for field in value.split("/")})
    if not signature or f"Json<{request_type}>" not in signature.group(1) or fields != documented:
        errors.append(f"New manufacturing POST fields differ from current declaration: {handler}")
    manufacturing_new_request_fields[handler] = {"request_type": request_type, "fields": fields}
if len(manufacturing_new_request_fields) != 23:
    errors.append("Expected twenty-three explicitly documented new manufacturing request shapes")

quality_section = checklist.split("#### 同模块质量写入口的实际字段", 1)[1].split(
    "#### 同模块财务入口的实际字段", 1)[0]
quality_rows = re.findall(
    r"^\| (GET|POST) `([^`]+)` \| `([A-Za-z_][A-Za-z0-9_:]*)` / `(\w+)` \| ([^|]+) \|", quality_section, re.M)
document_quality_operations = [(method, path, handler) for method, path, handler, _, _ in quality_rows]
if len(quality_rows) != 19 or sorted(document_quality_operations) != sorted(registered_quality_operations):
    errors.append("Documented quality HTTP operations differ from current manufacturing-module registration")
quality_request_fields = {}
for method, path, handler, request_type, field_cell in quality_rows:
    handler_file = "crates/erp-api/src/manufacturing.rs"
    handler_name = handler
    if "::" in handler:
        module, handler_name = handler.rsplit("::", 1)
        if module != "inspection_outcomes":
            errors.append(f"Unverified quality handler module: {handler}")
        handler_file = "crates/erp-api/src/manufacturing/inspection_outcomes.rs"
    handler_source = source_texts.get(handler_file, "")
    signature = re.search(r"async fn " + re.escape(handler_name) + r"\s*\((.*?)\)\s*->", handler_source, re.S)
    body = re.search(r"struct " + re.escape(request_type) + r"\s*\{(.*?)^\}", handler_source, re.M | re.S)
    fields = sorted(re.findall(r"^\s*(\w+)\s*:\s*", body.group(1), re.M)) if body else []
    documented = sorted({field for value in re.findall(r"`([^`]+)`", field_cell) for field in value.split("/")})
    extractor = "Query" if method == "GET" else "Json"
    if not signature or f"{extractor}<{request_type}>" not in signature.group(1) or fields != documented:
        errors.append(f"Quality {method} fields differ from current declaration: {handler}")
    quality_request_fields[f"{method} {path}"] = {"handler": handler, "request_type": request_type,
                                                "extractor": extractor, "fields": fields}
    add_symbol(handler_file, handler_name)
    add_symbol(handler_file, request_type)

finance_section = checklist.split("#### 同模块财务入口的实际字段", 1)[1].split(
    "#### 同模块导出入口的实际字段", 1)[0]
finance_rows = re.findall(
    r"^\| (GET|POST) `([^`]+)` \| `(\w+)` / `(\w+)` \| ([^|]+) \|", finance_section, re.M)
document_finance_operations = [(method, path, handler) for method, path, handler, _, _ in finance_rows]
if len(finance_rows) != 20 or sorted(document_finance_operations) != sorted(registered_finance_operations):
    errors.append("Documented finance HTTP operations differ from current manufacturing-module registration")
finance_request_fields = {}
for method, path, handler, request_type, field_cell in finance_rows:
    signature = re.search(r"async fn " + re.escape(handler) + r"\s*\((.*?)\)\s*->", manufacturing_source, re.S)
    body = re.search(r"struct " + re.escape(request_type) + r"\s*\{(.*?)^\}", manufacturing_source, re.M | re.S)
    fields = sorted(re.findall(r"^\s*(\w+)\s*:\s*", body.group(1), re.M)) if body else []
    documented = sorted({field for value in re.findall(r"`([^`]+)`", field_cell) for field in value.split("/")})
    extractor = "Query" if method == "GET" else "Json"
    if not signature or f"{extractor}<{request_type}>" not in signature.group(1) or fields != documented:
        errors.append(f"Finance {method} fields differ from current declaration: {handler}")
    finance_request_fields[f"{method} {path}"] = {"handler": handler, "request_type": request_type,
                                                "extractor": extractor, "fields": fields}
    add_symbol("crates/erp-api/src/manufacturing.rs", handler)
    add_symbol("crates/erp-api/src/manufacturing.rs", request_type)

export_section = checklist.split("#### 同模块导出入口的实际字段", 1)[1].split(
    "### 正式查询的拟交付接口", 1)[0]
export_rows = re.findall(r"^\| (GET|POST) `([^`]+)` \| `(\w+)` / `(\w+)` \| ([^|]+) \|", export_section, re.M)
if len(export_rows) != 4 or sorted((method, path, handler) for method,path,handler,_,_ in export_rows) != sorted(registered_export_operations):
    errors.append("Documented export HTTP operations differ from current manufacturing-module registration")
export_request_fields = {}
for method, path, handler, request_type, field_cell in export_rows:
    signature = re.search(r"async fn " + re.escape(handler) + r"\s*\((.*?)\)\s*->", manufacturing_source, re.S)
    body = re.search(r"struct " + re.escape(request_type) + r"\s*\{(.*?)^\}", manufacturing_source, re.M | re.S)
    fields = sorted(re.findall(r"^\s*(\w+)\s*:\s*", body.group(1), re.M)) if body else []
    documented = sorted({field for value in re.findall(r"`([^`]+)`", field_cell) for field in value.split("/")})
    if not signature or f"Json<{request_type}>" not in signature.group(1) or fields != documented:
        errors.append(f"Export {method} fields differ from current declaration: {handler}")
    export_request_fields[f"{method} {path}"] = {"handler":handler, "request_type":request_type, "fields":fields}
    add_symbol("crates/erp-api/src/manufacturing.rs", handler)
    add_symbol("crates/erp-api/src/manufacturing.rs", request_type)

receipt_section = development_spec.split("## 十二 生产收货放行逐动作规格", 1)[1].split(
    "## 十三 质量来源与冻结标准逐动作规格", 1)[0]
receipt_row_counts = {}
for prefix, count in [("RECEIPT", 8), ("RECEIPT-PERF", 7)]:
    numbers = re.findall(r"^\| " + prefix + r"-([0-9]{2}) ", receipt_section, re.M)
    receipt_row_counts[prefix] = len(numbers)
    if numbers != [f"{i:02}" for i in range(1, count + 1)]:
        errors.append(f"Expected {count} ordered production receipt {prefix} tasks")
for token in ["NOT_RUN", "source_proof", "inspection_proof", "1001", "部分放行", "实际效果外键",
              "原快照", "末工序", "生产放行", "源码清单"]:
    if token not in receipt_section:
        errors.append(f"Missing production receipt boundary: {token}")

receipt_schema = source_texts.get("migrations/0290_production_receipt_release_sources.sql", "")
receipt_kernel = source_texts.get("crates/erp-repo/src/shared_contract/production_receipt_release.rs", "")
receipt_static_facts = {
    "new_original_results_and_effects_have_owned_foreign_keys": all(token in receipt_schema for token in [
        "create table mfg_receipt_release_commands", "create table mfg_receipt_release_effects",
        "references inventory_ledgers(tenant_id,warehouse_id,sku_id,ledger_id)",
        "references mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,ledger_id)",
        "references shared_stock_ledgers(tenant_id,warehouse_id,sku_id,ledger_id)"]),
    "immutable_original_results_sources_and_effects": all(token in receipt_schema for token in [
        "mfg_receipt_release_commands_immutable", "mfg_receipt_release_effects_immutable",
        "inventory_ledger_mfg_release_immutable", "command_receipt_mfg_release_immutable"]),
    "commit_guard_requires_two_conserved_effects": all(token in receipt_schema for token in [
        "actual_count<>2", "actual_delta is distinct from 0::numeric",
        "actual_frozen is distinct from -expected::numeric", "deferrable initially deferred"]),
    "receipt_replay_uses_original_total_and_current_access": all(token in receipt_kernel for token in [
        "available_total_micros from mfg_receipt_release_commands", "actor.require_warehouse",
        "SharedWrite::Replay(existing), available"]),
    "source_and_inspection_proofs_have_real_owned_inputs": all(token in receipt_kernel for token in [
        "source_proof(&mut tx", "inspection_proof(&mut tx", "lock_quality_source(&mut tx",
        "limit 1001 for share", "source.qty_micros.to_string()"]),
    "trusted_actor_is_derived_from_request_context": (
        "ctx.production_release_actor(&request_ctx)" in manufacturing_source
        and "fn production_release_actor" in source_texts.get("crates/erp-api/src/auth_context.rs", "")),
}
for fact, found in receipt_static_facts.items():
    if not found:
        errors.append(f"Refresh the production receipt specification's current-source fact: {fact}")

quality_source_code = source_texts.get("crates/erp-repo/src/shared_contract/inspection_sources.rs", "")
quality_source_schema = source_texts.get("migrations/0291_inspection_source_bindings.sql", "")
quality_command_schema = source_texts.get("migrations/0292_inspection_command_identity.sql", "")
quality_source_tests = source_texts.get("crates/erp-api/tests/free_erp_acceptance/production_quality_sources.rs", "")
quality_source_static_facts = {
    "typed_owned_sources_are_explicit": all(token in quality_source_code for token in [
        "WORK_ORDER", "PRODUCTION_RECEIPT", "RECEIPT", "SALES_ORDER", "value.len() > 32", "source.is_nil()"]),
    "copy_is_bounded_and_bulk_stored": all(token in quality_source_code for token in [
        "MAX_LINES: usize = 1000", "QueryBuilder", "push_values", "rows_affected()", "shared_inspection_source_bindings"]),
    "new_source_copy_and_reading_are_immutable": all(token in quality_source_schema for token in [
        "inspection_binding_immutable", "inspection_bound_header_immutable", "inspection_bound_line_immutable",
        "inspection_bound_reading_immutable", "inspection_line_parent_lock", "inspection_binding_complete"]),
    "original_text_is_verified_without_a_payload_btree": (
        "references command_receipts(tenant_id,request_id)" in quality_source_schema
        and "new.payload_hash" in quality_command_schema and "c.status='SUCCEEDED'" in quality_command_schema
        and "create unique index inspection_original_command_key" not in quality_source_schema
        and "create unique index inspection_command_identity" not in quality_command_schema),
    "review_stores_original_proof_and_allows_actual_sales_owner": (
        "create table mfg_quality_release_proofs" in quality_source_schema
        and "verify_inspection_review_parent" in quality_command_schema
        and "alter column work_order_id drop not null" in quality_command_schema),
    "receipt_proof_reads_typed_receipt_and_legacy_headers": all(token in receipt_kernel for token in [
        "scope_ids", "PRODUCTION_RECEIPT", "template_valid", "legacy_templates", "source_bindings"]),
    "database_write_failures_are_not_validation_422": (
        "fn inspection_write_error" in manufacturing_source
        and "downcast_ref::<sqlx::Error>()" in manufacturing_source),
    "actual_source_regressions_are_registered": (
        len(re.findall(r"^async fn production_quality_source_", quality_source_tests, re.M)) == 28
        and "mod production_quality_sources;" in (root / "crates/erp-api/tests/free_erp_acceptance.rs").read_text(encoding="utf8")),
}
for fact, found in quality_source_static_facts.items():
    if not found:
        errors.append(f"Refresh quality source specification current fact: {fact}")

outcome_schema = source_texts.get("migrations/0294_inspection_reading_outcomes.sql", "")
outcome_kernel = source_texts.get("crates/erp-repo/src/shared_contract/inspection_outcomes.rs", "")
outcome_api = source_texts.get("crates/erp-api/src/manufacturing/inspection_outcomes.rs", "")
outcome_test_files = ["inspection_outcomes", "inspection_outcome_edges", "inspection_outcome_commands", "inspection_outcome_sales"]
outcome_test_source = "\n".join(source_texts.get(
    f"crates/erp-api/tests/free_erp_acceptance/{name}.rs", "") for name in outcome_test_files)
outcome_static_facts = {
    "owns_actual_user_reading_freeze_source_and_original_command": all(token in outcome_schema for token in [
        "references users(tenant_id,user_id)", "references mfg_quality_items(tenant_id,item_id)",
        "references shared_inspection_source_bindings(tenant_id,freeze_id)",
        "references command_receipts(tenant_id,request_id)", "deferrable initially deferred"]),
    "immutable_events_reviews_commands_and_measured_snapshot": all(token in outcome_schema for token in [
        "inspection_outcome_immutable", "inspection_reading_review_immutable", "inspection_outcome_command_immutable",
        "new.snapshot @> jsonb_build_object", "verify_inspection_outcome", "verify_inspection_reading_review"]),
    "bounded_twenty_original_attempts": "attempt_no between 1 and 20" in outcome_schema,
    "separate_quality_permission_without_manufacturing_write_for_history": (
        'require_quality(&ctx, "quality.read")' in outcome_api
        and "match quality_tenant(&tenant_id, &ctx)" in outcome_api
        and "match require_tenant(&tenant_id, &ctx)" not in outcome_api),
    "physical_release_keeps_original_and_effective_history": all(token in receipt_kernel for token in [
        "effective_readings", "reading_events", "reading_reviews", "review_allows_release"]),
    "actual_http_samples_registered": (
        len(re.findall(r"^async fn inspection_outcome_", outcome_test_source, re.M)) == 27
        and all(f"mod {name};" in source_texts.get("crates/erp-api/tests/free_erp_acceptance.rs", "")
                for name in outcome_test_files)),
    "real_blocking_sql_commit_and_fault_samples_exist": all(token in outcome_test_source for token in [
        "pg_blocking_pids", "Barrier::new(21)", "inspection_outcome_actual_fault", "inspection_outcome_command_guard",
        "inspection_outcome_sql_guard", "tx.commit()"]),
}
for fact, found in outcome_static_facts.items():
    if not found:
        errors.append(f"Refresh inspection outcome specification current fact: {fact}")

gate_kernel = source_texts.get("crates/erp-repo/src/shared_contract/production_receipt_release.rs", "")
gate_stock = source_texts.get("crates/erp-repo/src/shared_contract/trade_stock.rs", "")
gate_tests = source_texts.get("crates/erp-api/tests/free_erp_acceptance/inspection_gates.rs", "")
gate_static_facts = {
    "source_locked_before_frozen_header_query": ("let owner = lock_source" in gate_kernel
        and gate_kernel.index("let owner = lock_source") < gate_kernel.index("select jsonb_build_object('freeze'")),
    "complete_legacy_template_and_current_effective_readings": all(token in gate_kernel for token in [
        "template_valid", "template_lines", "actual_copy != json!(expected)", "effective_readings"]),
    "bound_physical_inventory_posting_keeps_original_guard": all(token in gate_stock for token in [
        "apply_precise_with_tx", "shared_stock_ledger", "insert into shared_stock_ledgers", "change.frozen = delta"]),
    "bounded_four_and_eight_mib_source_bindings": all(token in gate_kernel for token in [
        "binding_bytes > 8 * 1024 * 1024", "4194304", "limit 1001"]),
    "fourteen_registered_actual_consumer_test_cases": (len(re.findall(r"^async fn inspection_gate_",gate_tests,re.M)) == 14
        and "mod inspection_gates;" in source_texts.get("crates/erp-api/tests/free_erp_acceptance.rs", "")),
    "actual_pg_source_lock_race_and_precise_zero_available_release_samples": all(token in gate_tests for token in [
        "blocker_count", "sales-revision:{sale}", "Barrier::new(21)", "blocker.commit()", "QA_AVAILABLE_HOLD", "release_stock_line"]),
}
for fact, found in gate_static_facts.items():
    if not found:
        errors.append(f"Refresh consumer quality gate current fact: {fact}")

command_spec = development_spec.split("## 十六 命令原结果和丢失响应逐动作规格", 1)[-1]
command_spec_counts = {}
for prefix, expected in [("CMD-R",8),("CMD-U",15),("CMD-P",8)]:
    ids = re.findall(r"^\| " + prefix + r"([0-9]{2})\b", command_spec, re.M)
    command_spec_counts[prefix] = len(ids)
    if ids != [f"{index:02}" for index in range(1,expected+1)]:
        errors.append(f"Command readback specification requires {expected} ordered {prefix} rows")
command_source = source_texts["crates/erp-repo/src/shared_contract/command_readback.rs"]
command_source_facts = {
    "read_only_consistent_snapshot": "set transaction isolation level repeatable read, read only" in command_source,
    "receipt_without_generic_metadata": "select request_id,payload_hash,status,effect_id from command_receipts" in command_source,
    "client_transition_refused": "reported != stored.status" in command_source,
    "current_owned_session": all(value in command_source for value in ["current.expires_at>now()", "current.credential_epoch=coalesce(e.epoch,0)", "u.status='ACTIVE'"]),
    "legacy_terminal_requires_reconciliation": "legacy generic completion has no authoritative business result" in command_source,
    "actual_material_object": "select work_order_id::text from mfg_material_moves where tenant_id=$1 and move_id=$2" in command_source,
    "no_business_write_statement": not re.search(r"(?i)(insert\s+into|update\s+command|delete\s+from)",command_source),
}
for fact, passed in command_source_facts.items():
    if not passed:
        errors.append(f"Refresh command readback current fact: {fact}")
declared_command_tests = re.findall(r"async fn (command_readback_[a-z0-9_]+)\s*\(",source_texts["crates/erp-api/tests/free_erp_acceptance/command_readback.rs"])
if len(declared_command_tests) != 15:
    errors.append("Expected fifteen declared command readback test cases; declaration is not runtime acceptance")

sales_route_section = checklist.split("### 贸易正式路由及需要补齐的动作", 1)[1].split(
    "**当前源码需要重点接续的差距：**", 1)[0]
document_sales_operations = re.findall(
    r"^\| [^|]+\| (GET|POST|PUT|PATCH) `([^`]+)` \| `(\w+)`",
    sales_route_section, re.M,
)
registered_sales_operations = []
sales_router = source_texts.get("crates/erp-api/src/sales_orders.rs", "").split(
    "#[derive", 1)[0]
for match in re.finditer(r'\.route\(\s*"([^"]+)"\s*,(.*?)\)\s*,?\s*\)', sales_router, re.S):
    path, methods = match.groups()
    path = re.sub(r":([A-Za-z_]+)", r"{\1}", path)
    for method, handler in re.findall(r"\b(get|post|put|patch)\((\w+)", methods):
        registered_sales_operations.append((method.upper(), path, handler))
if len(document_sales_operations) != 11 or sorted(document_sales_operations) != sorted(registered_sales_operations):
    errors.append("Documented sales HTTP operations differ from current registration")
if len({path for _, path, _ in registered_sales_operations}) != 8:
    errors.append("Expected eight current sales route paths")
for heading in [
    "### 贸易正式路由及需要补齐的动作",
    "### 单账号多端的逐函数任务",
    "### MRP从试算接到正式生产计划",
    "### 每个拆单任务的完成记录",
]:
    if heading not in checklist:
        errors.append(f"Missing concrete implementation breakdown: {heading}")

migration_names = {}
for path in sorted((root / "migrations").glob("*.sql")):
    match = re.match(r"(\d+)_", path.name)
    if match:
        migration_names.setdefault(match.group(1), []).append(path.name)
duplicates = {key: names for key, names in migration_names.items() if len(names) > 1}
if set(duplicates) != {"0060", "0073", "0074", "0079", "0095", "0166", "0167"}:
    errors.append("Migration-prefix inventory changed; refresh the document's source finding")
for name, digest in source_hashes.items():
    if hashlib.sha256((root / name).read_bytes()).hexdigest() != digest:
        errors.append(f"Source changed during document verification: {name}")
for name, digest in document_hashes.items():
    if hashlib.sha256((root / name).read_bytes()).hexdigest() != digest:
        errors.append(f"Document changed during verification: {name}")

historical_manifest = json.loads(
    (base / "free-erp-precision-fifth-source-after.json").read_text(encoding="utf-8-sig")
)
historical_input_drift = [
    name for name, digest in source_hashes.items()
    if name in historical_manifest["files"] and historical_manifest["files"][name] != digest
]

report = {
    "updated_at": datetime.now(timezone.utc).isoformat(),
    "status": "FAIL" if errors else "PASS",
    "scope": f"{len(texts)} documents: UTF-8, local and heading links, tables, 100 detailed cards, metric/task/case structure, JSON examples, all feature continuations, actual manufacturing/quality and sales routes, 40 implementation units with current locations/declarations, 28 planned page hosts/scopes, 14 business acceptance scripts, 22 performance execution tasks, precision migration, 8 batch tasks/16 samples/8 performance groups, 8 production receipt tasks/7 performance groups, 9 quality source tasks/16 samples/7 performance groups, 12 inspection outcome tasks/12 case groups/8 performance groups and 8 consumer gate tasks/14 samples/8 performance groups with current-source facts, plus {len(task_report_rows)} task/report/command/document/access operations and DTOs, eight command-read tasks/fifteen samples/eight performance groups, and twenty further development tasks/sixteen samples/eight performance groups. No business runtime certification.",
    "checks": {
        "utf8_files": len(texts), "feature_cards": len(cards), "unique_feature_cards": len(set(cards)),
        "field_lists": field_lists, "performance_gates": performance_gates,
        "page_operation_lists": page_operations, "integration_instructions": integration_instructions,
        "failure_assertions": failure_assertions,
        "metrics": len(metric_ids), "task_packages": len(task_ids), "acceptance_cases": len(case_ids),
        "unique_acceptance_cases": len(set(case_ids)), "inventory_entry_rows": inventory_count,
        "local_links": local_links, "internal_heading_links": internal_heading_links,
        "tables": table_count, "json_examples": len(json_examples),
        "current_source_files": len(source_names), "current_source_symbols": len(symbols),
        "referenced_rust_locations": len(referenced_rust_locations),
        "existing_rust_locations": sum(referenced_rust_locations.values()),
        "planned_or_uncreated_rust_locations": sum(not found for found in referenced_rust_locations.values()),
        "features_with_current_source_declarations": len(current_feature_sources),
        "documented_and_registered_manufacturing_post_routes": len(document_routes),
        "documented_and_registered_manufacturing_get_routes": len(document_manufacturing_gets),
        "manufacturing_get_query_shapes_verified": len(manufacturing_get_query_fields),
        "proposed_manufacturing_query_groups": proposed_query_groups,
        "documented_and_registered_sales_http_operations": len(document_sales_operations),
        "registered_sales_route_paths": len({path for _, path, _ in registered_sales_operations}),
        "concrete_implementation_breakdowns": 3,
        "concrete_implementation_units": len(implementation_units),
        "implementation_detail_fields": len(implementation_units) * 5,
        "implementation_current_files": len(implementation_current_locations),
        "implementation_explicit_current_declarations": len(implementation_current_declarations),
        "performance_execution_tasks": len(implementation_metrics),
        "implementation_json_examples": len(development_json_examples),
        "planned_example_arithmetic_checks": example_arithmetic_checks,
        "fixed_report_workloads": fixed_reports,
        "planned_page_hosts": len(page_rows), "unique_planned_page_scopes": len(set(page_scopes)),
        "business_acceptance_scripts": len(uat_ids), "free_feature_priority_counts": priority_counts,
        "per_feature_delivery_rows": len(delivery_rows),
        "delivery_priority_counts": delivery_priority_counts,
        "pages_with_domain_bindings_checked": len(page_domain_bindings),
        "real_account_implementation_tasks": iam_row_counts["IAM"],
        "real_account_acceptance_samples": iam_row_counts["IAM-U"],
        "real_account_performance_groups": iam_row_counts["IAM-P"],
        "real_account_current_source_facts": sum(iam_source_facts.values()),
        "real_account_sample_arithmetic_checks": sum(iam_arithmetic.values()),
        "batch_implementation_tasks": batch_row_counts["BATCH-DEV"],
        "batch_acceptance_samples": batch_row_counts["BATCH-CASE"],
        "batch_performance_groups": batch_row_counts["BATCH-PERF"],
        "batch_current_source_files": len(batch_current_locations),
        "batch_current_source_declarations": len(batch_current_declarations),
        "batch_current_source_facts_verified": sum(batch_static_facts.values()),
        "manufacturing_new_post_request_shapes_verified": len(manufacturing_new_request_fields),
        "documented_and_registered_quality_post_routes": sum(method == "POST" for method, *_ in quality_rows),
        "documented_and_registered_quality_get_routes": sum(method == "GET" for method, *_ in quality_rows),
        "quality_http_request_shapes_verified": len(quality_request_fields),
        "documented_and_registered_finance_post_routes": sum(method == "POST" for method, *_ in finance_rows),
        "documented_and_registered_finance_get_routes": sum(method == "GET" for method, *_ in finance_rows),
        "finance_http_request_shapes_verified": len(finance_request_fields),
        "documented_and_registered_export_http_operations": len(export_rows),
        "export_http_request_shapes_verified": len(export_request_fields),
        "production_receipt_implementation_tasks": receipt_row_counts["RECEIPT"],
        "production_receipt_performance_groups": receipt_row_counts["RECEIPT-PERF"],
        "production_receipt_current_source_facts_verified": sum(receipt_static_facts.values()),
        "quality_source_implementation_tasks": quality_source_rows["QSOURCE"],
        "quality_source_acceptance_samples": quality_source_rows["QSOURCE-CASE"],
        "quality_source_performance_groups": quality_source_rows["QSOURCE-PERF"],
        "quality_source_current_facts_verified": sum(quality_source_static_facts.values()),
        "inspection_outcome_implementation_tasks": outcome_rows["QREAD"],
        "inspection_outcome_acceptance_case_groups": outcome_rows["QREAD-CASE"],
        "inspection_outcome_performance_groups": outcome_rows["QREAD-PERF"],
        "consumer_quality_gate_implementation_tasks": gate_rows["QGATE"],
        "consumer_quality_gate_acceptance_samples": gate_rows["QGATE-CASE"],
        "consumer_quality_gate_performance_groups": gate_rows["QGATE-PERF"],
        "consumer_quality_gate_current_facts_verified": len(gate_static_facts),
        "documented_task_report_http_operations": len(task_report_rows),
        "task_report_request_shapes_verified": len(task_report_fields),
        "detail_manufacturing_tasks": detail_rows["ATX"],
        "detail_durable_task_tasks": detail_rows["TASK"],
        "detail_document_version_tasks": detail_rows["DOC"],
        "detail_acceptance_case_groups": detail_rows["DETAIL-U"],
        "detail_performance_groups": detail_rows["DETAIL-P"],
        "detail_current_declarations": sum(map(len, detail_current_declarations.values())),
        "detail_arithmetic_checks": sum(detail_arithmetic.values()),
        "command_readback_implementation_tasks": command_spec_counts["CMD-R"],
        "command_readback_acceptance_samples": command_spec_counts["CMD-U"],
        "command_readback_performance_groups": command_spec_counts["CMD-P"],
        "command_readback_current_source_facts_verified": sum(command_source_facts.values()),
        "inspection_outcome_current_facts_verified": sum(outcome_static_facts.values()),
        "documented_duplicate_migration_prefix_groups": len(duplicates),
        "business_tests_executed_by_this_document_check": 0,
        "rust_builds_executed_by_this_document_check": 0,
        "runtime_or_performance_claims": "Not inferred from documentation checks; repair evidence is recorded separately",
    },
    "errors": errors, "document_sha256": document_hashes,
    "referenced_source_sha256": source_hashes, "referenced_source_symbols": symbols,
    "current_feature_declarations": current_feature_sources,
    "batch_current_declarations": batch_current_declarations,
    "batch_current_source_facts": batch_static_facts,
    "manufacturing_registered_http_operations": registered_manufacturing_operations,
    "manufacturing_get_query_fields": manufacturing_get_query_fields,
    "manufacturing_new_post_request_fields": manufacturing_new_request_fields,
    "quality_registered_http_operations": registered_quality_operations,
    "quality_post_request_fields": quality_request_fields,
    "finance_registered_http_operations": registered_finance_operations,
    "finance_http_request_fields": finance_request_fields,
    "export_registered_http_operations": registered_export_operations,
    "export_http_request_fields": export_request_fields,
    "production_receipt_current_source_facts": receipt_static_facts,
    "quality_source_current_source_facts": quality_source_static_facts,
    "inspection_outcome_current_source_facts": outcome_static_facts,
    "consumer_quality_gate_current_source_facts": gate_static_facts,
    "delivery_page_domain_bindings": page_domain_bindings,
    "real_account_current_source_facts": iam_source_facts,
    "real_account_arithmetic": iam_arithmetic,
    "task_report_registered_operations": registered_task_report_operations,
    "task_report_request_fields": task_report_fields,
    "observed_duplicate_migration_prefixes": duplicates,
    "historical_runtime_evidence_comparison": {
        "manifest": "qa/acceptance/free-erp-precision-fifth-source-after.json",
        "comparison_scope": "Current referenced source inputs only; not all runtime dependencies",
        "changed_referenced_inputs": historical_input_drift,
        "current_runtime_certification": "PARTIAL" if historical_input_drift else "NOT_RUN",
        "business_tests_rerun": 0,
    },
    "previous_validation": {
        "updated_at": previous["updated_at"], "status": previous["status"],
        "checks": previous["checks"], "document_sha256": previous["document_sha256"],
    },
    "previous_validation_sha256": hashlib.sha256(previous_bytes).hexdigest(),
}
report_file.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print(json.dumps({"status": report["status"], "checks": report["checks"], "errors": errors}, ensure_ascii=False))
raise SystemExit(1 if errors else 0)
