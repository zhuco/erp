# Workbench Page Template Unified Template Design

**Date:** 2026-04-19

**Goal**

Define `WorkbenchPageTemplate` as the single public workbench template for the ERP, while moving reusable interaction behavior into shared internal contracts so future data-table pages can adopt one standard API without reimplementing workbench logic.

## Background

The current workbench direction already establishes:

- `WorkbenchPageTemplate` as the public page-shell entry
- `WorkbenchToolbar` as the public toolbar entry
- order review and order recheck as the first industrial-standard validation pages

However, important workbench behavior is still partially page-local:

- left-panel mounting and lifecycle
- selection / focus / active-record state coordination
- page-level saved-view hydration beyond the base hook
- keyboard shortcut registration
- context-menu orchestration
- overlay composition patterns

This creates a gap between the intended standard and the actual page implementation. New pages can reuse the shell, but they still tend to copy the interaction logic from order review or order recheck.

The ERP product direction is mid-to-high-end enterprise software. That requires:

- one stable public template API for page authors
- consistent workbench interaction behavior across modules
- internal modularity that allows controlled evolution without shell bloat

## Product Decision

The project will expose exactly one public workbench page template entry for table-centric pages:

- `WorkbenchPageTemplate`

The project will not introduce a second public workbench template such as:

- `StandardWorkbenchPage`
- `EnterpriseWorkbenchPage`
- additional `MainTableTemplate` / `ToolbarTemplate` families

The correct architectural model is:

- external single-template standard
- internal modular contracts and controllers

In other words:

- page authors see one unified template
- framework maintainers implement that template through shared modules

This preserves API simplicity for business teams while maintaining enterprise-grade internal structure.

## Scope

This design covers:

- the unified public API shape of `WorkbenchPageTemplate`
- the internal controller / hook / contract split behind that API
- standard handling for workbench panels, state persistence, selection, shortcuts, context menus, and overlay orchestration
- migration of order review and order recheck to the new standard
- first-wave rollout rules for future workbench pages

This design does not cover:

- redesigning the visual theme of the ERP
- turning business actions such as review / split / merge into template-owned actions
- forcing every existing historical page onto the new standard in one pass
- replacing all grid implementations with a single grid component immediately
- advanced user-personalized layout systems beyond the current workbench state needs

## Core Principles

### 1. Single Public Entry

All future table-centric ERP workbench pages should start from `WorkbenchPageTemplate`.

That means page authors should not need to understand panel stores, saved-view persistence plumbing, or interaction lifecycle wiring just to build a new page.

### 2. Shell Ownership Stays Narrow

`WorkbenchPageTemplate` remains the public shell API. It should own:

- page region composition
- layout timing / rhythm
- standard workbench interaction integration points
- panel mounting lifecycle
- footer / pagination placement
- overlay mounting

It should not own:

- business action semantics
- order-specific terminology
- per-page business queries
- domain-specific validation and save logic

### 3. Shared Interaction Standards Move Out of Business Pages

Reusable workbench behavior should move into shared workbench contracts and controllers rather than remaining embedded in order review or order recheck.

This is required for:

- consistency
- maintainability
- page-author ergonomics
- future scalability across many ERP pages

### 4. Business Pages Stay Business-Focused

Business pages should continue to own:

- data fetching
- field mappings
- column definitions
- business toolbar actions
- business dialogs / drawers / overlays
- business validation rules
- business detail rendering

Business pages should stop owning:

- low-level panel mounting
- generic selection / focus / active-row coordination
- generic keyboard listener lifecycle
- generic context-menu lifecycle
- repeated page-state persistence plumbing

## Unified Public API Design

The final public API should read as one cohesive workbench declaration instead of a loose collection of internal mechanics.

### A. Structure Inputs

`WorkbenchPageTemplate` should accept stable region-oriented inputs for page composition:

- `toolbar`
- `header`
- `leftPanel`
- `tableContent`
- `rightPanel`
- `bottomPanel`
- `overlays`
- `footerSummary`
- `pagination`
- `onPaginationChange`

Region visibility and sizing should be first-class public API rather than indirect behavior:

- `leftPanelVisible`
- `leftPanelWidth`
- `rightPanelVisible`
- `rightPanelWidth`
- `bottomPanelVisible`
- `bottomPanelHeight`

This formalizes the ERP workbench skeleton:

- left filter / navigation area
- main data table area
- right detail area
- bottom supplemental area

### B. Workbench State Inputs

The template should accept standardized workbench state integration points rather than forcing each page to hand-roll local state orchestration.

This state family should cover:

