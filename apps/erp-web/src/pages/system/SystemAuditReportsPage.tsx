import {
  Button,
  Col,
  Form,
  Row,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  getSystemAuditReportsCatalog,
  updateSystemAuditReportsCatalog,
  type AlertRule,
  type AuditReport,
  type ReportStatus,
  type SystemAuditReportsCatalog
} from '@/api/systemAuditReports';
import { getSystemWorkflowCatalog } from '@/api/systemWorkflows';
import { EmptyState } from '@/components/EmptyState';
import { ErrorState } from '@/components/QueryState';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

const statusColors: Record<ReportStatus, string> = {
  READY: 'green',
  GENERATING: 'blue',
  FAILED: 'red'
};

const moduleOptions = [
  { label: '订单', value: '订单' },
  { label: '库存', value: '库存' },
  { label: '采购', value: '采购' },
  { label: '售后', value: '售后' }
];

const ownerOptions = [
  { label: '系统管理员', value: '系统管理员' },
  { label: '审计专员', value: '审计专员' },
  { label: '运营主管', value: '运营主管' }
];

const formatDateTime = (value: string) => dayjs(value).format('YYYY-MM-DD HH:mm');

export const SystemAuditReportsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('SYS.SETTINGS.WRITE');
  const queryClient = useQueryClient();
  const [reports, setReports] = useState<AuditReport[]>([]);
  const [rules, setRules] = useState<AlertRule[]>([]);
  const [filters, setFilters] = useState<{ module?: string; owner?: string }>({});

  const catalogQuery = useQuery({
    queryKey: ['system-audit-reports', tenantId],
    queryFn: () => getSystemAuditReportsCatalog(tenantId),
    enabled: tenantReady
  });
  const workflowQuery = useQuery({
    queryKey: ['system-workflows', tenantId],
    queryFn: () => getSystemWorkflowCatalog(tenantId),
    enabled: tenantReady
  });

  useEffect(() => {
    if (!tenantReady) {
      setReports([]);
      setRules([]);
      return;
    }
    if (catalogQuery.data) {
      setReports(catalogQuery.data.reports);
      setRules(catalogQuery.data.rules);
    }
  }, [catalogQuery.data, tenantReady]);

  const saveMutation = useMutation({
    mutationFn: (payload: SystemAuditReportsCatalog) =>
      updateSystemAuditReportsCatalog(tenantId, payload),
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '保存审计报告配置失败');
    }
  });

  const catalogReady = tenantReady && !!catalogQuery.data;
  const writeDisabled = !canWrite || !catalogReady || saveMutation.isPending;
  const tableLoading = (catalogQuery.isLoading && !catalogQuery.data) || saveMutation.isPending;

  const persistCatalog = async (payload: SystemAuditReportsCatalog, successMessage: string) => {
    const saved = await saveMutation.mutateAsync(payload);
    setReports(saved.reports);
    setRules(saved.rules);
    await queryClient.invalidateQueries({ queryKey: ['system-audit-reports', tenantId] });
    message.success(successMessage);
  };

  const filteredReports = useMemo(
    () =>
      reports.filter((report) => {
        if (filters.module && report.module !== filters.module) {
          return false;
        }
        if (filters.owner && report.owner !== filters.owner) {
          return false;
        }
        return true;
      }),
    [filters.module, filters.owner, reports]
  );
  const activeGovernance = useMemo(
    () => workflowQuery.data?.workflows.find((item) => item.status === 'ACTIVE')?.governance,
    [workflowQuery.data]
  );

  const handleToggleRule = async (record: AlertRule, enabled: boolean) => {
    if (!canWrite) {
      return;
    }
    const nextRules = rules.map((item) =>
      item.rule_id === record.rule_id
        ? { ...item, enabled, updated_at: dayjs().toISOString() }
        : item
    );
    try {
      await persistCatalog(
        {
          reports,
          rules: nextRules
        },
        '告警规则已更新'
      );
    } catch {
      return;
    }
  };

  const reportColumns = useMemo<ColumnsType<AuditReport>>(
    () => [
      { title: '模块', dataIndex: 'module' },
      { title: '周期', dataIndex: 'period' },
      { title: '负责人', dataIndex: 'owner' },
      {
        title: '异常数量',
        dataIndex: 'anomalies',
        render: (value: number) => (value > 0 ? <Tag color="red">{value}</Tag> : 0)
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: ReportStatus) => <Tag color={statusColors[value]}>{value}</Tag>
      },
      {
        title: '生成时间',
        dataIndex: 'generated_at',
        render: (value: string) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: () => (
          <Button
            size="small"
            disabled={!catalogReady}
            onClick={() => message.info('审计报告已导出')}
          >
            导出
          </Button>
        )
      }
    ],
    [catalogReady]
  );

  const ruleColumns = useMemo<ColumnsType<AlertRule>>(
    () => [
      { title: '模块', dataIndex: 'module' },
      { title: '规则名称', dataIndex: 'name' },
      { title: '触发条件', dataIndex: 'condition' },
      {
        title: '启用',
        dataIndex: 'enabled',
        render: (value: boolean, record) => (
          <Switch
            checked={value}
            disabled={writeDisabled}
            loading={saveMutation.isPending}
            onChange={(checked) => void handleToggleRule(record, checked)}
          />
        )
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value: string) => formatDateTime(value)
      }
    ],
    [saveMutation.isPending, writeDisabled]
  );

  if (!tenantReady) {
    return (
      <UiModule07SettingsLayout title="审计报告">
        <EmptyState description="请先选择租户" />
      </UiModule07SettingsLayout>
    );
  }

  return (
    <UiModule07SettingsLayout title="审计报告">
      {catalogQuery.isError ? (
        <ErrorState
          title="加载审计报告失败"
          error={catalogQuery.error}
          onRetry={() => void catalogQuery.refetch()}
        />
      ) : null}
      <UiModule07SettingsSection title="审计报告">
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <Space wrap data-testid="system-audit-reports-governance-summary">
            <Tag color="blue">治理链路</Tag>
            <Tag color="processing">发布状态: {activeGovernance?.publish_status ?? 'DRAFT'}</Tag>
            <Tag>流程引擎: {activeGovernance?.engine ?? 'CAMUNDA8'}</Tag>
            <Tag>流程键: {activeGovernance?.process_key ?? '-'}</Tag>
            <Tag>生效时间: {activeGovernance?.effective_at ?? '-'}</Tag>
          </Space>
          <Form layout="vertical">
            <Row gutter={16}>
              <Col xs={24} md={6}>
                <Form.Item label="模块">
                  <Select
                    allowClear
                    placeholder="全部模块"
                    options={moduleOptions}
                    value={filters.module}
                    onChange={(value) => setFilters((prev) => ({ ...prev, module: value }))}
                  />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="负责人">
                  <Select
                    allowClear
                    placeholder="全部负责人"
                    options={ownerOptions}
                    value={filters.owner}
                    onChange={(value) => setFilters((prev) => ({ ...prev, owner: value }))}
                  />
                </Form.Item>
              </Col>
              <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
                <Space>
                  <Button onClick={() => setFilters({})}>重置</Button>
                </Space>
              </Col>
            </Row>
          </Form>
          <Table
            rowKey="report_id"
            columns={reportColumns}
            dataSource={filteredReports}
            pagination={false}
            loading={tableLoading}
          />
        </Space>
      </UiModule07SettingsSection>
      <UiModule07SettingsSection title="告警规则">
        <Table
          rowKey="rule_id"
          columns={ruleColumns}
          dataSource={rules}
          pagination={false}
          loading={tableLoading}
        />
      </UiModule07SettingsSection>
    </UiModule07SettingsLayout>
  );
};
