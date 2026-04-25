# Order Mark Standard Design

**Date:** 2026-04-19

**Goal**

Build a reusable order-mark standard for `WorkbenchPageTemplate` order pages so the main table and right detail panel share one display rule, while the right-side "add mark" action only exposes user-defined marks.

## Scope

This design covers:

- unified order mark display in the main grid and right detail panel
- standard builtin mark definitions for system-recognized order states
- standard right-panel add-mark menu rules for user-defined marks
- removal of the placeholder empty mark badge in all affected order pages

This design does not cover:

- mock persistence after browser `F5` refresh
- backend APIs for mark creation or persistence
- redesigning unrelated order page layout or toolbar behavior

## Product Decision

The order workbench will treat marks as two different classes with different responsibilities:

- builtin marks: recognized by the system from order state and displayed automatically
- custom marks: created by users and optionally attached to orders manually

The same order can display both classes together, but only custom marks are allowed in the right-panel add-mark menu.

## Core Rules

### 1. Unified Display Standard

All order pages that show a "记号" area must use one shared resolver to build display marks.

The resolver must:

- derive builtin marks from order state fields
- include user-defined custom marks from explicit tag fields
- deduplicate custom marks
- ignore builtin aliases when collecting custom marks

The affected UI areas must all use this same display result:

- order review main grid
- order query main grid
- order exceptions main grid
- right detail panel mark display

### 2. Builtin Mark Standard

Builtin marks remain system-defined and are not manually addable from the right panel.

The initial builtin set is:

- `匹`: 商品未匹配
- `缺`: 全部缺货或部分缺货
- `冻`: 自动策略或人工冻结
- `预`: 人工标记预售货品，或预售订单
- `超时`: 超过平台要求发货时间
- `退`: 平台有退款或部分退款
- `合`: 由多个订单合并
- `拆`: 订单被拆分或本身为拆分子单

Each builtin mark keeps its own background color and white text to preserve quick visual recognition.

### 3. Right-Panel Add Standard

The right-panel "添加记号" entry must only list custom marks that users are allowed to add manually.

The menu builder must:

- receive the available custom mark options
- remove any builtin marks from the candidate set
- remove any mark already attached to the current order
- preserve custom marks even when their names match common business labels such as `VIP`

This means:

- system builtin `匹 / 缺 / 冻 / 预 / 超时 / 退 / 合 / 拆` never appear in the add menu
- a user-created custom mark named `VIP` can appear and can be added
- if the current order already has a custom `VIP` mark, it is hidden from the add menu until removed

### 4. Empty-State Standard

When an order has no marks, the UI must render no placeholder badge and must not show the old circle with `-`.

This applies to both:

- main grid cells
- right detail panel mark area

## Architecture

### Shared Rule Center

`apps/erp-web/src/features/orderTags.ts` remains the single rule center for order mark behavior.

It should own:

- builtin mark definitions
- builtin mark resolution
- display mark resolution
- editable mark menu construction for custom marks

Pages should not duplicate filtering logic once the shared helper exists.

### UI Boundary

`UiModule05OrderReviewRightPanelTemplate.tsx` stays presentation-focused.

The template should:

- render resolved display marks
- render add-menu items provided by the page
- emit selection events

The template should not decide which marks are builtin or editable.

### Page Boundary

Order pages should only assemble inputs and consume shared outputs.

Pages may provide:

- the current order record
- the current order tags
- the available custom mark definitions

Pages should not reimplement:

- builtin mark filtering
- duplicate filtering
- special handling for custom labels such as `VIP`

## Testing Strategy

Before implementation, add or update tests to prove:

- builtin marks are displayed consistently across table and right panel
- the right add-mark menu excludes builtin marks
- the right add-mark menu excludes marks already attached to the current order
- a user-defined custom mark named `VIP` is still addable when not yet attached
- empty mark areas no longer render the placeholder `-`

## Acceptance Criteria

The design is complete when:

- "标记/异常" is standardized to "记号" in the relevant order workbench surfaces
- main grid and right panel show the same resolved mark set for the same order
- right-panel add-mark menus only expose user-defined custom marks
- builtin marks remain display-only and cannot be added manually from the menu
- no default `VIP` suppression remains for custom marks
- refreshing the browser still loses mock-only edits, and this is explicitly unchanged by this work
