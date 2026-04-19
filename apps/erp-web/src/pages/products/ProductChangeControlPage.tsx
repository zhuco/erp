import {
  Button,
  Card,
  Col,
  Drawer,
  Form,
  Input,
  Modal,
  Row,
  Select,
  Space,
  Steps,
  Tag,
  Timeline,
  Typography,
  message,
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type { ProductChangeRequestListQuery } from '@/api/productChangeRequests';
import {
  createProductChangeRequest,
  listProductChangeRequests,
  updateProductChangeRequest,
} from '@/api/productChangeRequests';
import { listSkus } from '@/api/skus';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useSessionStore } from '@/features/sessionStore';
import { useTenantId } from '@/features/tenant';

type ChangeStatus = 'DRAFT' | 'REVIEWING' | 'APPROVED' | 'REJECTED' | 'PUBLISHED';

type ChangeHistoryItem = {
  time: string;
  action: string;
  actor: string;
  note?: string;
};

type ChangeRequest = {
  id: string;
  sku_id: string;
  sku_code: string;
  sku_name: string;
  title: string;
  change_type: string;
  status: ChangeStatus;
  applicant: string;
  approver?: string;
  created_at: string;
  updated_at: string;
  reason: string;
  history: ChangeHistoryItem[];
};

type FilterValues = {
  keyword?: string;
  status?: ChangeStatus;
  change_type?: string;
};

type FormValues = {
  sku_id: string;
  name: string;
  title: string;
  change_type: string;
  reason: string;
};

const statusLabels: Record<ChangeStatus, string> = {
  DRAFT: '草稿',
  REVIEWING: '审批中',
  APPROVED: '已通过',
  REJECTED: '已驳回',
  PUBLISHED: '已生效',
};

const statusColors: Record<ChangeStatus, string> = {
  DRAFT: 'default',
  REVIEWING: 'gold',
  APPROVED: 'green',
  REJECTED: 'red',
  PUBLISHED: 'blue',
};

const changeTypeOptions = [
  { label: '基础信息', value: '基础信息' },
  { label: '规格条码', value: '规格条码' },
  { label: '渠道配置', value: '渠道配置' },
  { label: '包装层级', value: '包装层级' },
];

const addHistory = (
  request: ChangeRequest,
  action: string,
  actor: string,
  note?: string,
): ChangeRequest => ({
  ...request,
  history: [...request.history, { time: dayjs().toISOString(), action, actor, note }],
  updated_at: dayjs().toISOString(),
});

