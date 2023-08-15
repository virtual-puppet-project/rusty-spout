/*
# Rusty Spout

[Spout](https://spout.zeal.co/) bindings to Rust. Initially created for usage with
[Godot](https://github.com/godotengine/godot) and [gdext](https://github.com/godot-rust/gdext).

## Design decisions

### Return types

Return types are generally converted into Rust equivalents except for:

* `DWORD`
* `GLuint`
* `GLenum`

The above types are kept as ffi types, since they are always meant to be passed back to Spout.

### Unsafe blocks

Each time the library is pinned for access, an unsafe block is used instead of swallowing
the unsafe block inside of a helper function.
*/

#[cfg(feature = "godot")]
mod godot;

use std::{
    ffi::{CStr, CString},
    pin::Pin,
};

use autocxx::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unable to get spout handle")]
    NoHandle,
    #[error("Unable to create {ffi_type:?}: {context:?}")]
    FfiTypeInto { ffi_type: FfiType, context: String },
    #[error("Unable to convert {ffi_type:?}: {context:?}")]
    FfiTypeFrom { ffi_type: FfiType, context: String },
    #[error("Received unexpected nullptr")]
    NullPtr,
}

#[derive(Debug)]
pub enum FfiType {
    CString,
    CStr,
}

type Result<T> = std::result::Result<T, Error>;

// Typedefs using concrete types instead of ffi types for readability.

/// A Windows DWORD which _should_ be a ulong.
type DWORD = c_ulong;
/// An OpenGL uint which _should_ be a uint.
type GLuint = c_uint;
// TODO lookup the actual enums and redefine them here?
/// An OpenGL enum which _should_ be a uint.
type GLenum = c_uint;

include_cpp! {
    #include "SpoutLibrary.h"

    safety!(unsafe)

    generate!("GetSpout")
    generate!("SPOUTLIBRARY")
}

/// Helper for getting a usable library handle.
macro_rules! library {
    ($lib:expr) => {{
        match $lib {
            Some(v) => as_pin(v),
            None => return Err(Error::NoHandle),
        }
    }};
}

/// Wrapper around `SPOUTLIBRARY`.
pub struct RustySpout {
    library: Option<*mut ffi::SPOUTLIBRARY>,
}

impl Drop for RustySpout {
    fn drop(&mut self) {
        if let Some(lib) = self.library {
            unsafe {
                as_pin(lib).Release();
            }
        }
    }
}

impl RustySpout {
    /// Create a new, uninitialized handler.
    pub fn new() -> Self {
        Self { library: None }
    }

    /// Get a handle to spout.
    pub fn get_spout(&mut self) -> Result<()> {
        let handle = ffi::GetSpout();
        if handle.is_null() {
            return Err(Error::NoHandle);
        }

        self.library = Some(handle);

        Ok(())
    }

