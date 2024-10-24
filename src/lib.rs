pub use fyfth_core as core;
#[cfg(feature = "focus")]
pub use fyfth_focus as focus;
#[cfg(feature = "egui_terminal")]
pub use fyfth_terminal as terminal;

pub mod prelude {
    pub use fyfth_core::*;
    #[cfg(feature = "focus")]
    pub use fyfth_focus::*;
    #[cfg(feature = "egui_terminal")]
    pub use fyfth_terminal::*;
}
