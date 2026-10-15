"""Report verified material command rejection without promoting it to launch readiness."""
import hashlib
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'qa/acceptance'
edge=json.loads((BASE/'free-erp-material-rejection-oct10-edge-independent.json').read_text())
full=json.loads((BASE/'free-erp-material-rejection-oct10-full-independent.json').read_text())
strict=json.loads((BASE/'free-erp-material-rejection-oct10-strict.json').read_text())
assert edge['status']=='PASS' and full['status']=='PASS'
source=json.loads((BASE/'free-erp-complete-schema-oct8-material-rejection-full-after.json').read_text())['source_sha256']
assert all(hashlib.sha256((ROOT/n).read_bytes()).hexdigest()==h for n,h in source.items())
openapi=json.loads((BASE/'free-erp-material-rejection-oct10-openapi-final.json').read_text())
assert all(hashlib.sha256((ROOT/n).read_bytes()).hexdigest()==h for n,h in openapi.items())
assert 'OpenAPI gate verification passed' in (BASE/'free-erp-material-rejection-oct10-openapi-final.log').read_text(encoding='utf-8-sig')
names=['crates/erp-repo/src/shared_contract.rs','crates/erp-repo/src/shared_contract/material_documents.rs','crates/erp-repo/src/shared_contract/material_document_commit.rs','crates/erp-repo/src/shared_contract/material_rejections.rs','crates/erp-api/src/manufacturing/material_documents.rs','crates/erp-api/tests/free_erp_acceptance.rs','crates/erp-api/tests/free_erp_acceptance/material_rejections.rs','crates/erp-api/tests/free_erp_acceptance/material_confirmation.rs','crates/erp-api/tests/free_erp_acceptance/material_source_review.rs']+list(openapi)
report={'date':'2026-10-10','status':'PARTIAL','scope_status':'PASS_FOR_MATERIAL_DEFINITE_REJECTION_BACKEND_AND_CURRENT_REGISTERED_TARGET_WITH_PROTECTED_OLD_TENANT_ROWS',
'scope':'opt-in material save/source-adopt/confirm rejection before COMMIT, prerequisite for pending-command UI',
'changes':['new-command admission and business rejection only; matching DTO plus tenant/actor/work-order/document context','legacy success/error responses retained when flag omitted','SQL/COMMIT uncertainty, authorization denial and changed original ID never declare safe unlock','client recovery must durably acknowledge exact rejection before editing with a fresh ID'],
'initial_compile':{'status':'FAIL','business_tests_executed':0,'reason':'missing helper context argument; fixed before business verification','evidence':'qa/acceptance/free-erp-material-rejection-oct10-tests.log'},
'focused':{k:edge[k] for k in ['status','checks','tests','protected_tenants','protected_tables','protected_rows','source_inputs','actual_rejection_declarations','actual_protocol_fixtures']},
'full_registered_target':{k:full[k] for k in ['status','checks','tests','protected_tenants','protected_tables','protected_rows','source_inputs','actual_rejection_declarations','actual_protocol_fixtures']},
'strict_repository':strict,'openapi_gate':'PASS','openapi_descriptor_refinement':'After backend proof, rejected-field echo admits the actual rejected business value; no Rust changes or additional Rust build. Original OpenAPI window and bytes retained, final descriptor gate separately passed.','current_compiled_source_window':'all 1022 source inputs and OpenAPI inputs unchanged',
'boundaries':['Axum route/PostgreSQL contract tests; no new formal material UI or native/Web business acceptance','no real network post-commit disconnect injection and no persistent server rejection receipt; missing declaration remains Unknown','322 migrations are only appended to complete QA, not fresh 322-file installation or populated-business upgrade/restore','location/rework integration, real Worker recovery, Release load and hosted free-account acceptance remain incomplete','full 604 registered tests are not all 100 product functions, all historical targets, all performance targets or a passed release gate'],
'source_sha256':{n:hashlib.sha256((ROOT/n).read_bytes()).hexdigest() for n in names},
'evidence':['qa/acceptance/free-erp-material-rejection-oct10-edge-independent.json','qa/acceptance/free-erp-material-rejection-oct10-full-independent.json','qa/acceptance/free-erp-material-rejection-oct10-strict.json']}
dest=BASE/'free-erp-material-rejection-oct10-repair.json'
assert not dest.exists()
dest.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print('PARTIAL product; PASS scoped backend:',full['tests'],full['checks'],'independent checks')
