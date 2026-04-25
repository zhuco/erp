# Workbench Sequence Lock Design

**Date:** 2026-04-19

**Goal**

Extend the `WorkbenchPageTemplate` standard so the main-grid sequence column keeps a stable fixed number for each row in the current loaded dataset, even when users sort by other business columns.

## Scope

This design covers:

- fixed sequence numbering for workbench main-grid rows
- sequence-column behavior under business-column sorting
- sequence-column behavior under saved column-state restore
- a shared workbench implementation path instead of page-local fixes

This design does not cover:

- replacing client-side sorting with backend sorting
- cross-refresh persistence of the fixed sequence numbering
- changing pagination behavior beyond the existing current-page offset rules
- unrelated table interaction or layout changes

## Product Decision

The sequence column in `WorkbenchPageTemplate` represents the stable original row number of the current loaded dataset, not the row's current visible position after sorting.

That means:

- when the grid first renders a page of rows, the sequence column shows `1, 2, 3 ...`
- when users sort by any business column, the row order changes
- the sequence number attached to each record must remain the same as its original number in that loaded dataset

The sequence column remains a structural system column and never participates in user-controlled sorting.

## Core Rules

### 1. Stable Sequence Identity

Each main-grid row must receive a stable internal source sequence value when the workbench data adapter first prepares the row data for rendering.

This value must:

- be assigned once per loaded dataset
- remain attached to that record while users change client-side sorting
- be used as the only source of truth for the sequence column display

The visible sequence value is the stable source sequence plus the existing row-index offset contract when offsets are enabled.

### 2. Sorting Behavior

The sequence column must stay locked under all sorting scenarios.

Required behavior:

- users cannot click the sequence header to sort
- business-column sorting must not recalculate sequence numbers by visible row position
- restoring saved column state must not reapply sort to the sequence column
- column-settings UI must not treat the sequence column as a sortable business column

### 3. Shared Standard Boundary

This behavior must live in the shared workbench standard layer, not inside individual order pages.

The standard implementation should be owned by the shared workbench and grid infrastructure that already manages structural columns.

Pages should continue to consume the sequence column as a standard structural column without custom page logic.

## Architecture

### Stable Row Metadata

The shared row-adaptation path should attach a new stable row metadata field such as `__erpSourceRowIndex` to each prepared row.

This field represents the original zero-based position of the row within the currently loaded dataset.

It should be assigned before AG Grid applies client-side sorting so that later visible row-position changes do not affect it.

### Structural Sequence Column

The structural sequence column definition should read from the stable source row index first.

Its `valueGetter` should display:

- `__erpSourceRowIndex + rowIndexOffset + 1`

Its comparator should compare:

- `__erpSourceRowIndex`

The sequence column must never fall back to current visible row index when stable source metadata is available.

### Column-State Sanitization

The existing workbench column-state sanitization rule should continue stripping sort metadata from the sequence column.

No persisted or restored state may re-enable sequence sorting.

## Acceptance Criteria

The design is complete when:

- the first render of a workbench page shows sequential values such as `1, 2, 3 ...`
- sorting by any business column changes row order but does not change each record's displayed sequence number
- the sequence column itself remains unsortable
- restoring saved column state does not reintroduce sequence sorting
- the change is implemented in shared workbench/grid standard code rather than duplicated in business pages

## Testing Strategy

Before implementation, add or update tests to prove:

- shared row adaptation stores a stable source row index
- the structural sequence column displays the stable source row index instead of current visible row index
- the structural sequence comparator compares stable source row indexes
- workbench column-state sanitization still strips sequence sort metadata
- existing business-column sorting behavior remains intact
