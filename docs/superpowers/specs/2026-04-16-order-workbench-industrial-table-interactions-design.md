# Order Workbench Industrial Table Interactions Design

## Scope

This design upgrades the order workbench table interactions for `/orders/review` and `/orders/recheck`, while synchronizing the shared workbench template behavior used by order pages.

The work covers four agreed behavior changes:

1. When a left-panel module filter is applied, any active main-grid column filter effect must be cleared.
2. When users switch away from a workbench tab and come back, the page must restore its own focus state, selection state, left filters, and column filter state.
3. The main-grid selection checkbox column must be centered in the data area.
4. Default workbench columns must better reflect ERP list density and operational context.

## Current Architecture

The order workbench is not based on the generic `ServerTable` path. It uses a dedicated workbench stack:

- `WorkbenchMainTableTemplate` for layout
- `useWorkbenchSavedViewState` for per-page local persistence
- `useLeftFilterModule` for left-panel tree filtering
- `OrderReviewMainGrid` and `OrderExceptionsMainGrid` for AG Grid main tables
- `agGridWorkbenchStructuralColumns.tsx` for shared selection/index structural columns

This means the required fixes should be implemented in the order workbench shared layer first, then connected into the review and recheck pages.

## Root Causes

### 1. Left filter does not reliably clear column filter state

The left filter workflow and the main-grid column-filter workflow are stored separately for `/orders/recheck`, and partially overlap for `/orders/review`.

The clearing behavior currently happens too late or only in page-level handlers, so it is easy for left tree interactions to coexist with stale header-filter state.

### 2. Focus and selection are not part of saved view state

`useWorkbenchSavedViewState` currently persists only the query/filter view state that each page chooses to serialize. Neither `OrderReviewPage` nor `OrderExceptionsPage` serializes the current focused row, selected rows, or active row identity.

As a result, tab switching restores filters but loses operational positioning.

### 3. Selection column alignment is defined in shared structural columns

The selection checkbox column is produced by `agGridWorkbenchStructuralColumns.tsx`. The body alignment problem is therefore a shared structural-column issue, not a page-specific issue.

### 4. Default business columns are too thin for ERP operations

`OrderReviewMainGrid` currently exposes a compact but low-context column set. `OrderExceptionsMainGrid` exposes exception-oriented columns, but it still misses some high-value operational columns that already exist in the summary response.

At the same time, `/orders/recheck` cannot safely display columns like tags, item summary, or logistics company from the current exception summary API, because those fields are not available in `OrderExceptionSummaryResponse`.

## Approved Solution

### Shared Interaction Rules

The shared workbench interaction model should enforce:

- Left-panel leaf filter selection clears active main-grid column filters before the new left filter is applied.
- View persistence includes focus/selection context in addition to filter context.
- Shared structural selection column styles enforce centered header and body alignment.

### `/orders/review` State Model

`OrderReviewPage` should persist and restore:

- `searchInput`
- `searchKeyword`
- `quickFilterExpanded`
- `columnFilters`
- `leftScreenFiltersDraft`
- `leftScreenFiltersApplied`
- `selectedRowKeys`
- `focusedOrderKey`
- `selectionAnchorKey`
- `activeOrderKey`

On hydration, keys must be restored first, then reconciled against loaded rows so invalid keys are dropped without clearing valid ones.

### `/orders/recheck` State Model

`OrderExceptionsPage` should persist and restore:

- `keywordInput`
- `searchKeyword`
- `quickFilterExpanded`
- `columnFilters`
- `leftFilters`
- `selectedRowKeys`
- `focusedOrderKey`
- `activeOrderId`

On hydration, keys must be reconciled against the current display rows after data is available.

### Column Expansion Policy

#### `/orders/review`

Upgrade the default grid columns to better match ERP operations by including the following high-value business context in the main table:

- structural selection/index columns
- marker / exception indicator column
- order number
- platform order number
- platform
- shop
- created time
- warehouse
- logistics company
- receiver
- phone
- amount
- status / review context
- tracking number

This should remain within the summary data already available in `OrderSummary`.

#### `/orders/recheck`

Upgrade the default grid columns only with fields available in `OrderExceptionSummaryResponse`, such as:

- structural selection/index columns
- order number
- platform
- shop
- receiver
- amount
- audit status
- exception status
- fulfillment status
- reserve status
- target warehouse
- assignee
- latest action
- latest action time
- SLA status
- SLA remaining
- blocking reason

This avoids introducing detail-fetch N+1 behavior just to fill table columns.

## Testing Strategy

The implementation will follow TDD with targeted tests for:

1. left-filter entry behavior that clears active column-filter state
2. saved-view persistence that now includes focus and selection state
3. shared structural column configuration and CSS classes for centered checkbox cells
4. review/recheck main-grid column definitions containing the new agreed business columns

## Out of Scope

- Backend schema changes for richer exception summary payloads
- Per-row detail prefetch solely to render additional table columns
- Generic `ServerTable` or non-order workbench pages outside the shared order workbench behavior touched here
