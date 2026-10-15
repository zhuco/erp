use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Destination {
    Lab,
    Products,
    Orders,
    Reports,
    WarehouseTree,
    Close,
}

#[derive(Default)]
pub(crate) struct LeaveGuard {
    pending: Option<Destination>,
    saving: bool,
    error: Option<String>,
    #[cfg(target_arch = "wasm32")]
    before_unload: Option<wasm_bindgen::closure::Closure<dyn FnMut(web_sys::BeforeUnloadEvent)>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Choice {
    Save,
    Discard,
    KeepEditing,
}

impl LeaveGuard {
    pub(crate) fn request(
        &mut self,
        destination: Destination,
        dirty: bool,
    ) -> Option<Destination> {
        self.request_close(destination, dirty, false, false)
    }

    /// A window close with an unknown command is held until the original receipt is read.
    /// The close itself does not allocate a new command identity.
    pub(crate) fn request_close(
        &mut self,
        destination: Destination,
        dirty: bool,
        command_known: bool,
        client_unknown: bool,
    ) -> Option<Destination> {
        let hold = client_unknown || (command_known && dirty) || dirty;
        if hold {
            if self.pending.is_none() {
                self.error = None;
            }
            self.pending.get_or_insert(destination);
            None
        } else {
            Some(destination)
        }
    }

    pub(crate) fn pending(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn saving(&self) -> bool {
        self.saving
    }

    pub(crate) fn wait_for_save(&mut self) {
        self.saving = true;
        self.error = None;
    }

    pub(crate) fn save_failed(&mut self, error: String) {
        self.saving = false;
        self.error = Some(error);
    }

    pub(crate) fn resolve(&mut self, choice: Choice) -> Option<Destination> {
        match choice {
            Choice::Save | Choice::Discard => {
                self.saving = false;
                self.error = None;
                self.pending.take()
            }
            Choice::KeepEditing => {
                self.saving = false;
                self.error = None;
                self.pending = None;
                None
            }
        }
    }

    pub(crate) fn dialog(&self, ctx: &egui::Context, blocked: Option<&str>) -> Option<Choice> {
        self.pending?;
        let mut choice = None;
        egui::Window::new("Unsaved changes")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(if self.saving {
                    "Waiting for the server to confirm the save."
                } else {
                    blocked.unwrap_or(
                        "Save all edits before leaving, discard only unsent drafts, or keep editing.",
                    )
                });
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(blocked.is_none() && !self.saving, egui::Button::new("Save and leave")).clicked() {
                        choice = Some(Choice::Save);
                    }
                    if ui.add_enabled(blocked.is_none() && !self.saving, egui::Button::new("Discard drafts and leave")).clicked() {
                        choice = Some(Choice::Discard);
                    }
                    if ui.button("Keep editing").clicked() {
                        choice = Some(Choice::KeepEditing);
                    }
                });
            });
        choice
    }

    pub(crate) fn sync_before_unload(&mut self, dirty: bool) {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            let Some(window) = web_sys::window() else {
                return;
            };
            if dirty && self.before_unload.is_none() {
                let callback = wasm_bindgen::closure::Closure::wrap(Box::new(
                    |event: web_sys::BeforeUnloadEvent| {
                        event.prevent_default();
                        event.set_return_value("Unsaved changes");
                    },
                )
                    as Box<dyn FnMut(_)>);
                window.set_onbeforeunload(Some(callback.as_ref().unchecked_ref()));
                self.before_unload = Some(callback);
            } else if !dirty && self.before_unload.is_some() {
                window.set_onbeforeunload(None);
                self.before_unload = None;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = dirty;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_dirty_intent_is_frozen_until_the_user_resolves_it() {
        let mut guard = LeaveGuard::default();
        assert_eq!(guard.request(Destination::Products, true), None);
        assert_eq!(guard.request(Destination::Reports, true), None);
        assert_eq!(guard.resolve(Choice::Discard), Some(Destination::Products));
    }

    #[test]
    fn keep_editing_cancels_the_frozen_destination() {
        let mut guard = LeaveGuard::default();
        guard.request(Destination::Close, true);
        assert_eq!(guard.resolve(Choice::KeepEditing), None);
        assert!(!guard.pending());
    }
}