- saved-view persistence
- scroll restoration
- selected rows
- focused row
- active row / active record
- selection anchor
- page-level recoverable filters

The public API may expose these directly or, preferably, through standard hooks whose outputs are passed into the template. The key design rule is that page authors should consume one standard workbench state contract rather than inventing their own state topology.

### C. Interaction Inputs

The template should expose declarative workbench interaction inputs for standard behavior:

- keyboard shortcuts
- context menu configuration
- left-panel controller configuration
- optional workbench announcements / accessibility notification hooks

The template must orchestrate lifecycle and coordination, while pages provide business callbacks and page-specific menu / shortcut definitions.

### D. Explicit Non-Goals for the Public API

The template API must not become:

- a business-action registry
- an order-specific configuration surface
- a catch-all for page-local ad hoc state
- a replacement for page-owned business logic

## Internal Architecture

The internal implementation should follow a modular contract design behind the single public template entry.

### 1. `WorkbenchLayoutController`

Responsibility:

- mount and unmount left / right / bottom panels
- synchronize visibility
- synchronize default dimensions
- mount overlays through the shell

This controller owns region lifecycle only. It does not interpret business meaning.

### 2. `WorkbenchViewStateController`

Responsibility:

- saved-view hydration / serialization lifecycle
- scroll position persistence
- restoration of selected / focused / active workbench state
- page-level recoverable filter persistence

This controller should standardize workbench state storage patterns so pages do not each build their own `localStorage` orchestration.

### 3. `WorkbenchSelectionController`

Responsibility:

- `selectedRowKeys`
- `focusedRowKey`
- `activeRowKey`
- `selectionAnchorKey`
- filtered-result selection trimming
- active-row fallback behavior
- first-record initialization behavior

This controller should define the standard behavior for how a workbench maintains continuity when rows are refreshed, filtered, or paginated.

### 4. `WorkbenchLeftPanelController`

Responsibility:

- left-panel tab switching
- tree expansion / collapse behavior
- standard leaf-filter interaction handling
- structured filter apply / reset lifecycle
- coordination between left-panel filtering and main-grid filtering

The controller should not dictate the business content of the left panel. Pages still provide:

- node construction
- filter mapping
- structured filter form content
- stats content

### 5. `WorkbenchInteractionController`

Responsibility:

- keyboard shortcut registration and cleanup
- shortcut blocking in editable targets
- context-menu open / close / positioning lifecycle
- context record synchronization
- workbench announcement integration

Pages provide:

- shortcut definitions
- shortcut callbacks
- context-menu items
- context-menu action handlers

### 6. `WorkbenchMainGridBridge`

Responsibility:

- define the minimum bridge contract between `WorkbenchPageTemplate` and any main-grid implementation

The bridge should standardize support for:

- selection sync
- focus sync
- active-row sync
- row double-click callback
- row context-menu callback
- column-filter-change callback
- grid API exposure
- retry / empty / error handling

This allows the workbench standard to govern interaction behavior without forcing immediate convergence to one grid implementation.

## First-Wave Standardization Requirements

The first implementation must immediately standardize the following behavior.

### Required in Wave 1

1. Formal `leftPanel` support in `WorkbenchPageTemplate`
2. Standard left / right / bottom panel visibility and sizing lifecycle
3. Standard selection / focus / active-row / anchor-row contract
4. Standard saved-view and workbench state restoration contract
5. Standard keyboard shortcut lifecycle support
6. Standard context-menu lifecycle support
7. Main-grid bridge contract

These are high-value, repeated workbench behaviors that should stop living in page-local code.

### Intentionally Left Page-Owned in Wave 1

1. Business toolbar actions such as review / split / merge
2. Business dialogs and their domain logic
3. Left-panel business content
4. Right-panel business content
5. Bottom-panel business content
6. Business validation and save orchestration

The framework should standardize composition and lifecycle, not absorb domain semantics.

### Explicitly Deferred

1. A second public workbench template family
2. Template-owned business action systems
3. Immediate full migration of all historical pages
4. Immediate single-grid rewrite across the whole ERP
5. Advanced personalized layout systems beyond current workbench needs

## Migration Strategy

### Validation Pages

The first two validation pages for the unified standard should remain:

- order review
- order recheck / exception review

These two pages already exercise a large portion of the workbench interaction surface and are strong candidates for standard verification.

### Migration Step 1: Extract Shared Controllers

Before aggressively simplifying page code, implement the shared workbench controllers and contracts behind the existing public template entry.

This reduces risk because:

- the public entry remains stable
- existing pages can migrate incrementally
- behavior can be verified in slices

### Migration Step 2: Convert Validation Pages to Declarative Usage

