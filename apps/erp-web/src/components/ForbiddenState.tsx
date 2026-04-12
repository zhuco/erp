import { Result } from 'antd';
import type { ReactNode } from 'react';

type ForbiddenStateProps = {
  description?: string;
  action?: ReactNode;
};

export const ForbiddenState = ({
  description = '无权限访问该页面',
  action
}: ForbiddenStateProps) => (
  <Result status="403" title="403" subTitle={description} extra={action} />
);
