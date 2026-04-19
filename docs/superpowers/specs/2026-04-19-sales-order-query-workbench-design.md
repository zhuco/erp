# Sales Order Query Workbench Design

**Date:** 2026-04-19

**Goal**

Build a sales-facing order query workbench that uses the unified `WorkbenchPageTemplate` standard, integrates into the sales menu, and adopts the same structural language as order review without carrying review / split / merge business actions.

## Background

The ERP already has:

- a unified workbench-template direction centered on `WorkbenchPageTemplate`
- validation-oriented workbench pages such as order review and order recheck
- older order-query pages and sales-side query pages with partially reusable logic

However, the current sales-side query experience is not yet aligned to the unified workbench standard:

- there is no dedicated sales-menu “order query” entry in the main menu
- the existing sales order query page still follows older panel-mounting and page-composition patterns
- the existing generic order query page is functionally heavier and semantically closer to the order center than to a sales workbench

The product direction is a mid-to-high-end enterprise e-commerce ERP. For that direction, the sales query page should:

- feel like the same product family as order review
- keep the same workbench shell grammar
- remain role-appropriate for sales users
- avoid turning the sales entry into another validation or exception-handling console

## Product Decision

The project will create a new sales-facing order query page built directly on the unified workbench template standard.

This page will:

- live under the sales menu
- reuse the unified workbench shell structure
- visually and structurally reference the order review workbench
- deliberately omit review / split / merge / recheck business actions

This page will not be positioned as:

- a clone of order review
- a catch-all order center page
- a validation or fulfillment-operation console

Its product identity is:

- sales query workbench
- fast lookup
- structured filtering
- detail confirmation
- export / print / lightweight desk operations

## Scope

This design covers:

- the sales-menu “order query” entry
- the route and page ownership
- the workbench-template composition of the page
- the first-release feature set
- the relationship to existing query pages
- rollout and testing expectations

This design does not cover:

- adding review / merge / split actions to the sales page
- replacing every historical query page in one pass
- introducing a new public template beyond `WorkbenchPageTemplate`
- building advanced role-based personalization in the first release
- redesigning the entire order query domain model

## Core Principles

### 1. Same Shell, Different Role

The sales order query page should feel immediately familiar to users who know order review:

- left-side workbench navigation / filtering
- central table
- right-side detail panel
- bottom supplemental panel
- standard toolbar and footer rhythm

But it must remain clearly different in business posture:

- query-oriented
- confirmation-oriented
- low-risk desk operations only

### 2. Business Actions Stay Page-Scoped

The unified template remains structural, not business-specific.

For this page, the template should provide:

- toolbar placement
- left panel mounting
- main table region
- right panel region
- bottom panel region
- overlays
- pagination / footer summary

The page itself should own any sales-specific interactions such as:

- export
- print
- copy order information
- open detail
- optional light mark / note actions if enabled later

### 3. Sales First-Release Must Stay Focused

The first release should prioritize the most frequently used sales query loop:

1. find the order
2. narrow the list quickly
3. inspect the detail
4. export or copy information if needed

The first release should avoid bloating into:

- fulfillment operation tools
- exception disposal tools
- bulk processing consoles

### 4. Reuse Existing Domain Logic Carefully

Existing data adapters, row rendering helpers, and query logic may be reused where practical.

However, the new page should not inherit outdated shell wiring or page-local workbench runtime logic from older pages. Reuse should happen at the data / field / helper level, not at the historical layout-pattern level.

## Information Architecture

The sales menu will be extended within the existing sales group.

Recommended location:

- Sales
- Order Center
- Order Query

Recommended route:

- `/sales/order-query`

This route should be guarded with the same read-level permission family already used for order reading, unless later product policy introduces a distinct sales-query permission.

## Page Positioning

The page is the sales team’s operational query desk for online orders.

Primary scenarios:

- query by order number / platform order number / tracking number / receiver
- inspect current order status and fulfillment progress
- confirm order goods, amount, shop, platform, logistics, and notes
- export current result set
- print current list
- copy useful order information for downstream coordination

Non-goals for first release:

- approve / reject order
- merge orders
- split orders
- process exceptions
- submit fulfillment actions

## Workbench Composition

The page will use `WorkbenchPageTemplate` as the single page shell.

### A. Toolbar

The toolbar should be query-centric rather than action-centric.

Recommended first-release toolbar content:

- keyword search
- date range
- platform selector
- shop selector
- order-status selector
- fulfillment-status selector
- export
- print
- refresh

The toolbar should not include:

- review
- merge
- split
- exception handling

