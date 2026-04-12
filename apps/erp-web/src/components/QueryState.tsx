import { ReloadOutlined } from '@ant-design/icons';
import { Alert, Button, Space, Spin, Typography } from 'antd';
import type { ReactNode } from 'react';
import { useEffect, useState } from 'react';

import { getErrorMessage } from '@/utils/errors';

const DEFAULT_LOADING_TIMEOUT_MS = 12000;

const useSlowLoading = (loading: boolean, timeoutMs: number) => {
  const [slow, setSlow] = useState(false);

  useEffect(() => {
    if (!loading) {
      setSlow(false);
      return undefined;
    }
    const timer = window.setTimeout(() => setSlow(true), timeoutMs);
    return () => window.clearTimeout(timer);
  }, [loading, timeoutMs]);

  return slow;
};

type LoadingStateProps = {
  loading: boolean;
  timeoutMs?: number;
  message?: string;
  onRetry?: () => void;
};

export const LoadingState = ({
  loading,
  timeoutMs = DEFAULT_LOADING_TIMEOUT_MS,
  message = '正在加载数据',
  onRetry
}: LoadingStateProps) => {
  const slow = useSlowLoading(loading, timeoutMs);

  if (!loading) {
    return null;
  }

  return (
    <Space direction="vertical" align="center" style={{ width: '100%', padding: '32px 0' }}>
      <Spin />
      <Typography.Text>{message}</Typography.Text>
      {slow ? (
        <Typography.Text type="secondary">
          加载时间较长，请检查网络或稍后重试。
        </Typography.Text>
      ) : null}
      {slow && onRetry ? (
        <Button icon={<ReloadOutlined />} onClick={onRetry}>
          重试
        </Button>
      ) : null}
    </Space>
  );
};

type LoadingNoticeProps = {
  loading: boolean;
  timeoutMs?: number;
  onRetry?: () => void;
};

export const LoadingNotice = ({
  loading,
  timeoutMs = DEFAULT_LOADING_TIMEOUT_MS,
  onRetry
}: LoadingNoticeProps) => {
  const slow = useSlowLoading(loading, timeoutMs);

  if (!loading || !slow) {
    return null;
  }

  return (
    <Alert
      type="warning"
      showIcon
      message="加载时间较长"
      description="网络可能不稳定，请检查网络后重试。"
      action={
        onRetry ? (
          <Button size="small" icon={<ReloadOutlined />} onClick={onRetry}>
            重试
          </Button>
        ) : null
      }
    />
  );
};

type ErrorStateProps = {
  title?: string;
  error?: unknown;
  description?: string;
  hint?: string;
  onRetry?: () => void;
  action?: ReactNode;
};

export const ErrorState = ({
  title = '加载失败',
  error,
  description,
  hint = '请检查网络或稍后重试。',
  onRetry,
  action
}: ErrorStateProps) => {
  const message = description ?? getErrorMessage(error);
  const actionNode =
    action ??
    (onRetry ? (
      <Button size="small" icon={<ReloadOutlined />} onClick={onRetry}>
        重试
      </Button>
    ) : null);

  return (
    <Alert
      type="error"
      showIcon
      message={title}
      description={
        <Space direction="vertical" size={4}>
          <span>{message}</span>
          <Typography.Text type="secondary">{hint}</Typography.Text>
        </Space>
      }
      action={actionNode}
    />
  );
};
