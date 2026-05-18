import { Table } from 'antd';
import type { TableProps } from 'antd';

export type IndustrialTableProps<T extends object> = TableProps<T>;

export const IndustrialTable = <T extends object>({
  className,
  size = 'small',
  pagination = false,
  ...props
}: IndustrialTableProps<T>) => {
  const resolvedClassName = ['industrial-workbench-table', className].filter(Boolean).join(' ');
  return <Table<T> className={resolvedClassName} size={size} pagination={pagination} {...props} />;
};
