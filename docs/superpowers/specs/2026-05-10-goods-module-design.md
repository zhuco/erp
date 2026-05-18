# 货品模块工业化重构设计

## 背景

当前 ERP Web 已存在 `ProductsPage`、`KitsPage`、`KitEditorPage` 等与货品、组合装相关的页面和接口接入，但信息架构、路由命名、查询工作台模板、创建与修改交互形态并不统一，且仍沿用旧的 `products` / `kits` 入口语义。

本次目标是在当前系统内新增一级模块“货品”，以工业级 ERP 目标重构货品域前端入口与页面结构，同时直接复用现有后端接口，做到新建、修改、查询全部可实际使用。

## 已确认决策

### 菜单与模块

- 新增一个一级菜单（顶部模块）`货品`
- `货品` 模块位于 `销售` 与 `履约` 之间
- 二级分组分为：
  - `货品`
    - `货品新建`
    - `货品查询`
  - `组合装`
    - `新建组合装`
    - `组合装查询`

### 交互形态

- 货品新建：独立整页
- 组合装新建：独立整页
- 货品修改：从查询页发起，全屏弹层
- 组合装修改：从查询页发起，大尺寸弹层
- 查询页必须使用当前项目统一工作台模板

### 设计深度

- 参考图仅作为结构与视觉方向参考
- 字段与数据交互以当前项目现状和现有接口为准
- 不追求 1:1 复刻参考图所有字段

### 重构策略

- 采用“全新重写货品域页面，仅复用现有 API”的方案
- 旧入口 `/products`、`/kits`、`/kits/:kit_sku_id` 在本次一起收口为跳转到新货品模块

## 目标

### 业务目标

- 提供统一的一级货品模块入口
- 让用户能在新模块内完成货品、组合装的新建、查询、修改
- 将货品与组合装查询页统一纳入工业工作台模板
- 消除旧入口并存导致的认知分裂

### 工程目标

- 使用新的 `/goods/*` 路由命名空间建立货品域边界
- 保持对现有 `Product`、`Sku`、`Kit BOM` 接口的真实复用
- 在不破坏现有业务接口语义的前提下重写前端页面层
- 为后续清理旧页面源码保留过渡空间

## 信息架构与路由设计

### 新路由命名空间

新货品模块采用独立命名空间：

- `/goods/products/new`
- `/goods/products/query`
- `/goods/kits/new`
- `/goods/kits/query`

### 修改态路由

修改不暴露单独菜单，但需要支持深链恢复：

- 货品修改：`/goods/products/query?edit={productId}`
- 组合装修改：`/goods/kits/query?edit={kitSkuId}`

### 旧入口收口

旧入口统一降级为兼容跳转：

- `/products` -> `/goods/products/query`
- `/kits` -> `/goods/kits/query`
- `/kits/:kit_sku_id` -> `/goods/kits/query?edit={kit_sku_id}`

### 需要同步更新的路由层

- `apps/erp-web/src/app/layout/menu.tsx`
- `apps/erp-web/src/app/layout/headerMenuModel.ts`
- `apps/erp-web/src/app/routeRegistry.ts`
- `apps/erp-web/src/app/layout/routeMeta.ts`
- `apps/erp-web/src/app/navigation/routeAliases.ts`
- `apps/erp-web/src/app/router.tsx`
- `WorkbenchLayout` reduced shell 白名单与相关测试

## 页面结构设计

### 1. 货品查询工作台 `/goods/products/query`

该页面采用 `WorkbenchPageTemplate`，承担：

- 查询货品主档
- 查看货品详情
- 发起货品修改
- 发起与货品关联的组合装查看或新建动作

页面采用四段式工业工作台结构：

- 左侧：筛选面板
- 中部：主表
- 底部：多标签明细区
- 右侧：详情面板

#### 主表数据源

- 直接使用 `listProducts`

#### 主表职责

- 展示货品主档列表
- 提供批量启停用、删除、刷新等主操作
- 提供行级“修改”“查看关联组合装”等动作

#### 左侧筛选建议

- 关键字
- 品牌
- 分类
- 状态
- 其他已有模型可支持的主档筛选项

#### 底部标签建议

- 规格 / SKU 信息
- 存放仓库
- 销售价格或扩展信息占位
- 关联组合装
- 操作日志

#### 右侧详情建议

- 货品基础摘要
- 预警库存
- 批次 / 序列 / 保质期状态
- 关联仓库数量
- 关联组合装数量

### 2. 组合装查询工作台 `/goods/kits/query`

该页面也采用 `WorkbenchPageTemplate`，承担：

- 查询组合装
- 查看 BOM 详情
- 发起组合装修改

#### 主表数据源

- 直接使用 `listKits`

#### 主表职责

- 展示组合装主列表
- 提供复制、修改、删除、刷新等动作

#### 底部标签建议

- BOM 组件清单
- 包含货品维度信息
- 图片 / 图文描述占位
- 流转或日志信息

#### 右侧详情建议

- 组合装基础摘要
- 组件数量
- 最近更新时间
- 快捷操作区

### 3. 货品新建页 `/goods/products/new`

该页面为独立整页，不走工作台查询壳。视觉结构向参考图靠拢，但字段只承载当前可落地的数据模型。

建议结构：

- 顶部：页面标题、返回、保存、保存并继续
- 主区：
  - 基础信息区
  - 规格 / SKU 区
  - 仓库与供应信息区
  - 管理标签与批次序列区
  - 图片与说明区

该页需要与货品修改弹层共用核心表单组件。

### 4. 组合装新建页 `/goods/kits/new`

