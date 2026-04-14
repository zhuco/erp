const WEIGHTS = {
  parallel_isolation: 0.15,
  code_and_tests: 0.25,
  merge_governance: 0.2,
  observability_and_audit: 0.15,
  delivery_efficiency: 0.15,
  stability: 0.1
};

const RULE_DEFINITIONS = {
  main_direct_edits: {
    name: 'main 直改',
    type: 'total',
    value: 4.2,
    description: 'main 上直接修改，触发总分封顶'
  },
  success_without_persistence: {
    name: '成功但不落库',
    type: 'total',
    value: 3.9,
    description: '成功但未落库，触发总分封顶'
  },
  db_skip_or_pool_timeout: {
    name: 'DB skip/pool timeout',
    type: 'total',
    value: 4.0,
    description: 'DB skip 或 pool 超时，触发总分封顶'
  },
  rollback_plan_missing: {
    name: '缺少回滚方案',
    type: 'dimension',
    target: 'merge_governance',
    mode: 'max',
    value: 3.8,
    description: '缺少回滚方案，限制 merge governance'
  },
  multiple_ai_same_branch: {
    name: '双 AI 同分支',
    type: 'dimension',
    target: 'parallel_isolation',
    mode: 'max',
    value: 3.5,
    description: '双 AI 同分支，限制 parallel isolation'
  },
  logic_deleted_to_fix_compile: {
    name: '删逻辑过编译',
    type: 'dimension',
    target: 'code_and_tests',
    mode: 'min',
    value: 3.0,
    description: '删逻辑修复编译，设定 code_and_tests 下限'
  }
};

const REQUIRED_DIMENSIONS = Object.keys(WEIGHTS);

const normalizeScores = (scores = {}) => {
  const normalized = {};
  for (const dimension of REQUIRED_DIMENSIONS) {
    const raw = scores[dimension];
    normalized[dimension] = typeof raw === 'number' ? raw : 0;
  }
  return normalized;
};

export function calculateScorecard(payload = {}) {
  const inputScores = normalizeScores(payload.scores);
  const caps = payload.caps ?? {};
  const adjustedScores = { ...inputScores };
  const triggeredRules = [];

  for (const [key, rule] of Object.entries(RULE_DEFINITIONS)) {
    if (!caps[key]) continue;
    if (rule.type === 'dimension') {
      const current = adjustedScores[rule.target];
      if (rule.mode === 'max') {
        adjustedScores[rule.target] = Math.min(current, rule.value);
      } else {
        adjustedScores[rule.target] = Math.max(current, rule.value);
      }
      triggeredRules.push({
        key,
        name: rule.name,
        type: 'dimension',
        target: rule.target,
        mode: rule.mode,
        value: rule.value,
        description: rule.description
      });
    } else {
      triggeredRules.push({
        key,
        name: rule.name,
        type: 'total',
        cap: rule.value,
        description: rule.description
      });
    }
  }

  const weightedTotal = REQUIRED_DIMENSIONS.reduce((sum, dimension) => {
    return sum + (adjustedScores[dimension] ?? 0) * WEIGHTS[dimension];
  }, 0);

  const totalCaps = triggeredRules
    .filter(rule => rule.type === 'total')
    .map(rule => rule.cap);
  const totalCapValue = totalCaps.length > 0 ? Math.min(...totalCaps) : null;
  const finalTotal =
    totalCapValue !== null ? Math.min(weightedTotal, totalCapValue) : weightedTotal;

  const singleThresholdOk = REQUIRED_DIMENSIONS.every(
    dimension => adjustedScores[dimension] >= 4.2
  );
  const isQualified =
    finalTotal >= 4.5 &&
    singleThresholdOk &&
    (payload.recent_p0_regressions ?? 0) === 0 &&
    payload.gate_effective === true;

  return {
    date: payload.date ?? new Date().toISOString().slice(0, 10),
    originalScores: inputScores,
    adjustedScores,
    weightedTotal,
    finalTotal,
    isQualified,
    triggeredRules,
    totalCapValue,
    recent_p0_regressions: payload.recent_p0_regressions ?? 0,
    gate_effective: payload.gate_effective === true,
    risks: payload.risks ?? [],
    next_priorities: payload.next_priorities ?? [],
    notes: payload.notes ?? []
  };
}

const renderList = (title, items) => {
  if (!items || items.length === 0) {
    return `**${title}**：无`;
  }
  return `**${title}**：\n${items.map(item => `- ${item}`).join('\n')}`;
};

const formatRule = (rule) => {
  if (rule.type === 'total') {
    return `${rule.name}（总分封顶 ${rule.cap}）`;
  }
  const direction = rule.mode === 'max' ? '封顶' : '封底';
  return `${rule.name}（${rule.target} ${direction} ${rule.value}）`;
};

export function renderMarkdown(scorecard) {
  const dimensionLines = REQUIRED_DIMENSIONS.map(
    dimension => `- ${dimension}：${scorecard.originalScores[dimension].toFixed(1)}`
  ).join('\n');
  const adjustedLines = REQUIRED_DIMENSIONS.map(
    dimension => `- ${dimension}：${scorecard.adjustedScores[dimension].toFixed(1)}`
  ).join('\n');

  const triggered =
    scorecard.triggeredRules.length > 0
      ? scorecard.triggeredRules.map(rule => `- ${formatRule(rule)}`).join('\n')
      : '- 无';

  const risksSection = renderList('今日风险', scorecard.risks);
  const nextSection = renderList('次日重点', scorecard.next_priorities);
  const notesSection = renderList('备注', scorecard.notes);

  const qualificationText = scorecard.isQualified ? '达标' : '未达标';

  return `## 多 AI 并行评分卡
**日期**：${scorecard.date}

**六维分数**：
${dimensionLines}

**调整后的维度分数**：
${adjustedLines}

**加权总分**：${scorecard.finalTotal.toFixed(3)}
**是否达标**：${qualificationText}

**触发封顶/封底规则**：
${triggered}

${risksSection}

${nextSection}

${notesSection}
`;
}
