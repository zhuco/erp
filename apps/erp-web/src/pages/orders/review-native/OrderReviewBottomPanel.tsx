import { Button, Empty, Form, Input, InputNumber, Modal, Table, Tabs, message } from 'antd';
import type { MenuProps } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { Key } from 'react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import type { CellContextMenuEvent, ColDef, SelectionChangedEvent } from 'ag-grid-community';
import { AgGridReact } from 'ag-grid-react';

import { UiModule03ContextMenu, UiModule04BottomTable } from '@/components/ui-modules';
import type {
  OrderWorkbenchFeeRow,
  OrderWorkbenchPlatformOrderRow,
  OrderWorkbenchShipmentRow,
} from '@/pages/orders/shared/orderWorkbenchDetailData';
import {
  ORDER_WORKBENCH_GRID_HEADER_HEIGHT,
  ORDER_WORKBENCH_GRID_ROW_HEIGHT,
} from '@/pages/orders/shared/orderWorkbenchGridMetrics';
import type { OrderWorkbenchGoodsRow } from '@/pages/orders/shared/orderWorkbenchGoodsRows';

import { formatDateTimeSeconds, money } from './helpers';
import './OrderReviewBottomPanel.css';

type LogRow = Record<string, unknown>;

type GoodsEditorValues = {
  platform_sku_id?: string;
  outer_sku_id?: string;
  title?: string;
  quantity?: number;
  price_cent?: number;
};

type ContextMenuState<T> = {
  open: boolean;
  position: { x: number; y: number } | null;
  row: T | null;
};

type OrderReviewBottomPanelProps = {
  activeOrderId: string;
  goodsRows: OrderWorkbenchGoodsRow[];
  logRows: LogRow[];
  platformOrderRows?: OrderWorkbenchPlatformOrderRow[];
  shipmentRows?: OrderWorkbenchShipmentRow[];
  feeRows?: OrderWorkbenchFeeRow[];
  goodsSaving?: boolean;
  goodsEditable?: boolean;
  onPersistGoodsRows?: (nextRows: OrderWorkbenchGoodsRow[], successText: string) => Promise<void> | void;
};

const agShellClass = 'ag-theme-quartz order-review-bottom-ag-shell';

const normalizeText = (value: unknown) => String(value ?? '').trim();

const renderEmpty = (description: string) => <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={description} />;

const compareText = (left: unknown, right: unknown) =>
  normalizeText(left).localeCompare(normalizeText(right), 'zh-CN');

const toSortNumber = (value: unknown) => {
  const numeric = Number(value ?? 0);
  return Number.isFinite(numeric) ? numeric : 0;
};

const compareNumber = (left: unknown, right: unknown) => toSortNumber(left) - toSortNumber(right);

const toSortTimestamp = (value: unknown) => {
  const normalized = normalizeText(value);
  if (!normalized) {
    return 0;
  }
  const timestamp = Date.parse(normalized);
  return Number.isNaN(timestamp) ? 0 : timestamp;
};

const compareDateTime = (left: unknown, right: unknown) => toSortTimestamp(left) - toSortTimestamp(right);

const isSameGoods = (left: OrderWorkbenchGoodsRow, right: OrderWorkbenchGoodsRow) => {
  const leftSku = normalizeText(left.platform_sku_id);
  const rightSku = normalizeText(right.platform_sku_id);
  if (leftSku && rightSku) {
    return leftSku === rightSku;
  }

  const leftOuterSku = normalizeText(left.outer_sku_id);
  const rightOuterSku = normalizeText(right.outer_sku_id);
  if (leftOuterSku && rightOuterSku) {
    return leftOuterSku === rightOuterSku;
  }

  const leftId = normalizeText(left.id);
  const rightId = normalizeText(right.id);
  if (leftId && rightId) {
    return leftId === rightId;
  }

  const leftTitle = normalizeText(left.title).toLowerCase();
  const rightTitle = normalizeText(right.title).toLowerCase();
  return Boolean(leftTitle && rightTitle && leftTitle === rightTitle);
};