export const ProductChangeControlPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('skus.write');
  const userName = useSessionStore((state) => state.userName) || '当前用户';
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [activeRequestId, setActiveRequestId] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const listQuery = useMemo<ProductChangeRequestListQuery>(
    () => ({
      status: filters.status,
      change_type: filters.change_type,
      offset: 0,
      limit: 200,
    }),
    [filters.change_type, filters.status],
  );

  const { data, isLoading, isFetching, refetch } = useQuery({
    queryKey: ['product-change-requests', tenantId, listQuery],
    queryFn: () => listProductChangeRequests(tenantId, listQuery),
    enabled: Boolean(tenantId),
  });

  const skuQuery = useQuery({
    queryKey: ['skus', tenantId],
    queryFn: () => listSkus(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
  });

  const skuMap = useMemo(
    () => new Map((skuQuery.data?.data ?? []).map((sku) => [sku.sku_id, sku])),
    [skuQuery.data?.data],
  );

  const skuOptions = useMemo(
    () =>
      (skuQuery.data?.data ?? []).map((sku) => ({
        value: sku.sku_id,
        label: `${sku.sku_code}${sku.name ? ` | ${sku.name}` : ''}`,
      })),
    [skuQuery.data?.data],
  );

  const records = useMemo<ChangeRequest[]>(
    () =>
      (data?.items ?? []).map((item) => {
        const sku = skuMap.get(item.sku_id);
        return {
          id: item.request_id,
          sku_id: item.sku_id,
          sku_code: item.sku_code ?? sku?.sku_code ?? item.sku_id,
          sku_name: item.sku_name ?? sku?.name ?? '',
          title: item.title,
          change_type: item.change_type,
          status: item.status as ChangeStatus,
          applicant: item.applicant,
          approver: item.approver ?? undefined,
          created_at: item.created_at ?? '',
          updated_at: item.updated_at ?? item.created_at ?? '',
          reason: item.reason ?? '',
          history: item.history ?? [],
        };
      }),
    [data?.items, skuMap],
  );

  const activeRequest = useMemo(
    () => records.find((item) => item.id === activeRequestId) ?? null,
    [activeRequestId, records],
  );

  const filteredData = useMemo(() => {
    const keyword = filters.keyword?.trim();
    return records.filter((item) => {
      if (filters.status && item.status !== filters.status) {
        return false;
      }
      if (filters.change_type && item.change_type !== filters.change_type) {
        return false;
      }
      if (keyword) {
        const target = `${item.sku_code}${item.sku_name}${item.title}`.toLowerCase();
        return target.includes(keyword.toLowerCase());
      }
      return true;
    });
  }, [filters.change_type, filters.keyword, filters.status, records]);

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreateDrawer = () => {
    setDrawerOpen(true);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({ sku_id: '', name: '' });
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      if (!tenantId) {
        message.warning('请先选择租户');
        return;
      }
      setSaving(true);
      await createProductChangeRequest(tenantId, {
        sku_id: values.sku_id,
        title: values.title.trim(),
        change_type: values.change_type,
        status: 'DRAFT',
        applicant: userName,
        reason: values.reason.trim(),
        history: [
          {
            time: dayjs().toISOString(),
            action: '保存草稿',
            actor: userName,
          },
        ],
      });
      message.success('变更草稿已保存');
      setDrawerOpen(false);
      await queryClient.invalidateQueries({ queryKey: ['product-change-requests', tenantId] });
    } catch (error) {
      if (error instanceof Error && error.message) {
        message.error(error.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const updateRequest = async (
    record: ChangeRequest,
    updates: Partial<ChangeRequest>,
    action: string,
    actor: string,
    note?: string,
  ): Promise<boolean> => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return false;
    }
    const updated = addHistory({ ...record, ...updates }, action, actor, note);
    try {
      await updateProductChangeRequest(tenantId, record.id, {
        title: updated.title,
        change_type: updated.change_type,
        status: updated.status,
        approver: updated.approver ?? null,
        reason: updated.reason,
        history: updated.history,
      });
      if (activeRequestId === record.id) {
        setActiveRequestId(record.id);
      }
      await queryClient.invalidateQueries({ queryKey: ['product-change-requests', tenantId] });
      return true;
    } catch (error) {
      message.error(error instanceof Error ? error.message : '更新变更申请失败');
      return false;
    }
  };

  const handleSubmit = async (record: ChangeRequest) => {
    const success = await updateRequest(
      record,
      { status: 'REVIEWING', approver: '审批经理' },
      '提交审批',
      userName,
    );
    if (success) {
      message.success('已提交审批');
    }
  };

  const handleApprove = async (record: ChangeRequest) => {
    const success = await updateRequest(
      record,
      { status: 'APPROVED', approver: '审批经理' },
      '审批通过',
      '审批经理',
    );
    if (success) {
      message.success('审批已通过');
    }
  };

  const handleReject = async (record: ChangeRequest) => {
    const success = await updateRequest(
      record,
      { status: 'REJECTED', approver: '审批经理' },
      '审批驳回',
      '审批经理',
      '需要补充附件',
    );
    if (success) {
      message.warning('已驳回变更申请');
    }
  };

  const handlePublish = async (record: ChangeRequest) => {
    const success = await updateRequest(
      record,
      { status: 'PUBLISHED' },
      '同步下游',
      '主数据同步服务',
    );
    if (success) {
      message.success('变更已生效并同步下游');
    }
  };

  const columns = useMemo<ColumnsType<ChangeRequest>>(
    () => [
      { title: '变更单号', dataIndex: 'id' },
      { title: 'SKU', dataIndex: 'sku_code' },
      { title: '商品名称', dataIndex: 'sku_name' },
      { title: '变更主题', dataIndex: 'title' },
      { title: '类型', dataIndex: 'change_type' },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: ChangeStatus) => (
          <Tag color={statusColors[value]}>{statusLabels[value]}</Tag>
        ),
      },
      { title: '申请人', dataIndex: 'applicant' },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value: string) => dayjs(value).format('YYYY-MM-DD HH:mm'),
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space wrap>
            <Button
              size="small"
              onClick={() => {
                setActiveRequestId(record.id);
                setDetailOpen(true);
              }}
            >
              详情
            </Button>
            {record.status === 'DRAFT' && (
              <Button size="small" disabled={!canWrite} onClick={() => handleSubmit(record)}>
                提交审批
              </Button>
            )}
            {record.status === 'REVIEWING' && (
              <>
                <Button size="small" disabled={!canWrite} onClick={() => handleApprove(record)}>
                  通过
                </Button>
                <Button size="small" disabled={!canWrite} onClick={() => handleReject(record)}>
                  驳回
                </Button>
              </>
            )}
            {record.status === 'APPROVED' && (
              <Button size="small" disabled={!canWrite} onClick={() => handlePublish(record)}>
                发布
              </Button>
            )}
            {record.status === 'REJECTED' && (
              <Button size="small" disabled={!canWrite} onClick={() => handleSubmit(record)}>
                重新提交
              </Button>
            )}
          </Space>
        ),
      },
    ],
    [canWrite],
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }} data-testid="product-change-title">
          主数据变更控制
        </Typography.Title>
        <Button
          type="primary"
          onClick={openCreateDrawer}
          disabled={!canWrite}
          data-testid="product-change-create-button"
        >
          新建变更
        </Button>
      </Space>
      <Card title="审批流程" data-testid="product-change-steps">
        <Steps
          items={[
            { title: '提交申请', description: '填写变更内容与影响范围' },
            { title: '审批复核', description: '关联条码/包装/渠道影响' },
            { title: '同步生效', description: '下游系统自动同步' },
          ]}
        />
      </Card>
      <Card>
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSearch}
          data-testid="product-change-filter-form"
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键词">
                <Input allowClear placeholder="SKU/变更主题" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="change_type" label="变更类型">
                <Select allowClear options={changeTypeOptions} placeholder="全部类型" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select
                  allowClear
                  options={Object.entries(statusLabels).map(([value, label]) => ({
                    value,
                    label,
                  }))}
                  placeholder="全部状态"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={4} style={{ display: 'flex', alignItems: 'end' }}>
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
      <Card title="变更清单">
        <UiModule03MainTableSection<ChangeRequest>
          dataTestId="product-change-table"
          rowKey="id"
          columns={columns}
          data={filteredData}
          loading={isLoading || isFetching}
          onRetry={() => refetch()}
          errorTitle="变更加载失败"
          emptyText="暂无变更"
          total={filteredData.length}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="product.change.table.columns"
          enablePanels={false}
        />
      </Card>
      <Drawer
        title="新建变更"
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnHidden
        data-testid="product-change-drawer"
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saving} onClick={handleSave}>
              保存草稿
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical" data-testid="product-change-drawer-form">
          <Form.Item name="sku_id" label="SKU" rules={[{ required: true, message: '请选择SKU' }]}>
            <Select
              showSearch
              options={skuOptions}
              placeholder="选择SKU"
              loading={skuQuery.isLoading}
              filterOption={(input, option) =>
                (option?.label ?? '').toString().toLowerCase().includes(input.toLowerCase())
              }
              onChange={(value) => {
                const sku = skuMap.get(value);
                drawerForm.setFieldsValue({ name: sku?.name ?? '' });
              }}
            />
          </Form.Item>
          <Form.Item
            name="name"
            label="商品名称"
            rules={[{ required: true, message: '请输入商品名称' }]}
          >
            <Input placeholder="商品名称" disabled />
          </Form.Item>
          <Form.Item
            name="change_type"
            label="变更类型"
            rules={[{ required: true, message: '请选择变更类型' }]}
          >
            <Select options={changeTypeOptions} />
          </Form.Item>
          <Form.Item
            name="title"
            label="变更主题"
            rules={[{ required: true, message: '请输入变更主题' }]}
          >
            <Input placeholder="如：新增外箱条码" />
          </Form.Item>
          <Form.Item
            name="reason"
            label="变更说明"
            rules={[{ required: true, message: '请输入变更说明' }]}
          >
            <Input.TextArea rows={4} placeholder="说明变更原因与影响范围" />
          </Form.Item>
        </Form>
      </Drawer>
      <Modal
        open={detailOpen}
        title="变更详情"
        onCancel={() => setDetailOpen(false)}
        footer={[
          <Button key="close" onClick={() => setDetailOpen(false)}>
            关闭
          </Button>,
        ]}
        width={720}
        data-testid="product-change-detail-modal"
      >
        {activeRequest ? (
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Card size="small">
              <Typography.Title level={5} style={{ marginTop: 0 }}>
                {activeRequest.title}
              </Typography.Title>
              <Space wrap>
                <Tag color={statusColors[activeRequest.status]}>
                  {statusLabels[activeRequest.status]}
                </Tag>
                <Tag>{activeRequest.change_type}</Tag>
              </Space>
              <Typography.Paragraph style={{ marginTop: 12 }}>
                {activeRequest.reason}
              </Typography.Paragraph>
              <Typography.Text type="secondary">
                申请人：{activeRequest.applicant} · 更新时间：
                {dayjs(activeRequest.updated_at).format('YYYY-MM-DD HH:mm')}
              </Typography.Text>
            </Card>
            <Card size="small" title="审批记录">
              <Timeline
                items={activeRequest.history.map((item) => ({
                  children: (
                    <Space direction="vertical" size={4}>
                      <Typography.Text>
                        {item.action} · {item.actor}
                      </Typography.Text>
                      {item.note ? (
                        <Typography.Text type="secondary">{item.note}</Typography.Text>
                      ) : null}
                      <Typography.Text type="secondary">
                        {dayjs(item.time).format('YYYY-MM-DD HH:mm')}
                      </Typography.Text>
                    </Space>
                  ),
                }))}
              />
            </Card>
          </Space>
        ) : null}
      </Modal>
    </Space>
  );
};