该页面也是独立整页，但复杂度低于货品页。

建议结构：

- 基础信息区
- 组合项 BOM 编辑表格
- 图片与描述区
- 页脚保存动作

### 5. 货品修改弹层

从货品查询工作台发起，默认全屏弹层。

特点：

- 复用货品新建页同一套表单领域组件
- 支持深链恢复
- 保存后回刷查询页数据与详情面板

### 6. 组合装修改弹层

从组合装查询工作台发起，大尺寸弹层。

特点：

- 复用组合装新建页同一套表单领域组件
- 重点突出 BOM 编辑效率
- 支持深链恢复

## 领域模型与接口映射

### 原则

UI 可以全新重写，但领域模型不能造假。新页面必须显式承认当前系统的真实结构：

- 货品主档是 `Product`
- 组合装是 `Product + KIT Sku + BOM`

### 货品

#### 新建 / 修改

- 使用 `createProduct`
- 使用 `updateProduct`

#### 查询

- 使用 `listProducts`

#### 删除 / 启停用

- 使用 `deleteProduct`
- 使用 `updateProduct({ status })`

#### 可承载字段

以 `ProductCreateRequest` / `ProductUpdateRequest` 当前能承接的字段为准，包括：

- `product_code`
- `name`
- `alias`
- `brand`
- `category`
- `base_unit`
- `second_unit`
- `default_supplier_id`
- `default_warehouse_id`
- `primary_barcode`
- `product_tags`
- `remark`
- `warning_stock`
- `batch_enabled`
- `shelf_life_days`
- `forbid_sale_days`
- `serial_enabled`
- `serial_push_platform`
- 其他现有模型已支持的扩展字段

### 组合装

#### 新建链路

组合装新建不是单接口动作，必须分步完成：

1. 创建 `Product`
2. 创建 `Sku`，并指定 `sku_kind = 'KIT'`
3. 保存 BOM，使用 `saveKitBom`

#### 查询

- 使用 `listKits`
- 详情使用 `getKitDetail`

#### 修改

组合装修改属于编排型保存，可能包含三类动作：

- 修改基础货品信息：`updateProduct`
- 修改 KIT SKU 信息：`updateSku`
- 修改 BOM：`saveKitBom`

#### 删除

- 使用 `deleteSku`

### SKU 维度说明

本次不建议在第一版把 Product 与 Sku 编辑语义完全揉成一个单保存动作。推荐策略：

- 主弹层先保障 `Product` 主档编辑可完整可用
- SKU 相关编辑作为子区块或二级动作逐步接入

## 目录与代码组织

建议在 `src/pages/goods/` 下新建独立实现，不再把新页面继续塞入旧 `products/` 和 `kits/` 目录：

- `src/pages/goods/products-query/`
- `src/pages/goods/kits-query/`
- `src/pages/goods/product-form/`
- `src/pages/goods/kit-form/`
- `src/pages/goods/shared/`

这样可以：

- 保持新货品域边界清晰
- 避免与旧页面实现缠绕
- 为后续清理旧页面保留过渡空间

## 实施顺序

### 第一阶段：导航与路由收口

- 新增一级模块 `货品`
- 注册四个新路由
- 配置旧入口重定向
- 更新菜单、标题、路由登记、别名与测试

### 第二阶段：查询工作台

优先顺序：

1. 货品查询工作台
2. 组合装查询工作台

### 第三阶段：新建整页

优先顺序：

1. 货品新建页
2. 组合装新建页

### 第四阶段：修改弹层

- 货品查询页接入全屏修改弹层
- 组合装查询页接入大尺寸修改弹层
- 支持 `?edit=` 深链恢复

### 旧页面处理策略

本次不立即删除旧页面源码：

- 先把旧路由全部重定向
- 旧页面源码暂时保留作为行为参考
- 等新模块稳定后，再进行第二次清理提交

## 测试策略

### 1. 单元 / 契约测试

必须补充或更新：

- `menu.test.ts`
- `routeRegistry.test.ts`
- `WorkbenchLayout.reduced-ui-shell.test.ts`
- 新增货品模块标题、路由重定向、菜单叶子完整性测试

### 2. 页面结构测试

必须验证：

- 新查询页使用 `WorkbenchPageTemplate`
- 不退回旧的 `UiModule03MainTableSection` 直拼结构
- 新路由进入后顶部模块、标签页标题、工作台骨架稳定

### 3. E2E 测试

至少覆盖两条主链路：

- `货品查询 -> 打开修改弹层 -> 保存成功`
- `新建组合装 -> 配置 BOM -> 保存成功 -> 在组合装查询中可见`

如时间允许，补充：

- `/products` 自动跳转到 `/goods/products/query`
- `/kits/:kit_sku_id` 自动跳转到 `/goods/kits/query?edit=...`

测试统一使用：

- `apps/erp-web/e2e/fixtures.ts`
- `loginAndSelectTenant`

## 验收标准

- 用户能从一级菜单 `货品` 进入四个目标功能
- 货品查询与组合装查询全部进入工业工作台模板
- 货品新建、组合装新建可直接走现有接口并保存成功
- 查询页可发起修改并保存成功
- 旧入口全部自动跳转到新货品模块
- 菜单、路由、标题、结构测试与主链路 E2E 通过

## 最终建议

新增一级模块 `货品`，以 `/goods` 建立全新货品域前端，查询页统一工业工作台，新建走独立整页，修改走弹层，底层严格复用现有 `Product / Sku / Kit BOM` 接口与实体边界，旧 `/products` 与 `/kits` 全部重定向收口。
