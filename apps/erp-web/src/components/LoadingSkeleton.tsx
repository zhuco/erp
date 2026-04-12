import { Skeleton } from 'antd';

type LoadingSkeletonProps = {
  title?: boolean;
  rows?: number;
  active?: boolean;
};

export const LoadingSkeleton = ({
  title = true,
  rows = 4,
  active = true
}: LoadingSkeletonProps) => (
  <Skeleton active={active} title={title} paragraph={{ rows }} />
);
