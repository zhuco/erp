import { Button, Form, Input, Select, Space } from 'antd';
import type { FormProps } from 'antd';
import { useEffect, useMemo } from 'react';

import './workbenchTableQuery.css';

export type WorkbenchQueryOption = {
  label: string;
  value: string;
};

export type WorkbenchQueryFilter = {
  key: string;
  label: string;
  options: WorkbenchQueryOption[];
  placeholder?: string;
  mode?: 'multiple' | 'tags';
  allowClear?: boolean;
  width?: number;
};

export type WorkbenchQuerySearchField = {
  key: string;
  label: string;
  placeholder?: string;
};

export type WorkbenchQueryValues = {
  search_field?: string;
  keyword?: string;
  [key: string]: string | string[] | undefined;
};

export const defaultSearchFields: WorkbenchQuerySearchField[] = [
  { key: 'order_no', label: '订单号', placeholder: '请输入订单号' },
  { key: 'platform_order_no', label: '网店订单号', placeholder: '请输入网店订单号' },
  { key: 'receiver', label: '收货人', placeholder: '请输入收货人' },
  { key: 'address', label: '收货地址', placeholder: '请输入收货地址' },
  { key: 'sku', label: '货品/货号', placeholder: '请输入货品或货号' }
];

export type WorkbenchTableQueryProps = {
  filters?: WorkbenchQueryFilter[];
  searchFields?: WorkbenchQuerySearchField[];
  value?: WorkbenchQueryValues;
  onSearch?: (values: WorkbenchQueryValues) => void;
  onChange?: (values: WorkbenchQueryValues) => void;
  onReset?: () => void;
  showReset?: boolean;
  layout?: FormProps['layout'];
  size?: 'small' | 'middle' | 'large';
  className?: string;
};

export const WorkbenchTableQuery = ({
  filters = [],
  searchFields = defaultSearchFields,
  value,
  onSearch,
  onChange,
  onReset,
  showReset = true,
  layout = 'vertical',
  size = 'small',
  className
}: WorkbenchTableQueryProps) => {
  const [form] = Form.useForm<WorkbenchQueryValues>();
  const layoutClassName = `workbench-table-query-${layout}`;
  const searchFieldOptions = useMemo(
    () => searchFields.map((field) => ({ label: field.label, value: field.key })),
    [searchFields]
  );

  const defaultSearchField = searchFields[0]?.key ?? 'keyword';

  useEffect(() => {
    if (!value) {
      form.setFieldsValue({
        search_field: defaultSearchField,
        keyword: undefined
      });
      return;
    }
    form.setFieldsValue({
      search_field: value.search_field ?? defaultSearchField,
      keyword: value.keyword,
      ...value
    });
  }, [defaultSearchField, form, value]);

  const searchField = Form.useWatch('search_field', form) ?? defaultSearchField;
  const searchPlaceholder =
    searchFields.find((field) => field.key === searchField)?.placeholder ?? '请输入关键词';

  const handleSubmit = async () => {
    const values = await form.validateFields();
    onSearch?.(values);
    onChange?.(values);
  };

  const handleReset = () => {
    form.resetFields();
    form.setFieldsValue({ search_field: defaultSearchField });
    onReset?.();
    onChange?.({ search_field: defaultSearchField, keyword: undefined });
  };

  return (
    <Form
      form={form}
      layout={layout}
      className={`workbench-table-query workbench-table-query-form ${layoutClassName} ${className ?? ''}`}
      onFinish={handleSubmit}
    >
      {filters.map((filter) => (
        <Form.Item key={filter.key} label={filter.label} name={filter.key}>
          <Select
            size={size}
            options={filter.options}
            allowClear={filter.allowClear ?? true}
            mode={filter.mode}
            placeholder={filter.placeholder ?? `请选择${filter.label}`}
            style={filter.width ? { width: filter.width } : undefined}
          />
        </Form.Item>
      ))}
      <Form.Item label="搜索字段" name="search_field">
        <Select size={size} options={searchFieldOptions} />
      </Form.Item>
      <Form.Item label="关键词" name="keyword">
        <Input.Search
          size={size}
          placeholder={searchPlaceholder}
          allowClear
          onSearch={handleSubmit}
        />
      </Form.Item>
      <Form.Item>
        <Space>
          <Button size={size} type="primary" htmlType="submit">
            查询
          </Button>
          {showReset ? (
            <Button size={size} onClick={handleReset}>
              重置
            </Button>
          ) : null}
        </Space>
      </Form.Item>
    </Form>
  );
};
