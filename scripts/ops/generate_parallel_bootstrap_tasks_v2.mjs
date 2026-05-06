import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createTaskCard } from './task_card.mjs';

const currentFilePath = fileURLToPath(import.meta.url);
const scriptDir = path.dirname(currentFilePath);
const repoRoot = path.resolve(scriptDir, '..', '..');

const defaultDate = '2026-04-14';
const args = process.argv.slice(2);
let targetDate = defaultDate;
for (let i = 0; i < args.length; i += 1) {
  if (args[i] === '--date' && args[i + 1]) {
    targetDate = args[i + 1];
    i += 1;
  }
}

const matrixPath = path.join(repoRoot, 'scripts', 'ops', 'config', 'PARALLEL_WINDOW_MATRIX_V2.json');
const matrix = JSON.parse(fs.readFileSync(matrixPath, 'utf8'));
const outDir = path.join(repoRoot, 'tmp', 'parallel-tasks', targetDate);
fs.mkdirSync(outDir, { recursive: true });

const SUBMISSION_FORMAT = 'task | commit | changed_files | test_results | residual_risks';
const SLOT_ORDER = ['T1', 'T2', 'T3'];
const SLOT_LABELS = {
  T1: '进行中',
  T2: '待切换',
  T3: '待验证',
};

const CHINESE_SECTION_LABELS = {
  goal: '目标',
  background: '业务背景',
  dod: '完成定义（DoD）',
  preconditions: '前置依赖',
  testCommands: '测试命令',
  submissionFormat: '回执格式',
  deliveryFormat: '交付要求',
  emptyDod: '- 待补充',
  emptyPreconditions: '- 无',
  emptyTestCommands: '- 待补充',
};

const WINDOW_LABELS = {
  W1: 'OMS 订单引入与建单',
  W2: 'OMS 审核与异常处置',
  W3: 'OMS 结算与对账联动',
  W4: '售后逆向（退换补发）',
  W5: 'WMS 出库波次执行',
  W6: '库存与调拨盘点',
  W7: '采购与入库',
  W8: '商品 / SKU / 主数据',
  W9: '渠道店铺映射与同步',
  W10: '系统治理 / 可观测 / 发布门禁',
};

