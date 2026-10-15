-- Immutable links between a real material document and each committed movement.
create table mfg_material_document_effects (
 tenant_id uuid not null, document_id uuid not null, line_id uuid not null,
 move_id uuid not null, snapshot jsonb not null,
 primary key(tenant_id,document_id,line_id),unique(tenant_id,move_id),
 foreign key(tenant_id,document_id,line_id) references mfg_material_document_lines(tenant_id,document_id,line_id),
 foreign key(tenant_id,move_id) references mfg_material_moves(tenant_id,move_id)
);
create trigger mfg_material_document_effect_immutable before update or delete on mfg_material_document_effects
 for each row execute function protect_receipt_disposition_fact();
