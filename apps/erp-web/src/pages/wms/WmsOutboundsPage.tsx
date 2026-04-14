import {
  Button,
  Card,
  Col,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  auditWmsOutbound,
  createWmsOutbound,
  getWmsOutbound,
  listWmsOutbounds,
  printWmsOutbound,
  type WmsOutboundCreatePayload,
  type WmsOutboundDetail,
  type WmsOutboundDoc,
  type WmsOutboundListQuery
} from '@/api/wms';
import { listSettings, type Setting } from '@/api/system';
import { UiModule02ToolbarRight, UiModule03MainTableSection } from '@/components/ui-modules';
import { openPrintJobOutput, waitForPrintJob } from '@/features/printing/printJobFlow';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  warehouse_id?: string;
  status?: string;
  doc_type?: string;
  shipment_id?: string;
};

const statusOptions = [
  { label: 'DRAFT', value: 'DRAFT' },
  { label: 'AUDITED', value: 'AUDITED' },
  { label: 'PICKING', value: 'PICKING' },
  { label: 'PICKED', value: 'PICKED' },
  { label: 'PACKING', value: 'PACKING' },
  { label: 'PACKED', value: 'PACKED' },
  { label: 'SHIPPED', value: 'SHIPPED' }
];

const docTypeOptions = [
  { label: 'SALES', value: 'SALES' },
  { label: 'TRANSFER', value: 'TRANSFER' },
  { label: 'OTHER', value: 'OTHER' }
];

const parseSettingBoolean = (setting?: Setting | null) => {
  if (!setting) {
    return false;
  }
  const raw = setting.value_json as unknown;
  if (typeof raw === 'boolean') {
    return raw;
  }
  if (raw && typeof raw === 'object' && 'value' in raw) {
    return Boolean((raw as { value?: unknown }).value);
  }
  return false;
};

const formatDateTime = (value?: string | null) =>
  value ? new Date(value).toLocaleString() : '-';