const WINDOW_TASKS = {
  W1: {
    T1: {
      slug: 'oms-intake-validation-hardening',
      title: '订单引入幂等与字段校验加固',
      goal: '完善建单入口参数校验与幂等约束，降低重复建单和脏数据风险。',
      background: '订单引入是 OMS 主链路入口，输入质量决定后续审核与结算稳定性。',
      tests: ['cargo test -p erp-api --test oms_orders -- --nocapture'],
    },
    T2: {
      slug: 'query-and-api-parameter-alignment',
      title: '查询组件与建单 API 参数对齐',
      goal: '统一前端查询参数与后端建单字段语义，避免字段漂移。',
      background: '并行开发中，查询条件与 API 字段不一致会造成大量误判与返工。',
      tests: ['pnpm --filter erp-web test:unit', 'cargo test -p erp-api --test oms_orders -- --nocapture'],
    },
    T3: {
      slug: 'intake-chain-regression-pack',
      title: '订单引入链路回归收口',
      goal: '形成 W1 批次可合并证据并输出残余风险。',
      background: 'T3 负责将 T1/T2 的改动转为可审计、可合并的回归结果。',
      tests: ['cargo test -p erp-api --test oms_orders -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/order-review-query.spec.ts'],
    },
  },
  W2: {
    T1: {
      slug: 'review-action-guardrails',
      title: '审核动作守卫与状态约束加固',
      goal: '完善审核动作前置条件与状态迁移限制，防止非法流转。',
      background: '审核链路是订单执行中枢，守卫薄弱会放大异常处置成本。',
      tests: ['cargo test -p erp-api --test oms_orders -- --nocapture'],
    },
    T2: {
      slug: 'exception-workbench-sla-routing',
      title: '异常工作台分流与 SLA 字段补齐',
      goal: '补齐异常分流字段与优先级规则，提升异常处理效率。',
      background: '审核异常需要快速定位与分派，字段缺失会导致处理超时。',
      tests: ['cargo test -p erp-api --test order_exception_sla_integration -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/order-exceptions.spec.ts'],
    },
    T3: {
      slug: 'review-exception-regression-pack',
      title: '审核与异常联调回归收口',
      goal: '完成审核与异常处置联调回归，输出可合并回执。',
      background: 'T3 负责 W2 批次收敛，保证审核链路变更可控上线。',
      tests: ['cargo test -p erp-api --test order_exception_sla_integration -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/order-exceptions.spec.ts'],
    },
  },
  W3: {
    T1: {
      slug: 'settlement-state-machine-guard',
      title: '结算状态机与单据幂等约束',
      goal: '完善结算状态迁移守卫与单据幂等，避免重复结算。',
      background: '结算链路直接影响财务准确性，优先稳定状态机。',
      tests: ['cargo test -p erp-api --test finance_reconcile -- --nocapture'],
    },
    T2: {
      slug: 'finance-reconcile-field-alignment',
      title: '财务字段口径与对账映射对齐',
      goal: '统一结算与财务字段映射，降低对账偏差。',
      background: '结算域与财务域并行开发时，字段口径一致性是高风险点。',
      tests: ['cargo test -p erp-api --test finance_reconcile -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/order-processing-flow.spec.ts'],
    },
    T3: {
      slug: 'settlement-audit-regression-pack',
      title: '结算对账回归与审计收口',
      goal: '完成结算与对账批次回归并提供审计证据。',
      background: 'T3 聚合 W3 变更，输出主控可直接判定的合并依据。',
      tests: ['cargo test -p erp-api --test finance_reconcile -- --nocapture', 'cargo test -p erp-api --test system_audit_reports_integration -- --nocapture'],
    },
  },
  W4: {
    T1: {
      slug: 'after-sales-reverse-state-flow',
      title: '售后退换补发状态流转加固',
      goal: '补齐售后状态流转规则与动作限制，避免逆向流程串单。',
      background: '售后逆向是重点链路，需先稳定状态机后再扩展页面能力。',
      tests: ['cargo test -p erp-api --test after_sales_state_machine_guard -- --nocapture'],
    },
    T2: {
      slug: 'after-sales-ui-operation-consistency',
      title: '售后页面操作约束与可视化补齐',
      goal: '确保售后页面按钮行为与后端状态约束一致。',
      background: '前后端语义不一致会导致人工误操作与工单反复。',
      tests: ['cargo test -p erp-api --test after_sales -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/after-sales-workflow.spec.ts'],
    },
    T3: {
      slug: 'after-sales-regression-wrap-up',
      title: '售后主链路回归收口',
      goal: '完成售后窗口批次回归并提交残余风险。',
      background: 'T3 负责售后域合并前收口，降低跨域回归风险。',
      tests: ['cargo test -p erp-api --test after_sales_finance_reconcile -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/after-sales-return-resend-main-chain.spec.ts'],
    },
  },
  W5: {
    T1: {
      slug: 'wms-wave-execution-core',
      title: '出库波次执行主链路加固',
      goal: '完善波次执行状态与作业落库字段，保障现场执行稳定。',
      background: 'WMS 波次链路是履约核心路径，必须优先保证可追踪。',
      tests: ['cargo test -p erp-api --test wave_ship_integration -- --nocapture'],
    },
    T2: {
      slug: 'wms-wave-ui-action-idempotency',
      title: '波次作业看板与动作幂等保护',
      goal: '增强波次页面可视化和重复点击幂等保护。',
      background: '现场高频操作下幂等保护缺失会导致重复执行。',
      tests: ['cargo test -p erp-api --test wms_labor_slotting_flow -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/wms-wave-main-chain.spec.ts'],
    },
    T3: {
      slug: 'wms-wave-regression-pack',
      title: '波次履约批次回归收口',
      goal: '形成 W5 批次回归证据并输出可合并回执。',
      background: 'T3 汇总出库波次关键证据，支撑主控判断合并时机。',
      tests: ['cargo test -p erp-api --test wave_ship_integration -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/outbound-fulfillment-flow.spec.ts'],
    },
  },
  W6: {
    T1: {
      slug: 'inventory-transfer-freeze-rules',
      title: '库存调拨与冻结规则加固',
      goal: '完善库存调拨状态与冻结规则，降低跨仓错账风险。',
      background: '库存治理窗口优先保障可用量与调拨一致性。',
      tests: ['cargo test -p erp-api --test inventory_atp_fefo_integration -- --nocapture'],
    },
    T2: {
      slug: 'stock-count-difference-loop',
      title: '盘点差异闭环与调拨联动',
      goal: '补齐盘点差异处理流程并联动调拨修正动作。',
      background: '盘点差异是库存治理高频问题，需要建立闭环。',
      tests: ['cargo test -p erp-api --test wms_inbound_inventory_workflow_integration -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/inventory-governance-packaging.spec.ts'],
    },
    T3: {
      slug: 'inventory-governance-regression-pack',
      title: '库存治理回归与风险收口',
      goal: '输出库存窗口批次回归证据与残余风险。',
      background: 'T3 聚合 W6 变更，确保库存域并行提交可控。',
      tests: ['cargo test -p erp-api --test inventory_atp_fefo_integration -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/inventory.spec.ts'],
    },
  },
  W7: {
    T1: {
      slug: 'procurement-inbound-main-chain',
      title: '采购到货入库主链路打通',
      goal: '完善采购到货与入库确认链路，提升入库可追溯性。',
      background: '采购与入库链路断裂会直接影响库存与履约准确性。',
      tests: ['cargo test -p erp-api --test planning_replenishment_flow -- --nocapture'],
    },
    T2: {
      slug: 'inbound-exception-receive-flow',
      title: '入库异常拒收与补录流程加固',
      goal: '补齐拒收、补录与二次入库流程，降低异常积压。',
      background: '异常入库是仓采协同高风险环节，需要独立收敛。',
      tests: ['cargo test -p erp-api --test wms_inbound_inventory_workflow_integration -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/inbound-flow.spec.ts'],
    },
    T3: {
      slug: 'procurement-inbound-regression-pack',
      title: '采购入库回归与权限收口',
      goal: '完成采购入库批次回归并提交标准回执。',
      background: 'T3 用于确认采购域变更具备最小权限与主链路验证。',
      tests: ['cargo test -p erp-api --test procurement_permissions -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/procurement-pages.spec.ts'],
    },
  },
  W8: {
    T1: {
      slug: 'product-master-data-change-flow',
      title: '商品主数据变更流程加固',
      goal: '完善商品主数据变更流程与审计字段，提升可追溯性。',
      background: '主数据质量直接影响订单、库存与渠道一致性。',
      tests: ['cargo test -p erp-api --test skus -- --nocapture'],
    },
    T2: {
      slug: 'sku-barcode-unit-alignment',
      title: 'SKU 条码与单位换算对齐',
      goal: '统一 SKU 条码、单位换算与页面交互语义。',
      background: 'SKU 结构变更频繁，需保持 repo 与 UI 双侧一致。',
      tests: ['cargo test -p erp-api --test skus -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/skus.spec.ts'],
    },
    T3: {
      slug: 'product-sku-regression-pack',
      title: '商品与 SKU 链路回归收口',
      goal: '输出商品/SKU 窗口批次回归证据。',
      background: 'T3 负责主数据窗口合并前收口，避免主干回归。',
      tests: ['cargo test -p erp-api --test skus -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/products.spec.ts'],
    },
  },
  W9: {
    T1: {
      slug: 'channel-shop-mapping-rules',
      title: '渠道店铺映射规则与冲突检测',
      goal: '完善映射规则与冲突检测，降低错配风险。',
      background: '渠道映射错误会直接影响同步、履约与库存联动。',
      tests: ['cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture'],
    },
    T2: {
      slug: 'channel-adapter-sync-retry',
      title: '渠道同步重试与错误分类',
      goal: '增强同步失败重试与错误分类能力，提升稳定性。',
      background: '多平台适配器失败场景复杂，需要统一重试策略。',
      tests: ['cargo test -p erp-api --test platform_oauth_webhook_integration -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/channels-shops-list-visibility.spec.ts'],
    },
    T3: {
      slug: 'channel-mapping-regression-pack',
      title: '渠道映射同步回归收口',
      goal: '完成渠道窗口批次回归并输出残余风险。',
      background: 'T3 汇总渠道域证据，为主控提供合并判断依据。',
      tests: ['cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture', 'pnpm --filter erp-web test:e2e -- e2e/shop-product-match-governance.spec.ts'],
    },
  },
  W10: {
    T1: {
      slug: 'rbac-audit-baseline-check',
      title: '权限与审计基线巡检',
      goal: '补齐 RBAC 与审计日志关键校验，保证并行提交可追溯。',
      background: 'W10 只做系统治理与门禁，不做跨域业务实现。',
      tests: ['cargo test -p erp-api --test system_audit_reports_integration -- --nocapture'],
    },
    T2: {
      slug: 'openapi-gate-hardening',
      title: 'OpenAPI 门禁脚本与校验加固',
      goal: '增强 OpenAPI 相关门禁脚本可读性与一致性校验。',
      background: 'OpenAPI 文件是并行冲突高发点，需要治理窗口集中收口。',
      tests: ['pnpm run openapi:bundle', 'pnpm run repo:verify:openapi'],
      sharedLockTargets: ['scripts/openapi/lib/openapi_tools.mjs'],
    },
    T3: {
      slug: 'merge-gate-findings-and-ordering',
      title: '批次门禁 findings 与合并顺序建议',
      goal: '输出越界、缺测、锁冲突检查结果及合并顺序建议。',
      background: 'W10-T3 仅输出 findings 与合并建议，不做业务代码实现。',
      tests: ['pnpm run repo:verify', `node scripts/ops/generate_parallel_scorecard.mjs --input tmp/parallel-tasks/${targetDate}/parallel-scorecard-input.json --output tmp/parallel-tasks/${targetDate}/parallel-scorecard.md`],
    },
  },
};

