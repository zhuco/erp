import { Button, Form, Input, Modal, Popconfirm, Select, Typography } from 'antd';
import type { FormInstance, SelectProps } from 'antd';
import { DeleteOutlined, HolderOutlined } from '@ant-design/icons';
import type { DragEvent, ReactElement, ReactNode } from 'react';

import './UiModule01QuickFilters.css';

export type UiModule01QuickFilterItem = {
  id: string;
  name: string;
};

export type UiModule01QuickFilterConfig = UiModule01QuickFilterItem & {
  builtIn?: boolean;
};

export type UiModule01QuickFilterFormValues = {
  name: string;
  platform?: string[];
  shopId?: string[];
  shipWarehouseId?: string[];
  reviewStatus?: string;
  tagged?: boolean;
  exceptionFlags?: string[];
  omsStatus?: string[];
};

type UiModule01QuickFiltersPanelProps = {
  filters: UiModule01QuickFilterItem[];
  activeFilterId: string;
  onOpenSettings: () => void;
  onSelectFilter: (filterId: string) => void;
  maxLabelLength?: number;
  dataTestId?: string;
  settingsButtonTestId?: string;
  filterButtonTestIdPrefix?: string;
};

const truncateLabel = (value: string, maxLength: number) => {
  const chars = Array.from(value);
  if (chars.length <= maxLength) {
    return value;
  }
  return `${chars.slice(0, maxLength).join('')}…`;
};

export const UiModule01QuickFiltersPanel = ({
  filters,
  activeFilterId,
  onOpenSettings,
  onSelectFilter,
  maxLabelLength = 8,
  dataTestId,
  settingsButtonTestId,
  filterButtonTestIdPrefix,
}: UiModule01QuickFiltersPanelProps) => (
  <div className="order-review-quick-filters" data-testid={dataTestId}>
    <div className="order-review-quick-filters-header">
      <Typography.Text strong>快速查询</Typography.Text>
      <Button size="small" type="text" onClick={onOpenSettings} data-testid={settingsButtonTestId}>
        设置
      </Button>
    </div>
    <div className="order-review-quick-filters-list">
      {filters.map((filter) => {
        const isActive = filter.id === activeFilterId;
        const displayName = truncateLabel(filter.name, maxLabelLength);
        const filterButtonTestId = filterButtonTestIdPrefix
          ? `${filterButtonTestIdPrefix}-${filter.id}`
          : undefined;
        return (
          <div
            key={filter.id}
            className={`order-review-quick-filter-item${isActive ? ' active' : ''}`}
          >
            <Button
              size="small"
              type="text"
              block
              className="order-review-quick-filter-button"
              onClick={() => onSelectFilter(filter.id)}
              title={filter.name}
              data-testid={filterButtonTestId}
            >
              <span className="order-review-quick-filter-label">{displayName}</span>
            </Button>
          </div>
        );
      })}
    </div>
  </div>
);

type UiModule01QuickFiltersSettingsModalProps = {
  open: boolean;
  onCancel: () => void;
  onOk: () => void;
  title?: ReactNode;
  titleProps?: React.HTMLAttributes<HTMLSpanElement>;
  modalRender?: (modal: ReactElement) => ReactElement;
  width?: number;
  okText?: string;
  cancelText?: string;
  form: FormInstance<UiModule01QuickFilterFormValues>;
  reviewStatusOptions?: SelectProps['options'];
  platformOptions?: SelectProps['options'];
  shopOptions?: SelectProps['options'];
  warehouseOptions?: SelectProps['options'];
  orderStatusOptions?: SelectProps['options'];
  taggedOptions?: SelectProps['options'];
  exceptionFlagOptions?: SelectProps['options'];
  maxNameLength?: number;
  onCreateFilter: () => void;
  orderedFilters: UiModule01QuickFilterConfig[];
  disabledFilterIds: string[];
  draggingFilterId?: string | null;
  onDragOver: (event: DragEvent<HTMLDivElement>) => void;
  onDrop: (targetId: string) => (event: DragEvent<HTMLDivElement>) => void;
  onDragStart: (filterId: string) => (event: DragEvent<HTMLSpanElement>) => void;
  onDragEnd: () => void;
  onToggleFilterDisabled: (filterId: string) => void;
  onDeleteFilter: (filterId: string) => void;
};

