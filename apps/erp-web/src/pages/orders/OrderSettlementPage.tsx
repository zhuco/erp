import {
  Button,
  Card,
  Col,
  DatePicker,
  Descriptions,
  Empty,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Spin,
  Table,
  Tabs,
  Tag,
  Typography,
  message,
} from 'antd';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs, { type Dayjs } from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';

import { createExportJob } from '@/api/exports';
import {
  exportOrderSettlement,
  getOrderReconciliationDetail,
  getOrderSettlementDetail,
  getOrderSettlementRule,
  listOrderReconciliations,
  listOrderSettlements,
  lockOrderSettlement,
  settleOrderReconciliation,
  unlockOrderSettlement,
  updateOrderSettlementRule,
  type OrderReconciliationDetail,
  type OrderReconciliationSummary,
  type OrderSettlementDetail,
  type OrderSettlementSummary,
} from '@/api/orders';
import { WorkbenchListPage } from '@/components/workbench/WorkbenchListPage';
import { UiModule02ToolbarRight } from '@/components/ui-modules';
import { platformOptions } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useSessionStore } from '@/features/sessionStore';
import { useTenantId } from '@/features/tenant';
import { shopOptions } from '@/features/tenantOptions';

type ActiveTab = 'settlement' | 'reconcile';

type SettlementFilterValues = {
  range?: [Dayjs, Dayjs];
  platform?: string;
  shop_id?: string;
  keyword?: string;
  status?: 'OPEN' | 'LOCKED' | 'EXPORTED';
};

type ReconcileFilterValues = {
  range?: [Dayjs, Dayjs];
  platform?: string;
  shop_id?: string;
  keyword?: string;
  status?: 'PENDING' | 'DONE';
};

type SettlementRuleFormValues = {
  stack_mode: 'STACKED' | 'EXCLUSIVE' | 'PRIORITY';
  member_discount_rate: number;
  promo_discount_rate: number;
  max_discount_rate: number;
};

const settlementStatusLabels: Record<string, string> = {
  OPEN: '待结算',
  LOCKED: '已锁定',
  EXPORTED: '已导出',
};

const settlementStatusColors: Record<string, string> = {
  OPEN: 'blue',
  LOCKED: 'gold',
  EXPORTED: 'green',
};

const reconcileStatusLabels: Record<string, string> = {
  PENDING: '待对账',
  DONE: '已对账',
};

const diffReasonLabels: Record<string, string> = {
  SHIPPING_FAILED: '物流异常',
  REVIEW_REJECTED: '审核驳回',
  INVENTORY_SHORTAGE: '缺货',
  MAPPING_MISSING: '映射缺失',
  EXCEPTION_OPEN: '人工异常',
  AMOUNT_MISMATCH: '金额不一致',
  ORDER_NOT_FOUND: '订单缺失',
  SHIPMENT_NOT_FOUND: '运单缺失',
  FEE_MISMATCH: '费用不一致',
};

const defaultRuleFormValues: SettlementRuleFormValues = {
  stack_mode: 'STACKED',
  member_discount_rate: 3,
  promo_discount_rate: 5,
  max_discount_rate: 12,
};

const defaultSettlementFilters: SettlementFilterValues = {
  range: [dayjs().subtract(30, 'day'), dayjs()],
};

const defaultReconcileFilters: ReconcileFilterValues = {
  range: [dayjs().subtract(30, 'day'), dayjs()],
};

const formatCurrency = (value?: number | null) => `￥${((value ?? 0) / 100).toFixed(2)}`;

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const buildOperatorPayload = (userId: string, userName: string) => ({
  operator_id: userId || null,
  operator_name: userName || null,
});

type OrderSettlementPageProps = {
  initialTab?: ActiveTab;
  pageTitle?: string;
};

