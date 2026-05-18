import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('goods module source contract', () => {
  it('creates the goods domain structure for query pages, forms, and shared utilities', () => {
    expect(existsSync(resolve(__dirname, './products-query'))).toBe(true);
    expect(existsSync(resolve(__dirname, './kits-query'))).toBe(true);
    expect(existsSync(resolve(__dirname, './product-form'))).toBe(true);
    expect(existsSync(resolve(__dirname, './kit-form'))).toBe(true);
    expect(existsSync(resolve(__dirname, './shared'))).toBe(true);
  });

  it('implements goods products query with the unified workbench template and real product api', () => {
    const source = readFileSync(
      resolve(__dirname, './products-query/GoodsProductsQueryPage.tsx'),
      'utf8',
    );

    expect(source).toContain('WorkbenchPageTemplate');
    expect(source).toContain('WorkbenchToolbar');
    expect(source).toContain('WorkbenchLeftPanelTemplate');
    expect(source).toContain('UiModule05DetailPanel');
    expect(source).toContain('UiModule04BottomTable');
    expect(source).toContain('ErpDataGridNext');
    expect(source).toContain('listProducts');
    expect(source).toContain('useSearchParams');
    expect(source).toContain('buildGoodsProductCreatePath');
    expect(source).toContain('buildGoodsProductEditPath');
    expect(source).toContain("'/goods/products/query'");
    expect(source).not.toContain('GoodsProductEditorDialog');
  });

  it('implements goods kits query with the unified workbench template and real kit api', () => {
    const source = readFileSync(
      resolve(__dirname, './kits-query/GoodsKitsQueryPage.tsx'),
      'utf8',
    );

    expect(source).toContain('WorkbenchPageTemplate');
    expect(source).toContain('WorkbenchToolbar');
    expect(source).toContain('WorkbenchLeftPanelTemplate');
    expect(source).toContain('UiModule05DetailPanel');
    expect(source).toContain('UiModule04BottomTable');
    expect(source).toContain('ErpDataGridNext');
    expect(source).toContain('listKits');
    expect(source).toContain('getKitDetail');
    expect(source).toContain('useSearchParams');
    expect(source).toContain('buildGoodsKitCreatePath');
    expect(source).toContain('buildGoodsKitEditPath');
    expect(source).toContain("'/goods/kits/query'");
    expect(source).not.toContain('GoodsKitEditorDialog');
  });

  it('implements goods create pages with direct api wiring instead of placeholder text', () => {
    const productCreateSource = readFileSync(
      resolve(__dirname, './product-form/GoodsProductCreatePage.tsx'),
      'utf8',
    );
    const kitCreateSource = readFileSync(
      resolve(__dirname, './kit-form/GoodsKitCreatePage.tsx'),
      'utf8',
    );
    const createWorkbenchSource = readFileSync(
      resolve(__dirname, './shared/GoodsCreateWorkbench.tsx'),
      'utf8',
    );
    const productEditSource = readFileSync(
      resolve(__dirname, './product-form/GoodsProductEditPage.tsx'),
      'utf8',
    );
    const kitEditSource = readFileSync(
      resolve(__dirname, './kit-form/GoodsKitEditPage.tsx'),
      'utf8',
    );

    expect(createWorkbenchSource).toContain('WorkbenchPageTemplate');
    expect(createWorkbenchSource).toContain('WorkbenchToolbar');
    expect(createWorkbenchSource).toContain('UiModule05DetailPanel');
    expect(createWorkbenchSource).toContain('buildGoodsCompactWorkbenchPath');

    expect(productCreateSource).toContain('GoodsCreateWorkbench');
    expect(productCreateSource).toContain('createProduct');
    expect(productCreateSource).toContain('buildGoodsProductEditPath');
    expect(productCreateSource).toContain('buildGoodsQueryPath');
    expect(productCreateSource).toContain('useSearchParams');
    expect(productCreateSource).not.toContain('UiModule07SettingsLayout');
    expect(productCreateSource).not.toContain('bordered={false}');
    expect(productCreateSource).not.toContain('货品模块基础路由已接入');

    expect(kitCreateSource).toContain('GoodsCreateWorkbench');
    expect(kitCreateSource).toContain('createProduct');
    expect(kitCreateSource).toContain('createSku');
    expect(kitCreateSource).toContain('saveKitBom');
    expect(kitCreateSource).toContain('buildGoodsKitEditPath');
    expect(kitCreateSource).toContain('buildGoodsQueryPath');
    expect(kitCreateSource).toContain('useSearchParams');
    expect(kitCreateSource).not.toContain('UiModule07SettingsLayout');
    expect(kitCreateSource).not.toContain('bordered={false}');
    expect(kitCreateSource).not.toContain('货品模块基础路由已接入');

    expect(productEditSource).toContain('GoodsCreateWorkbench');
    expect(productEditSource).toContain('getProduct');
    expect(productEditSource).toContain('updateProduct');
    expect(productEditSource).toContain('buildGoodsQueryPath');

    expect(kitEditSource).toContain('GoodsCreateWorkbench');
    expect(kitEditSource).toContain('getKitDetail');
    expect(kitEditSource).toContain('getSkuDetail');
    expect(kitEditSource).toContain('updateProduct');
    expect(kitEditSource).toContain('updateSku');
    expect(kitEditSource).toContain('saveKitBom');
    expect(kitEditSource).toContain('buildGoodsQueryPath');
  });

  it('keeps goods forms compatible with current antd patterns', () => {
    const productFormSource = readFileSync(
      resolve(__dirname, './product-form/ProductForm.tsx'),
      'utf8',
    );
    const kitFormSource = readFileSync(
      resolve(__dirname, './kit-form/KitForm.tsx'),
      'utf8',
    );
    expect(productFormSource).not.toContain('<Space direction="vertical"');
    expect(productFormSource).toContain('<Flex vertical');

    expect(kitFormSource).not.toContain('<Space direction="vertical"');
    expect(kitFormSource).toContain('<Flex vertical');
    expect(kitFormSource).not.toContain('{...field}');
  });

  it('uses compact form composition for goods create and edit workbenches', () => {
    const productFormSource = readFileSync(
      resolve(__dirname, './product-form/ProductForm.tsx'),
      'utf8',
    );
    const kitFormSource = readFileSync(
      resolve(__dirname, './kit-form/KitForm.tsx'),
      'utf8',
    );
    const workbenchCssSource = readFileSync(
      resolve(__dirname, './shared/goodsCreateWorkbench.css'),
      'utf8',
    );

    expect(productFormSource).toContain('className="goods-compact-form"');
    expect(productFormSource).toContain('className="goods-form-section goods-form-section--compact"');
    expect(productFormSource).toContain('gutter={[12, 4]}');
    expect(productFormSource).toContain('rows={3}');

    expect(kitFormSource).toContain('className="goods-compact-form"');
    expect(kitFormSource).toContain('className="goods-form-section goods-form-section--compact"');
    expect(kitFormSource).toContain('className="goods-form-bom-list"');
    expect(kitFormSource).toContain('className="goods-form-bom-row"');
    expect(kitFormSource).toContain('gutter={[12, 4]}');
    expect(kitFormSource).toContain('rows={3}');
    expect(kitFormSource).toContain("type=\"dashed\"");

    expect(workbenchCssSource).toContain('.goods-compact-form');
    expect(workbenchCssSource).toContain('.goods-form-section--compact');
    expect(workbenchCssSource).toContain('.goods-form-bom-list');
    expect(workbenchCssSource).toContain('gap: 12px;');
    expect(workbenchCssSource).toContain('padding: 18px 20px;');
  });

  it('keeps goods create and edit pages focused on the main form body', () => {
    const productCreateSource = readFileSync(
      resolve(__dirname, './product-form/GoodsProductCreatePage.tsx'),
      'utf8',
    );
    const productEditSource = readFileSync(
      resolve(__dirname, './product-form/GoodsProductEditPage.tsx'),
      'utf8',
    );
    const kitCreateSource = readFileSync(
      resolve(__dirname, './kit-form/GoodsKitCreatePage.tsx'),
      'utf8',
    );
    const kitEditSource = readFileSync(
      resolve(__dirname, './kit-form/GoodsKitEditPage.tsx'),
      'utf8',
    );

    expect(productCreateSource).not.toContain('metrics={[');
    expect(productCreateSource).not.toContain('<Typography.Paragraph');

    expect(productEditSource).not.toContain('metrics={[');
    expect(productEditSource).not.toContain('<Typography.Paragraph');

    expect(kitCreateSource).not.toContain('metrics={[');
    expect(kitCreateSource).not.toContain('<Typography.Paragraph');

    expect(kitEditSource).not.toContain('metrics={[');
    expect(kitEditSource).not.toContain('<Typography.Paragraph');
  });
});