export const UiModule01QuickFiltersSettingsModal = ({
  open,
  onCancel,
  onOk,
  title = '快速查询设置',
  titleProps,
  modalRender,
  width = 760,
  okText = '完成',
  cancelText = '关闭',
  form,
  reviewStatusOptions,
  platformOptions,
  shopOptions,
  warehouseOptions,
  orderStatusOptions,
  taggedOptions,
  exceptionFlagOptions,
  maxNameLength = 8,
  onCreateFilter,
  orderedFilters,
  disabledFilterIds,
  draggingFilterId,
  onDragOver,
  onDrop,
  onDragStart,
  onDragEnd,
  onToggleFilterDisabled,
  onDeleteFilter,
}: UiModule01QuickFiltersSettingsModalProps) => (
  <Modal
    title={<span {...titleProps}>{title}</span>}
    open={open}
    onCancel={onCancel}
    onOk={onOk}
    width={width}
    okText={okText}
    cancelText={cancelText}
    modalRender={modalRender}
    className="order-review-quick-settings-modal"
  >
    <div className="order-review-quick-settings">
      <div className="order-review-quick-settings-section order-review-quick-settings-create">
        <Typography.Text strong>新增按钮</Typography.Text>
        <Form
          layout="horizontal"
          labelAlign="left"
          labelCol={{ flex: '0 0 84px' }}
          wrapperCol={{ flex: 1 }}
          form={form}
          size="small"
          className="order-review-quick-settings-form"
        >
          <Form.Item
            label="按钮名称"
            name="name"
            rules={[
              { required: true, message: '请输入名称' },
              {
                validator: (_, value) => {
                  const length = Array.from((value ?? '').trim()).length;
                  if (!value || length <= maxNameLength) {
                    return Promise.resolve();
                  }
                  return Promise.reject(new Error(`最多 ${maxNameLength} 个字符`));
                },
              },
            ]}
          >
            <Input maxLength={maxNameLength} showCount />
          </Form.Item>
          <Form.Item label="审核状态" name="reviewStatus">
            <Select options={reviewStatusOptions} allowClear />
          </Form.Item>
          <Form.Item label="平台" name="platform">
            <Select mode="multiple" options={platformOptions} allowClear maxTagCount="responsive" />
          </Form.Item>
          <Form.Item label="销售渠道" name="shopId">
            <Select mode="multiple" options={shopOptions} allowClear maxTagCount="responsive" />
          </Form.Item>
          <Form.Item label="发货仓库" name="shipWarehouseId">
            <Select
              mode="multiple"
              options={warehouseOptions}
              allowClear
              maxTagCount="responsive"
            />
          </Form.Item>
          <Form.Item label="订单状态" name="omsStatus">
            <Select
              mode="multiple"
              options={orderStatusOptions}
              allowClear
              maxTagCount="responsive"
            />
          </Form.Item>
          <Form.Item label="标签" name="tagged">
            <Select options={taggedOptions} allowClear />
          </Form.Item>
          <Form.Item label="异常标记" name="exceptionFlags">
            <Select mode="multiple" options={exceptionFlagOptions} allowClear />
          </Form.Item>
          <Form.Item className="order-review-quick-settings-actions">
            <Button size="small" type="primary" onClick={onCreateFilter}>
              添加
            </Button>
          </Form.Item>
        </Form>
      </div>
      <div className="order-review-quick-settings-section order-review-quick-settings-sort">
        <Typography.Text strong>排序/删除</Typography.Text>
        <Typography.Text type="secondary">拖动调整按钮顺序</Typography.Text>
        <div className="order-review-quick-sort-list">
          {orderedFilters.map((filter) => {
            const isDisabled = disabledFilterIds.includes(filter.id);
            return (
              <div
                key={filter.id}
                className={`order-review-quick-sort-item${
                  draggingFilterId === filter.id ? ' dragging' : ''
                }${isDisabled ? ' is-disabled' : ''}`}
                onDragOver={onDragOver}
                onDrop={onDrop(filter.id)}
              >
                <span
                  className="order-review-quick-sort-handle"
                  draggable
                  onDragStart={onDragStart(filter.id)}
                  onDragEnd={onDragEnd}
                >
                  <HolderOutlined />
                </span>
                <span className="order-review-quick-sort-label">{filter.name}</span>
                {filter.builtIn ? (
                  <Button
                    size="small"
                    type="text"
                    onClick={() => onToggleFilterDisabled(filter.id)}
                  >
                    {isDisabled ? '启用' : '禁用'}
                  </Button>
                ) : (
                  <Popconfirm
                    title={'确认删除该按钮？'}
                    okText={'删除'}
                    cancelText={'取消'}
                    onConfirm={() => onDeleteFilter(filter.id)}
                  >
                    <Button size="small" type="text" danger icon={<DeleteOutlined />}>
                      {'删除'}
                    </Button>
                  </Popconfirm>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  </Modal>
);
