import { Alert, Button, Empty, Input, InputNumber, Modal, Select, Space, Spin, Table, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo } from 'react';

import {
  createWorkbenchRecordLineItemDraft,
  type WorkbenchOrderRecordDraft,
  type WorkbenchRecordEditorOption,
  type WorkbenchRecordLineItemDraft,
} from './types';

type WorkbenchRecordEditorDialogProps = {
  open: boolean;
  title?: string;
  loading?: boolean;
  saving?: boolean;
  error?: unknown;
  draft: WorkbenchOrderRecordDraft | null;
  validationMessages?: string[];
  warehouseOptions?: WorkbenchRecordEditorOption[];
  logisticsOptions?: WorkbenchRecordEditorOption[];
  onCancel: () => void;
  onSave: () => void;
  onDraftChange: (updater: (current: WorkbenchOrderRecordDraft) => WorkbenchOrderRecordDraft) => void;
};

const normalizeText = (value: unknown) => String(value ?? '').trim();

const normalizeLineItems = (items: WorkbenchRecordLineItemDraft[]) => items.map((item) => createWorkbenchRecordLineItemDraft(item));

export const WorkbenchRecordEditorDialog = ({
  open,
  title = '工作台记录编辑',
  loading = false,
  saving = false,
  error,
  draft,
  validationMessages = [],
  warehouseOptions = [],
  logisticsOptions = [],
  onCancel,
  onSave,
  onDraftChange,
}: WorkbenchRecordEditorDialogProps) => {
  const totalGoodsQty = useMemo(
    () => (draft?.items ?? []).reduce((total, item) => total + Math.max(0, Number(item.quantity ?? 0)), 0),
    [draft?.items],
  );

  const totalGoodsAmountCent = useMemo(
    () =>
      (draft?.items ?? []).reduce(
        (total, item) =>
          total +
          Math.max(0, Number(item.quantity ?? 0)) * Math.max(0, Number(item.priceCent ?? 0)),
        0,
      ),
    [draft?.items],
  );

  const updateDraftField = <TKey extends keyof WorkbenchOrderRecordDraft>(
    field: TKey,
    value: WorkbenchOrderRecordDraft[TKey],
  ) => {
    onDraftChange((current) => ({
      ...current,
      [field]: value,
    }));
  };

  const updateLineItem = (draftKey: string, updater: (current: WorkbenchRecordLineItemDraft) => WorkbenchRecordLineItemDraft) => {
    onDraftChange((current) => ({
      ...current,
      items: normalizeLineItems(
        current.items.map((item) => (item.draftKey === draftKey ? updater(item) : item)),
      ),
    }));
  };

  const appendLineItem = () => {
    onDraftChange((current) => ({
      ...current,
      items: [...normalizeLineItems(current.items), createWorkbenchRecordLineItemDraft()],
    }));
  };

  const removeLineItem = (draftKey: string) => {
    onDraftChange((current) => ({
      ...current,
      items: normalizeLineItems(current.items.filter((item) => item.draftKey !== draftKey)),
    }));
  };

  const lineItemColumns: ColumnsType<WorkbenchRecordLineItemDraft> = [
    {
      title: '平台SKU',
      dataIndex: 'platformSkuId',
      width: 160,
      render: (value: string | null | undefined, record) => (
        <Input
          size="small"
          value={value ?? ''}
          placeholder="可空"
          onChange={(event) => {
            updateLineItem(record.draftKey, (current) => ({
              ...current,
              platformSkuId: normalizeText(event.target.value) || null,
            }));
          }}
        />
      ),
    },
    {
      title: '商家SKU',
      dataIndex: 'outerSkuId',
      width: 160,
      render: (value: string | null | undefined, record) => (
        <Input
          size="small"
          value={value ?? ''}
          placeholder="可空"
          onChange={(event) => {
            updateLineItem(record.draftKey, (current) => ({
              ...current,
              outerSkuId: normalizeText(event.target.value) || null,
            }));
          }}
        />
      ),
    },
    {
      title: '标题',
      dataIndex: 'title',
      render: (value: string | null | undefined, record) => (
        <Input
          size="small"
          value={value ?? ''}
          placeholder="请输入货品标题"
          onChange={(event) => {
            updateLineItem(record.draftKey, (current) => ({
              ...current,
              title: normalizeText(event.target.value) || null,
            }));
          }}
        />
      ),
    },
    {
      title: '数量',
      dataIndex: 'quantity',
      width: 96,
      render: (value: number, record) => (
        <InputNumber
          size="small"
          min={1}
          precision={0}
          value={value}
          style={{ width: '100%' }}
          onChange={(nextValue) => {
            updateLineItem(record.draftKey, (current) => ({
              ...current,
              quantity: Math.max(1, Math.trunc(Number(nextValue ?? 1) || 1)),
            }));
          }}
        />
      ),
    },
    {
      title: '单价(分)',
      dataIndex: 'priceCent',
      width: 112,
      render: (value: number, record) => (
        <InputNumber
          size="small"
          min={0}
          precision={0}
          value={value}
          style={{ width: '100%' }}
          onChange={(nextValue) => {
            updateLineItem(record.draftKey, (current) => ({
              ...current,
              priceCent: Math.max(0, Math.trunc(Number(nextValue ?? 0) || 0)),
            }));
          }}
        />
      ),
    },
    {
      title: '操作',
      key: 'actions',
      width: 88,
      render: (_value, record) => (
        <Button
          size="small"
          danger
          onClick={() => {
            removeLineItem(record.draftKey);
          }}
        >
          删除
        </Button>
      ),
    },
  ];

  return (
    <Modal
      title={title}
      open={open}
      width={1080}
      onCancel={onCancel}
      onOk={onSave}
      confirmLoading={saving}
      okText="保存"
      cancelText="取消"
      okButtonProps={{ disabled: loading || !draft || validationMessages.length > 0 }}
      destroyOnHidden
    >
      {loading ? (
        <div style={{ paddingBlock: 48, textAlign: 'center' }}>
          <Spin tip="正在加载记录详情..." />
        </div>
      ) : null}

      {!loading && error ? (
        <Alert
          type="error"
          showIcon
          style={{ marginBottom: 16 }}
          message="记录详情加载失败"
          description={error instanceof Error ? error.message : '请稍后重试'}
        />
      ) : null}

      {!loading && !error && validationMessages.length > 0 ? (
        <Alert
          type="warning"
          showIcon
          style={{ marginBottom: 16 }}
          message="保存前请先修正以下问题"
          description={
            <ul style={{ margin: 0, paddingInlineStart: 18 }}>
              {validationMessages.map((message) => (
                <li key={message}>{message}</li>
              ))}
            </ul>
          }
        />
      ) : null}

      {!loading && !draft ? <Empty description="暂无可编辑记录" /> : null}

      {!loading && draft ? (
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <div style={{ display: 'grid', gap: 12, gridTemplateColumns: 'repeat(4, minmax(0, 1fr))' }}>
            <Input value={draft.orderNo} addonBefore="订单号" readOnly />
            <Input value={draft.platformOrderId ?? ''} addonBefore="平台单号" readOnly />
            <Input value={draft.platform ?? ''} addonBefore="平台" readOnly />
            <Input value={draft.shopId ?? ''} addonBefore="店铺" readOnly />
            <Input
              value={draft.receiverName}
              addonBefore="收件人"
              onChange={(event) => updateDraftField('receiverName', event.target.value)}
            />
            <Input
              value={draft.receiverPhone}
              addonBefore="手机号"
              onChange={(event) => updateDraftField('receiverPhone', event.target.value)}
            />
            <Select
              value={draft.shipWarehouseId}
              placeholder="请选择发货仓"
              options={warehouseOptions}
              allowClear
              onChange={(value) => updateDraftField('shipWarehouseId', value)}
            />
            <Select
              value={draft.logisticsCompany}
              placeholder="请选择物流公司"
              options={logisticsOptions}
              allowClear
              onChange={(value) => updateDraftField('logisticsCompany', value)}
            />
          </div>

          <Input.TextArea
            value={draft.buyerMessage ?? ''}
            rows={2}
            placeholder="买家留言"
            onChange={(event) => updateDraftField('buyerMessage', event.target.value)}
          />
          <Input.TextArea
            value={draft.sellerRemark ?? ''}
            rows={2}
            placeholder="卖家备注"
            onChange={(event) => updateDraftField('sellerRemark', event.target.value)}
          />

          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8 }}>
              <Typography.Text strong>货品明细</Typography.Text>
              <Space size={12}>
                <Typography.Text type="secondary">{`商品数 ${totalGoodsQty}`}</Typography.Text>
                <Typography.Text type="secondary">{`货值 ${(totalGoodsAmountCent / 100).toFixed(2)} 元`}</Typography.Text>
                <Button size="small" type="primary" onClick={appendLineItem}>
                  新增货品
                </Button>
              </Space>
            </div>
            <Table<WorkbenchRecordLineItemDraft>
              size="small"
              rowKey="draftKey"
              pagination={false}
              columns={lineItemColumns}
              dataSource={draft.items}
              locale={{ emptyText: '当前订单暂无货品' }}
              scroll={{ x: 920, y: 280 }}
            />
          </div>
        </Space>
      ) : null}
    </Modal>
  );
};
