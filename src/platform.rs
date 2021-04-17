/*!
Half of an `imgui-rs` backend
*/

#[cfg(feature = "sdl2")]
pub mod sdl2;
#[cfg(feature = "sdl2")]
pub use self::sdl2::ImGuiSdl2;
