import { Alert, Empty, Spin, Tag } from 'antd';
import dayjs from 'dayjs';
import {
  type CellFocusedEvent,
  type ColDef,
  type GridApi,
  type GridReadyEvent,
  type ICellRendererParams,
  type SelectionChangedEvent,
  type ValueFormatterParams,
  type ValueGetterParams,
} from 'ag-grid-community';
import { ModuleRegistry } from 'ag-grid-community';
import { AgGridReact } from 'ag-grid-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import type { ShopProductSku } from '@/api/shopProducts';
import { ORDER_WORKBENCH_ENTERPRISE_MODULES } from '@/components/agGridEnterpriseModules';
import { ERP_AG_GRID_LOCALE_TEXT_ZH_CN } from '@/components/agGridColumnState';
import { PlatformTag } from '@/components/PlatformTag';
import { buildOrderReviewMainGridDefaultColDef } from '@/pages/orders/review-native/OrderReviewMainGrid';
import { buildAgGridWorkbenchStructuralColDefs } from '@/pages/orders/shared/agGridWorkbenchStructuralColumns';
import { toggleAgGridHeaderSelection } from '@/pages/orders/shared/agGridSelectionHelpers';
import { useAgGridWorkbenchSelection } from '@/pages/orders/shared/useAgGridWorkbenchSelection';

ModuleRegistry.registerModules([...ORDER_WORKBENCH_ENTERPRISE_MODULES]);

type ShopProductWorkbenchMainTableProps = {
  rows: ShopProductSku[];
  loading?: boolean;
  error?: unknown;
  selectedRowKeys: string[];
  activeSkuRecordId?: string | null;
  shopNameResolver: (shopId: string) => string;
  onRetry: () => void;
  onSelectionChange: (keys: string[], rows: ShopProductSku[]) => void;
  onActiveRowChange: (skuRecordId: string) => void;
};

const toDate = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');
const toMoney = (value?: number | null) => (value == null ? '-' : (value / 100).toFixed(2));

const renderMatchTag = (row: ShopProductSku) => {
  if (row.mapping_status === 'CONFLICT') {
    return <Tag color="red">CONFLICT</Tag>;
  }
  if (!row.mapping_id) {
    return <Tag>UNMATCHED</Tag>;
  }
  if (row.match_source === 'MANUAL') {
    return <Tag color="blue">MANUAL</Tag>;
  }
  if (row.match_source === 'AUTO') {
    return <Tag color="green">AUTO</Tag>;
  }
  return <Tag color="purple">MATCHED</Tag>;
};

const buildBusinessColDefs = (
  shopNameResolver: (shopId: string) => string,
): ColDef<ShopProductSku>[] => [
  {
    colId: 'platform',
    field: 'platform',
    headerName: '平台',
    width: 90,
    minWidth: 72,
    cellRenderer: (params: ICellRendererParams<ShopProductSku>) =>
      params.data?.platform ? <PlatformTag platform={params.data.platform} /> : '-',
  },
  {
    colId: 'shop_id',
    field: 'shop_id',
    headerName: '店铺',
    width: 170,
    minWidth: 120,
    valueFormatter: (params: ValueFormatterParams<ShopProductSku>) => shopNameResolver(String(params.value ?? '')),
  },
  {
    colId: 'platform_sku_id',
    field: 'platform_sku_id',
    headerName: '平台 SKU',
    width: 160,
    minWidth: 120,
  },
  {
    colId: 'spec_text',
    field: 'spec_text',
    headerName: '平台 SKU 名称',
    width: 220,
    minWidth: 160,
    valueGetter: (params: ValueGetterParams<ShopProductSku>) =>
      params.data?.spec_text ?? params.data?.sku_title ?? params.data?.product_title ?? '',
  },
  {
    colId: 'merchant_sku_code',
    field: 'merchant_sku_code',
    headerName: '商家 SKU',
    width: 160,
    minWidth: 120,
    valueGetter: (params: ValueGetterParams<ShopProductSku>) =>
      params.data?.merchant_sku_code ?? params.data?.platform_sku_id ?? '',
  },
  {
    colId: 'mapped_sku_code',
    field: 'mapped_sku_code',
    headerName: 'ERP SKU',
    width: 220,
    minWidth: 160,
    valueGetter: (params: ValueGetterParams<ShopProductSku>) =>
      params.data?.mapped_sku_code ? `${params.data.mapped_sku_code} ${params.data.mapped_sku_name ?? ''}` : '-',
  },
  {
    colId: 'match_source',
    field: 'match_source',
    headerName: '匹配结果',
    width: 120,
    minWidth: 96,
    cellRenderer: (params: ICellRendererParams<ShopProductSku>) =>
      params.data ? renderMatchTag(params.data) : null,
  },
  {
    colId: 'conflict_reason',
    field: 'conflict_reason',
    headerName: '冲突原因',
    width: 160,
    minWidth: 120,
    valueFormatter: (params: ValueFormatterParams<ShopProductSku>) => String(params.value ?? '-'),
  },
  {
    colId: 'price_cent',
    field: 'price_cent',
    headerName: '价格',
    width: 100,
    minWidth: 84,
    cellStyle: { textAlign: 'right' },
    valueFormatter: (params: ValueFormatterParams<ShopProductSku>) =>
      toMoney((params.value as number | null | undefined) ?? null),
  },
  {
    colId: 'platform_updated_at',
    field: 'platform_updated_at',
    headerName: '平台更新时间',
    width: 160,
    minWidth: 136,
    valueGetter: (params: ValueGetterParams<ShopProductSku>) =>
      params.data?.platform_updated_at ?? params.data?.updated_at ?? '',
    valueFormatter: (params: ValueFormatterParams<ShopProductSku>) =>
      toDate((params.value as string | null | undefined) ?? null),
  },
];

