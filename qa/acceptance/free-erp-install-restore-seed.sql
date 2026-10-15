-- Synthetic owned restoration canaries. No API or posted-business-effect claim.
INSERT INTO tenants(tenant_id,name) VALUES(@tenant@,'QA-V323-INSTALL-RESTORE-ONLY');
INSERT INTO users(tenant_id,user_id,email,password_hash)
 VALUES(@tenant@,@user@,'restore-only@qa.invalid',@password_hash@);
INSERT INTO shops(shop_id,tenant_id,platform,platform_shop_id,shop_name)
 VALUES(@shop@,@tenant@,'pdd','qa-v323-restore','QA restore-only shop');
INSERT INTO products(tenant_id,product_id,product_code,name,brand,attrs,remark)
 VALUES(@tenant@,@product@,'QA-V323-RESTORE','恢复商品·生产与贸易',NULL,
 jsonb_build_object('decimal','0.250001','unicode','中文','nullable',NULL),E'采购''生产\n二行');
INSERT INTO skus(tenant_id,sku_id,product_id,sku_code,name,cost_cent)
 VALUES(@tenant@,@sku@,@product@,'QA-V323-SKU','恢复SKU',12345);
INSERT INTO warehouses(tenant_id,warehouse_id,name,code)
 VALUES(@tenant@,@warehouse@,'QA恢复仓','QA-V323-RESTORE');
INSERT INTO inventory_balances(tenant_id,warehouse_id,sku_id,on_hand,on_hand_micros,
 reserved_micros,frozen_micros,in_transit_micros)
 VALUES(@tenant@,@warehouse@,@sku@,0,250001,0,0,0);
INSERT INTO inbox_raw_payloads(tenant_id,platform,shop_id,body)
 VALUES(@tenant@,'pdd',@shop@,decode('00ff27e4b8ade696870a','hex'));
INSERT INTO inbox_dedup(tenant_id,dedup_key) VALUES(@tenant@,'QA-V323-restore-中文');
INSERT INTO mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros)
 VALUES(@tenant@,@order@,'QA-V323-WO','QA-V323-RESTORE','PLANNED',1000000);
INSERT INTO command_receipts(tenant_id,request_id,payload_hash,status,effect_id)
 VALUES(@tenant@,'qa-v323-restore-report','qa-restore-only','SUCCEEDED',@report@),
       (@tenant@,'qa-v323-restore-draft','qa-restore-only','SUCCEEDED',@document@);
INSERT INTO mfg_reports(tenant_id,report_id,work_order_id,operation_code,good_micros,rejected_micros,
 rework_micros,minutes,operator_id,session_user_id,session_id)
 VALUES(@tenant@,@report@,@order@,'QA-RESTORE-ONLY',250001,0,0,17,'restore-worker',@user@,'qa-restore-session');
INSERT INTO mfg_operation_report_outcomes(tenant_id,request_id,work_order_id,report_id,
 actor_user_id,payload_hash,snapshot)
 VALUES(@tenant@,'qa-v323-restore-report',@order@,@report@,@user@,'qa-restore-only',
 jsonb_build_object('restore_canary',true,'good_micros',250001,'request_id','qa-v323-restore-report'));
INSERT INTO mfg_material_documents(tenant_id,document_id,work_order_id,version_no,source_snapshot)
 VALUES(@tenant@,@document@,@order@,1,jsonb_build_object('restore_canary',true,'quantity_micros',250001));
INSERT INTO mfg_material_document_revisions(tenant_id,document_id,version_no,revision_id,
 request_id,actor_user_id,snapshot)
 VALUES(@tenant@,@document@,1,@revision@,'qa-v323-restore-draft',@user@,
 jsonb_build_object('restore_canary',true,'note',E'中文\n原版本'));
