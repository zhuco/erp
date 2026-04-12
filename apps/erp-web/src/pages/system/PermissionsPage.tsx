import { Space, Typography } from 'antd';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import type { ColumnsType } from 'antd/es/table';

import { listPermissions, type PermissionDefinition } from '@/api/system';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { isTenantSelected, useTenantId } from '@/features/tenant';

export const PermissionsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const [pagination, setPagination] = useState({ current: 1, pageSize: 20 });

  const query = useQuery({
    queryKey: ['permissions', tenantId],
    queryFn: () => listPermissions(tenantId),
    enabled: tenantReady
  });

  const data = query.data ?? [];
  const current = pagination.current ?? 1;
  const pageSize = pagination.pageSize ?? 20;
  const start = (current - 1) * pageSize;
  const pageData = data.slice(start, start + pageSize);

  const columns = useMemo<ColumnsType<PermissionDefinition>>(
    () => [
      { title: '权限编码', dataIndex: 'code' },
      { title: '模块', dataIndex: 'module' },
      { title: '名称', dataIndex: 'name' },
      { title: '描述', dataIndex: 'description' }
    ],
    []
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        权限目录
      </Typography.Title>
    </Space>
  );

  return (
    <UiModule03MainTableSection<PermissionDefinition>
      header={header}
      columns={columns}
      data={pageData}
      loading={query.isFetching}
      error={query.error}
      onRetry={() => query.refetch()}
      errorTitle="加载权限失败"
      emptyText="暂无权限"
      total={data.length}
      pagination={pagination}
      onPaginationChange={setPagination}
      rowKey="code"
      enablePanels={false}
    />
  );
};
