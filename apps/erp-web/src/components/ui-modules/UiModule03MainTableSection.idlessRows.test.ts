import { expect, it } from 'vitest';

import { UiModule03MainTableSection } from './UiModule03MainTableSection';

type TransferRow = {
  transfer_id: string;
  status: string;
};

it('accepts row types that rely on rowKey instead of a literal id field', () => {
  const props: Parameters<typeof UiModule03MainTableSection<TransferRow>>[0] = {
    rowKey: 'transfer_id',
    columns: [
      {
        title: '调拨单号',
        dataIndex: 'transfer_id',
      },
      {
        title: '状态',
        dataIndex: 'status',
      },
    ],
    data: [
      {
        transfer_id: 'TR-001',
        status: 'DRAFT',
      },
    ],
    total: 1,
    pagination: { current: 1, pageSize: 20 },
    onPaginationChange: () => undefined,
    onRetry: () => undefined,
  };

  expect(props.rowKey).toBe('transfer_id');
});
