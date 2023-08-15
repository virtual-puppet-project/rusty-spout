#[cfg(feature = "godot")]
mod godot;

use std::{ffi::CString, pin::Pin};

use autocxx::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unable to get spout handle")]
    NoHandle,
    #[error("Unable to create {ffi_type:?}: {context:?}")]
    FfiTypeInto { ffi_type: FfiType, context: String },
    #[error("Unable to convert {ffi_type:?}: {context:?}")]
    FfiTypeFrom { ffi_type: FfiType, context: String },
}

#[derive(Debug)]
pub enum FfiType {
    CString,
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
        if let Some(lib) = self.library {
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
                as_pin(lib).SetSenderName(name.as_ptr());
            }

            return Ok(());
        }

        Err(Error::NoHandle)
    }

    /// Set the sender DX11 shared texture format. `format` is actually a Windows `DWORD`,
    /// which _should_ be covered by `c_long`.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// The `format` that is sent is **_not_** guaranteed to be safe.
    pub fn set_sender_format(&mut self, format: DWORD) -> Result<()> {
        if let Some(lib) = self.library {
            unsafe {
                as_pin(lib).SetSenderFormat(format);
            }

            return Ok(());
        }

        Err(Error::NoHandle)
    }

    /// Close sender and free resources. A sender is created or updated by all
    /// sending functions.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// The `msec` that is sent is **_not_** guaranteed to be safe.
    pub fn release_sender(&mut self, msec: DWORD) -> Result<()> {
        if let Some(lib) = self.library {
            unsafe {
                as_pin(lib).ReleaseSender(msec);
            }

            return Ok(());
        }

        Err(Error::NoHandle)
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
    pub fn send_fbo(
        &mut self,
        fbo_id: GLuint,
        width: c_uint,
        height: c_uint,
        invert: bool,
    ) -> Result<bool> {
        if let Some(lib) = self.library {
            unsafe { return Ok(as_pin(lib).SendFbo(fbo_id, width, height, invert)) }
        }

        Err(Error::NoHandle)
    }

    /// Send an OpenGL texture.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn send_texture(
        &mut self,
        texture_id: GLuint,
        texture_target: GLuint,
        width: c_uint,
        height: c_uint,
        invert: bool,
        host_fbo: GLuint,
    ) -> Result<bool> {
        if let Some(lib) = self.library {
            unsafe {
                return Ok(as_pin(lib).SendTexture(
                    texture_id,
                    texture_target,
                    width,
                    height,
                    invert,
                    host_fbo,
                ));
            }
        }

        Err(Error::NoHandle)
    }

    /// Send image pixels. NOTE: this is very slow.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn send_image(
        &mut self,
        pixels: *const u8,
        width: c_uint,
        height: c_uint,
        gl_format: GLenum,
        invert: bool,
    ) -> Result<bool> {
        if let Some(lib) = self.library {
            unsafe { return Ok(as_pin(lib).SendImage(pixels, width, height, gl_format, invert)) }
        }

        Err(Error::NoHandle)
    }

    /// Gets the sender name.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_name(&mut self) -> Result<String> {
        if let Some(lib) = self.library {
            // FIXME rust is dropping the pointer, thus causing memory corruption I think
            let name = unsafe { CString::from_raw(as_pin(lib).GetName().cast_mut()) };
            return match String::from_utf8(name.into_bytes()) {
                Ok(v) => Ok(v),
                Err(e) => Err(Error::FfiTypeFrom {
                    ffi_type: FfiType::CString,
                    context: format!("get_name: {e}"),
                }),
            };
        }

        Err(Error::NoHandle)
    }

    /// Get the sender width.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_width(&mut self) -> Result<u32> {
        if let Some(lib) = self.library {
            unsafe {
                return Ok(as_pin(lib).GetWidth().0);
            }
        }

        Err(Error::NoHandle)
    }

    /// Get the sender height.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_height(&mut self) -> Result<u32> {
        if let Some(lib) = self.library {
            unsafe {
                return Ok(as_pin(lib).GetHeight().0);
            }
        }

        Err(Error::NoHandle)
    }

    /// Get the sender frame rate.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_fps(&mut self) -> Result<f64> {
        if let Some(lib) = self.library {
            unsafe {
                return Ok(as_pin(lib).GetFps());
            }
        }

        Err(Error::NoHandle)
    }

    /// Get the sender frame number.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_frame(&mut self) -> Result<i32> {
        if let Some(lib) = self.library {
            unsafe {
                return Ok(as_pin(lib).GetFrame().0);
            }
        }

        Err(Error::NoHandle)
    }

    /// Get the sender share handle.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// No safety guarantees are made about the void pointer `c_void`.
    pub fn get_handle(&mut self) -> Result<*mut c_void> {
        if let Some(lib) = self.library {
            unsafe {
                return Ok(as_pin(lib).GetHandle());
            }
        }

        Err(Error::NoHandle)
    }

    /// Get the sender sharing method.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_cpu(&mut self) -> Result<bool> {
        if let Some(lib) = self.library {
            unsafe {
                return Ok(as_pin(lib).GetCPU());
            }
        }

        Err(Error::NoHandle)
    }

    /// Get the sender GL/DX hardware compatibility.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_gl_dx(&mut self) -> Result<bool> {
        if let Some(lib) = self.library {
            unsafe {
                return Ok(as_pin(lib).GetGLDX());
            }
        }

        Err(Error::NoHandle)
    }
}

/// Convencience function for pinning a pointer.
unsafe fn as_pin<'a>(ptr: *mut ffi::SPOUTLIBRARY) -> Pin<&'a mut ffi::SPOUTLIBRARY> {
    Pin::new_unchecked(&mut *ptr)
}
