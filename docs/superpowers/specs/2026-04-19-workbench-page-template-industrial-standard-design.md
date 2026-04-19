# Workbench Page Template Industrial Standard Design

**Date:** 2026-04-19

**Goal**

Build an industrial-grade `WorkbenchPageTemplate` standard for all future data-table workbench pages, then validate that standard on the current order review and order recheck pages before adding new business pages.

## Scope

This design covers the shared workbench standard for pages with a main data table. It includes:

- unified workbench main-grid behavior
- locked system-column policy
- double-click record editing
- mock persistence contract for template-level save flows
- simulated order record rules
- universal column filtering
- order approval guard rules tied to goods lines

This design does not include:

- restoring hidden pages
- replacing mock persistence with real backend persistence
- redesigning the visual theme of the entire ERP
- refactoring every historical page to the new workbench standard in one pass

## Product Decision

The project will maintain two top-level page template families:

- `WorkbenchPageTemplate` for table-centric workbench pages
- `SettingsPageTemplate` for settings-centric pages

This work focuses on the first family only.

The workbench standard will not be implemented as a single giant component with business logic hardcoded inside. Instead, it will be split into a stable template shell plus a standard contract for main-grid behavior and record editing.

## Core Design

### 1. Template Boundary

`WorkbenchPageTemplate` remains the top-level page shell and continues to own:

- toolbar placement
- main content shell
- right panel mounting
- bottom panel mounting
- overlay mounting
- footer/pagination placement

The workbench standard is extended through an adjacent contract layer rather than by overloading the shell component itself. The shell stays layout-focused; interaction standards are supplied by dedicated workbench modules.

### 2. Standard Main-Grid Contract

All future workbench pages with a main data table should be able to plug into one standard main-grid contract with these guarantees:

- every business data column has a column filter
- the system sequence column cannot be sorted
- users cannot restore sort on the system sequence column through menu/state recovery
- double-clicking a data row opens a record editor dialog
- save flows use a template-level persistence contract rather than page-local ad hoc logic

This contract will be first validated on order review and order recheck.

### 3. Standard Record Editor Contract

The editor pattern will follow the approved hybrid model:

- the template layer provides a built-in base editor framework
- pages can inject extension fields
- pages can inject an optional detail subtable editor area

That means the common dialog owns:

- dialog lifecycle
- loading/saving/error state
- base field rendering contract
- validation entry points
- save orchestration

Pages own:

- business-specific field schema
- custom extension field sections
- optional detail-row editor section
- business-specific validation extensions
- persistence adapter implementation

### 4. Persistence Strategy

The approved strategy is:

- define a standard save interface at the workbench level
- validate the flow using mock persistence first
- keep the persistence contract backend-replaceable

The first implementation will save order edits to the current mock order source and maintain a realistic end-to-end edit flow without coupling the template standard to one specific backend endpoint.

## Functional Rules

### Rule A: Sequence Column

The main-grid sequence column is a system column. It must:

- render as display-only
- never be sortable
- ignore user attempts to restore sort from persisted column state
- avoid showing misleading sort affordances

This applies to every page using the standard workbench main grid.

### Rule B: Universal Column Filtering

Every business data column in the main grid must support filtering.

Expected defaults:

- text-like columns use text filter
- enumerations/status columns use set filter
- numeric columns use number filter
- dates/time columns use date filter

System columns may be exempt where filtering is meaningless, but business columns are not.

### Rule C: Double-Click Editing

Every row in the main data area must open an editor dialog on double click.

The common behavior will be:

- row double-click opens the workbench editor
- the dialog loads the selected record draft
- save runs validation
- save persists through the standard persistence adapter
- successful save updates the table data source

### Rule D: Simulated Order Goods

Simulated order data must always model goods explicitly through an item collection.

Orders should use a structure equivalent to:

- `order_no`
- primary order fields
- `items`
- derived goods count

The goods count must be derived from the line items rather than manually maintained as an independent truth source.

### Rule E: Approval Guard