export const OrderSettlementPage = ({
  initialTab,
  pageTitle = '销售结算',
}: OrderSettlementPageProps = {}) => {
  const tenantId = useTenantId();
  const canWrite = useCan('orders.write');
  const canExport = useCan('exports.write');
  const userId = useSessionStore((state) => state.userId);
  const userName = useSessionStore((state) => state.userName);
  const queryClient = useQueryClient();
  const [searchParams] = useSearchParams();
  const [activeTab, setActiveTab] = useState<ActiveTab>(
    initialTab ?? (searchParams.get('tab') === 'reconcile' ? 'reconcile' : 'settlement')
  );
  const [settlementForm] = Form.useForm<SettlementFilterValues>();
  const [reconcileForm] = Form.useForm<ReconcileFilterValues>();
  const [ruleForm] = Form.useForm<SettlementRuleFormValues>();
  const [settlementFilters, setSettlementFilters] =
    useState<SettlementFilterValues>(defaultSettlementFilters);
  const [reconcileFilters, setReconcileFilters] =
    useState<ReconcileFilterValues>(defaultReconcileFilters);
  const [settlementPagination, setSettlementPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
  });
  const [detailOpen, setDetailOpen] = useState(false);
  const [activeOrderId, setActiveOrderId] = useState<string | null>(null);
  const [activeBatchId, setActiveBatchId] = useState<string | null>(null);

  useEffect(() => {
    settlementForm.setFieldsValue(settlementFilters);
  }, [settlementFilters, settlementForm]);

  useEffect(() => {
    reconcileForm.setFieldsValue(reconcileFilters);
  }, [reconcileFilters, reconcileForm]);

  const ruleQuery = useQuery({
    queryKey: ['order-settlement-rule', tenantId],
    queryFn: () => getOrderSettlementRule(tenantId),
    enabled: Boolean(tenantId),
  });

  useEffect(() => {
    if (ruleQuery.data) {
      ruleForm.setFieldsValue({
        stack_mode: ruleQuery.data.stack_mode,
        member_discount_rate: ruleQuery.data.member_discount_rate,
        promo_discount_rate: ruleQuery.data.promo_discount_rate,
        max_discount_rate: ruleQuery.data.max_discount_rate,
      });
    } else {
      ruleForm.setFieldsValue(defaultRuleFormValues);
    }
  }, [ruleForm, ruleQuery.data]);

  const settlementQuery = useMemo(() => {
    const range = settlementFilters.range;
    return {
      platform: settlementFilters.platform,
      shop_id: settlementFilters.shop_id,
      keyword: settlementFilters.keyword?.trim() || undefined,
      status: settlementFilters.status,
      created_from: range?.[0]?.startOf('day')?.toISOString(),
      created_to: range?.[1]?.endOf('day')?.toISOString(),
      offset: ((settlementPagination.current ?? 1) - 1) * (settlementPagination.pageSize ?? 20),
      limit: settlementPagination.pageSize ?? 20,
    };
  }, [settlementFilters, settlementPagination.current, settlementPagination.pageSize]);

  const settlementListQuery = useQuery({
    queryKey: ['order-settlements', tenantId, settlementQuery],
    queryFn: () => listOrderSettlements(tenantId, settlementQuery),
    enabled: activeTab === 'settlement' && Boolean(tenantId),
    keepPreviousData: true,
  });

  const reconcileQuery = useMemo(() => {
    const range = reconcileFilters.range;
    return {
      platform: reconcileFilters.platform,
      shop_id: reconcileFilters.shop_id,
      keyword: reconcileFilters.keyword?.trim() || undefined,
      status: reconcileFilters.status,
      created_from: range?.[0]?.startOf('day')?.toISOString(),
      created_to: range?.[1]?.endOf('day')?.toISOString(),
      offset: 0,
      limit: 200,
    };
  }, [reconcileFilters]);

  const reconcileListQuery = useQuery({
    queryKey: ['order-reconciliations', tenantId, reconcileQuery],
    queryFn: () => listOrderReconciliations(tenantId, reconcileQuery),
    enabled: activeTab === 'reconcile' && Boolean(tenantId),
    keepPreviousData: true,
  });

  const settlementDetailQuery = useQuery({
    queryKey: ['order-settlement-detail', tenantId, activeOrderId],
    queryFn: () => getOrderSettlementDetail(tenantId, activeOrderId!),
    enabled: Boolean(tenantId && detailOpen && activeOrderId),
  });

  const reconcileDetailQuery = useQuery({
    queryKey: ['order-reconciliation-detail', tenantId, activeBatchId],
    queryFn: () => getOrderReconciliationDetail(tenantId, activeBatchId!),
    enabled: Boolean(tenantId && activeBatchId),
  });

  const refreshSettlementWorkbench = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['order-settlements', tenantId] }),
      queryClient.invalidateQueries({ queryKey: ['order-settlement-detail', tenantId] }),
      queryClient.invalidateQueries({ queryKey: ['order-reconciliations', tenantId] }),
      queryClient.invalidateQueries({ queryKey: ['order-reconciliation-detail', tenantId] }),
    ]);
  };

  const ruleMutation = useMutation({
    mutationFn: (values: SettlementRuleFormValues) =>
      updateOrderSettlementRule(tenantId, {
        ...values,
        expected_version: ruleQuery.data?.version,
      }),
    onSuccess: async () => {
      message.success('结算规则已更新');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['order-settlement-rule', tenantId] }),
        queryClient.invalidateQueries({ queryKey: ['order-settlements', tenantId] }),
        queryClient.invalidateQueries({ queryKey: ['order-settlement-detail', tenantId] }),
        queryClient.invalidateQueries({ queryKey: ['order-reconciliations', tenantId] }),
      ]);
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '更新结算规则失败');
    },
  });

  const lockMutation = useMutation({
    mutationFn: (orderId: string) =>
      lockOrderSettlement(tenantId, orderId, buildOperatorPayload(userId, userName)),
    onSuccess: async () => {
      message.success('订单结算已锁定');
      await refreshSettlementWorkbench();
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '锁定订单结算失败');
    },
  });

  const unlockMutation = useMutation({
    mutationFn: (orderId: string) =>
      unlockOrderSettlement(tenantId, orderId, buildOperatorPayload(userId, userName)),
    onSuccess: async () => {
      message.success('订单结算已解锁');
      await refreshSettlementWorkbench();
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '解锁订单结算失败');
    },
  });

  const exportMutation = useMutation({
    mutationFn: async (order: OrderSettlementSummary) => {
      const job = await createExportJob({
        type: 'ORDER_EXPORT',
        params: { order_id: order.order_id, settlement: true },
      });
      return exportOrderSettlement(tenantId, order.order_id, {
        ...buildOperatorPayload(userId, userName),
        export_job_id: job.job_id,
      });
    },
    onSuccess: async () => {
      message.success('订单结算导出任务已创建');
      await refreshSettlementWorkbench();
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '导出订单结算失败');
    },
  });

  const reconcileMutation = useMutation({
    mutationFn: (batchId: string) =>
      settleOrderReconciliation(tenantId, batchId, {
        ...buildOperatorPayload(userId, userName),
        note: '差异已复核',
      }),
    onSuccess: async () => {
      message.success('销售对账已完成');
      await refreshSettlementWorkbench();
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '完成销售对账失败');
    },
  });

  const settlementItems = settlementListQuery.data?.items ?? [];
  const settlementSummary = settlementListQuery.data?.summary ?? {
    total_orders: 0,
    open_orders: 0,
    locked_orders: 0,
    exported_orders: 0,
    total_amount_cent: 0,
    receivable_amount_cent: 0,
  };
  const reconcileItems = reconcileListQuery.data?.items ?? [];
  const reconcileSummary = reconcileListQuery.data?.summary ?? {
    total_batches: 0,
    pending_batches: 0,
    done_batches: 0,
    expected_total_cent: 0,
    diff_total_cent: 0,
  };

  const handleSettlementSearch = (values: SettlementFilterValues) => {
    setSettlementFilters({ ...defaultSettlementFilters, ...values });
    setSettlementPagination((current) => ({ ...current, current: 1 }));
  };

  const handleSettlementReset = () => {
    settlementForm.resetFields();
    settlementForm.setFieldsValue(defaultSettlementFilters);
    setSettlementFilters(defaultSettlementFilters);
    setSettlementPagination((current) => ({ ...current, current: 1 }));
  };

  const handleReconcileSearch = (values: ReconcileFilterValues) => {
    setReconcileFilters({ ...defaultReconcileFilters, ...values });
  };

  const handleReconcileReset = () => {
    reconcileForm.resetFields();
    reconcileForm.setFieldsValue(defaultReconcileFilters);
    setReconcileFilters(defaultReconcileFilters);
  };

  const settlementColumns = useMemo<ColumnsType<OrderSettlementSummary>>(
    () => [
      { title: '订单号', dataIndex: 'order_no', ellipsis: true },
      { title: '平台', dataIndex: 'platform' },
      { title: '店铺', dataIndex: 'shop_id' },
      {
        title: '下单时间',
        dataIndex: 'created_at',
        render: (value: string) => formatDateTime(value),
      },
      {
        title: '订单金额',
        dataIndex: 'total_amount_cent',
        render: (value: number) => formatCurrency(value),
      },
      {
        title: '优惠金额',
        dataIndex: 'discount_amount_cent',
        render: (value: number) => formatCurrency(value),
      },
      {
        title: '应收金额',
        dataIndex: 'receivable_amount_cent',
        render: (value: number) => formatCurrency(value),
      },
      {
        title: '结算状态',
        dataIndex: 'settlement_status',
        render: (value: string) => (
          <Tag color={settlementStatusColors[value] ?? 'default'}>
            {settlementStatusLabels[value] ?? value}
          </Tag>
        ),
      },
      {
        title: '操作',
        key: 'action',
        render: (_, record) => (
          <Space size={8}>
            <Button
              size="small"
              data-testid="settlement-detail-button"
              onClick={() => {
                setActiveOrderId(record.order_id);
                setDetailOpen(true);
              }}
            >
              详情
            </Button>
            {record.settlement_status === 'OPEN' ? (
              <Button
                size="small"
                disabled={!canWrite}
                loading={lockMutation.isPending}
                onClick={() => lockMutation.mutate(record.order_id)}
              >
                锁定
              </Button>
            ) : (
              <Button
                size="small"
                disabled={!canWrite || record.settlement_status !== 'LOCKED'}
                loading={unlockMutation.isPending}
                onClick={() => unlockMutation.mutate(record.order_id)}
              >
                解锁
              </Button>
            )}
            <Button
              size="small"
              disabled={!canExport || record.settlement_status !== 'LOCKED'}
              loading={exportMutation.isPending}
              onClick={() => exportMutation.mutate(record)}
            >
              导出
            </Button>
          </Space>
        ),
      },
    ],
    [canExport, canWrite, exportMutation, lockMutation, unlockMutation]
  );

  const reconcileColumns = useMemo<ColumnsType<OrderReconciliationSummary>>(
    () => [
      { title: '渠道', dataIndex: 'platform' },
      { title: '店铺', dataIndex: 'shop_id' },
      {
        title: '期间',
        key: 'period',
        render: (_, record) => `${record.period_start} ~ ${record.period_end}`,
      },
      { title: '订单数', dataIndex: 'order_count' },
      {
        title: '应收金额',
        dataIndex: 'expected_cent',
        render: (value: number) => formatCurrency(value),
      },
      {
        title: '实收金额',
        dataIndex: 'actual_cent',
        render: (value: number) => formatCurrency(value),
      },
      {
        title: '差异',
        dataIndex: 'diff_cent',
        render: (value: number) =>
          value === 0 ? <Tag color="green">￥0.00</Tag> : <Tag color="orange">{formatCurrency(value)}</Tag>,
      },
      {
        title: '差异原因',
        dataIndex: 'reasons',
        render: (reasons: string[]) => (
          <Space size={[4, 4]} wrap>
            {reasons.length > 0
              ? reasons.map((reason) => (
                  <Tag key={reason}>{diffReasonLabels[reason] ?? reason}</Tag>
                ))
              : '-'}
          </Space>
        ),
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: string) => (
          <Tag color={value === 'DONE' ? 'green' : 'blue'}>
            {reconcileStatusLabels[value] ?? value}
          </Tag>
        ),
      },
      {
        title: '操作',
        key: 'action',
        render: (_, record) => (
          <Space size={8}>
            <Button size="small" onClick={() => setActiveBatchId(record.batch_id)}>
              查看订单
            </Button>
            <Button
              size="small"
              type="primary"
              disabled={!canWrite || record.status === 'DONE'}
              loading={reconcileMutation.isPending}
              onClick={() => reconcileMutation.mutate(record.batch_id)}
            >
              完成对账
            </Button>
          </Space>
        ),
      },
    ],
    [canWrite, reconcileMutation]
  );

  const settlementQueryPanel = (
    <Card size="small">
      <Form
        form={settlementForm}
        layout="vertical"
        initialValues={defaultSettlementFilters}
        onFinish={handleSettlementSearch}
      >
        <Row gutter={12}>
          <Col xs={24}>
            <Form.Item name="range" label="下单时间">
              <DatePicker.RangePicker style={{ width: '100%' }} />
            </Form.Item>
          </Col>
          <Col xs={24}>
            <Form.Item name="platform" label="平台">
              <Select allowClear options={platformOptions} placeholder="选择平台" />
            </Form.Item>
          </Col>
          <Col xs={24}>
            <Form.Item name="shop_id" label="店铺">
              <Select allowClear options={shopOptions} showSearch placeholder="选择店铺" />
            </Form.Item>
          </Col>
          <Col xs={24}>
            <Form.Item name="status" label="状态">
              <Select
                allowClear
                options={[
                  { label: '待结算', value: 'OPEN' },
                  { label: '已锁定', value: 'LOCKED' },
                  { label: '已导出', value: 'EXPORTED' },
                ]}
                placeholder="选择状态"
              />
            </Form.Item>
          </Col>
          <Col xs={24}>
            <Form.Item name="keyword" label="关键词">
              <Input allowClear placeholder="订单号 / 收件人 / 平台单号" />
            </Form.Item>
          </Col>
          <Col xs={24}>
            <Space>
              <Button type="primary" htmlType="submit" size="small">
                查询
              </Button>
              <Button onClick={handleSettlementReset} size="small">
                重置
              </Button>
            </Space>
          </Col>
        </Row>
      </Form>
    </Card>
  );

  const reconcileQueryPanel = (
    <Card size="small">
      <Form
        form={reconcileForm}
        layout="vertical"
        initialValues={defaultReconcileFilters}
        onFinish={handleReconcileSearch}
      >
        <Row gutter={12}>
          <Col xs={24}>
            <Form.Item name="range" label="下单时间">
              <DatePicker.RangePicker style={{ width: '100%' }} />
            </Form.Item>
          </Col>
          <Col xs={24}>
            <Form.Item name="platform" label="平台">
              <Select allowClear options={platformOptions} placeholder="选择平台" />
            </Form.Item>
          </Col>
          <Col xs={24}>
            <Form.Item name="shop_id" label="店铺">
              <Select allowClear options={shopOptions} showSearch placeholder="选择店铺" />
            </Form.Item>
          </Col>
          <Col xs={24}>
            <Form.Item name="status" label="状态">
              <Select
                allowClear
                options={[
                  { label: '待对账', value: 'PENDING' },
                  { label: '已对账', value: 'DONE' },
                ]}
                placeholder="选择状态"
              />
            </Form.Item>
          </Col>
          <Col xs={24}>
            <Form.Item name="keyword" label="关键词">
              <Input allowClear placeholder="订单号 / 收件人 / 平台单号" />
            </Form.Item>
          </Col>
          <Col xs={24}>
            <Space>
              <Button type="primary" htmlType="submit" size="small">
                查询
              </Button>
              <Button onClick={handleReconcileReset} size="small">
                重置
              </Button>
            </Space>
          </Col>
        </Row>
      </Form>
    </Card>
  );

  const settlementTabContent = (
    <Space direction="vertical" size={12} style={{ width: '100%' }}>
      <Card size="small" title="促销结算规则">
        <Form
          form={ruleForm}
          layout="vertical"
          initialValues={defaultRuleFormValues}
          onFinish={(values) => ruleMutation.mutate(values)}
        >
          <Row gutter={[16, 16]}>
            <Col xs={24} md={6}>
              <Form.Item
                name="stack_mode"
                label="优惠叠加"
                rules={[{ required: true, message: '请选择叠加策略' }]}
              >
                <Select
                  options={[
                    { label: '可叠加', value: 'STACKED' },
                    { label: '互斥取优', value: 'EXCLUSIVE' },
                    { label: '按优先级', value: 'PRIORITY' },
                  ]}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item
                name="member_discount_rate"
                label="会员折扣(%)"
                rules={[{ required: true, message: '请输入会员折扣' }]}
              >
                <InputNumber min={0} max={100} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item
                name="promo_discount_rate"
                label="渠道促销(%)"
                rules={[{ required: true, message: '请输入渠道促销折扣' }]}
              >
                <InputNumber min={0} max={100} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item
                name="max_discount_rate"
                label="折扣上限(%)"
                rules={[{ required: true, message: '请输入折扣上限' }]}
              >
                <InputNumber min={0} max={100} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
              <Button type="primary" htmlType="submit" loading={ruleMutation.isPending}>
                保存规则
              </Button>
            </Col>
          </Row>
        </Form>
      </Card>
      <Card size="small" title="结算概览">
        <Descriptions size="small" column={4} bordered>
          <Descriptions.Item label="订单数">{settlementSummary.total_orders}</Descriptions.Item>
          <Descriptions.Item label="待结算">{settlementSummary.open_orders}</Descriptions.Item>
          <Descriptions.Item label="已锁定">{settlementSummary.locked_orders}</Descriptions.Item>
          <Descriptions.Item label="已导出">{settlementSummary.exported_orders}</Descriptions.Item>
          <Descriptions.Item label="订单总额">
            {formatCurrency(settlementSummary.total_amount_cent)}
          </Descriptions.Item>
          <Descriptions.Item label="应收总额">
            {formatCurrency(settlementSummary.receivable_amount_cent)}
          </Descriptions.Item>
        </Descriptions>
      </Card>
      <Card size="small" title="订单结算列表">
        <Table
          rowKey="order_id"
          columns={settlementColumns}
          dataSource={settlementItems}
          loading={settlementListQuery.isFetching}
          pagination={{
            ...settlementPagination,
            total: settlementListQuery.data?.total ?? 0,
            onChange: (page, pageSize) =>
              setSettlementPagination({ current: page, pageSize }),
          }}
        />
      </Card>
    </Space>
  );

  const reconcileTabContent = (
    <Space direction="vertical" size={12} style={{ width: '100%' }}>
      <Card size="small" title="对账概览">
        <Descriptions size="small" column={4} bordered>
          <Descriptions.Item label="对账批次">{reconcileSummary.total_batches}</Descriptions.Item>
          <Descriptions.Item label="待对账">{reconcileSummary.pending_batches}</Descriptions.Item>
          <Descriptions.Item label="已对账">{reconcileSummary.done_batches}</Descriptions.Item>
          <Descriptions.Item label="应收合计">
            {formatCurrency(reconcileSummary.expected_total_cent)}
          </Descriptions.Item>
          <Descriptions.Item label="差异合计">
            {formatCurrency(reconcileSummary.diff_total_cent)}
          </Descriptions.Item>
        </Descriptions>
      </Card>
      <Card size="small" title="对账汇总">
        <Table
          rowKey="batch_id"
          columns={reconcileColumns}
          dataSource={reconcileItems}
          loading={reconcileListQuery.isFetching}
          pagination={false}
        />
      </Card>
    </Space>
  );

  return (
    <>
      <WorkbenchListPage
        query={activeTab === 'settlement' ? settlementQueryPanel : reconcileQueryPanel}
        toolbarLeft={
          <Typography.Title level={4} style={{ margin: 0 }}>
            {pageTitle}
          </Typography.Title>
        }
        toolbarRight={
          <UiModule02ToolbarRight
            onExport={() => {
              const target = settlementItems.find((item) => item.order_id === activeOrderId);
              if (activeTab !== 'settlement' || !target) {
                message.info('请先打开需要导出的结算单详情');
                return;
              }
              exportMutation.mutate(target);
            }}
            onPrint={() => message.info('打印模板待接入')}
            onHelp={() => message.info('请使用行内操作完成锁定、导出与对账结案')}
          />
        }
        table={
          <Tabs
            activeKey={activeTab}
            onChange={(key) => {
              setActiveTab(key === 'reconcile' ? 'reconcile' : 'settlement');
              setDetailOpen(false);
              setActiveOrderId(null);
              setActiveBatchId(null);
            }}
            items={[
              { key: 'settlement', label: '订单结算', children: settlementTabContent },
              { key: 'reconcile', label: '销售对账', children: reconcileTabContent },
            ]}
            tabBarStyle={{ marginBottom: 12 }}
          />
        }
        showRightPanel={false}
        showBottomPanel={false}
      />

      <Modal
        title="结算明细"
        open={detailOpen}
        onCancel={() => {
          setDetailOpen(false);
          setActiveOrderId(null);
        }}
        footer={[
          <Button
            key="close"
            onClick={() => {
              setDetailOpen(false);
              setActiveOrderId(null);
            }}
          >
            关闭
          </Button>,
        ]}
        width={760}
      >
        {settlementDetailQuery.isLoading ? (
          <div style={{ display: 'flex', justifyContent: 'center', padding: 48 }}>
            <Spin />
          </div>
        ) : settlementDetailQuery.data ? (
          <SettlementDetailContent detail={settlementDetailQuery.data} />
        ) : (
          <Empty description="未获取到结算详情" />
        )}
      </Modal>

      <Modal
        title="对账详情"
        open={Boolean(activeBatchId)}
        onCancel={() => setActiveBatchId(null)}
        footer={[
          <Button key="close" onClick={() => setActiveBatchId(null)}>
            关闭
          </Button>,
        ]}
        width={860}
      >
        {reconcileDetailQuery.isLoading ? (
          <div style={{ display: 'flex', justifyContent: 'center', padding: 48 }}>
            <Spin />
          </div>
        ) : reconcileDetailQuery.data ? (
          <ReconcileDetailContent detail={reconcileDetailQuery.data} />
        ) : (
          <Empty description="未获取到对账详情" />
        )}
      </Modal>
    </>
  );
};

