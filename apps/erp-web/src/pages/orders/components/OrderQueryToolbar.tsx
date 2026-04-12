import { Form, Select } from 'antd';
import type { FormProps } from 'antd';

type Option = {
  label: string;
  value: string | number;
};

type OrderQueryToolbarLeftProps = {
  form: FormProps['form'];
  filters: FormProps['initialValues'];
  onValuesChange: FormProps['onValuesChange'];
  timeRangeOptions: Option[];
  archiveStatusOptions: Option[];
  platformOptions: Option[];
};

export const OrderQueryToolbarLeft = ({
  form,
  filters,
  onValuesChange,
  timeRangeOptions,
  archiveStatusOptions,
  platformOptions
}: OrderQueryToolbarLeftProps) => (
  <Form
    form={form}
    layout="inline"
    initialValues={filters}
    onValuesChange={onValuesChange}
    size="small"
    className="order-query-toolbar-form"
  >
    <Form.Item name="time_range" label="时间范围">
      <Select options={timeRangeOptions} style={{ width: 120 }} />
    </Form.Item>
    <Form.Item name="archive_status" label="归档状态">
      <Select options={archiveStatusOptions} style={{ width: 140 }} />
    </Form.Item>
    <Form.Item name="platform" label="平台">
      <Select options={platformOptions} allowClear style={{ width: 120 }} />
    </Form.Item>
  </Form>
);
