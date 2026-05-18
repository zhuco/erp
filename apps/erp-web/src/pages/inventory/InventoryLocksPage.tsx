import {
  Alert,
  Button,
  Col,
  DatePicker,
  Descriptions,
  Drawer,
  Form,
  Input,
  Modal,
  Radio,
  Row,
  Select,
  Space,
  Switch,
  Tag,
  Tooltip,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { ExportOutlined, ReloadOutlined, SafetyOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useSearchParams } from 'react-router-dom';

import type {
  InventoryLockDetail,
  InventoryLockLog,
  InventoryLockLogListQuery,
  InventoryLockType,
  NegativeStockPolicy,
  NegativeStockPolicyUpdatePayload
} from '@/api/inventoryLocks';
import {
  getInventoryLockDetail,
  listInventoryLockLogs,
  listNegativeStockPolicies,
  prioritizeInventoryLock,
  recalculateInventoryLocks,
  releaseInventoryLock,
  updateNegativeStockPolicy
} from '@/api/inventoryLocks';
import { createExportJob } from '@/api/exports';
import { listWarehouses } from '@/api/warehouses';
import { EmptyState } from '@/components/EmptyState';
import { ErrorState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  sku_id?: string;
  sku_code?: string;
  warehouse_id?: string;
  doc_no?: string;
  lock_type?: InventoryLockType;
  time_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

type RecalcFormValues = {
  scope: 'GLOBAL' | 'TARGET';
  sku_id?: string;
  warehouse_id?: string;
  reason?: string;
};

type PolicyFormValues = NegativeStockPolicyUpdatePayload;
type InventoryLockDocument = InventoryLockDetail['documents'][number];
type InventoryLockAuditEntry = InventoryLockDetail['audit_trail'][number];

const exportThreshold = 5000;

const lockTypeOptions = [
  { value: 'STANDARD', label: '常规' },
  { value: 'PRIORITY', label: '优先' },
  { value: 'PRE_SALE', label: '预售' },
  { value: 'TRANSFER', label: '调拨' },
  { value: 'SAFETY', label: '安全库存' }
];

const lockTypeColors: Record<string, string> = {
  STANDARD: 'default',
  PRIORITY: 'red',
  PRE_SALE: 'blue',
  TRANSFER: 'cyan',
  SAFETY: 'gold'
};

const sourceTypeLabels: Record<string, string> = {
  ORDER: '订单',
  OUTBOUND: '出库单',
  PRE_SALE: '预售',
  TRANSFER: '调拨',
  RETURN: '退货',
  MANUAL: '手工'
};

const statusLabels: Record<string, string> = {
  ACTIVE: '锁定中',
  RELEASED: '已释放',
  OVERRIDDEN: '已抢占'
};

const lockModelNotes = [
  { title: '锁定来源', description: '订单审核、出库单、预售、调拨、退货返库、手工调整。' },
  {
    title: '优先级',
    description: '加急抢占 > 已审核订单 > 预售 > 调拨；仓库可配置强制锁定优先级。'
  },
  {
    title: '释放规则',
    description: '单据取消/关闭、出库完成、超期释放或人工释放会回滚可用库存。'
  }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const InventoryLocksPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const [searchParams] = useSearchParams();
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const canWrite = useCan('inventory.write');
  const canExport = useCan('exports.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [recalcForm] = Form.useForm<RecalcFormValues>();
  const [policyForm] = Form.useForm<PolicyFormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [detailOpen, setDetailOpen] = useState(false);
  const [selectedLog, setSelectedLog] = useState<InventoryLockLog | null>(null);
  const [recalcOpen, setRecalcOpen] = useState(false);
  const [policyOpen, setPolicyOpen] = useState(false);
  const [exporting, setExporting] = useState(false);

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'inventory-locks'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const warehouseOptions = useMemo(
    () =>
      warehouseQuery.data?.items.map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })) ?? [],
    [warehouseQuery.data?.items]
  );

  const warehouseMap = useMemo(() => {
    const entries = warehouseQuery.data?.items ?? [];
    return new Map(entries.map((warehouse) => [warehouse.warehouse_id, warehouse.name]));
  }, [warehouseQuery.data?.items]);

  const query = useMemo<InventoryLockLogListQuery>(
    () => ({
      sku_id: filters.sku_id?.trim() || undefined,
      sku_code: filters.sku_code?.trim() || undefined,
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      doc_no: filters.doc_no?.trim() || undefined,
      lock_type: filters.lock_type,
      occurred_from: filters.time_range?.[0]?.toISOString(),
      occurred_to: filters.time_range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const logQuery = useQuery({
    queryKey: ['inventory-locks', tenantId, query],
    queryFn: () => listInventoryLockLogs(tenantId, query),
    enabled: tenantReady && inventoryFeature.enabled,
    placeholderData: (previousData) => previousData,
    retry: false
  });

  const detailQuery = useQuery({
    queryKey: ['inventory-lock-detail', tenantId, selectedLog?.lock_id],
    queryFn: () => getInventoryLockDetail(tenantId, selectedLog?.lock_id ?? ''),
    enabled: tenantReady && detailOpen && Boolean(selectedLog?.lock_id),
    retry: false
  });

  const policyQuery = useQuery({
    queryKey: ['inventory-negative-policy', tenantId],
    queryFn: () => listNegativeStockPolicies(tenantId, {}),
    enabled: tenantReady && policyOpen,
    retry: false
  });

  useEffect(() => {
    const initialSku = searchParams.get('sku_id') ?? undefined;
    const initialWarehouse = searchParams.get('warehouse_id') ?? undefined;
    const initialDocNo = searchParams.get('doc_no') ?? undefined;
    if (initialSku || initialWarehouse || initialDocNo) {
      const initial: FilterValues = {
        sku_id: initialSku,
        warehouse_id: initialWarehouse,
        doc_no: initialDocNo
      };
      setFilters(initial);
      form.setFieldsValue(initial);
    }
  }, [form, searchParams]);

  const recalcMutation = useMutation({
    mutationFn: (payload: RecalcFormValues) =>
      recalculateInventoryLocks(tenantId, payload),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['inventory-locks', tenantId] });
      if (selectedLog?.lock_id) {
        await queryClient.invalidateQueries({
          queryKey: ['inventory-lock-detail', tenantId, selectedLog.lock_id]
        });
      }
      setRecalcOpen(false);
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '重算锁定失败');
    }
  });

  const priorityMutation = useMutation({
    mutationFn: (payload: { lock_id: string; reason?: string }) =>
      prioritizeInventoryLock(tenantId, payload),
    onSuccess: async () => {
      message.success('已执行优先锁定');
      await queryClient.invalidateQueries({ queryKey: ['inventory-locks', tenantId] });
      if (selectedLog?.lock_id) {
        await detailQuery.refetch();
      }
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '抢占库存失败');
    }
  });

  const releaseMutation = useMutation({
    mutationFn: (payload: { lock_id: string; reason?: string }) =>
      releaseInventoryLock(tenantId, payload),
    onSuccess: async () => {
      message.success('锁定已释放');
      await queryClient.invalidateQueries({ queryKey: ['inventory-locks', tenantId] });
      if (selectedLog?.lock_id) {
        await detailQuery.refetch();
      }
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '释放失败');
    }
  });

  const policyMutation = useMutation({
    mutationFn: (payload: PolicyFormValues) => updateNegativeStockPolicy(tenantId, payload),
    onSuccess: async () => {
      message.success('负库存策略已更新');
      await policyQuery.refetch();
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '更新负库存策略失败');
    }
  });

  const columns = useMemo<ColumnsType<InventoryLockLog>>(
    () => [
      {
        title: '时间',
        dataIndex: 'occurred_at',
        render: (value) => formatDateTime(value)
      },
      {
        title: '货品/SKU',
        dataIndex: 'sku_code',
        render: (_, record) => record.sku_code ?? record.sku_id
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value: string) => warehouseMap.get(value) ?? value
      },
      {
        title: '锁定数量',
        dataIndex: 'locked_qty'
      },
      {
        title: '解锁数量',
        dataIndex: 'unlocked_qty'
      },
      {
        title: '锁定后可用',
        dataIndex: 'available_after'
      },
      {
        title: '锁定来源',
        dataIndex: 'source_type',
        render: (value: string) => sourceTypeLabels[value] ?? value
      },
      {
        title: '来源单号',
        dataIndex: 'source_doc_no'
      },
      {
        title: '锁定类型',
        dataIndex: 'lock_type',
        render: (value: string) => (
          <Tag color={lockTypeColors[value] ?? 'default'}>
            {lockTypeOptions.find((item) => item.value === value)?.label ?? value}
          </Tag>
        )
      },
      {
        title: '操作人/系统',
        dataIndex: 'operator_name',
        render: (_, record) =>
          record.operator_name ?? (record.operator_type === 'SYSTEM' ? '系统' : '-')
      },
      {
        title: '操作',
        key: 'action',
        render: (_, record) => (
          <Button
            size="small"
            onClick={() => {
              setSelectedLog(record);
              setDetailOpen(true);
            }}
          >
            查看详情
          </Button>
        )
      }
    ],
    [warehouseMap]
  );

  const docColumns = useMemo<ColumnsType<InventoryLockDocument>>(
    () => [
      { title: '单据类型', dataIndex: 'doc_type' },
      { title: '单据号', dataIndex: 'doc_no' },
      { title: '数量', dataIndex: 'qty' },
      { title: '优先级', dataIndex: 'priority' },
      { title: '状态', dataIndex: 'status' }
    ],
    []
  );

  const auditColumns = useMemo<ColumnsType<InventoryLockAuditEntry>>(
    () => [
      { title: '时间', dataIndex: 'created_at', render: (value) => formatDateTime(value) },
      { title: '操作', dataIndex: 'action' },
      { title: '操作人', dataIndex: 'operator' },
      { title: '说明', dataIndex: 'note' }
    ],
    []
  );

  const policyColumns = useMemo<ColumnsType<NegativeStockPolicy>>(
    () => [
      {
        title: '范围',
        dataIndex: 'scope',
        render: (value: string) => (value === 'SKU' ? '货品+仓库' : '仓库全局')
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value: string) => warehouseMap.get(value) ?? value
      },
      {
        title: 'SKU',
        dataIndex: 'sku_id',
        render: (value?: string | null) => value ?? '-'
      },
      {
        title: '允许负库存',
        dataIndex: 'allow_negative',
        render: (value: boolean) => (
          <Tag color={value ? 'green' : 'red'}>{value ? '允许' : '禁止'}</Tag>
        )
      },
      {
        title: '更新人',
        dataIndex: 'updated_by',
        render: (value?: string | null) => value ?? '-'
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value?: string | null) => formatDateTime(value)
      }
    ],
    [warehouseMap]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleOpenRecalc = () => {
    const hasTarget = Boolean(filters.sku_id || filters.warehouse_id);
    recalcForm.setFieldsValue({
      scope: hasTarget ? 'TARGET' : 'GLOBAL',
      sku_id: filters.sku_id,
      warehouse_id: filters.warehouse_id,
      reason: ''
    });
    setRecalcOpen(true);
  };

  const handleRecalcSubmit = async () => {
    const values = await recalcForm.validateFields();
    if (values.scope === 'TARGET' && !values.sku_id && !values.warehouse_id) {
      message.warning('请至少指定 SKU 或仓库');
      return;
    }
    message.info({ content: '正在重算锁定...', key: 'recalc' });
    recalcMutation.mutate(values, {
      onSuccess: (result) => {
        message.success({
          content: `锁定已重算：${result.recalculated} 条记录`,
          key: 'recalc'
        });
      },
      onError: (err) => {
        message.error({
          content: err instanceof Error ? err.message : '重算失败',
          key: 'recalc'
        });
      }
    });
  };

  const handleExport = async () => {
    if (!canExport) {
      message.warning('暂无导出权限');
      return;
    }
    setExporting(true);
    try {
      const total = logQuery.data?.total ?? 0;
      const job = await createExportJob({
        type: 'INVENTORY_LOCKS_EXPORT',
        params: query
      });
      if (total > exportThreshold) {
        message.success(`导出任务已创建：${job.job_id}，请到导出中心下载`);
      } else {
        message.success(`导出任务已提交：${job.job_id}`);
      }
    } catch (err) {
      message.error(err instanceof Error ? err.message : '导出失败');
    } finally {
      setExporting(false);
    }
  };

  const handlePriorityLock = (detail?: InventoryLockDetail | null) => {
    if (!detail?.lock_id) {
      return;
    }
    if (!canWrite) {
      message.warning('暂无抢占权限');
      return;
    }
    let note = '';
    Modal.confirm({
      title: '确认抢占锁定',
      content: (
        <Input.TextArea
          placeholder="填写抢占原因（用于审计）"
          onChange={(event) => {
            note = event.target.value;
          }}
        />
      ),
      onOk: () => priorityMutation.mutate({ lock_id: detail.lock_id, reason: note })
    });
  };

  const handleReleaseLock = (detail?: InventoryLockDetail | null) => {
    if (!detail?.lock_id) {
      return;
    }
    if (!canWrite) {
      message.warning('暂无释放权限');
      return;
    }
    let note = '';
    Modal.confirm({
      title: '确认释放锁定',
      content: (
        <Input.TextArea
          placeholder="填写释放原因（用于审计）"
          onChange={(event) => {
            note = event.target.value;
          }}
        />
      ),
      onOk: () => releaseMutation.mutate({ lock_id: detail.lock_id, reason: note })
    });
  };

  const policyScope = Form.useWatch('scope', policyForm) ?? 'WAREHOUSE';

  const handlePolicySubmit = async () => {
    if (!canWrite) {
      message.warning('暂无配置权限');
      return;
    }
    const values = await policyForm.validateFields();
    if (values.scope === 'SKU' && !values.sku_id) {
      message.warning('请填写 SKU ID');
      return;
    }
    policyMutation.mutate(values);
  };

  const handlePolicyRowClick = (record: NegativeStockPolicy) => {
    policyForm.setFieldsValue({
      scope: record.scope,
      warehouse_id: record.warehouse_id,
      sku_id: record.sku_id ?? undefined,
      sku_code: record.sku_code ?? undefined,
      allow_negative: record.allow_negative,
      note: record.note ?? undefined
    });
  };

  const detail = detailQuery.data;
  const policyRows = policyQuery.data?.items ?? [];

  return (
    <>
      {!inventoryFeature.enabled ? (
        <Alert
          type="warning"
          showIcon
          message="库存功能未启用"
          description="请先在系统设置中启用库存管理后使用锁定诊断。"
        />
      ) : null}
      {warehouseQuery.error ? (
        <ErrorState
          title="仓库加载失败"
          error={warehouseQuery.error}
          onRetry={() => warehouseQuery.refetch()}
        />
      ) : null}
      <UiModule03MainTableSection<InventoryLockLog>
        toolbarLeft={
          <Space direction="vertical" size={2}>
            <Typography.Title level={3} style={{ margin: 0 }}>
              库存锁定
            </Typography.Title>
            <Typography.Text type="secondary">锁定日志、占用来源与释放审计统一诊断。</Typography.Text>
          </Space>
        }
        toolbarRight={
          <Space>
            <Tooltip title={!canWrite ? '暂无权限' : undefined}>
              <Button
                data-testid="inventory-locks-recalc-button"
                type="primary"
                icon={<ReloadOutlined />}
                onClick={handleOpenRecalc}
                loading={recalcMutation.isPending}
                disabled={!canWrite}
              >
                重算锁定
              </Button>
            </Tooltip>
            <Tooltip title={!canExport ? '暂无权限' : undefined}>
              <Button
                data-testid="inventory-locks-export-button"
                icon={<ExportOutlined />}
                onClick={handleExport}
                loading={exporting}
                disabled={!canExport}
              >
                导出
              </Button>
            </Tooltip>
          </Space>
        }
        actionBar={
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Form
              data-testid="inventory-locks-filter-form"
              form={form}
              layout="vertical"
              onFinish={handleSearch}
            >
              <Row gutter={[16, 16]}>
                <Col xs={24} md={6}>
                  <Form.Item name="warehouse_id" label="仓库">
                    <Select
                      allowClear
                      options={warehouseOptions}
                      placeholder="选择仓库"
                      loading={warehouseQuery.isFetching}
                    />
                  </Form.Item>
                </Col>
                <Col xs={24} md={6}>
                  <Form.Item name="sku_id" label="SKU ID">
                    <Input allowClear placeholder="输入 SKU ID" />
                  </Form.Item>
                </Col>
                <Col xs={24} md={6}>
                  <Form.Item name="sku_code" label="SKU 编码">
                    <Input allowClear placeholder="输入 SKU 编码" />
                  </Form.Item>
                </Col>
                <Col xs={24} md={6}>
                  <Form.Item name="doc_no" label="来源单号">
                    <Input allowClear placeholder="输入单据号" />
                  </Form.Item>
                </Col>
                <Col xs={24} md={6}>
                  <Form.Item name="lock_type" label="锁定类型">
                    <Select allowClear options={lockTypeOptions} placeholder="选择类型" />
                  </Form.Item>
                </Col>
                <Col xs={24} md={10}>
                  <Form.Item name="time_range" label="日期范围">
                    <DatePicker.RangePicker
                      showTime
                      style={{ width: '100%' }}
                      placeholder={['开始时间', '结束时间']}
                    />
                  </Form.Item>
                </Col>
                <Col xs={24} md={8} style={{ display: 'flex', alignItems: 'end' }}>
                  <Space wrap>
                    <Button data-testid="inventory-locks-search-button" type="primary" htmlType="submit">
                      查询
                    </Button>
                    <Button data-testid="inventory-locks-reset-button" onClick={handleReset}>
                      重置
                    </Button>
                    <Button
                      data-testid="inventory-locks-policy-button"
                      icon={<SafetyOutlined />}
                      onClick={() => {
                        policyForm.resetFields();
                        setPolicyOpen(true);
                      }}
                    >
                      负库存策略
                    </Button>
                  </Space>
                </Col>
              </Row>
            </Form>
            <Space direction="vertical" size={6}>
              {lockModelNotes.map((item) => (
                <Typography.Text key={item.title} type="secondary">
                  {item.title}：{item.description}
                </Typography.Text>
              ))}
            </Space>
          </Space>
        }
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        rowKey="lock_id"
        columns={columns}
        data={logQuery.data?.items ?? []}
        loading={logQuery.isFetching}
        error={logQuery.error}
        onRetry={() => logQuery.refetch()}
        errorTitle="锁定日志加载失败"
        emptyText="暂无锁定日志"
        total={logQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="inventory.locks.table.columns"
        showColumnSettings={true}
        dataTestId="inventory-locks-main-grid"
        enablePanels={false}
      />
      <Drawer
        title="锁定详情"
        width={720}
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        extra={
          <Space>
            <Tooltip title={!canWrite ? '暂无权限' : undefined}>
              <Button
                type="primary"
                onClick={() => handlePriorityLock(detail)}
                loading={priorityMutation.isPending}
                disabled={!canWrite}
              >
                抢占库存
              </Button>
            </Tooltip>
            <Tooltip title={!canWrite ? '暂无权限' : undefined}>
              <Button
                onClick={() => handleReleaseLock(detail)}
                loading={releaseMutation.isPending}
                disabled={!canWrite}
              >
                释放锁定
              </Button>
            </Tooltip>
          </Space>
        }
      >
        {detailQuery.isFetching ? (
          <Typography.Text type="secondary">详情加载中...</Typography.Text>
        ) : detail ? (
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Descriptions size="small" column={2}>
              <Descriptions.Item label="锁定单号">{detail.lock_id}</Descriptions.Item>
              <Descriptions.Item label="状态">
                {statusLabels[detail.status] ?? detail.status}
              </Descriptions.Item>
              <Descriptions.Item label="货品/SKU">
                {detail.sku_code ?? detail.sku_id}
              </Descriptions.Item>
              <Descriptions.Item label="仓库">
                {warehouseMap.get(detail.warehouse_id) ?? detail.warehouse_id}
              </Descriptions.Item>
              <Descriptions.Item label="锁定数量">{detail.locked_qty}</Descriptions.Item>
              <Descriptions.Item label="解锁数量">{detail.unlocked_qty}</Descriptions.Item>
              <Descriptions.Item label="锁定后可用">{detail.available_after}</Descriptions.Item>
              <Descriptions.Item label="锁定来源">
                {sourceTypeLabels[detail.source_type] ?? detail.source_type}
              </Descriptions.Item>
              <Descriptions.Item label="来源单号">{detail.source_doc_no}</Descriptions.Item>
              <Descriptions.Item label="锁定类型">
                {lockTypeOptions.find((item) => item.value === detail.lock_type)?.label ??
                  detail.lock_type}
              </Descriptions.Item>
              <Descriptions.Item label="优先级">{detail.priority}</Descriptions.Item>
              <Descriptions.Item label="操作人">{detail.operator_name ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="更新时间">
                {formatDateTime(detail.updated_at)}
              </Descriptions.Item>
            </Descriptions>
            <UiModule03MainTableSection<InventoryLockDocument>
              toolbarLeft={
                <Space direction="vertical" size={2}>
                  <Typography.Title level={5} style={{ margin: 0 }}>
                    关联单据
                  </Typography.Title>
                  <Typography.Text type="secondary">查看锁定来源单据与优先级。</Typography.Text>
                </Space>
              }
              gridPreset="industrial-review-compact"
              selectionCellMode="custom"
              selectionHeaderMode="allInvert"
              rowKey={(record) => `${record.doc_type}-${record.doc_no}`}
              columns={docColumns}
              data={detail.documents}
              loading={false}
              error={null}
              onRetry={() => undefined}
              emptyText="暂无关联单据"
              total={detail.documents.length}
              pagination={{ current: 1, pageSize: detail.documents.length || 10 }}
              onPaginationChange={() => undefined}
              showColumnSettings={false}
              showQueryRow={false}
              tableFooter={null}
              dataTestId="inventory-locks-documents-grid"
              enablePanels={false}
            />
            <UiModule03MainTableSection<InventoryLockAuditEntry>
              toolbarLeft={
                <Space direction="vertical" size={2}>
                  <Typography.Title level={5} style={{ margin: 0 }}>
                    审计记录
                  </Typography.Title>
                  <Typography.Text type="secondary">查看锁定变更、释放与抢占审计。</Typography.Text>
                </Space>
              }
              gridPreset="industrial-review-compact"
              selectionCellMode="custom"
              selectionHeaderMode="allInvert"
              rowKey={(record) => `${record.action}-${record.created_at}`}
              columns={auditColumns}
              data={detail.audit_trail}
              loading={false}
              error={null}
              onRetry={() => undefined}
              emptyText="暂无审计记录"
              total={detail.audit_trail.length}
              pagination={{ current: 1, pageSize: detail.audit_trail.length || 10 }}
              onPaginationChange={() => undefined}
              showColumnSettings={false}
              showQueryRow={false}
              tableFooter={null}
              dataTestId="inventory-locks-audit-grid"
              enablePanels={false}
            />
          </Space>
        ) : (
          <EmptyState description="暂无锁定详情" />
        )}
      </Drawer>
      <Modal
        title="重算锁定"
        open={recalcOpen}
        onCancel={() => setRecalcOpen(false)}
        onOk={handleRecalcSubmit}
        okButtonProps={{ loading: recalcMutation.isPending, disabled: !canWrite }}
        destroyOnHidden
      >
        <Form form={recalcForm} layout="vertical" initialValues={{ scope: 'GLOBAL' }}>
          <Form.Item name="scope" label="范围">
            <Radio.Group>
              <Radio value="GLOBAL">全局重算</Radio>
              <Radio value="TARGET">按货品/仓库</Radio>
            </Radio.Group>
          </Form.Item>
          <Form.Item name="warehouse_id" label="仓库">
            <Select allowClear options={warehouseOptions} placeholder="选择仓库" />
          </Form.Item>
          <Form.Item name="sku_id" label="SKU ID">
            <Input allowClear placeholder="输入 SKU ID" />
          </Form.Item>
          <Form.Item name="reason" label="重算原因">
            <Input.TextArea rows={2} placeholder="填写重算原因" />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="负库存策略"
        open={policyOpen}
        onCancel={() => setPolicyOpen(false)}
        onOk={handlePolicySubmit}
        okButtonProps={{ loading: policyMutation.isPending, disabled: !canWrite }}
        destroyOnHidden
        width={720}
      >
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <Form
            form={policyForm}
            layout="vertical"
            initialValues={{ scope: 'WAREHOUSE', allow_negative: false }}
          >
            <Row gutter={16}>
              <Col xs={24} md={8}>
                <Form.Item name="scope" label="范围">
                  <Select
                    options={[
                      { value: 'WAREHOUSE', label: '仓库全局' },
                      { value: 'SKU', label: '货品+仓库' }
                    ]}
                  />
                </Form.Item>
              </Col>
              <Col xs={24} md={8}>
                <Form.Item
                  name="warehouse_id"
                  label="仓库"
                  rules={[{ required: true, message: '请选择仓库' }]}
                >
                  <Select options={warehouseOptions} placeholder="选择仓库" />
                </Form.Item>
              </Col>
              <Col xs={24} md={8}>
                <Form.Item
                  name="sku_id"
                  label="SKU ID"
                  rules={
                    policyScope === 'SKU'
                      ? [{ required: true, message: '请填写 SKU ID' }]
                      : []
                  }
                >
                  <Input disabled={policyScope !== 'SKU'} placeholder="SKU ID" />
                </Form.Item>
              </Col>
              <Col xs={24} md={8}>
                <Form.Item name="sku_code" label="SKU 编码">
                  <Input disabled={policyScope !== 'SKU'} placeholder="SKU 编码" />
                </Form.Item>
              </Col>
              <Col xs={24} md={8}>
                <Form.Item
                  name="allow_negative"
                  label="允许负库存"
                  valuePropName="checked"
                >
                  <Switch checkedChildren="允许" unCheckedChildren="禁止" />
                </Form.Item>
              </Col>
              <Col xs={24} md={8}>
                <Form.Item name="note" label="备注">
                  <Input placeholder="说明" />
                </Form.Item>
              </Col>
            </Row>
          </Form>
          <UiModule03MainTableSection<NegativeStockPolicy>
            toolbarLeft={
              <Space direction="vertical" size={2}>
                <Typography.Title level={5} style={{ margin: 0 }}>
                  现行策略
                </Typography.Title>
                <Typography.Text type="secondary">点击行可回填上方策略配置。</Typography.Text>
              </Space>
            }
            gridPreset="industrial-review-compact"
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            rowKey="policy_id"
            columns={policyColumns}
            data={policyRows}
            loading={policyQuery.isFetching}
            error={policyQuery.error}
            onRetry={() => policyQuery.refetch()}
            errorTitle="负库存策略加载失败"
            emptyText="暂无负库存策略"
            total={policyRows.length}
            pagination={{ current: 1, pageSize: policyRows.length || 10 }}
            onPaginationChange={() => undefined}
            showColumnSettings={false}
            showQueryRow={false}
            tableFooter={null}
            dataTestId="inventory-locks-policy-grid"
            enablePanels={false}
            onRow={(record) => ({
              onClick: () => handlePolicyRowClick(record)
            })}
          />
        </Space>
      </Modal>
    </>
  );
};
