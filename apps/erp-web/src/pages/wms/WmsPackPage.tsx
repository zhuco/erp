import {
  Alert,
  Button,
  Card,
  Col,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import {
  getWmsOutbound,
  checkWmsOutbound,
  listWmsOutbounds,
  packWmsOutbound,
  weighWmsOutbound,
  type WmsOutboundDetail,
  type WmsOutboundDoc,
  type WmsOutboundListQuery,
  type WmsOutboundCheckPayload,
  type WmsOutboundWeighPayload,
  type WmsPackPayload
} from '@/api/wms';
import { listWarehouses } from '@/api/warehouses';
import { getWarehouseWorkflowConfig } from '@/api/workflowConfigs';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useSessionStore } from '@/features/sessionStore';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  warehouse_id?: string;
  status?: string;
};

type PackSerialForm = {
  sku_id: string;
  serial_nos: string;
};

type PackBatchForm = {
  sku_id: string;
  batch_no?: string;
  qty?: number;
};

type PackFormValues = Omit<WmsPackPayload, 'serials' | 'batches'> & {
  serials?: PackSerialForm[];
  batches?: PackBatchForm[];
};

type CheckFormValues = WmsOutboundCheckPayload;

type WeighFormValues = WmsOutboundWeighPayload;

export const WmsPackPage = () => {
  const tenantId = useTenantId();
  const operatorId = useSessionStore((state) => state.userId);
  const packPerformanceFlag = useFeatureFlag('flows.pack_performance');
  const inspectionFlag = useFeatureFlag('wms.enable_outbound_inspection');
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<FilterValues>({ status: 'PICKED' });
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [packOpen, setPackOpen] = useState(false);
  const [checkOpen, setCheckOpen] = useState(false);
  const [weighOpen, setWeighOpen] = useState(false);
  const [activeDetail, setActiveDetail] = useState<WmsOutboundDetail | null>(null);
  const [packForm] = Form.useForm<PackFormValues>();
  const [checkForm] = Form.useForm<CheckFormValues>();
  const [weighForm] = Form.useForm<WeighFormValues>();
  const [filterForm] = Form.useForm<FilterValues>();

  const warehousesQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'wms-pack'],
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

  const packRequired = workflowQuery.data?.require_pack ?? true;
  const weighRequired = workflowQuery.data?.require_weighing ?? false;
  const inspectRequired = workflowQuery.data?.require_inspection ?? false;
  const packOptions = useMemo(
    () =>
      activeDetail?.pack_records.map((record) => ({
        label: record.package_no,
        value: record.package_no,
        weight_gram: record.weight_gram ?? undefined
      })) ?? [],
    [activeDetail?.pack_records]
  );
  const packGovernanceHints = useMemo(() => {
    if (!activeDetail) {
      return [];
    }
    return activeDetail.items.flatMap((item) => {
      const hints: string[] = [];
      if (item.batch_enabled) {
        hints.push(`SKU ${item.sku_id} 已启用食品批次治理，打包时必须录入可出库批次。`);
      }
      if (item.serial_enabled) {
        hints.push(`SKU ${item.sku_id} 已启用数码序列号治理，打包时必须录入唯一序列号。`);
      }
      return hints;
    });
  }, [activeDetail]);

  const query = useMemo<WmsOutboundListQuery>(() => {
    return {
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['wms-pack-outbounds', tenantId, query],
    queryFn: () => listWmsOutbounds(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const columns = useMemo<ColumnsType<WmsOutboundDoc>>(
    () => [
      {
        title: '出库单号',
        dataIndex: 'doc_no'
      },
      {
        title: '状态',
        dataIndex: 'status'
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id'
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              data-testid="pack-action"
              disabled={!packPerformanceFlag.enabled || !packRequired}
              onClick={async () => {
                try {
                  const detail = await getWmsOutbound(tenantId, record.outbound_doc_id);
                  setActiveDetail(detail);
                  packForm.setFieldsValue({
                    package_no: '',
                    weight_gram: undefined,
                    operator_id: operatorId || undefined,
                    items: detail.items.map((item) => ({
                      sku_id: item.sku_id,
                      packed_qty: Math.max(item.planned_qty - item.packed_qty, 0)
                    })),
                    serials: [],
                    batches: []
                  });
                  setPackOpen(true);
                } catch (err) {
                  message.error(err instanceof Error ? err.message : '加载出库单失败');
                }
              }}
            >
              打包
            </Button>
            <Button
              size="small"
              data-testid="pack-check-action"
              disabled={!inspectionFlag.enabled || !inspectRequired}
              onClick={async () => {
                try {
                  const detail = await getWmsOutbound(tenantId, record.outbound_doc_id);
                  if (!detail.pack_records.length) {
                    message.warning('请先完成打包');
                    return;
                  }
                  setActiveDetail(detail);
                  checkForm.setFieldsValue({
                    package_no: detail.pack_records[0]?.package_no ?? '',
                    passed: true,
                    reason: undefined
                  });
                  setCheckOpen(true);
                } catch (err) {
                  message.error(err instanceof Error ? err.message : '加载出库单失败');
                }
              }}
            >
              复核
            </Button>
            <Button
              size="small"
              data-testid="pack-weigh-action"
              disabled={!weighRequired}
              onClick={async () => {
                try {
                  const detail = await getWmsOutbound(tenantId, record.outbound_doc_id);
                  if (!detail.pack_records.length) {
                    message.warning('请先完成打包');
                    return;
                  }
                  setActiveDetail(detail);
                  weighForm.setFieldsValue({
                    package_no: detail.pack_records[0]?.package_no ?? '',
                    weight_gram: detail.pack_records[0]?.weight_gram ?? undefined
                  });
                  setWeighOpen(true);
                } catch (err) {
                  message.error(err instanceof Error ? err.message : '加载出库单失败');
                }
              }}
            >
              称重
            </Button>
          </Space>
        )
      }
    ],
    [
      checkForm,
      inspectRequired,
      inspectionFlag.enabled,
      packForm,
      operatorId,
      packPerformanceFlag.enabled,
      packRequired,
      tenantId,
      weighForm,
      weighRequired
    ]
  );

  const handlePack = async (values: PackFormValues) => {
    if (!activeDetail) {
      return;
    }
    try {
      const serials = (values.serials ?? [])
        .map((entry) => {
          const serial_nos = entry.serial_nos
            .split(/[\s,]+/)
            .map((value) => value.trim())
            .filter(Boolean);
          if (!entry.sku_id || serial_nos.length === 0) {
            return null;
          }
          return { sku_id: entry.sku_id, serial_nos };
        })
        .filter((entry): entry is NonNullable<typeof entry> => entry !== null);
      const batches = (values.batches ?? [])
        .map((entry) => {
          if (!entry.sku_id || !entry.qty || !entry.batch_no?.trim()) {
            return null;
          }
          return { sku_id: entry.sku_id, batch_no: entry.batch_no.trim(), qty: entry.qty };
        })
        .filter((entry): entry is NonNullable<typeof entry> => entry !== null);

      const payload: WmsPackPayload = {
        package_no: values.package_no,
        weight_gram: values.weight_gram,
        operator_id: values.operator_id,
        items: values.items,
        serials: serials.length ? serials : undefined,
        batches: batches.length ? batches : undefined
      };

      await packWmsOutbound(tenantId, activeDetail.doc.outbound_doc_id, payload);
      message.success('打包已提交');
      setPackOpen(false);
      packForm.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['wms-pack-outbounds', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '打包失败');
    }
  };

  const handleCheck = async (values: CheckFormValues) => {
    if (!activeDetail) {
      return;
    }
    try {
      await checkWmsOutbound(tenantId, activeDetail.doc.outbound_doc_id, values);
      message.success('复核已提交');
      setCheckOpen(false);
      checkForm.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['wms-pack-outbounds', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '复核提交失败');
    }
  };

  const handleWeigh = async (values: WeighFormValues) => {
    if (!activeDetail) {
      return;
    }
    try {
      await weighWmsOutbound(tenantId, activeDetail.doc.outbound_doc_id, values);
      message.success('称重已提交');
      setWeighOpen(false);
      weighForm.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['wms-pack-outbounds', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '称重提交失败');
    }
  };

  const toolbarLeft = (
    <Typography.Title level={3} data-testid="pack-title" style={{ margin: 0 }}>
      打包复核
    </Typography.Title>
  );

  const header = (
    <Space direction="vertical" size={12}>
      {!packRequired ? (
        <Alert
          type="info"
          showIcon
          message="当前仓库未启用打包登记"
          description="打包步骤已关闭，系统允许直接发货。"
        />
      ) : null}
      {!packPerformanceFlag.enabled ? (
        <Alert
          type="warning"
          showIcon
          message="打包绩效流程已关闭"
          description={packPerformanceFlag.reason}
        />
      ) : null}
      <Card>
        <Form
          form={filterForm}
          layout="vertical"
          onFinish={(values) => {
            setFilters(values);
            setPagination((prev) => ({ ...prev, current: 1 }));
          }}
          initialValues={filters}
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
      </Card>
    </Space>
  );

  return (
    <>
      <div data-testid="pack-table">
        <UiModule03MainTableSection<WmsOutboundDoc>
          toolbarLeft={toolbarLeft}
          header={header}
          rowKey="outbound_doc_id"
          columns={columns}
          data={data?.items ?? []}
          loading={isLoading || isFetching}
          error={error}
          onRetry={() => refetch()}
          errorTitle="出库单加载失败"
          emptyText="暂无待打包出库单"
          total={data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="wms.pack.table.columns"
        />
      </div>
      <Modal
        title="提交打包"
        open={packOpen}
        onCancel={() => setPackOpen(false)}
        onOk={() => packForm.submit()}
        width={720}
        data-testid="pack-modal"
        okButtonProps={{ 'data-testid': 'pack-submit' }}
      >
        <Form form={packForm} layout="vertical" onFinish={handlePack}>
          {packGovernanceHints.length ? (
            <Alert
              type="info"
              showIcon
              data-testid="pack-governance-alert"
              message="打包治理提示"
              description={
                <Space direction="vertical" size={4}>
                  {packGovernanceHints.map((hint) => (
                    <Typography.Text key={hint}>{hint}</Typography.Text>
                  ))}
                </Space>
              }
              style={{ marginBottom: 16 }}
            />
          ) : null}
          <Form.Item name="package_no" label="包裹号" rules={[{ required: true }]}>
            <Input data-testid="pack-package-no-input" />
          </Form.Item>
          {weighRequired ? (
            <Form.Item
              name="weight_gram"
              label="重量(克)"
              rules={[{ required: true, message: '请输入称重信息' }]}
            >
              <InputNumber min={0} style={{ width: '100%' }} data-testid="pack-weight-input" />
            </Form.Item>
          ) : null}
          <Form.Item name="operator_id" label="操作人ID">
            <Input placeholder="默认当前账号" />
          </Form.Item>
          <Form.List name="items">
            {(fields) => (
              <Space direction="vertical" size={12} style={{ width: '100%' }}>
                {fields.map((field) => (
                  <Space key={field.key} align="baseline">
                    <Form.Item
                      {...field}
                      label="SKU ID"
                      name={[field.name, 'sku_id']}
                      rules={[{ required: true }]}
                    >
                      <Input disabled />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      label="打包数量"
                      name={[field.name, 'packed_qty']}
                      rules={[{ required: true }]}
                    >
                      <InputNumber min={1} />
                    </Form.Item>
                    {activeDetail?.items[field.name]?.serial_enabled ? (
                      <Tag color="purple">序列号必填</Tag>
                    ) : null}
                    {activeDetail?.items[field.name]?.batch_enabled ? (
                      <Tag color="blue">批次必填</Tag>
                    ) : null}
                  </Space>
                ))}
              </Space>
            )}
          </Form.List>
          <Form.List name="serials">
            {(fields, { add, remove }) => (
              <Space direction="vertical" size={12} style={{ width: '100%' }}>
                <Typography.Text strong>序列号采集</Typography.Text>
                {fields.map((field) => (
                  <Space key={field.key} align="baseline" style={{ width: '100%' }}>
                    <Form.Item
                      {...field}
                      label="SKU ID"
                      name={[field.name, 'sku_id']}
                      rules={[{ required: true }]}
                    >
                      <Input />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      label="序列号"
                      name={[field.name, 'serial_nos']}
                      rules={[{ required: true }]}
                    >
                      <Input.TextArea rows={2} />
                    </Form.Item>
                    <Button type="link" danger onClick={() => remove(field.name)}>
                      删除
                    </Button>
                  </Space>
                ))}
                <Button type="dashed" onClick={() => add()}>
                  新增序列号
                </Button>
              </Space>
            )}
          </Form.List>
          <Form.List name="batches">
            {(fields, { add, remove }) => (
              <Space direction="vertical" size={12} style={{ width: '100%' }}>
                <Typography.Text strong>批次采集</Typography.Text>
                {fields.map((field) => (
                  <Space key={field.key} align="baseline" style={{ width: '100%' }}>
                    <Form.Item
                      {...field}
                      label="SKU ID"
                      name={[field.name, 'sku_id']}
                      rules={[{ required: true }]}
                    >
                      <Input />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      label="批次号"
                      name={[field.name, 'batch_no']}
                      rules={[{ required: true }]}
                    >
                      <Input />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      label="数量"
                      name={[field.name, 'qty']}
                      rules={[{ required: true }]}
                    >
                      <InputNumber min={1} />
                    </Form.Item>
                    <Button type="link" danger onClick={() => remove(field.name)}>
                      删除
                    </Button>
                  </Space>
                ))}
                <Button type="dashed" onClick={() => add()}>
                  新增批次
                </Button>
              </Space>
            )}
          </Form.List>
        </Form>
      </Modal>
      <Modal
        title="出库复核"
        open={checkOpen}
        onCancel={() => setCheckOpen(false)}
        onOk={() => checkForm.submit()}
        width={520}
        data-testid="pack-check-modal"
        okButtonProps={{ 'data-testid': 'pack-check-submit' }}
      >
        <Form form={checkForm} layout="vertical" onFinish={handleCheck}>
          <Form.Item
            name="package_no"
            label="包裹号"
            rules={[{ required: true, message: '请选择包裹号' }]}
          >
            <Select
              options={packOptions}
              placeholder="选择包裹"
              data-testid="pack-check-package-select"
            />
          </Form.Item>
          <Form.Item
            name="passed"
            label="复核结果"
            rules={[{ required: true, message: '请选择结果' }]}
          >
            <Select
              options={[
                { label: '通过', value: true },
                { label: '失败', value: false }
              ]}
              data-testid="pack-check-result-select"
            />
          </Form.Item>
          <Form.Item
            name="reason"
            label="失败原因"
            dependencies={['passed']}
            rules={[
              ({ getFieldValue }) => ({
                validator: (_, value) => {
                  if (getFieldValue('passed') === false && !value?.trim()) {
                    return Promise.reject(new Error('请输入失败原因'));
                  }
                  return Promise.resolve();
                }
              })
            ]}
          >
            <Input.TextArea
              rows={3}
              placeholder="失败原因"
              disabled={checkForm.getFieldValue('passed') !== false}
              data-testid="pack-check-reason-input"
            />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="出库称重"
        open={weighOpen}
        onCancel={() => setWeighOpen(false)}
        onOk={() => weighForm.submit()}
        width={520}
        data-testid="pack-weigh-modal"
        okButtonProps={{ 'data-testid': 'pack-weigh-submit' }}
      >
        <Form form={weighForm} layout="vertical" onFinish={handleWeigh}>
          <Form.Item
            name="package_no"
            label="包裹号"
            rules={[{ required: true, message: '请选择包裹号' }]}
          >
            <Select
              options={packOptions}
              placeholder="选择包裹"
              onChange={(value) => {
                const match = packOptions.find((item) => item.value === value);
                if (match?.weight_gram) {
                  weighForm.setFieldsValue({ weight_gram: match.weight_gram });
                }
              }}
              data-testid="pack-weigh-package-select"
            />
          </Form.Item>
          <Form.Item
            name="weight_gram"
            label="重量(克)"
            rules={[{ required: true, message: '请输入重量' }]}
          >
            <InputNumber min={1} style={{ width: '100%' }} data-testid="pack-weigh-weight-input" />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};
