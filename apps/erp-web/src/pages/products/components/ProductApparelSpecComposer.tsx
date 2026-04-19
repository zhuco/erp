import { Button, Card, Input, Space, Typography, message } from 'antd';
import { useState } from 'react';

import { buildApparelSpecs, type ProductSpecForm } from '@/pages/products/productIndustry';

type ProductApparelSpecComposerProps = {
  specs: ProductSpecForm[];
  onApply: (nextSpecs: ProductSpecForm[]) => void;
};

export const ProductApparelSpecComposer = ({
  specs,
  onApply,
}: ProductApparelSpecComposerProps) => {
  const [colorsText, setColorsText] = useState('');
  const [sizesText, setSizesText] = useState('');

  const handleGenerate = () => {
    const nextSpecs = buildApparelSpecs(colorsText, sizesText, specs);
    if (!nextSpecs.length) {
      message.warning('请先录入颜色与尺码');
      return;
    }

    onApply(nextSpecs);
    message.success(`已合并 ${nextSpecs.length - specs.length} 条鞋服规格，当前共 ${nextSpecs.length} 条`);
  };

  return (
    <Card
      size="small"
      title="鞋服规格批量生成"
      style={{ marginBottom: 8 }}
      data-testid="products-apparel-composer"
    >
      <Space direction="vertical" size={8} style={{ width: '100%' }}>
        <Typography.Text type="secondary">
          用逗号、中文逗号、斜杠、分号或换行分隔颜色与尺码，系统会按颜色 x 尺码生成规格草稿；
          已有同组合规格会自动保留扩展字段并跳过重复追加。
        </Typography.Text>
        <Input.TextArea
          value={colorsText}
          onChange={(event) => setColorsText(event.target.value)}
          rows={2}
          placeholder="示例：黑色，白色"
          data-testid="products-apparel-colors-input"
        />
        <Input.TextArea
          value={sizesText}
          onChange={(event) => setSizesText(event.target.value)}
          rows={2}
          placeholder="示例：41，42"
          data-testid="products-apparel-sizes-input"
        />
        <Space size={8} style={{ justifyContent: 'space-between', width: '100%' }}>
          <Typography.Text type="secondary">当前规格 {specs.length} 条</Typography.Text>
          <Button
            type="primary"
            size="small"
            onClick={handleGenerate}
            data-testid="products-apparel-generate-button"
          >
            合并鞋服规格
          </Button>
        </Space>
      </Space>
    </Card>
  );
};
