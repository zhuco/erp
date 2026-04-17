import {
  Button,
  Card,
  Col,
  Drawer,
  Form,
  Input,
  Popconfirm,
  Row,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Supplier, SupplierCreatePayload } from '@/api/suppliers';
import { createSupplier, deleteSupplier, listSuppliers, updateSupplier } from '@/api/suppliers';
import { ServerTable } from '@/components/ServerTable';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
};

type FormValues = {
  name: string;
  contact?: string;
  settlement_terms?: string;
  status?: string;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const statusOptions = [
  { label: 'ACTIVE', value: 'ACTIVE' },
  { label: 'INACTIVE', value: 'INACTIVE' }
];

export const SuppliersPage = () => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingSupplier, setEditingSupplier] = useState<Supplier | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['suppliers', tenantId, query],
    queryFn: () => listSuppliers(tenantId, query),
    keepPreviousData: true
  });

  const deleteMutation = useMutation({
    mutationFn: (supplierId: string) => deleteSupplier(tenantId, supplierId),
    onMutate: (supplierId) => {
      setDeletingId(supplierId);
    },
    onSuccess: () => {
      message.success('Supplier deleted');
      queryClient.invalidateQueries({ queryKey: ['suppliers', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : 'Failed to delete supplier');
    },
    onSettled: () => {
      setDeletingId(null);
    }
  });

  const columns = useMemo<ColumnsType<Supplier>>(
    () => [
      {
        title: 'Supplier',
        dataIndex: 'name'
      },
      {
        title: 'Contact',
        dataIndex: 'contact'
      },
      {
        title: 'Settlement',
        dataIndex: 'settlement_terms'
      },
      {
        title: 'Status',
        dataIndex: 'status',
        render: (value?: string | null) => (
          <Tag color={value === 'ACTIVE' ? 'green' : 'default'}>{value ?? '-'}</Tag>
        )
      },
      {
        title: 'Updated At',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      },
      {
        title: 'Actions',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              onClick={() => {
                setEditingSupplier(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  name: record.name,
                  contact: record.contact ?? undefined,
                  settlement_terms: record.settlement_terms ?? undefined,
                  status: record.status ?? undefined
                });
              }}
            >
              Edit
            </Button>
            <Popconfirm
              title="Confirm delete this supplier?"
              okText="Delete"
              cancelText="Cancel"
              onConfirm={() => deleteMutation.mutate(record.supplier_id)}
            >
              <Button
                size="small"
                danger
                loading={deleteMutation.isPending && deletingId === record.supplier_id}
              >
                Delete
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [deleteMutation, deletingId, drawerForm]
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

  const openCreateDrawer = () => {
    setEditingSupplier(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: SupplierCreatePayload = {
        name: values.name.trim(),
        contact: values.contact?.trim() || undefined,
        settlement_terms: values.settlement_terms?.trim() || undefined,
        status: values.status
      };
      setSaving(true);
      if (editingSupplier) {
        await updateSupplier(tenantId, editingSupplier.supplier_id, payload);
        message.success('Supplier updated');
      } else {
        await createSupplier(tenantId, payload);
        message.success('Supplier created');
      }
      setDrawerOpen(false);
      setEditingSupplier(null);
      await queryClient.invalidateQueries({ queryKey: ['suppliers', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Row justify="space-between" align="middle">
        <Typography.Title level={3} style={{ margin: 0 }} data-testid="suppliers-title">
          Suppliers
        </Typography.Title>
        <Space>
          <Button type="primary" onClick={openCreateDrawer} data-testid="suppliers-create">
            New Supplier
          </Button>
        </Space>
      </Row>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={[16, 16]}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="Keyword">
                <Input allowClear placeholder="Search by supplier name" />
              </Form.Item>
            </Col>
            <Col
              xs={24}
              md={16}
              style={{ display: 'flex', alignItems: 'end', justifyContent: 'flex-end' }}
            >
              <Space>
                <Button type="primary" htmlType="submit" data-testid="suppliers-search">
                  Search
                </Button>
                <Button onClick={handleReset} data-testid="suppliers-reset">
                  Reset
                </Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
      <ServerTable<Supplier>
        rowKey="supplier_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="Failed to load suppliers"
        emptyText="No suppliers"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="suppliers.table.columns"
      />
      <Drawer
        title={editingSupplier ? 'Edit supplier' : 'New supplier'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>Cancel</Button>
            <Button type="primary" loading={saving} onClick={handleSave}>
              Save
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item
            name="name"
            label="Supplier name"
            rules={[{ required: true, message: 'Please enter supplier name' }]}
          >
            <Input placeholder="Supplier name" />
          </Form.Item>
          <Form.Item name="contact" label="Contact">
            <Input placeholder="Contact person / phone" />
          </Form.Item>
          <Form.Item name="settlement_terms" label="Settlement terms">
            <Input placeholder="Monthly / Weekly" />
          </Form.Item>
          <Form.Item name="status" label="Status">
            <Select allowClear options={statusOptions} placeholder="Select status" />
          </Form.Item>
        </Form>
      </Drawer>
    </Space>
  );
};
