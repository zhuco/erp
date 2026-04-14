import { Button, Form, Input, InputNumber, Modal, Tabs, message } from 'antd';
import type { MenuProps } from 'antd';
import type { Key } from 'react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import type { CellContextMenuEvent, ColDef, SelectionChangedEvent } from 'ag-grid-community';
import { AgGridReact } from 'ag-grid-react';

import { UiModule03ContextMenu, UiModule04BottomTable } from '@/components/ui-modules';

import { money } from './helpers';

type GoodsGridRow = {
  __localKey: string;
  id?: string;
  platform_sku_id?: string | null;
  outer_sku_id?: string | null;
  title?: string | null;
  quantity?: number | null;
  price_cent?: number | null;
};

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
  goodsRows: GoodsGridRow[];
  logRows: LogRow[];
  goodsSaving?: boolean;
  onPersistGoodsRows?: (nextRows: GoodsGridRow[], successText: string) => Promise<void> | void;
};

const agShellClass = 'ag-theme-quartz order-review-bottom-ag-shell';

const normalizeText = (value: unknown) => String(value ?? '').trim();

const isSameGoods = (left: GoodsGridRow, right: GoodsGridRow) => {
  const leftSku = normalizeText(left.platform_sku_id);
  const rightSku = normalizeText(right.platform_sku_id);
  if (leftSku && rightSku) return leftSku === rightSku;

  const leftOuterSku = normalizeText(left.outer_sku_id);
  const rightOuterSku = normalizeText(right.outer_sku_id);
  if (leftOuterSku && rightOuterSku) return leftOuterSku === rightOuterSku;

  const leftId = normalizeText(left.id);
  const rightId = normalizeText(right.id);
  if (leftId && rightId) return leftId === rightId;

  const leftTitle = normalizeText(left.title).toLowerCase();
  const rightTitle = normalizeText(right.title).toLowerCase();
  if (leftTitle && rightTitle) return leftTitle === rightTitle;

  return false;
};