export const OrderReviewBottomPanel = ({
  activeOrderId,
  goodsRows,
  logRows,
  platformOrderRows = [],
  shipmentRows = [],
  feeRows = [],
  goodsSaving = false,
  goodsEditable = true,
  onPersistGoodsRows,
}: OrderReviewBottomPanelProps) => {
  const [activeTab, setActiveTab] = useState('goods');
  const [goodsSelectedRowKeys, setGoodsSelectedRowKeys] = useState<Key[]>([]);
  const [goodsEditorOpen, setGoodsEditorOpen] = useState(false);
  const [goodsEditorMode, setGoodsEditorMode] = useState<'create' | 'replace'>('create');
  const [goodsEditorTargetKey, setGoodsEditorTargetKey] = useState<Key | null>(null);
  const [goodsContextMenu, setGoodsContextMenu] = useState<ContextMenuState<OrderWorkbenchGoodsRow>>({
    open: false,
    position: null,
    row: null,
  });
  const [goodsForm] = Form.useForm<GoodsEditorValues>();

  useEffect(() => {
    setGoodsSelectedRowKeys([]);
    setGoodsEditorOpen(false);
    setGoodsEditorTargetKey(null);
    setGoodsContextMenu({ open: false, position: null, row: null });
  }, [activeOrderId]);

  const goodsColDefs = useMemo<ColDef<OrderWorkbenchGoodsRow>[]>(
    () => [
      {
        colId: '__goodsSelection',
        headerName: '',
        width: 34,
        minWidth: 28,
        maxWidth: 42,
        sortable: false,
        filter: false,
        resizable: true,
      },
      { colId: 'id', headerName: '货品行ID', field: 'id', width: 140, minWidth: 88, sortable: false },
      { colId: 'platform_sku_id', headerName: '平台SKU', field: 'platform_sku_id', width: 132, minWidth: 88, sortable: false },
      { colId: 'outer_sku_id', headerName: '商家SKU', field: 'outer_sku_id', width: 132, minWidth: 88, sortable: false },
      {
        colId: 'shopProductName',
        headerName: '网店商品名',
        field: 'shopProductName',
        width: 180,
        minWidth: 120,
        sortable: false,
      },
      { colId: 'shopSpec', headerName: '规格', field: 'shopSpec', width: 128, minWidth: 96, sortable: false },
      {
        colId: 'mappingStatus',
        headerName: '匹配状态',
        width: 92,
        minWidth: 80,
        sortable: false,
        valueGetter: (params) => (params.data?.mappingMissing ? '未匹配' : '已匹配'),
      },
      {
        colId: 'quantity',
        headerName: '数量',
        field: 'quantity',
        width: 72,
        minWidth: 64,
        sortable: false,
        cellStyle: { textAlign: 'right' },
      },
      {
        colId: 'price_cent',
        headerName: '单价',
        field: 'price_cent',
        width: 92,
        minWidth: 76,
        sortable: false,
        cellStyle: { textAlign: 'right' },
        valueFormatter: (params) => money((params.value as number | null | undefined) ?? null),
      },
    ],
    [],
  );

  const resolvedGoodsColDefs = useMemo<ColDef<OrderWorkbenchGoodsRow>[]>(
    () =>
      goodsColDefs.map((colDef) => {
        switch (String(colDef.colId ?? '')) {
          case '__goodsSelection':
            return colDef;
          case 'id':
            return { ...colDef, width: 128, minWidth: 72, sortable: true };
          case 'platform_sku_id':
          case 'outer_sku_id':
            return { ...colDef, width: 118, minWidth: 72, sortable: true };
          case 'shopProductName':
            return { ...colDef, width: 160, minWidth: 108, sortable: true };
          case 'shopSpec':
            return { ...colDef, width: 112, minWidth: 84, sortable: true };
          case 'mappingStatus':
            return { ...colDef, width: 88, minWidth: 76, sortable: true };
          case 'quantity':
            return { ...colDef, width: 64, minWidth: 56, sortable: true };
          case 'price_cent':
            return { ...colDef, width: 88, minWidth: 68, sortable: true };
          default:
            return { ...colDef, sortable: true };
        }
      }),
    [goodsColDefs],
  );

  const defaultColDef = useMemo<ColDef>(
    () => ({
      resizable: true,
      filter: false,
      sortable: true,
      minWidth: 56,
      suppressHeaderMenuButton: true,
      suppressHeaderFilterButton: true,
      headerClass: 'order-review-main-header-center',
    }),
    [],
  );

  const logColumns = useMemo<ColumnsType<LogRow>>(
    () => [
      {
        title: '时间',
        dataIndex: 'created_at',
        width: 180,
        render: (value) => formatDateTimeSeconds(value as string | null | undefined),
      },
      {
        title: '操作',
        dataIndex: 'action',
        width: 140,
        render: (value) => normalizeText(value) || '-',
      },
      {
        title: '操作人',
        dataIndex: 'operator',
        width: 120,
        render: (value) => normalizeText(value) || '-',
      },
      {
        title: '详情',
        dataIndex: 'detail',
        width: 360,
        render: (value) => normalizeText(value) || '-',
      },
    ],
    [],
  );

  const platformOrderColumns = useMemo<ColumnsType<OrderWorkbenchPlatformOrderRow>>(
    () => [
      { title: '订单来源', dataIndex: 'orderSource', width: 120, render: (value) => value ?? '-' },
      { title: '订单类型', dataIndex: 'orderType', width: 120, render: (value) => value ?? '-' },
      { title: '订单号', dataIndex: 'orderNo', width: 220, render: (value) => value ?? '-' },
      { title: '订单状态', dataIndex: 'orderStatus', width: 140, render: (value) => value ?? '-' },
      {
        title: '金额',
        dataIndex: 'amountCent',
        width: 120,
        align: 'right',
        render: (value) => money((value as number | null | undefined) ?? null),
      },
      { title: '时间', dataIndex: 'time', width: 180, render: (value) => value ?? '-' },
    ],
    [],
  );

  const shipmentColumns = useMemo<ColumnsType<OrderWorkbenchShipmentRow>>(
    () => [
      { title: '发货单号', dataIndex: 'shipmentNo', width: 180, render: (value) => value ?? '-' },
      { title: '发货状态', dataIndex: 'shipmentStatus', width: 120, render: (value) => value ?? '-' },
      { title: '发货时间', dataIndex: 'shippedAt', width: 180, render: (value) => value ?? '-' },
      { title: '仓库', dataIndex: 'warehouseId', width: 120, render: (value) => value ?? '-' },
      { title: '物流公司', dataIndex: 'logisticsCompany', width: 140, render: (value) => value ?? '-' },
      { title: '物流单号', dataIndex: 'trackingNo', width: 180, render: (value) => value ?? '-' },
    ],
    [],
  );

  const feeColumns = useMemo<ColumnsType<OrderWorkbenchFeeRow>>(
    () => [
      { title: '费用项', dataIndex: 'feeName', width: 180, render: (value) => value ?? '-' },
      {
        title: '金额',
        dataIndex: 'amountCent',
        width: 120,
        align: 'right',
        render: (value) => money((value as number | null | undefined) ?? null),
      },
      { title: '备注', dataIndex: 'remark', width: 220, render: (value) => value ?? '-' },
    ],
    [],
  );

  const resolvedLogColumns = useMemo<ColumnsType<LogRow>>(
    () =>
      logColumns.map((column) => {
        switch (String(column.dataIndex ?? '')) {
          case 'created_at':
            return {
              ...column,
              width: 168,
              ellipsis: true,
              sorter: (left: LogRow, right: LogRow) => compareDateTime(left.created_at, right.created_at),
            };
          case 'action':
            return {
              ...column,
              width: 120,
              ellipsis: true,
              sorter: (left: LogRow, right: LogRow) => compareText(left.action, right.action),
            };
          case 'operator':
            return {
              ...column,
              width: 110,
              ellipsis: true,
              sorter: (left: LogRow, right: LogRow) => compareText(left.operator, right.operator),
            };
          case 'detail':
            return {
              ...column,
              width: 280,
              ellipsis: true,
              sorter: (left: LogRow, right: LogRow) => compareText(left.detail, right.detail),
            };
          default:
            return column;
        }
      }),
    [logColumns],
  );

  const resolvedPlatformOrderColumns = useMemo<ColumnsType<OrderWorkbenchPlatformOrderRow>>(
    () =>
      platformOrderColumns.map((column) => {
        switch (String(column.dataIndex ?? '')) {
          case 'orderSource':
            return {
              ...column,
              width: 100,
              ellipsis: true,
              sorter: (left: OrderWorkbenchPlatformOrderRow, right: OrderWorkbenchPlatformOrderRow) =>
                compareText(left.orderSource, right.orderSource),
            };
          case 'orderType':
            return {
              ...column,
              width: 96,
              ellipsis: true,
              sorter: (left: OrderWorkbenchPlatformOrderRow, right: OrderWorkbenchPlatformOrderRow) =>
                compareText(left.orderType, right.orderType),
            };
          case 'orderNo':
            return {
              ...column,
              width: 180,
              ellipsis: true,
              sorter: (left: OrderWorkbenchPlatformOrderRow, right: OrderWorkbenchPlatformOrderRow) =>
                compareText(left.orderNo, right.orderNo),
            };
          case 'orderStatus':
            return {
              ...column,
              width: 116,
              ellipsis: true,
              sorter: (left: OrderWorkbenchPlatformOrderRow, right: OrderWorkbenchPlatformOrderRow) =>
                compareText(left.orderStatus, right.orderStatus),
            };
          case 'amountCent':
            return {
              ...column,
              width: 100,
              sorter: (left: OrderWorkbenchPlatformOrderRow, right: OrderWorkbenchPlatformOrderRow) =>
                compareNumber(left.amountCent, right.amountCent),
            };
          case 'time':
            return {
              ...column,
              width: 168,
              ellipsis: true,
              sorter: (left: OrderWorkbenchPlatformOrderRow, right: OrderWorkbenchPlatformOrderRow) =>
                compareDateTime(left.time, right.time),
            };
          default:
            return column;
        }
      }),
    [platformOrderColumns],
  );

  const resolvedShipmentColumns = useMemo<ColumnsType<OrderWorkbenchShipmentRow>>(
    () =>
      shipmentColumns.map((column) => {
        switch (String(column.dataIndex ?? '')) {
          case 'shipmentNo':
            return {
              ...column,
              width: 156,
              ellipsis: true,
              sorter: (left: OrderWorkbenchShipmentRow, right: OrderWorkbenchShipmentRow) =>
                compareText(left.shipmentNo, right.shipmentNo),
            };
          case 'shipmentStatus':
            return {
              ...column,
              width: 100,
              ellipsis: true,
              sorter: (left: OrderWorkbenchShipmentRow, right: OrderWorkbenchShipmentRow) =>
                compareText(left.shipmentStatus, right.shipmentStatus),
            };
          case 'shippedAt':
            return {
              ...column,
              width: 156,
              ellipsis: true,
              sorter: (left: OrderWorkbenchShipmentRow, right: OrderWorkbenchShipmentRow) =>
                compareDateTime(left.shippedAt, right.shippedAt),
            };
          case 'warehouseId':
            return {
              ...column,
              width: 100,
              ellipsis: true,
              sorter: (left: OrderWorkbenchShipmentRow, right: OrderWorkbenchShipmentRow) =>
                compareText(left.warehouseId, right.warehouseId),
            };
          case 'logisticsCompany':
            return {
              ...column,
              width: 120,
              ellipsis: true,
              sorter: (left: OrderWorkbenchShipmentRow, right: OrderWorkbenchShipmentRow) =>
                compareText(left.logisticsCompany, right.logisticsCompany),
            };
          case 'trackingNo':
            return {
              ...column,
              width: 160,
              ellipsis: true,
              sorter: (left: OrderWorkbenchShipmentRow, right: OrderWorkbenchShipmentRow) =>
                compareText(left.trackingNo, right.trackingNo),
            };
          default:
            return column;
        }
      }),
    [shipmentColumns],
  );

  const resolvedFeeColumns = useMemo<ColumnsType<OrderWorkbenchFeeRow>>(
    () =>
      feeColumns.map((column) => {
        switch (String(column.dataIndex ?? '')) {
          case 'feeName':
            return {
              ...column,
              width: 160,
              ellipsis: true,
              sorter: (left: OrderWorkbenchFeeRow, right: OrderWorkbenchFeeRow) =>
                compareText(left.feeName, right.feeName),
            };
          case 'amountCent':
            return {
              ...column,
              width: 100,
              sorter: (left: OrderWorkbenchFeeRow, right: OrderWorkbenchFeeRow) =>
                compareNumber(left.amountCent, right.amountCent),
            };
          case 'remark':
            return {
              ...column,
              width: 180,
              ellipsis: true,
              sorter: (left: OrderWorkbenchFeeRow, right: OrderWorkbenchFeeRow) =>
                compareText(left.remark, right.remark),
            };
          default:
            return column;
        }
      }),
    [feeColumns],
  );

  const runPersist = useCallback(
    async (nextRows: OrderWorkbenchGoodsRow[], successText: string) => {
      if (!onPersistGoodsRows) {
        message.warning('当前页面未接入货品持久化');
        return;
      }
      await Promise.resolve(onPersistGoodsRows(nextRows, successText));
    },
    [onPersistGoodsRows],
  );

  const openGoodsCreate = useCallback(() => {
    if (!goodsEditable) {
      return;
    }
    setGoodsEditorMode('create');
    setGoodsEditorTargetKey(null);
    goodsForm.setFieldsValue({
      platform_sku_id: undefined,
      outer_sku_id: undefined,
      title: '',
      quantity: 1,
      price_cent: 0,
    });
    setGoodsEditorOpen(true);
    setGoodsContextMenu({ open: false, position: null, row: null });
  }, [goodsEditable, goodsForm]);

  const openGoodsReplace = useCallback(
    (targetKey?: Key | null) => {
      if (!goodsEditable) {
        return;
      }
      const key = targetKey ?? goodsSelectedRowKeys[0] ?? null;
      if (!key) {
        message.warning('请先选择要修改的货品行');
        return;
      }
      const row = goodsRows.find((item) => item.__localKey === String(key));
      if (!row) {
        message.warning('未找到要修改的货品行');
        return;
      }

      setGoodsEditorMode('replace');
      setGoodsEditorTargetKey(key);
      goodsForm.setFieldsValue({
        platform_sku_id: row.platform_sku_id ?? undefined,
        outer_sku_id: row.outer_sku_id ?? undefined,
        title: row.title ?? '',
        quantity: Number(row.quantity ?? 1),
        price_cent: Number(row.price_cent ?? 0),
      });
      setGoodsEditorOpen(true);
      setGoodsContextMenu({ open: false, position: null, row: null });
    },
    [goodsEditable, goodsForm, goodsRows, goodsSelectedRowKeys],
  );

  const removeGoodsRows = useCallback(
    async (targetKey?: Key | null) => {
      if (!goodsEditable) {
        return;
      }
      const keys = targetKey ? [targetKey] : goodsSelectedRowKeys;
      if (!keys.length) {
        message.warning('请先勾选要删除的货品行');
        return;
      }
      const deleteSet = new Set(keys.map((item) => String(item)));
      const nextRows = goodsRows.filter((item) => !deleteSet.has(item.__localKey));
      await runPersist(nextRows, '货品删除成功');
      setGoodsSelectedRowKeys([]);
      setGoodsContextMenu({ open: false, position: null, row: null });
    },
    [goodsEditable, goodsRows, goodsSelectedRowKeys, runPersist],
  );

  const submitGoodsEditor = useCallback(async () => {
    try {
      const values = await goodsForm.validateFields();
      const title = normalizeText(values.title);
      const quantity = Math.max(1, Number(values.quantity ?? 1));
      const priceCent = Math.max(0, Number(values.price_cent ?? 0));

      if (goodsEditorMode === 'create') {
        const id = `${activeOrderId || 'ORDER'}-NEW-${Date.now()}`;
        const newRow: OrderWorkbenchGoodsRow = {
          __localKey: id,
          id,
          platform_sku_id: normalizeText(values.platform_sku_id) || null,
          outer_sku_id: normalizeText(values.outer_sku_id) || null,
          title,
          shopProductName: title,
          shopSpec: null,
          quantity,
          price_cent: priceCent,
          mappingMissing: false,
          mapping_missing: false,
        };

        const hitIndex = goodsRows.findIndex((item) => isSameGoods(item, newRow));
        if (hitIndex >= 0) {
          const mergedRows = [...goodsRows];
          const hit = mergedRows[hitIndex];
          mergedRows[hitIndex] = {
            ...hit,
            platform_sku_id: hit.platform_sku_id ?? newRow.platform_sku_id,
            outer_sku_id: hit.outer_sku_id ?? newRow.outer_sku_id,
            title: hit.title ?? newRow.title,
            shopProductName: hit.shopProductName ?? newRow.shopProductName,
            quantity: Number(hit.quantity ?? 0) + quantity,
          };
          await runPersist(mergedRows, '货品已存在，已自动合并数量');
        } else {
          await runPersist([newRow, ...goodsRows], '货品新增成功');
        }
      } else {
        const targetKey = String(goodsEditorTargetKey ?? goodsSelectedRowKeys[0] ?? '');
        if (!targetKey) {
          message.warning('未找到要修改的货品行');
          return;
        }
        const nextRows = goodsRows.map((item) => {
          if (item.__localKey !== targetKey) {
            return item;
          }
          return {
            ...item,
            platform_sku_id: normalizeText(values.platform_sku_id) || null,
            outer_sku_id: normalizeText(values.outer_sku_id) || null,
            title,
            quantity,
            price_cent: priceCent,
            shopProductName: title,
          } as OrderWorkbenchGoodsRow;
        });
        await runPersist(nextRows, '货品修改成功');
      }

      setGoodsEditorOpen(false);
      setGoodsEditorTargetKey(null);
      setGoodsContextMenu({ open: false, position: null, row: null });
    } catch {
      return;
    }
  }, [
    activeOrderId,
    goodsEditorMode,
    goodsEditorTargetKey,
    goodsForm,
    goodsRows,
    goodsSelectedRowKeys,
    runPersist,
  ]);

  const goodsContextMenuConfig = useMemo<MenuProps>(
    () => ({
      items: [
        { key: 'create', label: '新增货品' },
        { key: 'edit', label: '修改货品' },
        { key: 'delete', label: '删除货品' },
      ],
      onClick: ({ key }) => {
        if (key === 'create') {
          openGoodsCreate();
          return;
        }
        if (key === 'edit') {
          const targetKey = goodsContextMenu.row?.__localKey ?? null;
          openGoodsReplace(targetKey);
          return;
        }
        if (key === 'delete') {
          const targetKey = goodsContextMenu.row?.__localKey ?? null;
          void removeGoodsRows(targetKey);
        }
      },
    }),
    [goodsContextMenu.row?.__localKey, openGoodsCreate, openGoodsReplace, removeGoodsRows],
  );

  const goodsToolbar = goodsEditable ? (
    <div>
      <Button size="small" type="primary" onClick={openGoodsCreate} loading={goodsSaving}>
        新增
      </Button>
      <Button
        size="small"
        style={{ marginInlineStart: 8 }}
        onClick={() => openGoodsReplace()}
        disabled={!goodsSelectedRowKeys.length}
        loading={goodsSaving}
      >
        修改
      </Button>
      <Button
        size="small"
        danger
        style={{ marginInlineStart: 8 }}
        onClick={() => {
          void removeGoodsRows();
        }}
        disabled={!goodsSelectedRowKeys.length}
        loading={goodsSaving}
      >
        删除
      </Button>
    </div>
  ) : null;

  return (
    <div className="order-workbench-bottom-panel order-review-bottom-panel">
      <UiModule04BottomTable syncKey={`${activeOrderId || 'none'}:${activeTab}`}>
        <Tabs
          className="order-review-bottom-panel-tabs"
          size="small"
          activeKey={activeTab}
          animated={false}
          destroyOnHidden
          onChange={setActiveTab}
          items={[
            {
              key: 'goods',
              label: '货品',
              children: (
                <div className="order-review-goods-tab" data-testid="order-review-goods-tab">
                  <div className="order-review-goods-toolbar">
                    {goodsToolbar}
                  </div>
                  <div className={agShellClass} data-testid="order-review-goods-grid">
                    <AgGridReact<OrderWorkbenchGoodsRow>
                      theme="legacy"
                      rowData={goodsRows}
                      columnDefs={resolvedGoodsColDefs}
                      defaultColDef={defaultColDef}
                      getRowId={(params) => params.data.__localKey}
                      rowSelection={{
                        mode: 'multiRow',
                        checkboxes: true,
                        headerCheckbox: false,
                        enableClickSelection: false,
                      }}
                      rowHeight={ORDER_WORKBENCH_GRID_ROW_HEIGHT}
                      headerHeight={ORDER_WORKBENCH_GRID_HEADER_HEIGHT}
                      animateRows={false}
                      enableCellTextSelection={false}
                      suppressContextMenu
                      onSelectionChanged={(event: SelectionChangedEvent<OrderWorkbenchGoodsRow>) => {
                        const keys = event.api
                          .getSelectedNodes()
                          .map((node) => node.data?.__localKey ?? '')
                          .filter(Boolean);
                        setGoodsSelectedRowKeys(keys);
                      }}
                      onCellContextMenu={(event: CellContextMenuEvent<OrderWorkbenchGoodsRow>) => {
                        if (!goodsEditable) {
                          return;
                        }
                        event.event?.preventDefault?.();
                        if (!event.data || !event.event) {
                          return;
                        }
                        const targetKey = String(event.data.__localKey);
                        if (!goodsSelectedRowKeys.some((item) => String(item) === targetKey)) {
                          setGoodsSelectedRowKeys([targetKey]);
                        }
                        setGoodsContextMenu({
                          open: true,
                          position: {
                            x: event.event.clientX,
                            y: event.event.clientY,
                          },
                          row: event.data,
                        });
                      }}
                    />
                  </div>
                </div>
              ),
            },
            {
              key: 'platformOrders',
              label: '平台订单',
              children: (
                <Table<OrderWorkbenchPlatformOrderRow>
                  size="small"
                  bordered
                  rowKey="key"
                  columns={resolvedPlatformOrderColumns}
                  dataSource={platformOrderRows}
                  pagination={false}
                  scroll={{ x: 'max-content' }}
                  showSorterTooltip={false}
                  locale={{ emptyText: renderEmpty('暂无平台订单信息') }}
                />
              ),
            },
            {
              key: 'shipments',
              label: '发货单',
              children: (
                <Table<OrderWorkbenchShipmentRow>
                  size="small"
                  bordered
                  rowKey="key"
                  columns={resolvedShipmentColumns}
                  dataSource={shipmentRows}
                  pagination={false}
                  scroll={{ x: 'max-content' }}
                  showSorterTooltip={false}
                  locale={{ emptyText: renderEmpty('暂无发货单信息') }}
                />
              ),
            },
            {
              key: 'fees',
              label: '订单费用',
              children: (
                <Table<OrderWorkbenchFeeRow>
                  size="small"
                  bordered
                  rowKey="key"
                  columns={resolvedFeeColumns}
                  dataSource={feeRows}
                  pagination={false}
                  scroll={{ x: 'max-content' }}
                  showSorterTooltip={false}
                  locale={{ emptyText: renderEmpty('暂无订单费用信息') }}
                />
              ),
            },
            {
              key: 'logs',
              label: '日志',
              children: (
                <Table<LogRow>
                  size="small"
                  bordered
                  rowKey={(record, index) =>
                    normalizeText(record.id) ||
                    `${normalizeText(record.created_at)}-${normalizeText(record.action)}-${index}`
                  }
                  columns={resolvedLogColumns}
                  dataSource={logRows}
                  pagination={false}
                  scroll={{ x: 'max-content' }}
                  showSorterTooltip={false}
                  locale={{ emptyText: renderEmpty('暂无操作日志') }}
                />
              ),
            },
          ]}
        />
      </UiModule04BottomTable>

      {goodsEditable ? (
        <UiModule03ContextMenu
          open={goodsContextMenu.open}
          position={goodsContextMenu.position}
          menu={goodsContextMenuConfig}
          className="order-review-goods-context-menu"
          onClose={() => setGoodsContextMenu({ open: false, position: null, row: null })}
        />
      ) : null}

      {goodsEditable ? (
        <Modal
          title={goodsEditorMode === 'create' ? '新增货品' : '修改货品'}
          open={goodsEditorOpen}
          onCancel={() => setGoodsEditorOpen(false)}
          onOk={() => {
            void submitGoodsEditor();
          }}
          confirmLoading={goodsSaving}
          destroyOnHidden
        >
          <Form form={goodsForm} layout="vertical">
            <Form.Item label="平台SKU" name="platform_sku_id">
              <Input placeholder="可选" maxLength={128} />
            </Form.Item>
            <Form.Item label="商家SKU" name="outer_sku_id">
              <Input placeholder="可选" maxLength={128} />
            </Form.Item>
            <Form.Item label="网店商品名" name="title" rules={[{ required: true, message: '请输入网店商品名' }]}>
              <Input placeholder="请输入网店商品名" maxLength={256} />
            </Form.Item>
            <Form.Item label="数量" name="quantity" rules={[{ required: true, message: '请输入数量' }]}>
              <InputNumber min={1} precision={0} style={{ width: '100%' }} />
            </Form.Item>
            <Form.Item label="单价（分）" name="price_cent" rules={[{ required: true, message: '请输入单价' }]}>
              <InputNumber min={0} precision={0} style={{ width: '100%' }} />
            </Form.Item>
          </Form>
        </Modal>
      ) : null}
    </div>
  );
};