const SettlementDetailContent = ({ detail }: { detail: OrderSettlementDetail }) => (
  <Space direction="vertical" size={16} style={{ width: '100%' }}>
    <Descriptions size="small" bordered column={2}>
      <Descriptions.Item label="订单号">{detail.order_no}</Descriptions.Item>
      <Descriptions.Item label="平台">{detail.platform}</Descriptions.Item>
      <Descriptions.Item label="店铺">{detail.shop_id}</Descriptions.Item>
      <Descriptions.Item label="收件人">{detail.receiver_name || '-'}</Descriptions.Item>
      <Descriptions.Item label="下单时间">{formatDateTime(detail.created_at)}</Descriptions.Item>
      <Descriptions.Item label="结算状态">
        <Tag color={settlementStatusColors[detail.settlement_status] ?? 'default'}>
          {settlementStatusLabels[detail.settlement_status] ?? detail.settlement_status}
        </Tag>
      </Descriptions.Item>
      <Descriptions.Item label="订单金额">
        {formatCurrency(detail.total_amount_cent)}
      </Descriptions.Item>
      <Descriptions.Item label="优惠金额">
        {formatCurrency(detail.discount_amount_cent)}
      </Descriptions.Item>
      <Descriptions.Item label="应收金额">
        {formatCurrency(detail.receivable_amount_cent)}
      </Descriptions.Item>
      <Descriptions.Item label="锁定时间">{formatDateTime(detail.locked_at)}</Descriptions.Item>
      <Descriptions.Item label="导出任务">{detail.export_job_id || '-'}</Descriptions.Item>
      <Descriptions.Item label="最近动作">{detail.latest_action || '-'}</Descriptions.Item>
    </Descriptions>

    <Table
      rowKey={(record) => `${record.promotion_type}-${record.name}`}
      pagination={false}
      dataSource={detail.promotions}
      columns={[
        { title: '促销名称', dataIndex: 'name' },
        { title: '类型', dataIndex: 'promotion_type' },
        { title: '比例', dataIndex: 'rate', render: (value: number) => `${value}%` },
        {
          title: '金额',
          dataIndex: 'amount_cent',
          render: (value: number) => formatCurrency(value),
        },
      ]}
    />

    <Table
      rowKey={(record) => `${record.action}-${record.created_at}`}
      pagination={false}
      dataSource={detail.action_logs}
      locale={{ emptyText: '暂无结算动作记录' }}
      columns={[
        { title: '动作', dataIndex: 'action' },
        { title: '说明', dataIndex: 'message' },
        {
          title: '操作人',
          render: (_, record) => record.operator_name || record.operator_id || '系统',
        },
        {
          title: '时间',
          dataIndex: 'created_at',
          render: (value: string) => formatDateTime(value),
        },
      ]}
    />
  </Space>
);

