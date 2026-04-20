import type { ReactNode } from 'react';

import { resolveBuiltinOrderTag, type BuiltinOrderTag } from '@/features/orderTags';

import './OrderTagBadges.css';

type OrderTagBadgesProps = {
  tags?: Array<string | BuiltinOrderTag>;
  emptyText?: ReactNode;
  singleLine?: boolean;
};

export const OrderTagBadges = ({ tags, emptyText = '-', singleLine = false }: OrderTagBadgesProps) => {
  const normalized = (tags ?? [])
    .map((tag) => (typeof tag === 'string' ? tag.trim() : tag))
    .filter(Boolean);
  if (!normalized.length) {
    return <>{emptyText}</>;
  }

  const className = singleLine ? 'order-tag-badges order-tag-badges--single-line' : 'order-tag-badges';

  return (
    <div className={className}>
      {normalized.map((tag, index) => {
        const meta = typeof tag === 'string' ? resolveBuiltinOrderTag(tag) : tag;
        if (meta) {
          return (
            <span
              key={`${meta.key}-${index}`}
              className="order-tag-badge order-tag-badge-builtin"
              title={meta.name}
              style={{
                backgroundColor: meta.backgroundColor,
                color: meta.textColor,
              }}
            >
              <span className={meta.codeClassName}>{meta.code}</span>
            </span>
          );
        }
        const rawTag = typeof tag === 'string' ? tag : tag.name;
        return (
          <span key={`${rawTag}-${index}`} className="order-tag-badge order-tag-badge-custom" title={rawTag}>
            {rawTag}
          </span>
        );
      })}
    </div>
  );
};
