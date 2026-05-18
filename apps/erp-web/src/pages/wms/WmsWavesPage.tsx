import {
  Alert,
  Button,
  Col,
  DatePicker,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Space,
  Switch,
  TimePicker,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import dayjs from 'dayjs';

import {
  createWmsWave,
  createWmsWaveRule,
  getWmsWave,
  listWmsWaveRules,
  listWmsWaves,
  printWmsWave,
  runWmsWave,
  shipWmsWave,
  updateWmsWaveRule,
  type WmsWave,
  type WmsWaveCreatePayload,
  type WmsWaveDetail,
  type WmsWaveListQuery,
  type WmsWaveRule,
  type WmsWaveRuleCreatePayload,
  type WmsWaveRuleUpdatePayload,
  type WmsWaveRunPayload,
  type WmsWaveShipRequest,
  type WmsWaveShipResponse
} from '@/api/wms';
import { listWarehouses } from '@/api/warehouses';
import { getWarehouseWorkflowConfig } from '@/api/workflowConfigs';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { openPrintJobOutput, waitForPrintJob } from '@/features/printing/printJobFlow';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import { IndustrialCard } from '@/components/industrial';

type FilterValues = {
  warehouse_id?: string;
  status?: string;
};

type WaveRuleFormValues = {
  enabled: boolean;
  carrier_code?: string;
  max_orders?: number;
  max_qty?: number;
  cutoff_time?: dayjs.Dayjs;
  strategy?: string;
};

type WaveShipFormValues = {
  continue_on_error: boolean;
  shipped_at?: dayjs.Dayjs;
};

const normalizeWaveRule = (rule?: WmsWaveRule | null): WaveRuleFormValues => {
  if (!rule) {
    return { enabled: true };
  }
  const cutoffTime = rule.cutoff_time
    ? dayjs(`1970-01-01T${rule.cutoff_time}`)
    : undefined;
  return {
    enabled: rule.enabled,
    carrier_code: rule.carrier_code ?? undefined,
    max_orders: rule.max_orders ?? undefined,
    max_qty: rule.max_qty ?? undefined,
    cutoff_time: cutoffTime?.isValid() ? cutoffTime : undefined,
    strategy: rule.strategy ?? undefined
  };
};

const buildWaveRulePayload = (
  values: WaveRuleFormValues
): WmsWaveRuleCreatePayload | WmsWaveRuleUpdatePayload => {
  const carrierCode = values.carrier_code?.trim();
  const strategy = values.strategy?.trim();
  return {
    enabled: values.enabled,
    carrier_code: carrierCode ? carrierCode : undefined,
    max_orders: values.max_orders,
    max_qty: values.max_qty,
    cutoff_time: values.cutoff_time ? values.cutoff_time.format('HH:mm:ss') : undefined,
    strategy: strategy ? strategy : undefined
  };
};

export const WmsWavesPage = () => {
  const tenantId = useTenantId();
  const canPrint = useCan('print_jobs.write');
  const canWrite = useCan('wms.write');
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [detailOpen, setDetailOpen] = useState(false);
  const [detail, setDetail] = useState<WmsWaveDetail | null>(null);
  const [printing, setPrinting] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
  const [ruleOpen, setRuleOpen] = useState(false);
  const [ruleSaving, setRuleSaving] = useState(false);
  const [runSubmitting, setRunSubmitting] = useState(false);
  const [shipSubmitting, setShipSubmitting] = useState(false);
  const [shipResult, setShipResult] = useState<WmsWaveShipResponse | null>(null);
  const [actionLocks, setActionLocks] = useState<Record<string, boolean>>({});
  const actionLockRef = useRef<Set<string>>(new Set());
  const [createForm] = Form.useForm<WmsWaveCreatePayload>();
  const [ruleForm] = Form.useForm<WaveRuleFormValues>();
  const [shipForm] = Form.useForm<WaveShipFormValues>();
  const [filterForm] = Form.useForm<FilterValues>();

  const warehousesQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'wms-waves'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId)
  });

  const defaultWarehouseId = useMemo(() => {
    const warehouses = warehousesQuery.data?.items ?? [];
    return warehouses.find((item) => item.is_default)?.warehouse_id ?? warehouses[0]?.warehouse_id;
  }, [warehousesQuery.data]);

  const activeWarehouseId = filters.warehouse_id?.trim() || defaultWarehouseId;

  const workflowQuery = useQuery({
    queryKey: ['workflow-config', tenantId, activeWarehouseId],
    queryFn: () => getWarehouseWorkflowConfig(tenantId, activeWarehouseId ?? ''),
    enabled: Boolean(activeWarehouseId)
  });

  const waveEnabled = workflowQuery.data?.enable_wave ?? true;

  const waveRuleQuery = useQuery({
    queryKey: ['wms-wave-rules', tenantId, activeWarehouseId],
    queryFn: () =>
      listWmsWaveRules(tenantId, {
        warehouse_id: activeWarehouseId,
        offset: 0,
        limit: 20
      }),
    enabled: Boolean(activeWarehouseId && tenantId)
  });

  const activeRule = waveRuleQuery.data?.items?.[0] ?? null;
  const runActionKey = activeWarehouseId ? `run:${activeWarehouseId}` : 'run:unknown';
  const shipActionKey = detail ? `ship:${detail.wave.wave_id}` : 'ship:unknown';

  const runWithActionLock = async (key: string, execute: () => Promise<void>) => {
    if (actionLockRef.current.has(key)) {
      return;
    }
    actionLockRef.current.add(key);
    setActionLocks((prev) => ({ ...prev, [key]: true }));
    try {
      await execute();
    } finally {
      actionLockRef.current.delete(key);
      setActionLocks((prev) => {
        const next = { ...prev };
        delete next[key];
        return next;
      });
    }
  };

  useEffect(() => {
    ruleForm.resetFields();
    ruleForm.setFieldsValue(normalizeWaveRule(activeRule));
  }, [activeRule, ruleForm]);

  const query = useMemo<WmsWaveListQuery>(() => {
    return {
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['wms-waves', tenantId, query],
    queryFn: () => listWmsWaves(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const columns = useMemo<ColumnsType<WmsWave>>(
    () => [
      {
        title: '波次ID',
        dataIndex: 'wave_id'
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id'
      },
      {
        title: '状态',
        dataIndex: 'status'
      },
      {
        title: '创建时间',
        dataIndex: 'created_at'
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Button
            size="small"
            onClick={async () => {
              try {
                const detailResponse = await getWmsWave(tenantId, record.wave_id);
                setDetail(detailResponse);
                setShipResult(null);
                shipForm.resetFields();
                setDetailOpen(true);
              } catch (err) {
                message.error(err instanceof Error ? err.message : '加载波次失败');
              }
            }}
          >
            查看
          </Button>
        )
      }
    ],
    [tenantId]
  );

  const handleCreate = async (values: WmsWaveCreatePayload) => {
    if (!waveEnabled) {
      message.warning('当前仓库未启用波次');
      return;
    }
    try {
      await createWmsWave(tenantId, values);
      message.success('波次已创建');
      setCreateOpen(false);
      createForm.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['wms-waves', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '创建波次失败');
    }
  };

  const handlePrint = async () => {
    if (!detail) {
      return;
    }
    setPrinting(true);
    try {
      const job = await printWmsWave(tenantId, detail.wave.wave_id);
      const completed = await waitForPrintJob(tenantId, job.job_id);
      openPrintJobOutput(completed);
      if (completed.status === 'DONE') {
        message.success('打印任务已完成');
      } else if (completed.status === 'FAILED') {
        message.error(completed.last_error || '打印失败');
      } else {
        message.success('打印任务已提交');
      }
    } catch (err) {
      message.error(err instanceof Error ? err.message : '打印波次失败');
    } finally {
      setPrinting(false);
    }
  };

  const handleShipWave = async () => {
    if (!detail) {
      return;
    }
    const currentWaveId = detail.wave.wave_id;
    const actionKey = `ship:${currentWaveId}`;
    await runWithActionLock(actionKey, async () => {
      try {
        const values = await shipForm.validateFields();
        const payload: WmsWaveShipRequest = {
          continue_on_error: values.continue_on_error,
          shipped_at: values.shipped_at ? values.shipped_at.toISOString() : undefined
        };
        setShipSubmitting(true);
        const response = await shipWmsWave(tenantId, currentWaveId, payload);
        setShipResult(response);
        message.success('波次发货已执行');
        await queryClient.invalidateQueries({ queryKey: ['wms-waves', tenantId] });
        queryClient.invalidateQueries({ queryKey: ['wms-outbounds', tenantId] });
        queryClient.invalidateQueries({ queryKey: ['wms-ship-outbounds', tenantId] });
        const latest = await getWmsWave(tenantId, currentWaveId);
        setDetail(latest);
        queryClient.invalidateQueries({ queryKey: ['inventory-summary', tenantId] });
        queryClient.invalidateQueries({ queryKey: ['inventory-atp', tenantId] });
      } catch (err) {
        if (err instanceof Error && err.message) {
          message.error(err.message);
        } else {
          message.error('波次发货失败');
        }
      } finally {
        setShipSubmitting(false);
      }
    });
  };

  const handleRunWave = async () => {
    if (!activeWarehouseId) {
      message.warning('请选择仓库');
      return;
    }
    const currentWarehouseId = activeWarehouseId;
    const actionKey = `run:${currentWarehouseId}`;
    await runWithActionLock(actionKey, async () => {
      setRunSubmitting(true);
      try {
        const payload: WmsWaveRunPayload = { warehouse_id: currentWarehouseId };
        const response = await runWmsWave(tenantId, payload);
        if (response.status === 'disabled') {
          message.warning('波次规则未启用');
        } else if (response.status === 'empty') {
          message.info('暂无可生成波次');
        } else {
          message.success('波次已生成');
          if (response.wave) {
            setDetail(response.wave);
            setDetailOpen(true);
          }
          await queryClient.invalidateQueries({ queryKey: ['wms-waves', tenantId] });
        }
      } catch (err) {
        message.error(err instanceof Error ? err.message : '执行波次失败');
      } finally {
        setRunSubmitting(false);
      }
    });
  };

  const handleSaveRule = async () => {
    if (!activeWarehouseId) {
      message.warning('请选择仓库');
      return;
    }
    try {
      const values = await ruleForm.validateFields();
      setRuleSaving(true);
      const payload = buildWaveRulePayload(values);
      if (activeRule) {
        await updateWmsWaveRule(tenantId, activeRule.rule_id, payload);
      } else {
        await createWmsWaveRule(tenantId, {
          ...(payload as WmsWaveRuleCreatePayload),
          warehouse_id: activeWarehouseId
        });
      }
      message.success('波次规则已更新');
      setRuleOpen(false);
      await queryClient.invalidateQueries({
        queryKey: ['wms-wave-rules', tenantId, activeWarehouseId]
      });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setRuleSaving(false);
    }
  };

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        波次管理
      </Typography.Title>
      <Button
        onClick={handleRunWave}
        loading={runSubmitting}
        disabled={!waveEnabled || !canWrite || Boolean(actionLocks[runActionKey])}
        data-testid="wms-wave-run-button"
      >
        执行波次
      </Button>
      <Button onClick={() => setRuleOpen(true)} disabled={!activeWarehouseId}>
        波次规则
      </Button>
      <Button type="primary" disabled={!waveEnabled} onClick={() => setCreateOpen(true)}>
        新建波次
      </Button>
    </Space>
  );

  const header = (
    <Space direction="vertical" size={12}>
      {!waveEnabled ? (
        <Alert
          type="warning"
          showIcon
          message="当前仓库未启用波次"
          description="请在流程配置中开启波次后再创建波次。"
          data-testid="wms-wave-disabled-alert"
        />
      ) : null}
      <IndustrialCard>
        <Form
          form={filterForm}
          layout="vertical"
          onFinish={(values) => {
            setFilters(values);
            setPagination((prev) => ({ ...prev, current: 1 }));
          }}
        >
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="warehouse_id" label="仓库ID">
                <Input allowClear placeholder="仓库ID" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Input allowClear placeholder="状态" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  查询
                </Button>
                <Button
                  onClick={() => {
                    filterForm.resetFields();
                    setFilters({});
                    setPagination((prev) => ({ ...prev, current: 1 }));
                  }}
                >
                  重置
                </Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </IndustrialCard>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<WmsWave>
        dataTestId="wms-waves-table"
        toolbarLeft={toolbarLeft}
        header={header}
        rowKey="wave_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="波次加载失败"
        emptyText="暂无波次"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        showColumnSettings
        columnSettingsKey="wms.waves.table.columns"
      />
      <Drawer
        title="波次详情"
        open={detailOpen}
        onClose={() => {
          setDetailOpen(false);
          setShipResult(null);
          shipForm.resetFields();
        }}
        width={720}
        destroyOnHidden
        data-testid="wms-wave-detail-drawer"
      >
        {detail ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Space>
              <Button
                type="primary"
                onClick={handlePrint}
                disabled={!canPrint}
                loading={printing}
              >
                打印拣货单
              </Button>
            </Space>
            <IndustrialCard size="small" title="波次发货">
              <Form<WaveShipFormValues>
                form={shipForm}
                layout="vertical"
                initialValues={{ continue_on_error: true }}
                onFinish={handleShipWave}
              >
                <Row gutter={12}>
                  <Col xs={24} md={12}>
                    <Form.Item
                      name="continue_on_error"
                      label="失败继续"
                      valuePropName="checked"
                    >
                      <Switch />
                    </Form.Item>
                  </Col>
                  <Col xs={24} md={12}>
                    <Form.Item name="shipped_at" label="发货时间">
                      <DatePicker showTime style={{ width: '100%' }} />
                    </Form.Item>
                  </Col>
                </Row>
                <Space>
                  <Button
                    type="primary"
                    htmlType="submit"
                    loading={shipSubmitting}
                    disabled={!canWrite || Boolean(actionLocks[shipActionKey])}
                  >
                    波次发货
                  </Button>
                  <Button
                    onClick={() => shipForm.resetFields()}
                    disabled={shipSubmitting || Boolean(actionLocks[shipActionKey])}
                  >
                    重置
                  </Button>
                </Space>
              </Form>
              {shipResult ? (
                <Space direction="vertical" size={4} style={{ marginTop: 12 }}>
                  <Typography.Text>
                    总数 {shipResult.total} / 成功 {shipResult.success} / 失败{' '}
                    {shipResult.failed}
                  </Typography.Text>
                  {shipResult.items.length === 0
                    ? '暂无详情'
                    : shipResult.items.map((item) => (
                        <Typography.Text key={item.outbound_doc_id}>
                          {item.outbound_doc_id} / {item.status}
                          {item.message ? ` / ${item.message}` : ''}
                        </Typography.Text>
                      ))}
                </Space>
              ) : null}
            </IndustrialCard>
            <Typography.Text>波次：{detail.wave.wave_id}</Typography.Text>
            <Typography.Text>仓库：{detail.wave.warehouse_id}</Typography.Text>
            <Typography.Text>状态：{detail.wave.status}</Typography.Text>
            <Typography.Title level={5}>出库单</Typography.Title>
            {detail.docs.length === 0
              ? '暂无出库单'
              : detail.docs.map((doc) => (
                  <Typography.Text key={doc.outbound_doc_id}>
                    {doc.doc_no} / {doc.status}
                  </Typography.Text>
                ))}
            <Typography.Title level={5}>拣货任务</Typography.Title>
            {detail.tasks.length === 0
              ? '暂无任务'
              : detail.tasks.map((task) => (
                  <Typography.Text key={task.pick_task_id}>
                    {task.sku_id} 数量 {task.qty} 状态 {task.status}
                  </Typography.Text>
                ))}
          </Space>
        ) : null}
      </Drawer>
      <Modal
        title="新建波次"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={() => createForm.submit()}
        okText="提交"
      >
        <Form form={createForm} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="warehouse_id" label="仓库ID" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="cutoff_at" label="截止时间">
            <Input placeholder="RFC3339时间，可选" />
          </Form.Item>
          <Form.Item name="max_docs" label="最大单量">
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="波次规则"
        open={ruleOpen}
        onCancel={() => setRuleOpen(false)}
        onOk={handleSaveRule}
        confirmLoading={ruleSaving}
        okButtonProps={{ disabled: !canWrite || !activeWarehouseId }}
        width={640}
      >
        <Form form={ruleForm} layout="vertical">
          {!activeWarehouseId ? (
            <Alert type="info" showIcon message="请选择仓库后再维护波次规则" />
          ) : null}
          <Form.Item
            name="enabled"
            label="启用波次规则"
            valuePropName="checked"
            rules={[{ required: true, message: '请选择是否启用波次规则' }]}
          >
            <Switch />
          </Form.Item>
          <Form.Item name="carrier_code" label="承运商">
            <Input placeholder="例如 SF" />
          </Form.Item>
          <Form.Item name="max_orders" label="单次最大单量">
            <InputNumber min={1} style={{ width: '100%' }} placeholder="默认50" />
          </Form.Item>
          <Form.Item name="max_qty" label="单次最大出库数量">
            <InputNumber min={1} style={{ width: '100%' }} placeholder="例如 100" />
          </Form.Item>
          <Form.Item name="cutoff_time" label="固定截止时间">
            <TimePicker style={{ width: '100%' }} format="HH:mm" />
          </Form.Item>
          <Form.Item name="strategy" label="执行策略">
            <Input placeholder="例如 FAST" />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};

