import {
  AllCommunityModule,
  CellSelectionModule,
  ColumnsToolPanelModule,
  FiltersToolPanelModule,
  MenuModule,
  PivotModule,
  RowGroupingModule,
  SetFilterModule,
  SideBarModule,
  StatusBarModule,
} from 'ag-grid-enterprise';

export const ERP_DATA_GRID_ENTERPRISE_MODULES = [
  AllCommunityModule,
  MenuModule,
  SetFilterModule,
  SideBarModule,
  ColumnsToolPanelModule,
  FiltersToolPanelModule,
  StatusBarModule,
  CellSelectionModule,
] as const;

export const ORDER_WORKBENCH_ENTERPRISE_MODULES = [
  AllCommunityModule,
  MenuModule,
  SetFilterModule,
  CellSelectionModule,
] as const;

export const ENTERPRISE_DEMO_MODULES = [
  AllCommunityModule,
  MenuModule,
  SetFilterModule,
  CellSelectionModule,
  SideBarModule,
  ColumnsToolPanelModule,
  FiltersToolPanelModule,
  StatusBarModule,
  RowGroupingModule,
  PivotModule,
] as const;
