//! XR_KHR_android_thread_settings — declares the application's main and render
//! threads as XR-critical so the OpenXR runtime can prioritize them.
//!
//! On Android OpenXR runtimes (Meta Quest, PICO, etc.), the runtime cannot
//! know which of the host application's threads are render-critical unless the
//! app declares them. Without tagging, main and render threads compete with
//! the VR runtime's own threads (sensors, tracking, compositor) and
//! background worker pools for CPU time.
//!
//! Measured on Meta Quest 3: −30% CPU frame time (19.7 → 13.7 ms).

use bevy_ecs::resource::Resource;
#[cfg(target_os = "android")]
use bevy_log::{error, info, warn};
#[cfg(target_os = "android")]
use crate::resources::OxrInstance;
#[cfg(target_os = "android")]
use crate::session::OxrSession;

/// The OS thread ID (Linux `gettid()`) of the application's main thread.
/// Captured in `OxrInitPlugin::build()` (main thread) and consumed by
/// [`tag_android_threads`] in the render world.
#[derive(Resource, Debug, Clone, Copy)]
pub struct MainThreadTid(pub u32);

/// Tags APPLICATION_MAIN + RENDERER_MAIN as XR-critical.
///
/// Call from `create_xr_session` after the session is created — at that point
/// the instance, session, and render thread are all in scope.
#[cfg(target_os = "android")]
pub fn tag_android_threads(instance: &OxrInstance, session: &OxrSession, main_tid: u32) {
    let entry = match unsafe { openxr::Entry::load() } {
        Ok(entry) => entry,
        Err(err) => {
            error!("XR thread tagging: Entry::load failed: {err}");
            return;
        }
    };
    let khr = match unsafe { openxr::raw::AndroidThreadSettingsKHR::load(&entry, instance.as_raw()) } {
        Ok(khr) => khr,
        Err(err) => {
            warn!("XR thread tagging: AndroidThreadSettingsKHR not available ({err}); skipping");
            return;
        }
    };
    let session_raw = session.as_raw();

    let result = unsafe {
        (khr.set_android_application_thread)(
            session_raw,
            openxr::sys::AndroidThreadTypeKHR::APPLICATION_MAIN,
            main_tid,
        )
    };
    if result == openxr::sys::Result::SUCCESS {
        info!("XR thread tagging: APPLICATION_MAIN tagged (tid={main_tid})");
    } else {
        warn!("XR thread tagging: APPLICATION_MAIN failed: {result:?}");
    }

    let render_tid = unsafe { libc::gettid() } as u32;
    let result = unsafe {
        (khr.set_android_application_thread)(
            session_raw,
            openxr::sys::AndroidThreadTypeKHR::RENDERER_MAIN,
            render_tid,
        )
    };
    if result == openxr::sys::Result::SUCCESS {
        info!("XR thread tagging: RENDERER_MAIN tagged (tid={render_tid})");
    } else {
        warn!("XR thread tagging: RENDERER_MAIN failed: {result:?}");
    }
}