    /// Set the sender name.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn set_sender_name<T: AsRef<str>>(&mut self, name: T) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        let name = match CString::new(name.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::FfiTypeInto {
                    ffi_type: FfiType::CString,
                    context: format!("set_sender_name: {e}"),
                })
            }
        };

        unsafe {
            lib.SetSenderName(name.as_ptr());
        }

        Ok(())
    }

    /// Set the sender DX11 shared texture format. `format` is actually a Windows `DWORD`,
    /// which _should_ be covered by `c_long`.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// The `format` that is sent is **_not_** guaranteed to be safe.
    pub fn set_sender_format(&mut self, format: DWORD) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.SetSenderFormat(format);

        Ok(())
    }

    /// Close sender and free resources. A sender is created or updated by all
    /// sending functions.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// The `msec` that is sent is **_not_** guaranteed to be safe.
    pub fn release_sender(&mut self, msec: DWORD) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.ReleaseSender(msec);

        Ok(())
    }

    /// Send a texture attached to an FBO.
    /// * The FBO must be currently bound
    /// * The sending texture can be larger than the size that the sender is set up for
    ///   * For example, if the application is only using a portion of the allocated texture space, such as
    ///     for FreeFrame plugins. (The 2.006 equivalent is DrawToSharedTexture).
    /// * To send the OpenGL default framebuffer, specify "0" for the `fbo_id`, `width`, and `height`.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// No safety guarantees can be made about the `fbo_id`.
    pub fn send_fbo(
        &mut self,
        fbo_id: GLuint,
        width: c_uint,
        height: c_uint,
        invert: bool,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.SendFbo(fbo_id, width, height, invert))
    }

    /// Send an OpenGL texture.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// No safety guarantees can be made about the `texture_id`, `texture_target`, or `host_fbo`.
    pub fn send_texture(
        &mut self,
        texture_id: GLuint,
        texture_target: GLuint,
        width: c_uint,
        height: c_uint,
        invert: bool,
        host_fbo: GLuint,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.SendTexture(texture_id, texture_target, width, height, invert, host_fbo))
    }

    /// Send image pixels. NOTE: this is very slow.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// The `pixels` pointer must be valid.
    pub fn send_image(
        &mut self,
        pixels: *const u8,
        width: c_uint,
        height: c_uint,
        gl_format: GLenum,
        invert: bool,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let success = unsafe { lib.SendImage(pixels, width, height, gl_format, invert) };

        Ok(success)
    }

    /// Gets the sender name.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    /// An [Error] is returned if the name is a `nullptr`.
    ///
    /// #Panic
    /// Panics if the string is not nul terminated.
    pub fn get_name(&mut self) -> Result<String> {
        let lib = unsafe { library!(self.library) };

        let name = lib.GetName();
        if name.is_null() {
            return Err(Error::NullPtr);
        }

        let name = unsafe { CStr::from_ptr(name) };
        match name.to_str() {
            Ok(v) => Ok(v.to_string()),
            Err(e) => Err(Error::FfiTypeFrom {
                ffi_type: FfiType::CStr,
                context: format!("get_name: {e}"),
            }),
        }
    }

    /// Get the sender width.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_width(&mut self) -> Result<u32> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetWidth().0)
    }

    /// Get the sender height.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_height(&mut self) -> Result<u32> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetHeight().0)
    }

    /// Get the sender frame rate.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_fps(&mut self) -> Result<f64> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetFps())
    }

    /// Get the sender frame number.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_frame(&mut self) -> Result<i32> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetFrame().0)
    }

    /// Get the sender share handle.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// No safety guarantees are made about the void pointer `c_void`.
    pub fn get_handle(&mut self) -> Result<*mut c_void> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetHandle())
    }

    /// Get the sender sharing method.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_cpu(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetCPU())
    }

    /// Get the sender GL/DX hardware compatibility.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_gl_dx(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetGLDX())
    }

    /// Specify a sender for connection.
    /// * If a name is specified, the receiver will not connect to any other unless the user selects one
    /// * If that sender closes, the receiver will wait for the nominated sender to open
    /// * If no name is specified, the receiver will connect to the active sender
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// The `sender_name` is copied into a new [CString] and is supposed to be valid when passed to Spout.
    pub fn set_receiver_name<T: AsRef<str>>(&mut self, sender_name: T) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        let name = match CString::new(sender_name.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::FfiTypeInto {
                    ffi_type: FfiType::CString,
                    context: format!("set_receiver_name: {e}"),
                })
            }
        };
        unsafe {
            lib.SetReceiverName(name.as_ptr());
        }

        Ok(())
    }

    /// Close receiver and release resources ready to connect to another sender.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn release_receiver(&mut self) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.ReleaseReceiver();

        Ok(())
    }

    /// Receive texture.
    ///
    /// If no arguments, connect to a sender and retrieve texture details ready for access. (see
    /// `bind_shared_texture` and `unbind_shared_texture`). Connect to a sender and inform the
    /// application to update the receiving texture if it has changed dimensions. For no change, copy
    /// the sender shared texture to the application texture.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn receive_texture(
        &mut self,
        texture_id: GLuint,
        texture_target: GLuint,
        invert: bool,
        host_fbo: GLuint,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.ReceiveTexture(texture_id, texture_target, invert, host_fbo))
    }

    /// Receive image pixels.
    ///
    /// Connect to a sender and inform the application to update the receiving buffer if it has changed
    /// dimensions. For no change, copy the sender shared texture to the pixel buffer.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// It is up to the developer to make sure the `pixels` buffer is large enough.
    pub fn receive_image(
        &mut self,
        pixels: *const u8,
        gl_format: GLenum,
        invert: bool,
        host_fbo: GLuint,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let success = unsafe { lib.ReceiveImage(pixels.cast_mut(), gl_format, invert, host_fbo) };

        Ok(success)
    }

    /// Query whether the sender has changed.
    ///
    /// Checked at every cycle before receiving data.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn is_updated(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.IsUpdated())
    }

    /// Query sender connection.
    ///
    /// If the sender closes, receiving functions return `false`.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn is_connected(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.IsConnected())
    }

    /// Query received frame status.
    ///
    /// The receiving texture or pixel buffer is only refreshed if the sender has produced a new frame.
    /// This can be queried to process texture data only for a new frames.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn is_frame_new(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.IsFrameNew())
    }

    /// Get the sender name.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    /// Returns [Error] if the name is a `nullptr`.
    ///
    /// # Panic
    /// Panics if the name is not nul terminated.
    pub fn get_sender_name(&mut self) -> Result<String> {
        let lib = unsafe { library!(self.library) };

        let name = lib.GetSenderName();
        if name.is_null() {
            return Err(Error::NullPtr);
        }

        let name = unsafe { CStr::from_ptr(name) };
        match name.to_str() {
            Ok(v) => Ok(v.to_string()),
            Err(e) => Err(Error::FfiTypeFrom {
                ffi_type: FfiType::CStr,
                context: format!("get_sender_name: {e}"),
            }),
        }
    }

    /// Get the sender width.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_sender_width(&mut self) -> Result<u32> {
        if let Some(lib) = self.library {
            unsafe { return Ok(as_pin(lib).GetSenderWidth().0) }
        }

        Err(Error::NoHandle)
    }

    /// Get the sender height.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_sender_height(&mut self) -> Result<u32> {
        if let Some(lib) = self.library {
            unsafe { return Ok(as_pin(lib).GetSenderHeight().0) }
        }

        Err(Error::NoHandle)
    }

    /// Get the sender DirectX texture format.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_sender_format(&mut self) -> Result<DWORD> {
        if let Some(lib) = self.library {
            unsafe { return Ok(as_pin(lib).GetSenderFormat()) }
        }

        Err(Error::NoHandle)
    }

    /// Get the sender frame rate.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_sender_fps(&mut self) -> Result<f64> {
        if let Some(lib) = self.library {
            unsafe { return Ok(as_pin(lib).GetSenderFps()) }
        }

        Err(Error::NoHandle)
    }

    /// Get the sender frame number.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_sender_frame(&mut self) -> Result<i32> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetSenderFrame().0)
    }

    /// Get the received sender share handle.
    ///
    /// # Important
    /// No safety guarantees are made about the void pointer `c_void`.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_sender_handle(&mut self) -> Result<*mut c_void> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetSenderHandle())
    }

    /// Get the received sender sharing mode.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_sender_cpu(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetSenderCPU())
    }

    /// Get the received sender GL/DX compatibility.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_sender_gl_dx(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetSenderGLDX())
    }

    /// Open the sender selection dialog.
    ///
    /// # Important
    /// **No guarantees are made about how this actually works. Use at your own risk!**
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn select_sender(&mut self) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.SelectSender();

        Ok(())
    }

    /// Enable or disable frame counting globally.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn set_frame_count(&mut self, enable: bool) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.SetFrameCount(enable);

        Ok(())
    }
}

/// Convencience function for pinning a pointer.
///
/// # Safety
/// The `ptr` must be a valid `SPOUTLIBRARY` pointer.
unsafe fn as_pin<'a>(ptr: *mut ffi::SPOUTLIBRARY) -> Pin<&'a mut ffi::SPOUTLIBRARY> {
    Pin::new_unchecked(&mut *ptr)
}