const ReconcileDetailContent = ({ detail }: { detail: OrderReconciliationDetail }) => (
  <Space direction="vertical" size={16} style={{ width: '100%' }}>
    <Descriptions size="small" bordered column={2}>
      <Descriptions.Item label="渠道">{detail.platform}</Descriptions.Item>
      <Descriptions.Item label="店铺">{detail.shop_id}</Descriptions.Item>
      <Descriptions.Item label="期间">
        {detail.period_start} ~ {detail.period_end}
      </Descriptions.Item>
      <Descriptions.Item label="批次状态">
        <Tag color={detail.status === 'DONE' ? 'green' : 'blue'}>
          {reconcileStatusLabels[detail.status] ?? detail.status}
        </Tag>
      </Descriptions.Item>
      <Descriptions.Item label="订单数">{detail.order_count}</Descriptions.Item>
      <Descriptions.Item label="应收金额">{formatCurrency(detail.expected_cent)}</Descriptions.Item>
      <Descriptions.Item label="实收金额">{formatCurrency(detail.actual_cent)}</Descriptions.Item>
      <Descriptions.Item label="差异金额">{formatCurrency(detail.diff_cent)}</Descriptions.Item>
      <Descriptions.Item label="结案时间">{formatDateTime(detail.settled_at)}</Descriptions.Item>
      <Descriptions.Item label="结案备注">{detail.note || '-'}</Descriptions.Item>
    </Descriptions>

    <Space size={[4, 4]} wrap>
      {detail.reasons.length > 0
        ? detail.reasons.map((reason) => (
            <Tag key={reason}>{diffReasonLabels[reason] ?? reason}</Tag>
          ))
        : <Tag color="green">无差异原因</Tag>}
    </Space>

    <Table
      rowKey="order_id"
      pagination={false}
      dataSource={detail.orders}
      columns={[
        { title: '订单号', dataIndex: 'order_no', ellipsis: true },
        {
          title: '收件人',
          dataIndex: 'receiver_name',
          render: (value?: string | null) => value || '-',
        },
        {
          title: '订单金额',
          dataIndex: 'total_amount_cent',
          render: (value: number) => formatCurrency(value),
        },
        {
          title: '应收金额',
          dataIndex: 'receivable_amount_cent',
          render: (value: number) => formatCurrency(value),
        },
        { title: '结算状态', dataIndex: 'settlement_status' },
        { title: '审核状态', dataIndex: 'audit_status' },
        { title: '异常状态', dataIndex: 'exception_status' },
        { title: '阻塞原因', dataIndex: 'blocking_reason', ellipsis: true },
      ]}
    />
  </Space>
);