const indexLines = [];
indexLines.push(`# ${targetDate} 并行窗口首批任务卡（全中文描述版）`);
indexLines.push('');
indexLines.push('> 说明：任务卡 front matter 键名保持英文（供脚本解析），键值与正文均为中文，方便直接分发。');
indexLines.push('');
indexLines.push('## 字段中文说明');
indexLines.push('- `name`: 任务名称');
indexLines.push('- `goal`: 任务目标');
indexLines.push('- `background`: 业务背景');
indexLines.push('- `owner/window/domain/slot`: 责任窗口与槽位');
indexLines.push('- `allowed_files/disallowed_files`: 允许与禁止修改路径');
indexLines.push('- `shared_lock_targets`: 共享锁目标（命中锁文件时必填）');
indexLines.push('- `test_commands`: 最小必跑测试');
indexLines.push('- `submission_format`: 回执字段格式');
indexLines.push('');
indexLines.push('## 启动顺序（建议）');
indexLines.push('1. 全窗口先启动 T1。');
indexLines.push('2. 同窗口 T2 在 T1 字段契约稳定后切换。');
indexLines.push('3. T3 仅在 T1/T2 有候选提交后启动收口。');
indexLines.push('');

let generatedCount = 0;

for (const windowMeta of matrix.windows) {
  const taskSet = WINDOW_TASKS[windowMeta.id];
  if (!taskSet) {
    throw new Error(`missing task definition for ${windowMeta.id}`);
  }

  indexLines.push(`## ${windowMeta.id} · ${WINDOW_LABELS[windowMeta.id] || windowMeta.domain}`);
  const promptLines = [];
  promptLines.push(`你是窗口 ${windowMeta.id}，只负责 ${WINDOW_LABELS[windowMeta.id] || windowMeta.domain}。`);
  promptLines.push('你可并行处理 3 个子任务，但每个子任务必须独立提交。');
  promptLines.push('严格要求：');
  promptLines.push('1) 仅修改任务卡 allowed_files。');
  promptLines.push('2) 提交前必须运行：node scripts/ops/validate_task_boundary.mjs --task <TASK_CARD> --mode all');
  promptLines.push('3) 执行任务卡内最小测试集。');
  promptLines.push('4) 回执格式：task / commit / changed_files / test_results / residual_risks。');
  promptLines.push('禁止跨域顺手改文件，禁止触碰共享锁文件（除非主控授锁）。');
  promptLines.push('本窗口首批任务卡：');

  for (const slot of SLOT_ORDER) {
    const task = taskSet[slot];
    if (!task) {
      throw new Error(`missing task ${windowMeta.id} ${slot}`);
    }

    const fileName = `${windowMeta.id.toLowerCase()}-${slot.toLowerCase()}-${task.slug}.md`;
    const relPath = `tmp/parallel-tasks/${targetDate}/${fileName}`;
    const sharedLockTargets = task.sharedLockTargets || [];

    const dependencies = ['任务卡已创建并确认窗口边界'];
    if (slot === 'T2') {
      dependencies.push('T1 核心字段契约已稳定');
    }
    if (slot === 'T3') {
      dependencies.push('T1/T2 已有候选提交');
    }
    if (sharedLockTargets.length) {
      dependencies.push('已完成共享锁登记并持锁编辑');
    }

    const metadata = {
      name: `${windowMeta.id}-${slot} ${task.title}`,
      goal: task.goal,
      background: task.background,
      owner: `${windowMeta.id}-owner`,
      window: windowMeta.id,
      domain: windowMeta.domain,
      slot,
      branch: `codex/${windowMeta.id.toLowerCase()}-${windowMeta.domain}-${slot.toLowerCase()}-${targetDate.replaceAll('-', '')}`,
      worktree: `.worktrees/${windowMeta.id.toLowerCase()}-${windowMeta.domain}-${slot.toLowerCase()}-${targetDate.replaceAll('-', '')}`,
      allowedFiles: windowMeta.owned_paths,
      disallowedFiles: windowMeta.blocked_paths,
      sharedLockTargets,
      dependencies,
      dod: [
        '边界校验 PASS，且无越界改动',
        '任务卡规定的最小测试通过',
        '按标准回执格式输出结果与残余风险',
      ],
      testCommands: [
        `node scripts/ops/validate_task_boundary.mjs --task ${relPath} --mode all`,
        ...task.tests,
      ],
      submissionFormat: SUBMISSION_FORMAT,
      delivery: '按标准回执格式返回：task / commit / changed_files / test_results / residual_risks',
      sectionLabels: CHINESE_SECTION_LABELS,
    };

    fs.writeFileSync(path.join(outDir, fileName), createTaskCard(metadata), 'utf8');
    indexLines.push(`- ${slot}（${SLOT_LABELS[slot]}）: [${metadata.name}](${relPath})`);
    promptLines.push(`${slot}: ${relPath}`);
    generatedCount += 1;
  }

  indexLines.push('');
  indexLines.push(`### ${windowMeta.id} 可直接发送提示词`);
  indexLines.push('```text');
  indexLines.push(...promptLines);
  indexLines.push('```');
  indexLines.push('');
}

indexLines.push('## 主控窗口可直接发送提示词');
indexLines.push('```text');
indexLines.push('你是并行主控。请按 10 窗口责任矩阵派发子任务，并严格执行共享锁机制。');
indexLines.push('必须做到：');
indexLines.push('1) 所有子任务先有任务卡再实现。');
indexLines.push('2) 所有提交先跑边界校验。');
indexLines.push('3) 触及 OpenAPI 的任务必须执行 openapi:bundle + repo:verify:openapi。');
indexLines.push('4) 输出当前批次可合并顺序与阻塞项，不做越界补丁实现。');
indexLines.push('```');

const indexPath = path.join(outDir, 'WINDOW_TASKS_BOOTSTRAP_V2.md');
fs.writeFileSync(indexPath, `${indexLines.join('\n')}\n`, 'utf8');

console.log(`[generate_parallel_bootstrap_tasks_v2] generated ${generatedCount} task cards at ${outDir}`);
console.log(`[generate_parallel_bootstrap_tasks_v2] index: ${indexPath}`);