### B. Left Panel

The left panel should keep the workbench-navigation character of order review, but use sales-friendly filters.

Recommended left-panel groups:

- platform
- shop
- order status
- fulfillment status
- logistics status
- refund / after-sales related grouping
- order marks / tags

The left panel should be mounted through the standard `leftPanel` input of `WorkbenchPageTemplate`, not by directly manipulating panel stores.

### C. Main Table

The main table should feel dense, efficient, and enterprise-grade, with fields optimized for sales lookup.

Recommended first-release columns:

- order number
- platform order number
- platform
- shop
- receiver name
- receiver phone
- goods summary
- order time
- order status
- fulfillment status
- logistics company
- tracking number
- amount
- marks / tags

The table should preserve the unified workbench behavior for:

- row selection
- focus
- active row
- right detail synchronization
- pagination footer rhythm

### D. Right Detail Panel

The right panel is essential for the sales workbench because many query tasks end in confirmation rather than mutation.

Recommended first-release sections:

- order basic information
- receiver information
- shop / platform information
- logistics summary
- mark / note summary
- amount summary

The right panel should use the same template-driven detail rhythm already established in the workbench family, even if the exact fields differ from order review.

### E. Bottom Panel

The bottom panel should remain part of the standard workbench composition, but be lighter than the validation pages.

Recommended first-release tabs or sections:

- goods details
- package / shipment summary
- order logs or remarks

This preserves layout consistency while keeping the business posture query-first.

## Interaction Model

### Included in First Release

- search and filter
- row selection
- active-row driven detail display
- export current list
- print current list
- copy order number or tracking information from context actions

### Excluded from First Release

- review shortcuts
- merge / split shortcuts
- validation / exception context actions
- editing-heavy flows as default page behavior

### Optional Lightweight Actions

The page may later introduce controlled lightweight desk actions, but they are not required for the first release:

- add / remove marks
- add internal note
- copy receiver info

If introduced later, they must remain page-owned business actions and should not change the template contract.

## Relationship to Existing Pages

### Existing `OrderQueryPage`

`OrderQueryPage` should not be used as the public template for the sales query page. It can remain a source of:

- field choices
- query logic
- helper functions
- data adapters

But its historical page shape should not define the new standard.

### Existing `ShopOrderQueryPage`

`ShopOrderQueryPage` is closer to the sales domain and may contribute:

- sales-side terminology
- filter dimensions
- column candidates
- query behavior patterns

But it should not be directly promoted as-is. The new sales query workbench should either:

- supersede it as the new standard entry, or
- reuse selected logic while establishing a cleaner template-based implementation

Recommended direction:

- create the new unified-template sales query page
- point the sales menu to that page
- treat older sales query code as migration material, not as the final shell

## Implementation Shape

Recommended implementation structure:

- create a dedicated sales workbench page under the sales pages area
- compose it directly with `WorkbenchPageTemplate`
- reuse shared workbench controllers for left panel, selection, interaction, and view-state behavior
- reuse existing order-query domain helpers only where they fit the new structure cleanly

Recommended page ownership:

- route and sales-menu integration
- sales toolbar configuration
- sales left-panel node mapping
- sales table column set
- sales detail-field mapping
- export / print / copy actions

Recommended shared ownership:

- workbench shell layout
- panel lifecycle
- selection / focus / active-row contracts
- saved-view behavior
- keyboard / context-menu lifecycle infrastructure where relevant

## First-Release Success Criteria

The first release is successful when:

- the sales menu contains a dedicated “Order Query” entry
- the page uses the unified `WorkbenchPageTemplate` standard
- the page visually aligns with the workbench family
- the page does not expose review / merge / split actions
- sales users can search, filter, inspect details, and export from one workbench flow
- the implementation can serve as a template reference for future sales-side workbench pages

## Testing Expectations

Testing should cover:

- route registration and sales-menu visibility
- `WorkbenchPageTemplate` adoption
- left-panel mounting through `leftPanel`
- main-table rendering with sales-centric columns
- right-panel detail synchronization from active row
- export / print availability
- absence of review / merge / split action wiring

Where appropriate, adoption tests should verify the page uses the shared workbench contracts rather than direct page-local panel-store wiring.

## Recommendation

The correct route for the ERP is to build a new sales-facing order query workbench on top of the unified `WorkbenchPageTemplate` standard and integrate it into the sales menu as a dedicated sales order-center entry.

This gives the product:

- a consistent enterprise workbench family
- clear role separation
- a reusable implementation standard for future pages
- a cleaner long-term foundation than retrofitting the older query pages in place
