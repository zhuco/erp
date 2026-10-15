from pathlib import Path
p=Path('crates/erp-api/tests/free_erp_acceptance/material_identity.rs')
s=p.read_text(encoding='utf-8-sig')
s=s.replace('use std::sync::Arc;','use super::account_login::{AccountLogin,STOCK_PERMISSIONS,create_user};',1)
s=s.replace('use chrono::Utc;\n','',1).replace('AppState, Claims, JwtService, build_app','AppState, build_app',1)
s=s.replace('    order: Uuid,\n}', '    order: Uuid,\n    login:Option<AccountLogin>,\n}',1)
s=s.replace('    let f = Fixture {','    let mut f = Fixture {',1).replace('        order: Uuid::new_v4(),','        order: Uuid::new_v4(),\n        login:None,',1)
needle='    eprintln!("QA material identity tenant={} order={}", f.tenant, f.order);'
assert needle in s
s=s.replace(needle,"""    let actor=Uuid::new_v4();
    let login=create_user(pool,TenantId(f.tenant),actor,format!("{actor}@material.qa.invalid"),STOCK_PERMISSIONS).await?;
    sqlx::query("insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)")
        .bind(f.tenant).bind(actor).bind(f.warehouse).execute(pool).await?;
    f.login=Some(login);
"""+needle,1)
a=s.index('fn app_and_token(');b=s.index('async fn post(',a)
s=s[:a]+"""fn app_and_token(f:&Fixture)->Result<(axum::Router,String)> {
    let login=f.login.as_ref().ok_or_else(||anyhow::anyhow!("missing actual material login"))?;
    Ok((build_app(AppState::new(Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,login.jwt.clone())),login.token.clone()))
}

"""+s[b:]
old='    let (app, key) = app_and_token()?;\n    let token = token(f.tenant, &key)?;'
assert s.count(old)==3;s=s.replace(old,'    let (app, token) = app_and_token(&f)?;')
needle='        let (status, bytes) = post(&app, &f, &token, body).await?;\n        assert_eq!(\n            status,\n            StatusCode::UNPROCESSABLE_ENTITY,'
assert s.count(needle)==1
s=s.replace(needle,'        let expected=if body["request_id"]=="foreign" {StatusCode::FORBIDDEN}else{StatusCode::UNPROCESSABLE_ENTITY};\n        let (status, bytes) = post(&app, &f, &token, body).await?;\n        assert_eq!(\n            status,\n            expected,',1)
p.write_text(s,encoding='utf8',newline='')

# The existing shared manufacturing helper retains the observed DB connection name.
p=Path('crates/erp-api/tests/free_erp_acceptance/production_receipt_release.rs')
s=p.read_text(encoding='utf-8-sig')
a=s.index('pub(super) async fn http(');b=s.index('    let database =',a)
s=s[:a]+'pub(super) async fn http(f: &Fixture, path: &str, body: Value) -> Result<(StatusCode, Value)> {\n    let jwt=f.login.jwt.clone();\n    let token=f.login.token.clone();\n'+s[b:]
p.write_text(s,encoding='utf8',newline='')