After the shared workbench internals exist, convert order review and order recheck so they:

- declare left / right / bottom panel content through public template props
- consume shared workbench state and interaction contracts
- stop mounting panels manually
- stop hand-rolling repeated selection and interaction lifecycle logic

The validation target is that both pages should look like business declarations over a unified workbench platform rather than shell-plus-custom-runtime implementations.

## Expected Future Developer Experience

For future workbench pages, the intended development flow should be:

1. fetch data and prepare pagination
2. define the main-grid bridge implementation
3. define left / right / bottom panel content
4. define business toolbar actions and business overlays
5. connect shared workbench state / interaction configuration
6. render `WorkbenchPageTemplate`

Future page authors should not need to understand:

- direct panel-store usage
- generic saved-view restoration internals
- generic keyboard event wiring
- generic context-menu positioning
- generic active / focused / selected coordination rules

## File Strategy

Likely modified public entry files:

- `apps/erp-web/src/components/page-templates/workbench/WorkbenchPageTemplate.tsx`
- `apps/erp-web/src/components/page-templates/workbench/index.ts`
- `apps/erp-web/src/components/workbench/templates/OrderReviewWorkbenchTemplate.tsx`

Likely new shared internal files:

- `apps/erp-web/src/components/page-templates/workbench/contracts.ts`
- `apps/erp-web/src/components/page-templates/workbench/useWorkbenchLayoutController.ts`
- `apps/erp-web/src/components/page-templates/workbench/useWorkbenchViewStateController.ts`
- `apps/erp-web/src/components/page-templates/workbench/useWorkbenchSelectionController.ts`
- `apps/erp-web/src/components/page-templates/workbench/useWorkbenchLeftPanelController.ts`
- `apps/erp-web/src/components/page-templates/workbench/useWorkbenchInteractionController.ts`
- `apps/erp-web/src/components/page-templates/workbench/workbenchMainGridBridge.ts`

Likely validation-page migration files:

- `apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- `apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx`
- `apps/erp-web/src/pages/orders/shared/useLeftFilterModule.tsx`
- `apps/erp-web/src/pages/orders/shared/OrderWorkbenchMainGridEntry.tsx`
- `apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx`
- `apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.tsx`

## Risks

### Risk 1: Shell Bloat

If too much coordination logic is embedded directly inside `WorkbenchPageTemplate`, the public shell becomes fragile and hard to evolve.

Mitigation:

- keep shell ownership focused on composition
- place reusable behavior in shared controllers / hooks
- expose only stable public props

### Risk 2: False Standardization

If left-panel, selection, shortcut, and context-menu behavior stay page-local while the public API only changes names, the project will appear standardized without actually reducing duplication.

Mitigation:

- migrate real lifecycle logic into shared modules
- verify that validation pages stop using direct low-level mounting patterns

### Risk 3: Over-Abstracting Business Semantics

If business actions are pushed into the template in the name of standardization, the workbench will become domain-heavy and harder to reuse.

Mitigation:

- keep business actions page-owned
- standardize orchestration, not business semantics

## Validation Strategy

Validation should prove both public-template ergonomics and behavioral correctness.

### Unit-Level Validation

- controller tests for selection / focus / active-row lifecycle
- controller tests for left-panel lifecycle
- controller tests for shortcut lifecycle guard behavior
- controller tests for context-menu lifecycle
- controller tests for saved-view hydration / serialization

### Page-Level Validation

- order review still supports current selection, focus, left-panel, detail, and overlay behaviors
- order recheck still supports current review / exception workflow wiring
- both pages consume the unified template API without manual panel-store orchestration

### Regression Priorities

- no loss of current workbench pagination / footer behavior
- no regression in current main-grid interaction behavior
- no regression in saved-view continuity
- no regression in keyboard handling protections for editable targets

## Recommended Rollout

1. Extend `WorkbenchPageTemplate` public API to formally support left / right / bottom panel composition and workbench interaction configuration.
2. Introduce shared internal workbench contracts and controllers.
3. Migrate order review to the new standard.
4. Migrate order recheck to the same standard.
5. Verify both pages and lock the API for future workbench implementations.
6. Use this unified template as the only approved path for new ERP workbench pages.

## Success Criteria

This design is successful when all of the following are true:

- future page authors only need to learn `WorkbenchPageTemplate` as the workbench page entry
- order review and order recheck no longer hand-roll repeated workbench lifecycle logic
- workbench panel mounting is standardized across left / right / bottom regions
- reusable workbench behavior is implemented once in shared controllers rather than copied across pages
- business actions remain page-owned
- the ERP gains a credible enterprise-grade workbench platform standard for future expansion
