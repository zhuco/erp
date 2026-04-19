import { Card, Empty, Table, Tabs, Typography } from 'antd';
import type { TablePaginationConfig } from 'antd';

import type { ShopProductMatchFailure, ShopProductTask } from '@/api/shopProducts';
import { UiModule04BottomTable } from '@/components/ui-modules';

type ShopProductWorkbenchBottomPanelProps = {
  tasks: ShopProductTask[];
  failures: ShopProductMatchFailure[];
  selectedCount: number;
  createScope: 'selected' | 'filtered';
  taskLoading?: boolean;
  taskError?: unknown;
  taskTotal?: number;
  taskPagination?: TablePaginationConfig;
  onTaskPaginationChange?: (pagination: TablePaginationConfig) => void;
};

export const ShopProductWorkbenchBottomPanel = ({
  tasks,
  failures,
  selectedCount,
  createScope,
  taskLoading,
  taskError,
  taskTotal,
  taskPagination,
  onTaskPaginationChange,
}: ShopProductWorkbenchBottomPanelProps) => (
  <div className="order-workbench-bottom-panel" data-testid="shop-products-bottom-panel">
    <UiModule04BottomTable syncKey={`${selectedCount}-${createScope}`}>
      <Tabs
        size="small"
        items={[
          {
            key: 'tasks',
            label: '任务中心',
            forceRender: true,
            children: (
              <div data-testid="shop-products-task-center">
                {taskError ? (
                  <Card size="small">
                    <Typography.Text type="danger">任务加载失败</Typography.Text>
                  </Card>
                ) : tasks.length ? (
                  <Table<ShopProductTask>
                    rowKey="task_id"
                    loading={taskLoading}
                    pagination={
                      taskPagination
                        ? {
                            ...taskPagination,
                            total: taskTotal ?? taskPagination.total ?? tasks.length,
                            showSizeChanger: true,
                          }
                        : false
                    }
                    size="small"
                    dataSource={tasks}
                    onChange={(pagination) => {
                      onTaskPaginationChange?.(pagination);
                    }}
                    columns={[
                      { title: '任务类型', dataIndex: 'task_type' },
                      { title: '状态', dataIndex: 'status' },
                      { title: '成功', dataIndex: 'success_count' },
                      { title: '失败', dataIndex: 'failure_count' },
                    ]}
                  />
                ) : (
                  <Empty description="暂无处理任务" />
                )}
              </div>
            ),
          },
          {
            key: 'failures',
            label: '匹配失败',
            forceRender: true,
            children: (
              <div data-testid="shop-products-failure-center">
                {failures.length ? (
                  <div className="shop-products-bottom-failure-list">
                    {failures.map((failure) => (
                      <div
                        key={failure.sku_record_id}
                        className="shop-products-bottom-failure-item"
                      >
                        <Typography.Text strong>
                          {failure.platform_sku_id} / {failure.merchant_sku_code ?? '-'}
                        </Typography.Text>
                        <Typography.Paragraph style={{ marginBottom: 0 }}>
                          {failure.reason}
                        </Typography.Paragraph>
                      </div>
                    ))}
                  </div>
                ) : (
                  <Empty description="最近一次自动匹配无失败项" />
                )}
              </div>
            ),
          },
          {
            key: 'batch',
            label: '批量操作',
            forceRender: true,
            children: (
              <div data-testid="shop-products-batch-context">
                <Card size="small" variant="borderless">
                  <div className="shop-products-bottom-batch-hint">
                    <Typography.Text>当前已选 {selectedCount} 条</Typography.Text>
                    <Typography.Text>
                      当前建档作用范围：{createScope === 'selected' ? '所选记录' : '当前筛选结果'}
                    </Typography.Text>
                    <Typography.Text type="secondary">
                      批量动作基于已选集合或当前筛选范围执行，右侧详情始终跟随当前激活行。
                    </Typography.Text>
                  </div>
                </Card>
              </div>
            ),
          },
        ]}
      />
    </UiModule04BottomTable>
  </div>
);
