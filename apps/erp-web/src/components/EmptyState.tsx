import { Button, Empty } from 'antd';
import type { ReactNode } from 'react';

type EmptyStateProps = {
  description?: string;
  action?: ReactNode;
  onRetry?: () => void;
  retryText?: string;
};

export const EmptyState = ({
  description = '暂无数据',
  action,
  onRetry,
  retryText = '刷新'
}: EmptyStateProps) => {
  const actionNode =
    action ??
    (onRetry ? (
      <Button type="primary" size="small" onClick={onRetry}>
        {retryText}
      </Button>
    ) : null);

  return <Empty description={description}>{actionNode}</Empty>;
};
