import { Card } from 'antd';
import type { CardProps } from 'antd';

export type IndustrialCardProps = CardProps;

export const IndustrialCard = ({ className, size = 'small', ...props }: IndustrialCardProps) => {
  const resolvedClassName = ['industrial-workbench-card', className].filter(Boolean).join(' ');
  return <Card className={resolvedClassName} size={size} {...props} />;
};
