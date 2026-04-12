import { Dropdown, Table } from 'antd';
import type { MenuProps, TableProps } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { Key } from 'react';

type UiModule06StrategyTableProps<T> = {
  rowKey: TableProps<T>['rowKey'];
  columns: ColumnsType<T>;
  data: T[];
  selectedRowKeys: Key[];
  onSelectionChange: (keys: Key[]) => void;
  onRowClick?: (record: T) => void;
  onRowDoubleClick?: (record: T) => void;
  onRowContextMenu?: (record: T) => void;
  menuItems: MenuProps['items'];
  onMenuClick: MenuProps['onClick'];
  emptyText?: string;
};

export const UiModule06StrategyTable = <T,>({
  rowKey,
  columns,
  data,
  selectedRowKeys,
  onSelectionChange,
  onRowClick,
  onRowDoubleClick,
  onRowContextMenu,
  menuItems,
  onMenuClick,
  emptyText = '暂无数据'
}: UiModule06StrategyTableProps<T>) => (
  <Dropdown menu={{ items: menuItems, onClick: onMenuClick }} trigger={['contextMenu']}>
    <div>
      <Table
        rowKey={rowKey}
        size="small"
        columns={columns}
        dataSource={data}
        pagination={false}
        onRow={(record) => ({
          onClick: () => onRowClick?.(record),
          onDoubleClick: () => onRowDoubleClick?.(record),
          onContextMenu: () => onRowContextMenu?.(record)
        })}
        rowSelection={{
          selectedRowKeys,
          onChange: (keys) => onSelectionChange(keys)
        }}
        locale={{ emptyText }}
      />
    </div>
  </Dropdown>
);
