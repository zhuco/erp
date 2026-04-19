# ¹²Ïí±í¸ñÁп刂朴氤־û¯ʵÏּƻ®

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 AG Grid ԭÉúÁв˵ブ形Ļ¯¡¢±íͷÓҼп刂ơ¢ÁÐ״̬³־û³Áµ½¹²Ïí `ErpDataGrid`£¬²¢Èö©µ¥²éѯ¡¢Íøµ궩单查ѯ、物流查ѯ¼°¶©µ¥Éóº˶ÔÆëͬһÌ×Áн»»¥ЭÒ顣

**Architecture:** 以 `ErpDataGrid` 作Ϊ¹²ÏíÄÜÁ¦Ï³恋㣬ÐÂÔö¶ÀÁ¢µÄÁв˵¥/列״̬¹¤¾ߺ¯Êý£¬ҳ面ֻÌṩ `columnSettingsKey` 或ҳÃ漶 key。`ErpDataGridNext` 与 `UiModule03MainTableSection` ²»Öظ´ʵ现能力，ֻ负责͸传，订单审核ͨ¹ý¸´Óù蚕砉ぞ߱£³ÖÐÐΪһÖ¡£

**Tech Stack:** React, TypeScript, AG Grid Enterprise, Vitest, Playwright

---

### Task 1: 锁ס共享表格行Ϊ测试

**Files:**
- Create: `apps/erp-web/src/components/ErpDataGrid.columnState.test.tsx`
- Modify: `apps/erp-web/e2e/orders.spec.ts`

- [ ] **Step 1: дʧ°ܲ馐ԣ哺侵形Ĳ˵¥ӳ射与列״̬保存/»ָ´**
- [ ] **Step 2: 运行 `pnpm vitest run src/components/ErpDataGrid.columnState.test.tsx`，ȷ认因功能ȱʧ而ʧ败**
- [ ] **Step 3: 补һ条共享链· E2E，验֤列显隐ˢÐº笕Ա£Áô**
- [ ] **Step 4: 运行 `pnpm playwright test e2e/orders.spec.ts`，ȷÈÏÐ¶ÏÑÔÏÈʧ败**

### Task 2: Ôڹ蚕聿阆³ÁԭÉúÁпØÖÆ

**Files:**
- Create: `apps/erp-web/src/components/agGridColumnState.ts`
- Modify: `apps/erp-web/src/components/ErpDataGrid.tsx`
- Modify: `apps/erp-web/src/components/ErpDataGridNext.tsx`
- Modify: `apps/erp-web/src/components/ui-modules/UiModule03MainTableSection.tsx`

- [ ] **Step 1: 抽出中文 localeText¡¢²ü舨˵ハ´洢键构造与读д工具**
- [ ] **Step 2: 在 `ErpDataGrid` 接入表ͷÓҼüԭÉú²˵ビ胫形Ĳ˵¥**
- [ ] **Step 3: 在 `ErpDataGrid` 接入列显隐/¹̶¨ÁÐ/˳序/宽度/排序״̬³־û¯**
- [ ] **Step 4: 让 `ErpDataGridNext`、`UiModule03MainTableSection` ͸传 `columnSettingsKey` ÒÔÐγÉҳÃ漶³־û¯Èë¿Ú**

### Task 3: ҳÃæ½ÓÈëÓ붩µ¥Éóº˶ÔÆë

**Files:**
- Modify: `apps/erp-web/src/pages/orders/OrdersPage.tsx`
- Modify: `apps/erp-web/src/pages/sales/ShopOrderQueryPage.tsx`
- Modify: `apps/erp-web/src/pages/logistics/LogisticsTrackingPage.tsx`
- Modify: `apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx`

- [ ] **Step 1: Ϊ共享链·ҳÃ油Îȶ¨µÄ `columnSettingsKey`**
- [ ] **Step 2: Èö┑ド蠛酥鞅砀从ù蚕砹в˵ゲü粲胫形奈İ¸£¬±ÜÃâ˫份ά护**
- [ ] **Step 3: È˹ぜ觳橹鞅砜蛳ߡ¢±íͷÓҼü¡¢ÁÐÍÏקÓë³־ûǷ袢ԷϹ¤ҵ级交互**

### Task 4: 完整验֤

**Files:**
- Verify only

- [ ] **Step 1: 运行 `pnpm typecheck`**
- [ ] **Step 2: 运行 `pnpm vitest run src/components/ErpDataGrid.columnState.test.tsx src/pages/orders/review-native/OrderReviewMainGrid.test.ts`**
- [ ] **Step 3: 运行 `pnpm playwright test e2e/orders.spec.ts e2e/order-review-selection.spec.ts e2e/order-review-left-panel-regression.spec.ts`**
- [ ] **Step 4: 记¼结果与ʣÓà·çÏգ¬ȷ保ģ板链·ҳÃæºóÐø¿ɸ´ÓÃ**