export const OrderReviewBottomPanel = ({
  activeOrderId,
  goodsRows,
  logRows,
  goodsSaving = false,
  onPersistGoodsRows,
}: OrderReviewBottomPanelProps) => {
  const [goodsSelectedRowKeys, setGoodsSelectedRowKeys] = useState<Key[]>([]);
  const [goodsEditorOpen, setGoodsEditorOpen] = useState(false);
  const [goodsEditorMode, setGoodsEditorMode] = useState<'create' | 'replace'>('create');
  const [goodsEditorTargetKey, setGoodsEditorTargetKey] = useState<Key | null>(null);
  const [goodsContextMenu, setGoodsContextMenu] = useState<ContextMenuState<GoodsGridRow>>({
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

  const goodsColDefs = useMemo<ColDef<GoodsGridRow>[]>(
    () => [
      {
        colId: '__goodsSelection',
        headerName: '',
        width: 34,
        minWidth: 28,
        maxWidth: 42,
        checkboxSelection: true,
        headerCheckboxSelection: false,
        sortable: false,
        filter: false,
        resizable: true,
      },
      { colId: 'id', headerName: '货品行ID', field: 'id', width: 140, minWidth: 88, sortable: false },
      { colId: 'platform_sku_id', headerName: '平台SKU', field: 'platform_sku_id', width: 132, minWidth: 88, sortable: false },
      { colId: 'outer_sku_id', headerName: '商家SKU', field: 'outer_sku_id', width: 132, minWidth: 88, sortable: false },
      { colId: 'title', headerName: '标题', field: 'title', width: 180, minWidth: 100, sortable: false },
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

  const logColDefs = useMemo<ColDef<LogRow>[]>(
    () => [
      { colId: 'created_at', headerName: '时间', field: 'created_at', width: 156, minWidth: 104, sortable: false },
      { colId: 'action', headerName: '操作', field: 'action', width: 96, minWidth: 72, sortable: false },
      { colId: 'operator', headerName: '操作者', field: 'operator', width: 96, minWidth: 72, sortable: false },
      { colId: 'detail', headerName: '详情', field: 'detail', width: 180, minWidth: 100, sortable: false },
    ],
    [],
  );

  const defaultColDef = useMemo<ColDef>(
    () => ({
      resizable: true,
      filter: false,
      sortable: false,
      minWidth: 64,
      suppressHeaderMenuButton: true,
      suppressHeaderFilterButton: true,
      headerClass: 'order-review-main-header-center',
    }),
    [],
  );

  const runPersist = useCallback(
    async (nextRows: GoodsGridRow[], successText: string) => {
      if (!onPersistGoodsRows) {
        message.warning('当前页面未接入货品持久化');
        return;
      }
      await Promise.resolve(onPersistGoodsRows(nextRows, successText));
    },
    [onPersistGoodsRows],
  );

  const openGoodsCreate = useCallback(() => {
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
  }, [goodsForm]);

  const openGoodsReplace = useCallback(
    (targetKey?: Key | null) => {
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
    [goodsForm, goodsRows, goodsSelectedRowKeys],
  );

  const removeGoodsRows = useCallback(
    async (targetKey?: Key | null) => {
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
    [goodsRows, goodsSelectedRowKeys, runPersist],
  );

  const submitGoodsEditor = useCallback(async () => {
    try {
      const values = await goodsForm.validateFields();
      const title = normalizeText(values.title);
      const quantity = Math.max(1, Number(values.quantity ?? 1));
      const priceCent = Math.max(0, Number(values.price_cent ?? 0));

      if (goodsEditorMode === 'create') {
        const id = `${activeOrderId || 'ORDER'}-NEW-${Date.now()}`;
        const newRow: GoodsGridRow = {
          __localKey: id,
          id,
          platform_sku_id: normalizeText(values.platform_sku_id) || null,
          outer_sku_id: normalizeText(values.outer_sku_id) || null,
          title,
          quantity,
          price_cent: priceCent,
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
          } as GoodsGridRow;
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

  return (
    <div className="order-workbench-bottom-panel">
      <UiModule04BottomTable syncKey={activeOrderId || 'none'}>
        <Tabs
          size="small"
          items={[
            {
              key: 'goods',
              label: '货品',
              children: (
                <div className="order-review-goods-tab" data-testid="order-review-goods-tab">
                  <div className="order-review-goods-toolbar">
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
                    <span data-testid="order-review-goods-selected-count">已选 {goodsSelectedRowKeys.length} 行</span>
                  </div>

                  <div className={agShellClass} data-testid="order-review-goods-grid">
                    <AgGridReact<GoodsGridRow>
                      theme="legacy"
                      rowData={goodsRows}
                      columnDefs={goodsColDefs}
                      defaultColDef={defaultColDef}
                      getRowId={(params) => params.data.__localKey}
                      rowSelection={{
                        mode: 'multiRow',
                        checkboxes: false,
                        headerCheckbox: false,
                        enableClickSelection: false,
                      }}
                      suppressRowClickSelection
                      rowHeight={28}
                      headerHeight={32}
                      animateRows={false}
                      enableCellTextSelection={false}
                      suppressContextMenu
                      onSelectionChanged={(event: SelectionChangedEvent<GoodsGridRow>) => {
                        const keys = event.api
                          .getSelectedNodes()
                          .map((node) => node.data?.__localKey ?? '')
                          .filter(Boolean);
                        setGoodsSelectedRowKeys(keys);
                      }}
                      onCellContextMenu={(event: CellContextMenuEvent<GoodsGridRow>) => {
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
              key: 'logs',
              label: '日志',
              children: (
                <div className={agShellClass}>
                  <AgGridReact<LogRow>
                    theme="legacy"
                    rowData={logRows}
                    columnDefs={logColDefs}
                    defaultColDef={defaultColDef}
                    getRowId={(params) =>
                      String(
                        params.data.audit_id ??
                          params.data.id ??
                          `${String(params.data.created_at ?? '')}-${String(params.data.action ?? '')}`,
                      )
                    }
                    rowHeight={28}
                    headerHeight={32}
                    animateRows={false}
                    enableCellTextSelection={false}
                    suppressContextMenu
                  />
                </div>
              ),
            },
          ]}
        />
      </UiModule04BottomTable>

      <UiModule03ContextMenu
        open={goodsContextMenu.open}
        position={goodsContextMenu.position}
        menu={goodsContextMenuConfig}
        className="order-review-goods-context-menu"
        onClose={() => setGoodsContextMenu({ open: false, position: null, row: null })}
      />

      <Modal
        title={goodsEditorMode === 'create' ? '新增货品' : '修改货品'}
        open={goodsEditorOpen}
        onCancel={() => setGoodsEditorOpen(false)}
        onOk={() => {
          void submitGoodsEditor();
        }}
        confirmLoading={goodsSaving}
        destroyOnClose
      >
        <Form form={goodsForm} layout="vertical">
          <Form.Item label="平台SKU" name="platform_sku_id">
            <Input placeholder="可选" maxLength={128} />
          </Form.Item>
          <Form.Item label="商家SKU" name="outer_sku_id">
            <Input placeholder="可选" maxLength={128} />
          </Form.Item>
          <Form.Item label="标题" name="title" rules={[{ required: true, message: '请输入货品标题' }]}>
            <Input placeholder="请输入货品标题" maxLength={256} />
          </Form.Item>
          <Form.Item label="数量" name="quantity" rules={[{ required: true, message: '请输入数量' }]}>
            <InputNumber min={1} precision={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item label="单价（分）" name="price_cent" rules={[{ required: true, message: '请输入单价' }]}>
            <InputNumber min={0} precision={0} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};
