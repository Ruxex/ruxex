//! Settings-adjacent popup surfaces for `ChatWidget`.
//!
//! This keeps theme UI out of the main orchestration module.

use super::*;

impl ChatWidget {
    pub(super) fn open_theme_picker(&mut self) {
        let genex_home = genex_utils_home_dir::find_genex_home().ok();
        let params = crate::theme_picker::build_theme_picker_params(
            self.local_settings.tui.theme.as_deref(),
            genex_home.as_deref(),
            self.last_rendered_width.get(),
        );
        self.bottom_pane.show_selection_view(params);
    }
}