export const WmsOutboundsPage = () => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();
  const canWrite = useCan('wms.write');
  const canPrint = useCan('print_jobs.write');
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [detailOpen, setDetailOpen] = useState(false);
  const [detail, setDetail] = useState<WmsOutboundDetail | null>(null);
  const [printing, setPrinting] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
  const [createForm] = Form.useForm<WmsOutboundCreatePayload>();
  const [filterForm] = Form.useForm<FilterValues>();

  const auditSettingQuery = useQuery({
    queryKey: ['settings', tenantId, 'outbound-audit'],
    queryFn: () =>
      listSettings(tenantId, {
        scope_type: 'TENANT',
        key: 'inv.outbound_require_audit'
      }),
    enabled: Boolean(tenantId),
    retry: false
  });

  const auditRequired = useMemo(
    () => parseSettingBoolean(auditSettingQuery.data?.[0]),
    [auditSettingQuery.data]
  );

  const query = useMemo<WmsOutboundListQuery>(() => {
    return {
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      doc_type: filters.doc_type?.trim() || undefined,
      shipment_id: filters.shipment_id?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['wms-outbounds', tenantId, query],
    queryFn: () => listWmsOutbounds(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const auditMutation = useMutation({
    mutationFn: (outboundDocId: string) => auditWmsOutbound(tenantId, outboundDocId),
    onSuccess: () => {
      message.success('出库单已审核');
      queryClient.invalidateQueries({ queryKey: ['wms-outbounds', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '审核出库单失败');
    }
  });

  const columns = useMemo<ColumnsType<WmsOutboundDoc>>(
    () => [
      {
        title: '出库单号',
        dataIndex: 'doc_no'
      },
      {
        title: '类型',
        dataIndex: 'doc_type'
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
        title: '发货单',
        dataIndex: 'shipment_id',
        render: (_, record) => record.shipment_id ?? '-'
      },
      {
        title: '创建时间',
        dataIndex: 'created_at'
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              data-testid="outbound-detail-link"
              onClick={async () => {
                try {
                  const detailResponse = await getWmsOutbound(
                    tenantId,
                    record.outbound_doc_id
                  );
                  setDetail(detailResponse);
                  setDetailOpen(true);
                } catch (err) {
                  message.error(err instanceof Error ? err.message : '加载详情失败');
                }
              }}
            >
              查看
            </Button>
            {auditRequired ? (
              <Button
                size="small"
                disabled={!canWrite || record.status !== 'DRAFT'}
                loading={auditMutation.isPending}
                onClick={() => auditMutation.mutate(record.outbound_doc_id)}
              >
                审核
              </Button>
            ) : null}
          </Space>
        )
      }
    ],
    [auditMutation.isPending, auditRequired, canWrite, tenantId]
  );

  const handleCreate = async (values: WmsOutboundCreatePayload) => {
    try {
      await createWmsOutbound(tenantId, values);
      message.success('出库单已创建');
      setCreateOpen(false);
      createForm.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['wms-outbounds', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '创建出库单失败');
    }
  };

  const handlePrint = async () => {
    if (!detail) {
      return;
    }
    setPrinting(true);
    try {
      const job = await printWmsOutbound(tenantId, detail.doc.outbound_doc_id);
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
      message.error(err instanceof Error ? err.message : '打印出库单失败');
    } finally {
      setPrinting(false);
    }
  };

  const toolbarLeft = (
    <Space align="center" size={12} wrap={false}>
      <Button type="primary" onClick={() => setCreateOpen(true)}>
        新建出库单
      </Button>
    </Space>
  );

  const toolbarRight = (
    <UiModule02ToolbarRight
      onExport={() => message.info('导出功能待接入')}
      onPrint={() => message.info('打印功能待接入')}
      onHelp={() => message.info('帮助功能待接入')}
    />
  );

  const header = (
    <Card>
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
            <Form.Item name="warehouse_id" label="仓库">
              <Input allowClear placeholder="仓库ID" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="status" label="状态">
              <Select allowClear options={statusOptions} placeholder="选择状态" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="doc_type" label="类型">
              <Select allowClear options={docTypeOptions} placeholder="选择类型" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="shipment_id" label="发货单">
              <Input allowClear placeholder="发货单号" />
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
  );

  return (
    <>
      <UiModule03MainTableSection<WmsOutboundDoc>
        toolbarLeft={toolbarLeft}
        toolbarRight={toolbarRight}
        header={header}
        rowKey="outbound_doc_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="出库单加载失败"
        emptyText="暂无出库单"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="wms.outbounds.table.columns"
      />
      <Drawer
        title="出库单详情"
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={720}
        destroyOnClose
        data-testid="outbound-detail-drawer"
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
                打印装箱单
              </Button>
            </Space>
            <Typography.Text data-testid="outbound-detail-doc-no">
              单号：{detail.doc.doc_no}
            </Typography.Text>
            <Typography.Text>状态：{detail.doc.status}</Typography.Text>
            <Typography.Text>仓库：{detail.doc.warehouse_id}</Typography.Text>
            <Typography.Text>发货单：{detail.doc.shipment_id ?? '-'}</Typography.Text>
            <Typography.Title level={5}>明细</Typography.Title>
            {detail.items.map((item) => (
              <Typography.Text key={item.sku_id}>
                {item.sku_id} 计划 {item.planned_qty} 拣货 {item.picked_qty} 打包{' '}
                {item.packed_qty}
              </Typography.Text>
            ))}
            <Typography.Title level={5}>拣货任务</Typography.Title>
            {detail.pick_tasks.length === 0
              ? '暂无任务'
              : detail.pick_tasks.map((task) => (
                  <Typography.Text key={task.pick_task_id}>
                    {task.sku_id} 数量 {task.qty} 状态 {task.status}
                  </Typography.Text>
                ))}
            <Typography.Title level={5}>打包记录</Typography.Title>
            {detail.pack_records.length === 0
              ? '暂无记录'
              : detail.pack_records.map((record) => (
                  <Typography.Text key={record.pack_record_id}>
                    包裹 {record.package_no} / 重量 {record.weight_gram ?? '-'}g / 操作人{' '}
                    {record.operator_id ?? '-'} / 打包时间 {formatDateTime(record.packed_at)}
                  </Typography.Text>
                ))}
          </Space>
        ) : null}
      </Drawer>
      <Modal
        title="新建出库单"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={() => createForm.submit()}
        okText="提交"
      >
        <Form form={createForm} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="warehouse_id" label="仓库ID" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="doc_type" label="单据类型" rules={[{ required: true }]}>
            <Select options={docTypeOptions} />
          </Form.Item>
          <Form.Item name="doc_no" label="单号" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="shipment_id" label="发货单号">
            <Input />
          </Form.Item>
          <Form.List name="items" initialValue={[{ sku_id: '', planned_qty: 1 }]}>
            {(fields, { add, remove }) => (
              <Space direction="vertical" size={12} style={{ width: '100%' }}>
                {fields.map((field) => (
                  <Space key={field.key} align="baseline">
                    <Form.Item
                      {...field}
                      label="SKU ID"
                      name={[field.name, 'sku_id']}
                      rules={[{ required: true }]}
                    >
                      <Input placeholder="sku_id" />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      label="数量"
                      name={[field.name, 'planned_qty']}
                      rules={[{ required: true }]}
                    >
                      <InputNumber min={1} />
                    </Form.Item>
                    <Button onClick={() => remove(field.name)}>删除</Button>
                  </Space>
                ))}
                <Button onClick={() => add({ sku_id: '', planned_qty: 1 })}>添加SKU</Button>
              </Space>
            )}
          </Form.List>
        </Form>
      </Modal>
    </>
  );
};
