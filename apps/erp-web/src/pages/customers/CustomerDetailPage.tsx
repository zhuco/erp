import { Button, Descriptions, Drawer, Form, Input, Select, Space, Tag, message } from 'antd';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Customer } from '@/api/customers';
import {
  getCustomerDetail,
  replaceCustomerTags,
  updateCustomer
} from '@/api/customers';
import { listCustomerCategories } from '@/api/customerCategories';
import { listCustomerTags } from '@/api/customerTags';
import { listMemberLevels } from '@/api/memberLevels';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FormValues = {
  name: string;
  phone?: string;
  address?: string;
  note?: string;
  status?: string;
  category_id?: string;
  member_level_id?: string;
};

const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'INACTIVE' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const CustomerDetailPage = () => {
  const { customer_id } = useParams();
  const navigate = useNavigate();
  const tenantId = useTenantId();
  const canWrite = useCan('customers.write');
  const queryClient = useQueryClient();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [tagSelection, setTagSelection] = useState<string[]>([]);
  const [form] = Form.useForm<FormValues>();

  const { data, error, refetch } = useQuery({
    queryKey: ['customer-detail', tenantId, customer_id],
    queryFn: () => getCustomerDetail(tenantId, customer_id ?? ''),
    enabled: Boolean(customer_id)
  });

  const { data: categoryData } = useQuery({
    queryKey: ['customer-categories', tenantId],
    queryFn: () =>
      listCustomerCategories(tenantId, {
        offset: 0,
        limit: 200
      })
  });

  const { data: levelData } = useQuery({
    queryKey: ['member-levels', tenantId],
    queryFn: () =>
      listMemberLevels(tenantId, {
        offset: 0,
        limit: 200
      })
  });

  const { data: tagData } = useQuery({
    queryKey: ['customer-tags', tenantId],
    queryFn: () =>
      listCustomerTags(tenantId, {
        offset: 0,
        limit: 200
      })
  });

  const customer = data?.customer;
  const tags = data?.tags ?? [];

  useEffect(() => {
    if (tags.length) {
      setTagSelection(tags.map((tag) => tag.tag_id));
    } else {
      setTagSelection([]);
    }
  }, [tags]);

  const categoryOptions = useMemo(
    () =>
      (categoryData?.items ?? []).map((item) => ({
        label: item.name,
        value: item.category_id
      })),
    [categoryData]
  );

  const levelOptions = useMemo(
    () =>
      (levelData?.items ?? []).map((item) => ({
        label: item.name,
        value: item.level_id
      })),
    [levelData]
  );

  const tagOptions = useMemo(
    () =>
      (tagData?.items ?? []).map((item) => ({
        label: item.name,
        value: item.tag_id
      })),
    [tagData]
  );

  const tagMutation = useMutation({
    mutationFn: (tagIds: string[]) =>
      replaceCustomerTags(tenantId, customer_id ?? '', { tag_ids: tagIds }),
    onSuccess: () => {
      message.success('客户标签已更新');
      queryClient.invalidateQueries({ queryKey: ['customer-detail', tenantId, customer_id] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '更新客户标签失败');
    }
  });

  const handleSave = async () => {
    if (!customer_id) {
      return;
    }
    try {
      const values = await form.validateFields();
      const payload = {
        name: values.name.trim(),
        phone: values.phone?.trim() || undefined,
        address: values.address?.trim() || undefined,
        note: values.note?.trim() || undefined,
        status: values.status,
        category_id: values.category_id,
        member_level_id: values.member_level_id
      };
      setSaving(true);
      await updateCustomer(tenantId, customer_id, payload);
      message.success('客户已更新');
      setDrawerOpen(false);
      await queryClient.invalidateQueries({ queryKey: ['customer-detail', tenantId, customer_id] });
      await queryClient.invalidateQueries({ queryKey: ['customers', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const openEdit = (detail: Customer) => {
    form.setFieldsValue({
      name: detail.name,
      phone: detail.phone ?? undefined,
      address: detail.address ?? undefined,
      note: detail.note ?? undefined,
      status: detail.status ?? undefined,
      category_id: detail.category_id ?? undefined,
      member_level_id: detail.member_level_id ?? undefined
    });
    setDrawerOpen(true);
  };

  if (error) {
    return (
      <UiModule07SettingsLayout title="客户详情">
        <UiModule07SettingsSection title="客户详情">
          <Space direction="vertical">
            <div>加载客户失败</div>
            <Button onClick={() => refetch()}>重试</Button>
          </Space>
        </UiModule07SettingsSection>
      </UiModule07SettingsLayout>
    );
  }

  return (
    <UiModule07SettingsLayout title="客户详情">
      <UiModule07SettingsSection
        title="客户信息"
        actions={
          <Space>
            <Button onClick={() => navigate(-1)}>返回</Button>
            <Button
              type="primary"
              disabled={!canWrite || !customer}
              onClick={() => customer && openEdit(customer)}
            >
              编辑客户
            </Button>
          </Space>
        }
      >
        <Descriptions column={3} bordered size="small">
          <Descriptions.Item label="客户姓名">{customer?.name ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="电话">{customer?.phone ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="状态">
            {customer ? (
              <Tag color={customer.status === 'ACTIVE' ? 'green' : 'default'}>
                {customer.status ?? '-'}
              </Tag>
            ) : (
              '-'
            )}
          </Descriptions.Item>
          <Descriptions.Item label="客户分类">{customer?.category_name ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="会员等级">{customer?.member_level_name ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="更新时间">
            {formatDateTime(customer?.updated_at ?? customer?.created_at)}
          </Descriptions.Item>
          <Descriptions.Item label="地址" span={3}>
            {customer?.address ?? '-'}
          </Descriptions.Item>
          <Descriptions.Item label="备注" span={3}>
            {customer?.note ?? '-'}
          </Descriptions.Item>
        </Descriptions>
      </UiModule07SettingsSection>
      <UiModule07SettingsSection title="客户标签">
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          <Select
            mode="multiple"
            allowClear
            value={tagSelection}
            onChange={(values) => setTagSelection(values)}
            options={tagOptions}
            placeholder="选择客户标签"
          />
          <Button
            type="primary"
            disabled={!canWrite}
            loading={tagMutation.isPending}
            onClick={() => tagMutation.mutate(tagSelection)}
          >
            保存标签
          </Button>
        </Space>
      </UiModule07SettingsSection>
      <Drawer
        title="编辑客户"
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saving} onClick={handleSave}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="name"
            label="客户姓名"
            rules={[{ required: true, message: '请输入客户姓名' }]}
          >
            <Input placeholder="客户姓名" />
          </Form.Item>
          <Form.Item name="phone" label="电话">
            <Input placeholder="手机号" />
          </Form.Item>
          <Form.Item name="address" label="地址">
            <Input placeholder="地址" />
          </Form.Item>
          <Form.Item name="note" label="备注">
            <Input.TextArea rows={3} placeholder="备注" />
          </Form.Item>
          <Form.Item name="category_id" label="客户分类">
            <Select allowClear options={categoryOptions} placeholder="选择分类" />
          </Form.Item>
          <Form.Item name="member_level_id" label="会员等级">
            <Select allowClear options={levelOptions} placeholder="选择等级" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select allowClear options={statusOptions} placeholder="选择状态" />
          </Form.Item>
        </Form>
      </Drawer>
    </UiModule07SettingsLayout>
  );
};
