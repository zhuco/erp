import { Space, Tag } from 'antd';
import type { ReactNode } from 'react';

import { resolveBuiltinOrderTag } from '@/features/orderTags';

import './OrderTagBadges.css';

type OrderTagBadgesProps = {
  tags?: string[];
  emptyText?: ReactNode;
};

export const OrderTagBadges = ({ tags, emptyText = '-' }: OrderTagBadgesProps) => {
  const normalized = (tags ?? []).map((tag) => tag.trim()).filter(Boolean);
  if (!normalized.length) {
    return <>{emptyText}</>;
  }
  return (
    <Space size={[4, 4]} wrap>
      {normalized.map((tag, index) => {
        const meta = resolveBuiltinOrderTag(tag);
        if (meta) {
          return (
            <Tag key={`${tag}-${index}`} color={meta.color} title={meta.name}>
              <span className={meta.codeClassName}>{meta.code}</span>
            </Tag>
          );
        }
        return (
          <Tag key={`${tag}-${index}`} title={tag}>
            {tag}
          </Tag>
        );
      })}
    </Space>
  );
};