export const ShopProductWorkbenchMainTable = ({
  rows,
  loading,
  error,
  selectedRowKeys,
  activeSkuRecordId,
  shopNameResolver,
  onRetry,
  onSelectionChange,
  onActiveRowChange,
}: ShopProductWorkbenchMainTableProps) => {
  const shellRef = useRef<HTMLDivElement | null>(null);
  const apiRef = useRef<GridApi<ShopProductSku> | null>(null);
  const syncingSelectionRef = useRef(false);
  const [gridApi, setGridApi] = useState<GridApi<ShopProductSku> | null>(null);

  const selectedKeySet = useMemo(() => new Set(selectedRowKeys.map((key) => String(key))), [selectedRowKeys]);
  const selectedInGridCount = useMemo(
    () => rows.reduce((count, row) => (selectedKeySet.has(String(row.sku_record_id)) ? count + 1 : count), 0),
    [rows, selectedKeySet],
  );

  const emitSelectionKeys = useCallback(
    (api: GridApi<ShopProductSku>) => {
      const selectedNodes = api.getSelectedNodes();
      const keys = selectedNodes
        .map((node) => String(node.data?.sku_record_id ?? ''))
        .filter(Boolean);
      const selectedRows = selectedNodes.map((node) => node.data).filter((row): row is ShopProductSku => row != null);
      onSelectionChange(keys, selectedRows);
    },
    [onSelectionChange],
  );

  const {
    handleCellClicked,
    handleCellFocused,
    handleCellKeyDown,
    handleGridMouseDownCapture,
    handleRowClicked,
    handleSpaceKeyDownCapture,
  } = useAgGridWorkbenchSelection<ShopProductSku>({
    apiRef,
    debugKey: 'shop-products-grid',
    syncingSelectionRef,
    focusedRowKey: activeSkuRecordId ?? null,
    resolveRowKey: (record) => String(record.sku_record_id ?? '').trim() || null,
    emitSelectionKeys,
    onRowActive: (record) => {
      onActiveRowChange(record.sku_record_id);
    },
    onFocus: (record) => {
      onActiveRowChange(record.sku_record_id);
    },
  });

  const refreshSelectionChrome = useCallback((api?: GridApi<ShopProductSku> | null) => {
    const targetApi = api ?? apiRef.current;
    if (!targetApi) {
      return;
    }
    targetApi.refreshHeader();
    targetApi.refreshCells({
      force: true,
      columns: ['__erpSelection'],
    });
  }, []);

  const colDefs = useMemo<ColDef<ShopProductSku>[]>(
    () => [
      ...buildAgGridWorkbenchStructuralColDefs<ShopProductSku>({
        onToggleAllSelection: () => {
          const api = apiRef.current;
          if (!api) {
            return;
          }

          syncingSelectionRef.current = true;
          toggleAgGridHeaderSelection({
            headerChecked: rows.length > 0 && selectedInGridCount === rows.length,
            deselectAll: () => api.deselectAll(),
            forEachNodeAfterFilter: (callback) => {
              api.forEachNodeAfterFilter(callback);
            },
          });
          syncingSelectionRef.current = false;
          emitSelectionKeys(api);
          refreshSelectionChrome(api);
        },
      }),
      ...buildBusinessColDefs(shopNameResolver),
    ],
    [emitSelectionKeys, refreshSelectionChrome, rows.length, selectedInGridCount, shopNameResolver],
  );

  const defaultColDef = useMemo<ColDef<ShopProductSku>>(
    () => buildOrderReviewMainGridDefaultColDef(false),
    [],
  );

  useEffect(() => {
    const api = gridApi ?? apiRef.current;
    if (!api) {
      return;
    }

    syncingSelectionRef.current = true;
    api.forEachNode((node) => {
      const rowKey = String(node.data?.sku_record_id ?? '');
      const shouldSelect = Boolean(rowKey) && selectedKeySet.has(rowKey);
      if (node.isSelected() !== shouldSelect) {
        node.setSelected(shouldSelect);
      }
    });
    syncingSelectionRef.current = false;
    refreshSelectionChrome(api);
  }, [gridApi, refreshSelectionChrome, selectedKeySet, rows]);

  useEffect(() => {
    if (!gridApi) {
      return;
    }
    gridApi.redrawRows();
  }, [activeSkuRecordId, gridApi, rows]);

  const handleGridReady = useCallback((event: GridReadyEvent<ShopProductSku>) => {
    apiRef.current = event.api;
    setGridApi(event.api);
    refreshSelectionChrome(event.api);
  }, [refreshSelectionChrome]);

  const handleGridSelectionChanged = useCallback(
    (event: SelectionChangedEvent<ShopProductSku>) => {
      if (syncingSelectionRef.current) {
        return;
      }
      emitSelectionKeys(event.api);
      refreshSelectionChrome(event.api);
    },
    [emitSelectionKeys, refreshSelectionChrome],
  );

  const handleGridCellFocused = useCallback(
    (event: CellFocusedEvent<ShopProductSku>) => {
      handleCellFocused(event);
    },
    [handleCellFocused],
  );

  return (
    <div className="shop-product-workbench-table order-review-grid-main" data-testid="shop-products-main-table">
      <div
        ref={shellRef}
        className="ag-theme-quartz order-review-main-grid-shell shop-products-grid-shell"
        data-testid="shop-products-grid"
        onKeyDownCapture={handleSpaceKeyDownCapture}
        onMouseDownCapture={handleGridMouseDownCapture}
      >
        {error ? (
          <div className="order-review-main-grid-state">
            <Alert
              type="error"
              showIcon
              message="网店商品加载失败"
              description={
                <a
                  onClick={(event) => {
                    event.preventDefault();
                    onRetry();
                  }}
                >
                  点击重试
                </a>
              }
            />
          </div>
        ) : (
          <AgGridReact<ShopProductSku>
            theme="legacy"
            rowData={rows}
            columnDefs={colDefs}
            defaultColDef={defaultColDef}
            localeText={ERP_AG_GRID_LOCALE_TEXT_ZH_CN}
            getRowId={(params) => params.data.sku_record_id}
            rowSelection={{
              mode: 'multiRow',
              checkboxes: false,
              headerCheckbox: false,
              enableClickSelection: false,
            }}
            suppressRowClickSelection
            singleClickEdit={false}
            readOnlyEdit
            rowHeight={28}
            headerHeight={32}
            animateRows={false}
            cellSelection
            suppressContextMenu
            enableCellTextSelection={false}
            onGridReady={handleGridReady}
            onSelectionChanged={handleGridSelectionChanged}
            onCellClicked={handleCellClicked}
            onCellFocused={handleGridCellFocused}
            onCellKeyDown={handleCellKeyDown}
            onRowClicked={handleRowClicked}
            getRowClass={(params) =>
              params.data?.sku_record_id === activeSkuRecordId ? 'order-review-main-row-active' : ''
            }
          />
        )}

        {!error && !loading && rows.length === 0 ? (
          <div className="order-review-main-grid-empty">
            <Empty description="暂无网店商品" />
          </div>
        ) : null}

        {loading ? (
          <div className="order-review-main-grid-loading-mask">
            <Spin size="large" tip="加载中..." />
          </div>
        ) : null}
      </div>
    </div>
  );
};
