import {
  Alert,
  Button,
  Card,
  Col,
  DatePicker,
  Descriptions,
  Drawer,
  Form,
  Input,
  Popconfirm,
  Progress,
  Row,
  Select,
  Skeleton,
  Space,
  Statistic,
  Switch,
  Table,
  Tabs,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import {
  ExportOutlined,
  QuestionCircleOutlined,
  ReloadOutlined,
  SettingOutlined
} from '@ant-design/icons';
import type { Dayjs } from 'dayjs';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import { createExportJob } from '@/api/exports';
import type {
  InventoryAlertLevel,
  InventoryAlertRecord,
  InventoryAlertRule,
  InventoryAlertRulePayload,
  InventoryAlertType
} from '@/api/inventoryInsights';
import {
  createInventoryAlertRule,
  deleteInventoryAlertRule,
  getInventoryAlertSummary,
  getInventoryAmountAnalysis,
  getInventoryTurnoverAnalysis,
  listInventoryAlertRules,
  listInventoryAlerts,
  updateInventoryAlertRule
} from '@/api/inventoryInsights';
import { listWarehouses } from '@/api/warehouses';
import { EmptyState } from '@/components/EmptyState';
import { ErrorState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import type { ServerTableSortState } from '@/components/ServerTable';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

import './inventoryAlertsPage.css';

type FilterValues = {
  dateRange?: [Dayjs, Dayjs];
  warehouse_id?: string;
  category?: string;
  brand?: string;
  keyword?: string;
  alert_level?: InventoryAlertLevel[];
};

type RuleFormValues = InventoryAlertRulePayload;

const alertTypeLabels: Record<InventoryAlertType, string> = {
  OUT_OF_STOCK: '缺货',
  LOW_STOCK: '低库存',
  HIGH_STOCK: '高库存',
  NO_SALES: '无销有库存'
};

const alertLevelLabels: Record<InventoryAlertLevel, string> = {
  HIGH: '高',
  MEDIUM: '中',
  LOW: '低'
};

const alertLevelColors: Record<InventoryAlertLevel, string> = {
  HIGH: 'red',
  MEDIUM: 'orange',
  LOW: 'green'
};

const brandOptions = ['得力', '晨光', '良品铺子', '三只松鼠', '南极人', '洁柔'];
const categoryOptions = ['办公文具', '休闲零食', '家居日用', '服饰配件', '美妆护理'];
const alertTabs: { key: InventoryAlertType; label: string }[] = [
  { key: 'OUT_OF_STOCK', label: '缺货' },
  { key: 'LOW_STOCK', label: '低库存' },
  { key: 'HIGH_STOCK', label: '高库存' },
  { key: 'NO_SALES', label: '无销有库存' }
];

const formatAmount = (value?: number | null) =>
  value == null ? '-' : value.toLocaleString('zh-CN');

const formatDays = (value?: number | null) => (value == null ? '-' : `${value}天`);

export const InventoryAlertsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const insightsFeature = useFeatureFlag('inventory_insights');
  const canExport = useCan('exports.write');
  const [form] = Form.useForm<FilterValues>();
  const [ruleForm] = Form.useForm<RuleFormValues>();
  const defaultFilters = useMemo(
    () => ({
      dateRange: [dayjs().subtract(30, 'day'), dayjs()]
    }),
    []
  );
  const [filters, setFilters] = useState<FilterValues>(defaultFilters);
  const [activeAlertType, setActiveAlertType] =
    useState<InventoryAlertType>('OUT_OF_STOCK');
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });
  const [sortState, setSortState] = useState<ServerTableSortState>({});
  const [detailDrawerOpen, setDetailDrawerOpen] = useState(false);
  const [ruleDrawerOpen, setRuleDrawerOpen] = useState(false);
  const [selectedAlert, setSelectedAlert] = useState<InventoryAlertRecord | null>(null);
  const [editingRule, setEditingRule] = useState<InventoryAlertRule | null>(null);
  const [ruleSubmitting, setRuleSubmitting] = useState(false);
  const [exporting, setExporting] = useState(false);

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'alerts'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && insightsFeature.enabled,
    retry: false
  });

  const warehouseOptions = useMemo(
    () =>
      (warehouseQuery.data?.items ?? []).map((warehouse) => ({
        value: warehouse.warehouse_id,
        label: warehouse.name
      })),
    [warehouseQuery.data?.items]
  );

  const normalizedFilters = useMemo(() => {
    const [from, to] = filters.dateRange ?? [];
    return {
      date_from: from ? from.startOf('day').toISOString() : undefined,
      date_to: to ? to.endOf('day').toISOString() : undefined,
      warehouse_id: filters.warehouse_id,
      category: filters.category,
      brand: filters.brand,
      keyword: filters.keyword?.trim() || undefined,
      alert_level:
        filters.alert_level && filters.alert_level.length > 0 ? filters.alert_level : undefined
    };
  }, [filters]);

  const summaryQuery = useQuery({
    queryKey: ['inventory-alert-summary', tenantId, normalizedFilters],
    queryFn: () => getInventoryAlertSummary(tenantId, normalizedFilters),
    enabled: tenantReady && insightsFeature.enabled,
    retry: false
  });

  const listQuery = useMemo(() => {
    const current = pagination.current ?? 1;
    const pageSize = pagination.pageSize ?? 10;
    const sortField = sortState.field;
    const sortOrder =
      sortState.order === 'ascend'
        ? 'asc'
        : sortState.order === 'descend'
          ? 'desc'
          : undefined;
    return {
      ...normalizedFilters,
      alert_type: activeAlertType,
      offset: (current - 1) * pageSize,
      limit: pageSize,
      sort_field: sortField,
      sort_order: sortOrder
    };
  }, [activeAlertType, normalizedFilters, pagination, sortState]);

  const alertsQuery = useQuery({
    queryKey: ['inventory-alerts', tenantId, listQuery],
    queryFn: () => listInventoryAlerts(tenantId, listQuery),
    enabled: tenantReady && insightsFeature.enabled,
    placeholderData: (previousData) => previousData,
    retry: false
  });

  const amountAnalysisQuery = useQuery({
    queryKey: ['inventory-amount-analysis', tenantId, normalizedFilters],
    queryFn: () => getInventoryAmountAnalysis(tenantId, normalizedFilters),
    enabled: tenantReady && insightsFeature.enabled,
    retry: false
  });

  const turnoverQuery = useQuery({
    queryKey: ['inventory-turnover-analysis', tenantId, normalizedFilters],
    queryFn: () => getInventoryTurnoverAnalysis(tenantId, normalizedFilters),
    enabled: tenantReady && insightsFeature.enabled,
    retry: false
  });

  const rulesQuery = useQuery({
    queryKey: ['inventory-alert-rules', tenantId],
    queryFn: () => listInventoryAlertRules(tenantId, { offset: 0, limit: 50 }),
    enabled: tenantReady && insightsFeature.enabled && ruleDrawerOpen,
    retry: false
  });

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
    setSelectedAlert(null);
    setDetailDrawerOpen(false);
  };

  const handleReset = () => {
    form.resetFields();
    handleSearch(defaultFilters);
  };

  const handleKpiClick = (type: InventoryAlertType) => {
    setActiveAlertType(type);
    setPagination((prev) => ({ ...prev, current: 1 }));
    setSelectedAlert(null);
    setDetailDrawerOpen(false);
  };

  const handleRefresh = () => {
    warehouseQuery.refetch();
    summaryQuery.refetch();
    alertsQuery.refetch();
    amountAnalysisQuery.refetch();
    turnoverQuery.refetch();
    if (ruleDrawerOpen) {
      rulesQuery.refetch();
    }
  };

  const handleExport = async () => {
    if (!canExport) {
      message.warning('暂无导出权限');
      return;
    }
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    setExporting(true);
    try {
      const job = await createExportJob({
        type: 'INVENTORY_ALERTS_EXPORT',
        params: {
          ...normalizedFilters,
          alert_type: activeAlertType
        }
      });
      message.success(`导出任务已提交：${job.job_id}`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导出失败');
    } finally {
      setExporting(false);
    }
  };

  const handleOpenRuleDrawer = () => {
    setRuleDrawerOpen(true);
    rulesQuery.refetch();
  };

  const handleRuleEdit = (rule: InventoryAlertRule) => {
    setEditingRule(rule);
    ruleForm.setFieldsValue({
      name: rule.name,
      warehouse_id: rule.warehouse_id ?? undefined,
      category: rule.category ?? undefined,
      brand: rule.brand ?? undefined,
      sku_keyword: rule.sku_keyword ?? undefined,
      available_qty_threshold: rule.available_qty_threshold,
      available_days_threshold: rule.available_days_threshold,
      alert_level: rule.alert_level,
      enabled: rule.enabled
    });
  };

  const handleRuleReset = () => {
    setEditingRule(null);
    ruleForm.resetFields();
  };

  const handleSubmitRule = async () => {
    if (!tenantId) {
      message.warning('请先选择商家');
      return;
    }
    try {
      const values = await ruleForm.validateFields();
      setRuleSubmitting(true);
      if (editingRule) {
        await updateInventoryAlertRule(tenantId, editingRule.rule_id, values);
        message.success('规则已更新');
      } else {
        await createInventoryAlertRule(tenantId, values);
        message.success('规则已创建');
      }
      handleRuleReset();
      rulesQuery.refetch();
      summaryQuery.refetch();
      alertsQuery.refetch();
    } catch (error) {
      if (error instanceof Error) {
        message.error(error.message);
      }
    } finally {
      setRuleSubmitting(false);
    }
  };

  const handleDeleteRule = async (ruleId: string) => {
    if (!tenantId) {
      message.warning('请先选择商家');
      return;
    }
    try {
      await deleteInventoryAlertRule(tenantId, ruleId);
      message.success('规则已删除');
      rulesQuery.refetch();
      summaryQuery.refetch();
      alertsQuery.refetch();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '删除失败');
    }
  };

  const kpiCards = useMemo(() => {
    const summary = summaryQuery.data;
    return [
      {
        type: 'OUT_OF_STOCK' as InventoryAlertType,
        label: '缺货',
        value: summary?.out_of_stock ?? 0
      },
      {
        type: 'LOW_STOCK' as InventoryAlertType,
        label: '低库存',
        value: summary?.low_stock ?? 0
      },
      {
        type: 'HIGH_STOCK' as InventoryAlertType,
        label: '高库存',
        value: summary?.high_stock ?? 0
      },
      {
        type: 'NO_SALES' as InventoryAlertType,
        label: '无销有库存',
        value: summary?.no_sales ?? 0
      }
    ];
  }, [summaryQuery.data]);

  const alertColumns = useMemo<ColumnsType<InventoryAlertRecord>>(
    () => [
      { title: '货品/SKU', dataIndex: 'sku_code', key: 'sku_code' },
      { title: '货品名称', dataIndex: 'sku_name', key: 'sku_name' },
      { title: '规格', dataIndex: 'spec', key: 'spec' },
      { title: '仓库', dataIndex: 'warehouse_name', key: 'warehouse_name' },
      {
        title: '可售数量',
        dataIndex: 'available_qty',
        key: 'available_qty',
        sorter: true,
        align: 'right'
      },
      {
        title: '可售天数',
        dataIndex: 'available_days',
        key: 'available_days',
        sorter: true,
        align: 'right',
        render: (value: number | null) => formatDays(value)
      },
      {
        title: '近30天销量',
        dataIndex: 'sales_30d',
        key: 'sales_30d',
        sorter: true,
        align: 'right',
        render: (value: number | null) => (value == null ? '-' : value)
      },
      {
        title: '预警类型',
        dataIndex: 'alert_type',
        key: 'alert_type',
        render: (value: InventoryAlertType) => alertTypeLabels[value] ?? value
      },
      {
        title: '预警等级',
        dataIndex: 'alert_level',
        key: 'alert_level',
        render: (value: InventoryAlertLevel) => (
          <Tag color={alertLevelColors[value]}>{alertLevelLabels[value] ?? value}</Tag>
        )
      },
      { title: '建议动作', dataIndex: 'suggestion', key: 'suggestion' }
    ],
    []
  );

  const rankingColumns: ColumnsType<Record<string, unknown>> = [
    {
      title: '排名',
      dataIndex: 'rank',
      width: 80,
      render: (_: number, __: unknown, index: number) => index + 1
    },
    { title: '对象', dataIndex: 'dimension_name' },
    {
      title: '库存金额',
      dataIndex: 'inventory_amount',
      align: 'right',
      render: (value: number) => formatAmount(value)
    },
    {
      title: '占比',
      dataIndex: 'share',
      align: 'right',
      render: (value: number) => <Progress percent={Math.round(value * 100)} size="small" />
    }
  ];

  const shareColumns: ColumnsType<Record<string, unknown>> = [
    { title: '对象', dataIndex: 'dimension_name' },
    {
      title: '库存量',
      dataIndex: 'inventory_qty',
      align: 'right'
    },
    {
      title: '库存金额',
      dataIndex: 'inventory_amount',
      align: 'right',
      render: (value: number) => formatAmount(value)
    },
    {
      title: '占比',
      dataIndex: 'share',
      align: 'right',
      render: (value: number) => <Progress percent={Math.round(value * 100)} size="small" />
    }
  ];

  const trendColumns: ColumnsType<Record<string, unknown>> = [
    { title: '月份', dataIndex: 'period' },
    {
      title: '库存金额',
      dataIndex: 'inventory_amount',
      align: 'right',
      render: (value: number) => formatAmount(value)
    }
  ];

  const turnoverBucketsColumns: ColumnsType<Record<string, unknown>> = [
    { title: '区间', dataIndex: 'label' },
    {
      title: 'SKU数量',
      dataIndex: 'sku_count',
      align: 'right'
    },
    {
      title: '库存金额',
      dataIndex: 'inventory_amount',
      align: 'right',
      render: (value: number) => formatAmount(value)
    },
    {
      title: '占比',
      dataIndex: 'share',
      align: 'right',
      render: (value: number) => <Progress percent={Math.round(value * 100)} size="small" />
    }
  ];

  const slowMovingColumns: ColumnsType<Record<string, unknown>> = [
    { title: '品牌', dataIndex: 'dimension_name' },
    {
      title: '平均周转天数',
      dataIndex: 'avg_turnover_days',
      align: 'right',
      render: (value: number) => formatDays(value)
    },
    { title: 'SKU数量', dataIndex: 'sku_count', align: 'right' }
  ];

  const historyColumns: ColumnsType<Record<string, unknown>> = [
    { title: '日期', dataIndex: 'date' },
    { title: '结存数量', dataIndex: 'inventory_qty', align: 'right' },
    {
      title: '结存金额',
      dataIndex: 'inventory_amount',
      align: 'right',
      render: (value: number) => formatAmount(value)
    }
  ];

  const deadColumns: ColumnsType<Record<string, unknown>> = [
    { title: '货品/SKU', dataIndex: 'sku_code' },
    { title: '货品名称', dataIndex: 'sku_name' },
    { title: '仓库', dataIndex: 'warehouse_name' },
    {
      title: '呆滞天数',
      dataIndex: 'dead_days',
      align: 'right',
      render: (value: number) => formatDays(value)
    },
    { title: '库存量', dataIndex: 'inventory_qty', align: 'right' },
    {
      title: '库存金额',
      dataIndex: 'inventory_amount',
      align: 'right',
      render: (value: number) => formatAmount(value)
    }
  ];

  const abcColumns: ColumnsType<Record<string, unknown>> = [
    { title: '分级', dataIndex: 'level' },
    { title: '指标维度', dataIndex: 'metric' },
    { title: 'SKU数量', dataIndex: 'sku_count', align: 'right' },
    {
      title: '库存金额',
      dataIndex: 'inventory_amount',
      align: 'right',
      render: (value: number) => formatAmount(value)
    },
    {
      title: '占比',
      dataIndex: 'share',
      align: 'right',
      render: (value: number) => <Progress percent={Math.round(value * 100)} size="small" />
    }
  ];

  const ruleColumns: ColumnsType<InventoryAlertRule> = [
    { title: '规则名称', dataIndex: 'name' },
    { title: '仓库', dataIndex: 'warehouse_id' },
    { title: '品牌', dataIndex: 'brand' },
    { title: '分类', dataIndex: 'category' },
    {
      title: '阈值(可售数量/天数)',
      dataIndex: 'available_qty_threshold',
      render: (_: number, record: InventoryAlertRule) =>
        `${record.available_qty_threshold}/${record.available_days_threshold}`
    },
    {
      title: '等级',
      dataIndex: 'alert_level',
      render: (value: InventoryAlertLevel) => (
        <Tag color={alertLevelColors[value]}>{alertLevelLabels[value]}</Tag>
      )
    },
    {
      title: '启用',
      dataIndex: 'enabled',
      render: (value: boolean) => (value ? '启用' : '停用')
    },
    {
      title: '操作',
      key: 'actions',
      render: (_: unknown, record: InventoryAlertRule) => (
        <Space size={8}>
          <Button size="small" type="link" onClick={() => handleRuleEdit(record)}>
            编辑
          </Button>
          <Popconfirm
            title="确认删除该规则？"
            onConfirm={() => handleDeleteRule(record.rule_id)}
          >
            <Button size="small" type="link" danger>
              删除
            </Button>
          </Popconfirm>
        </Space>
      )
    }
  ];

  const renderAnalysisTable = (
    columns: ColumnsType<Record<string, unknown>>,
    data: unknown[],
    emptyText: string,
    loading = false
  ) => (
    <Table
      size="small"
      rowKey={(record, index) => (record as { dimension_id?: string }).dimension_id ?? index ?? 0}
      columns={columns}
      dataSource={data}
      pagination={false}
      loading={loading}
      locale={{
        emptyText: <EmptyState description={emptyText} />
      }}
    />
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }} className="inventory-alerts-page">
      <Row justify="space-between" align="middle" className="inventory-alerts-header">
        <Typography.Title level={3} style={{ margin: 0 }}>
          库存预警看板
        </Typography.Title>
        <Space size={8} wrap>
          <Button icon={<ExportOutlined />} onClick={handleExport} loading={exporting}>
            导出
          </Button>
          <Button icon={<ReloadOutlined />} onClick={handleRefresh}>
            刷新
          </Button>
          <Button icon={<SettingOutlined />} onClick={handleOpenRuleDrawer}>
            配置预警规则
          </Button>
          <Button icon={<QuestionCircleOutlined />} onClick={() => message.info('查看帮助文档')}>
            帮助
          </Button>
        </Space>
      </Row>

      {!insightsFeature.enabled ? (
        <Alert
          type="warning"
          showIcon
          message="库存预警功能未启用"
          description="请联系管理员开启 inventory_insights 功能开关。"
        />
      ) : null}

      <Card size="small">
        <Form form={form} layout="vertical" onFinish={handleSearch} initialValues={defaultFilters}>
          <Row gutter={[16, 16]}>
            <Col xs={24} md={6}>
              <Form.Item name="dateRange" label="时间范围">
                <DatePicker.RangePicker allowClear style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="warehouse_id" label="仓库">
                <Select allowClear placeholder="选择仓库" options={warehouseOptions} />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="category" label="分类">
                <Select
                  allowClear
                  placeholder="货品分类"
                  options={categoryOptions.map((item) => ({ value: item, label: item }))}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="brand" label="品牌">
                <Select
                  allowClear
                  placeholder="品牌"
                  options={brandOptions.map((item) => ({ value: item, label: item }))}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键词">
                <Input placeholder="货品名称/SKU" allowClear />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="alert_level" label="预警等级">
                <Select
                  mode="multiple"
                  allowClear
                  placeholder="等级"
                  options={(Object.keys(alertLevelLabels) as InventoryAlertLevel[]).map(
                    (value) => ({
                      value,
                      label: alertLevelLabels[value]
                    })
                  )}
                />
              </Form.Item>
            </Col>
            <Col
              xs={24}
              md={10}
              style={{ display: 'flex', alignItems: 'end', justifyContent: 'flex-end' }}
            >
              <Space>
                <Button type="primary" htmlType="submit">
                  查询
                </Button>
                <Button onClick={handleReset}>重置</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>

      {summaryQuery.error ? (
        <ErrorState
          title="预警概览加载失败"
          error={summaryQuery.error}
          onRetry={() => summaryQuery.refetch()}
        />
      ) : (
        <Card size="small">
          {summaryQuery.isFetching ? (
            <Skeleton active paragraph={{ rows: 1 }} />
          ) : (
            <Row gutter={[16, 16]}>
              {kpiCards.map((card) => (
                <Col xs={24} sm={12} md={6} key={card.type}>
                  <Card
                    size="small"
                    hoverable
                    onClick={() => handleKpiClick(card.type)}
                    className={`inventory-alerts-kpi-card${
                      activeAlertType === card.type ? ' is-active' : ''
                    }`}
                  >
                    <Statistic title={card.label} value={card.value} />
                  </Card>
                </Col>
              ))}
            </Row>
          )}
        </Card>
      )}

      <Card size="small" className="inventory-alerts-table-card">
        <Tabs
          activeKey={activeAlertType}
          items={alertTabs}
          onChange={(key) => handleKpiClick(key as InventoryAlertType)}
        />
        <UiModule03MainTableSection<InventoryAlertRecord>
          rowKey={(record) => record.alert_id}
          columns={alertColumns}
          data={alertsQuery.data?.items ?? []}
          loading={alertsQuery.isFetching}
          error={alertsQuery.error}
          onRetry={() => alertsQuery.refetch()}
          errorTitle="预警明细加载失败"
          emptyText="暂无符合条件的预警记录"
          total={alertsQuery.data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          onSortChange={setSortState}
          sortState={sortState}
          rowClassName={(record) =>
            record.alert_id === selectedAlert?.alert_id ? 'inventory-alerts-row-active' : ''
          }
          onRow={(record) => ({
            onClick: () => {
              setSelectedAlert(record);
              setDetailDrawerOpen(true);
            }
          })}
          enablePanels={false}
        />
      </Card>

      <Card size="small" title="库存金额分析">
        {amountAnalysisQuery.error ? (
          <ErrorState
            title="库存金额分析加载失败"
            error={amountAnalysisQuery.error}
            onRetry={() => amountAnalysisQuery.refetch()}
          />
        ) : (
          <Tabs
            items={[
              {
                key: 'warehouse',
                label: '仓库排名',
                children: (
                  <div className="inventory-alerts-analysis-grid">
                    {renderAnalysisTable(
                      rankingColumns,
                      amountAnalysisQuery.data?.warehouse_ranking ?? [],
                      '暂无仓库排名数据',
                      amountAnalysisQuery.isFetching
                    )}
                  </div>
                )
              },
              {
                key: 'share',
                label: '品牌/分类占比',
                children: (
                  <div className="inventory-alerts-analysis-split">
                    <div className="inventory-alerts-mini-table">
                      <Typography.Text strong>品牌占比</Typography.Text>
                      {renderAnalysisTable(
                        shareColumns,
                        amountAnalysisQuery.data?.brand_share ?? [],
                        '暂无品牌占比数据',
                        amountAnalysisQuery.isFetching
                      )}
                    </div>
                    <div className="inventory-alerts-mini-table">
                      <Typography.Text strong>分类占比</Typography.Text>
                      {renderAnalysisTable(
                        shareColumns,
                        amountAnalysisQuery.data?.category_share ?? [],
                        '暂无分类占比数据',
                        amountAnalysisQuery.isFetching
                      )}
                    </div>
                  </div>
                )
              },
              {
                key: 'trend',
                label: '金额趋势',
                children: renderAnalysisTable(
                  trendColumns,
                  amountAnalysisQuery.data?.trend ?? [],
                  '暂无趋势数据',
                  amountAnalysisQuery.isFetching
                )
              }
            ]}
          />
        )}
      </Card>

      <Card size="small" title="周转 & 库龄 & 呆滞 & ABC">
        {turnoverQuery.error ? (
          <ErrorState
            title="周转分析加载失败"
            error={turnoverQuery.error}
            onRetry={() => turnoverQuery.refetch()}
          />
        ) : (
          <Tabs
            items={[
              {
                key: 'turnover',
                label: '周转分析',
                children: (
                  <div className="inventory-alerts-analysis-split">
                    <div className="inventory-alerts-mini-table">
                      <Typography.Text strong>周转天数分段</Typography.Text>
                      {renderAnalysisTable(
                        turnoverBucketsColumns,
                        turnoverQuery.data?.turnover_buckets ?? [],
                        '暂无周转数据',
                        turnoverQuery.isFetching
                      )}
                    </div>
                    <div className="inventory-alerts-mini-table">
                      <Typography.Text strong>周转慢品牌</Typography.Text>
                      {renderAnalysisTable(
                        slowMovingColumns,
                        turnoverQuery.data?.slow_moving ?? [],
                        '暂无慢周转品牌',
                        turnoverQuery.isFetching
                      )}
                    </div>
                    <div className="inventory-alerts-mini-table">
                      <Typography.Text strong>历史结存趋势</Typography.Text>
                      {renderAnalysisTable(
                        historyColumns,
                        turnoverQuery.data?.history ?? [],
                        '暂无结存趋势',
                        turnoverQuery.isFetching
                      )}
                    </div>
                  </div>
                )
              },
              {
                key: 'age',
                label: '库龄结构',
                children: renderAnalysisTable(
                  turnoverBucketsColumns,
                  turnoverQuery.data?.age_buckets ?? [],
                  '暂无库龄数据',
                  turnoverQuery.isFetching
                )
              },
              {
                key: 'dead',
                label: '呆滞库存',
                children: renderAnalysisTable(
                  deadColumns,
                  turnoverQuery.data?.dead_stock ?? [],
                  '暂无呆滞库存',
                  turnoverQuery.isFetching
                )
              },
              {
                key: 'abc',
                label: 'ABC 分析',
                children: renderAnalysisTable(
                  abcColumns,
                  turnoverQuery.data?.abc_classes ?? [],
                  '暂无 ABC 数据',
                  turnoverQuery.isFetching
                )
              }
            ]}
          />
        )}
      </Card>

      <Drawer
        width={520}
        title="库存详情"
        open={detailDrawerOpen}
        onClose={() => setDetailDrawerOpen(false)}
      >
        {selectedAlert ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Descriptions column={1} size="small" bordered>
              <Descriptions.Item label="货品/SKU">
                {selectedAlert.sku_code}
              </Descriptions.Item>
              <Descriptions.Item label="货品名称">
                {selectedAlert.sku_name}
              </Descriptions.Item>
              <Descriptions.Item label="仓库">
                {selectedAlert.warehouse_name}
              </Descriptions.Item>
              <Descriptions.Item label="可售数量">
                {selectedAlert.available_qty}
              </Descriptions.Item>
              <Descriptions.Item label="可售天数">
                {formatDays(selectedAlert.available_days)}
              </Descriptions.Item>
              <Descriptions.Item label="近30天销量">
                {selectedAlert.sales_30d ?? 0}
              </Descriptions.Item>
              <Descriptions.Item label="预警类型">
                {alertTypeLabels[selectedAlert.alert_type]}
              </Descriptions.Item>
              <Descriptions.Item label="预警等级">
                <Tag color={alertLevelColors[selectedAlert.alert_level]}>
                  {alertLevelLabels[selectedAlert.alert_level]}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label="建议动作">
                {selectedAlert.suggestion ?? '-'}
              </Descriptions.Item>
            </Descriptions>
            <Alert
              type="info"
              showIcon
              message="库存详情面板"
              description="支持后续接入库存明细、批次、调拨记录等信息。"
            />
          </Space>
        ) : (
          <EmptyState description="暂无库存详情" />
        )}
      </Drawer>

      <Drawer
        width={720}
        title="预警规则配置"
        open={ruleDrawerOpen}
        onClose={() => {
          setRuleDrawerOpen(false);
          handleRuleReset();
        }}
      >
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          {rulesQuery.error ? (
            <ErrorState
              title="规则加载失败"
              error={rulesQuery.error}
              onRetry={() => rulesQuery.refetch()}
            />
          ) : (
            <Table
              size="small"
              rowKey="rule_id"
              columns={ruleColumns}
              dataSource={rulesQuery.data?.items ?? []}
              pagination={false}
              loading={rulesQuery.isFetching}
              locale={{
                emptyText: <EmptyState description="暂无预警规则" />
              }}
            />
          )}

          <Card size="small" title={editingRule ? '编辑规则' : '新增规则'}>
            <Form
              layout="vertical"
              form={ruleForm}
              initialValues={{
                enabled: true,
                alert_level: 'MEDIUM',
                available_qty_threshold: 5,
                available_days_threshold: 7
              }}
            >
              <Row gutter={16}>
                <Col span={12}>
                  <Form.Item
                    label="规则名称"
                    name="name"
                    rules={[{ required: true, message: '请输入规则名称' }]}
                  >
                    <Input placeholder="请输入规则名称" />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item label="仓库" name="warehouse_id">
                    <Select allowClear options={warehouseOptions} placeholder="全部仓库" />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item label="品牌" name="brand">
                    <Select
                      allowClear
                      options={brandOptions.map((item) => ({ value: item, label: item }))}
                      placeholder="全部品牌"
                    />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item label="分类" name="category">
                    <Select
                      allowClear
                      options={categoryOptions.map((item) => ({ value: item, label: item }))}
                      placeholder="全部分类"
                    />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item label="SKU关键词" name="sku_keyword">
                    <Input placeholder="支持SKU或货品关键词" />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item
                    label="预警等级"
                    name="alert_level"
                    rules={[{ required: true, message: '请选择预警等级' }]}
                  >
                    <Select
                      options={(Object.keys(alertLevelLabels) as InventoryAlertLevel[]).map(
                        (value) => ({
                          value,
                          label: alertLevelLabels[value]
                        })
                      )}
                    />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item
                    label="可售数量阈值"
                    name="available_qty_threshold"
                    rules={[{ required: true, message: '请输入可售数量阈值' }]}
                  >
                    <Input type="number" min={0} />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item
                    label="可售天数阈值"
                    name="available_days_threshold"
                    rules={[{ required: true, message: '请输入可售天数阈值' }]}
                  >
                    <Input type="number" min={0} />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item label="启用状态" name="enabled" valuePropName="checked">
                    <Switch checkedChildren="启用" unCheckedChildren="停用" />
                  </Form.Item>
                </Col>
              </Row>
              <Space size={8}>
                <Button type="primary" onClick={handleSubmitRule} loading={ruleSubmitting}>
                  保存
                </Button>
                <Button onClick={handleRuleReset}>重置</Button>
              </Space>
            </Form>
          </Card>
        </Space>
      </Drawer>
    </Space>
  );
};