Orders without goods are not reviewable.

The workbench standard should allow record-level validation rules to block approval and return a user-readable reason. The order implementation must use that hook to prevent review when:

- `items` is empty

### Rule F: Order Number Generation

Order number generation must follow this rule:

- prefix `HC`
- followed by current date
- followed by a globally incrementing per-day serial number
- default display width starts at 4 digits
- when the sequence exceeds `9999`, it expands in width rather than resetting

The scope is per-day global uniqueness, not per-page uniqueness.

## Architecture

### New/Adjusted Building Blocks

#### `WorkbenchPageTemplate`

Keeps layout ownership only and remains the public shell API.

#### `WorkbenchMainGridPolicy`

New policy layer responsible for:

- auto-applying per-column filter defaults
- locking system-column sort behavior
- normalizing column definitions before grid render
- enforcing workbench-wide defaults consistently

#### `WorkbenchRecordEditorDialog`

New shared dialog component responsible for:

- opening/closing state presentation
- rendering base fields
- rendering extension field slot
- rendering optional detail editor slot
- save/cancel state handling
- validation feedback presentation

#### `useWorkbenchRecordEditor`

New shared hook responsible for:

- active record state
- draft state
- reset/open/close behavior
- calling validate/save adapters

#### `WorkbenchPersistenceAdapter`

Template-level contract defining:

- how a record is saved
- how an updated record is merged back
- how save failures are surfaced

#### `Order Mock Governance Utilities`

Order-specific mock support will be extracted into dedicated utilities for:

- order number generation
- simulated item generation
- derived goods-count calculation
- approval guard helpers

## File Strategy

Likely file targets for this work:

- `apps/erp-web/src/components/page-templates/workbench/WorkbenchPageTemplate.tsx`
- `apps/erp-web/src/components/page-templates/workbench/index.ts`
- `apps/erp-web/src/pages/orders/shared/OrderWorkbenchMainGridEntry.tsx`
- `apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx`
- `apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.tsx`
- `apps/erp-web/src/api/mocks/orders.ts`

Likely new files:

- `apps/erp-web/src/components/page-templates/workbench/WorkbenchRecordEditorDialog.tsx`
- `apps/erp-web/src/components/page-templates/workbench/useWorkbenchRecordEditor.ts`
- `apps/erp-web/src/components/page-templates/workbench/workbenchGridColumnPolicy.ts`
- `apps/erp-web/src/components/page-templates/workbench/types.ts`
- `apps/erp-web/src/pages/orders/shared/orderMockGovernance.ts`
- tests for dialog behavior, column policy, mock numbering, and approval guards

## Validation Strategy

Validation should be layered:

- unit tests for order number generation and goods-count derivation
- unit tests for approval guard logic
- unit tests for column policy normalization
- unit tests for editor save orchestration
- focused page tests for review/recheck double-click editor behavior

The first working acceptance target is:

- order review page uses the standard dialog flow
- all business columns expose filters
- sequence column is locked against sorting
- simulated orders have goods
- approval is blocked when goods are missing
- saves round-trip through mock persistence and refresh the visible record

## Risks

### Risk 1: Overloading the Template Shell

If too much business logic is placed inside `WorkbenchPageTemplate`, the template becomes order-specific again.

Mitigation:

- keep the shell layout-focused
- move behavior into dialog/policy/adapters

### Risk 2: Breaking Existing Grid Behavior

The order review grid already has significant selection, focus, and filter behavior.

Mitigation:

- treat column policy and editor lifecycle as additive layers
- avoid rewriting the entire grid at once
- verify existing focused tests after each slice

### Risk 3: Mock Persistence Diverging from Future Backend Contracts

Mitigation:

- define save interfaces in a backend-neutral shape
- keep mock persistence behind an adapter boundary

## Recommended Rollout

1. Introduce the workbench policy/editor primitives.
2. Apply them to order review.
3. Apply them to order recheck.
4. Confirm the workbench standard is stable.
5. Only then begin reconnecting new pages to the workbench standard.
