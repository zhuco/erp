import { Button, Empty, InputNumber, Modal, Select, Space, Switch, Table, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo } from 'react';

import type { WorkbenchColumnSettingsItem, WorkbenchColumnSortDirection } from './workbenchColumnSettings';

type WorkbenchColumnSettingsDialogProps = {
  open: boolean;
  items: WorkbenchColumnSettingsItem[];
  onApply: () => void;
  onCancel: () => void;
  onReset: () => void;
  onResetToDefault: () => void;
  onUpdateItem: (colId: string, updater: (current: WorkbenchColumnSettingsItem) => WorkbenchColumnSettingsItem) => void;
};

const resolveNextSortPriority = (items: WorkbenchColumnSettingsItem[]) =>
  items.reduce((maxValue, item) => {
    if (item.sortDirection === 'none' || typeof item.sortPriority !== 'number' || !Number.isFinite(item.sortPriority)) {
      return maxValue;
    }
    return Math.max(maxValue, Math.trunc(item.sortPriority));
  }, 0) + 1;

export const WorkbenchColumnSettingsDialog = ({
  open,
  items,
  onApply,
  onCancel,
  onReset,
  onResetToDefault,
  onUpdateItem,
}: WorkbenchColumnSettingsDialogProps) => {
  const columns = useMemo<ColumnsType<WorkbenchColumnSettingsItem>>(
    () => [
      {
        title: '显示',
        dataIndex: 'hidden',
        width: 92,
        render: (_value, record) => (
          <Switch
            size="small"
            checked={!record.hidden}
            checkedChildren="开"
            unCheckedChildren="关"
            onChange={(checked) => {
              onUpdateItem(record.colId, (current) => ({
                ...current,
                hidden: !checked,
              }));
            }}
          />
        ),
      },
      {
        title: '列名',
        dataIndex: 'label',
        width: 220,
        render: (value: string, record) => (
          <Space direction="vertical" size={0}>
            <Typography.Text strong>{value}</Typography.Text>
            <Typography.Text type="secondary">{record.colId}</Typography.Text>
          </Space>
        ),
      },
      {
        title: '排序',
        dataIndex: 'sortDirection',
        width: 150,
        render: (value: WorkbenchColumnSortDirection, record) => (
          <Select<WorkbenchColumnSortDirection>
            size="small"
            value={value}
            style={{ width: '100%' }}
            options={[
              { label: '不排序', value: 'none' },
              { label: '升序', value: 'asc' },
              { label: '降序', value: 'desc' },
            ]}
            onChange={(nextValue) => {
              onUpdateItem(record.colId, (current) => ({
                ...current,
                sortDirection: nextValue,
                sortPriority:
                  nextValue === 'none'
                    ? null
                    : typeof current.sortPriority === 'number' && Number.isFinite(current.sortPriority)
                      ? Math.max(1, Math.trunc(current.sortPriority))
                      : resolveNextSortPriority(items),
              }));
            }}
          />
        ),
      },
      {
        title: '排序优先级',
        dataIndex: 'sortPriority',
        width: 140,
        render: (_value, record) => (
          <InputNumber
            size="small"
            min={1}
            value={record.sortPriority ?? undefined}
            disabled={record.sortDirection === 'none'}
            style={{ width: '100%' }}
            placeholder="自动"
            onChange={(nextValue) => {
              onUpdateItem(record.colId, (current) => ({
                ...current,
                sortPriority:
                  record.sortDirection === 'none'
                    ? null
                    : typeof nextValue === 'number' && Number.isFinite(nextValue)
                      ? Math.max(1, Math.trunc(nextValue))
                      : 1,
              }));
            }}
          />
        ),
      },
    ],
    [items, onUpdateItem],
  );

  return (
    <Modal
      open={open}
      title="列显示与排序"
      width={760}
      className="workbench-column-settings-dialog"
      destroyOnClose={false}
      onCancel={onCancel}
      footer={[
        <Button key="reset-current" className="workbench-column-settings-dialog__button" onClick={onReset}>
          恢复当前
        </Button>,
        <Button key="reset-default" className="workbench-column-settings-dialog__button" onClick={onResetToDefault}>
          恢复默认
        </Button>,
        <Button key="cancel" className="workbench-column-settings-dialog__button" onClick={onCancel}>
          取消
        </Button>,
        <Button
          key="apply"
          type="primary"
          className="workbench-column-settings-dialog__button workbench-column-settings-dialog__button--primary"
          onClick={onApply}
        >
          应用
        </Button>,
      ]}
    >
      {items.length > 0 ? (
        <Table<WorkbenchColumnSettingsItem>
          size="small"
          rowKey="colId"
          pagination={false}
          columns={columns}
          dataSource={items}
          scroll={{ y: 420 }}
        />
      ) : (
        <Empty description="当前没有可配置的业务列" />
      )}
    </Modal>
  );
};
