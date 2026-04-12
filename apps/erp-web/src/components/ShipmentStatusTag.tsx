import { Tag } from 'antd';

type ShipmentStatusTagProps = {
  status?: string | null;
};

const statusMeta: Record<string, { color: string; label: string }> = {
  REQUESTED: { color: 'blue', label: 'REQUESTED' },
  SUCCESS: { color: 'green', label: 'SUCCESS' },
  FAILED: { color: 'red', label: 'FAILED' },
  CREATED: { color: 'blue', label: 'CREATED' },
  PARTIALLY_SHIPPED: { color: 'gold', label: 'PARTIALLY_SHIPPED' },
  SHIPPED: { color: 'green', label: 'SHIPPED' },
  CANCELLED: { color: 'default', label: 'CANCELLED' },
  ON_HOLD: { color: 'orange', label: 'ON_HOLD' }
};

export const ShipmentStatusTag = ({ status }: ShipmentStatusTagProps) => {
  if (!status) {
    return <Tag>-</Tag>;
  }
  const meta = statusMeta[status];
  if (!meta) {
    return <Tag>{status}</Tag>;
  }
  return <Tag color={meta.color}>{meta.label}</Tag>;
};
