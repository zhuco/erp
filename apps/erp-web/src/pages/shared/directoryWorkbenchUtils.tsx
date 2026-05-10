import { Empty, Table } from 'antd';
import type { TablePaginationConfig } from 'antd';

import { UiModule04BottomTable } from '@/components/ui-modules';

export type DirectoryBottomDetailRow = {
  key: string;
  field: string;
  value: string;
};

export const directoryBottomColumns = [
  { title: '字段', dataIndex: 'field', key: 'field', width: 180 },
  { title: '值', dataIndex: 'value', key: 'value', width: 320 },
];

export const trimText = (value?: string | null) => String(value ?? '').trim();

export const normalizeSelectionKeys = (
  keys: Array<string | number | null | undefined>,
): string[] =>
  Array.from(new Set(keys.map((key) => trimText(String(key ?? ''))).filter(Boolean)));

export const sliceRowsByPagination = <TRow,>(
  rows: TRow[],
  pagination: TablePaginationConfig,
): TRow[] => {
  const current = Math.max(1, Number(pagination.current ?? 1));
  const pageSize = Math.max(1, Number(pagination.pageSize ?? 20));
  const start = (current - 1) * pageSize;
  return rows.slice(start, start + pageSize);
};

export const buildStatsPieBackground = (
  segments: Array<{ value: number; color: string }>,
): string => {
  const total = segments.reduce((sum, segment) => sum + segment.value, 0) || 1;
  let offset = 0;
  const ranges = segments.map((segment) => {
    const start = offset;
    offset += (segment.value / total) * 360;
    return `${segment.color} ${start.toFixed(2)}deg ${offset.toFixed(2)}deg`;
  });
  if (offset < 360) {
    ranges.push(`#eef2f7 ${offset.toFixed(2)}deg 360deg`);
  }
  return `conic-gradient(${ranges.join(', ')})`;
};

export const renderDirectoryBottomTable = (
  active: boolean,
  rows: DirectoryBottomDetailRow[],
  emptyDescription: string,
  syncKey: string,
) =>
  active ? (
    <UiModule04BottomTable syncKey={syncKey}>
      <Table<DirectoryBottomDetailRow>
        size="small"
        rowKey="key"
        columns={directoryBottomColumns}
        dataSource={rows}
        pagination={false}
        scroll={{ x: 'max-content' }}
        locale={{ emptyText: <Empty description={emptyDescription} /> }}
      />
    </UiModule04BottomTable>
  ) : (
    <Empty description="请选择一条目录记录查看底部明细" />
  );
