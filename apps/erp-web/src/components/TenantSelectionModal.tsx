import { Button, Form, Modal, Select, Space, Typography, message } from 'antd';
import { useEffect } from 'react';

import { isTenantSelected } from '@/features/tenant';

type TenantSelectionForm = {
  tenantId: string;
};

type TenantSelectionModalProps = {
  open: boolean;
  options: Array<{ label: string; value: string }>;
  selectedTenantId?: string;
  onConfirm: (tenantId: string) => void;
};

export const TenantSelectionModal = ({
  open,
  options,
  selectedTenantId,
  onConfirm
}: TenantSelectionModalProps) => {
  const [form] = Form.useForm<TenantSelectionForm>();

  useEffect(() => {
    if (open) {
      form.setFieldsValue({ tenantId: selectedTenantId });
    }
  }, [form, open, selectedTenantId]);

  const handleSubmit = (values: TenantSelectionForm) => {
    if (!isTenantSelected(values.tenantId)) {
      message.error('请选择商家');
      return;
    }
    onConfirm(values.tenantId);
  };

  return (
    <Modal
      open={open}
      title="选择商家"
      footer={null}
      closable={false}
      maskClosable={false}
      keyboard={false}
      destroyOnClose
    >
      <Space direction="vertical" size={16} style={{ width: '100%' }}>
        <Typography.Text type="secondary">进入系统前请选择商家。</Typography.Text>
        <Form form={form} layout="vertical" onFinish={handleSubmit}>
          <Form.Item
            name="tenantId"
            label="商家"
            rules={[{ required: true, message: '请选择商家' }]}
          >
            <Select
              placeholder="选择商家"
              options={options}
              data-testid="tenant-select"
            />
          </Form.Item>
          <Button type="primary" htmlType="submit" block data-testid="tenant-submit">
            进入系统
          </Button>
        </Form>
      </Space>
    </Modal>
  );
};
