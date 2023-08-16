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
    #[error("Spout function not bindable to Rust")]
    Unbindable,
    #[error("Unexpected value: {context:?}")]
    UnexpectedValue { context: String },
}

#[derive(Debug)]
pub enum FfiType {
    CString,
    CStr,
    CInt,
}

#[derive(Debug)]
pub enum DxgiGpuPreference {
    NotRegistered,
    Unspecified,
    MinimumPower,
    HighPerformance,
}

impl TryInto<String> for DxgiGpuPreference {
    type Error = Error;

    fn try_into(self) -> std::result::Result<String, Self::Error> {
        match self {
            DxgiGpuPreference::Unspecified => Ok("DXGI_GPU_PREFERENCE_UNSPECIFIED".to_string()),
            DxgiGpuPreference::MinimumPower => Ok("DXGI_GPU_PREFERENCE_MINIMUM_POWER".to_string()),
            DxgiGpuPreference::HighPerformance => {
                Ok("DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE".to_string())
            }
            _ => Err(Error::UnexpectedValue {
                context: "DxgiGpuPreference::try_into".to_string(),
            }),
        }
    }
}

impl TryInto<i32> for DxgiGpuPreference {
    type Error = Error;

    fn try_into(self) -> std::result::Result<i32, Self::Error> {
        match self {
            DxgiGpuPreference::NotRegistered => Ok(-1),
            DxgiGpuPreference::Unspecified => Ok(0),
            DxgiGpuPreference::MinimumPower => Ok(1),
            DxgiGpuPreference::HighPerformance => Ok(2),
            _ => Err(Error::UnexpectedValue {
                context: "DxgiGpuPreference::try_into".to_string(),
            }),
        }
    }
}

impl TryFrom<i32> for DxgiGpuPreference {
    type Error = Error;

    fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::NotRegistered),
            0 => Ok(Self::Unspecified),
            1 => Ok(Self::MinimumPower),
            2 => Ok(Self::HighPerformance),
            _ => Err(Error::UnexpectedValue {
                context: "DxgiGpuPreference::try_from".to_string(),
            }),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

// Typedefs using concrete types instead of ffi types for readability.

/// A Windows DWORD which _should_ be a ulong.
pub type DWORD = c_ulong;
/// A void pointer.
pub type HANDLE = ffi::HANDLE;
/// An OpenGL uint which _should_ be a uint.
pub type GLuint = c_uint;
// TODO lookup the actual enums and redefine them here?
/// An OpenGL enum which _should_ be a uint.
pub type GLenum = c_uint;
/// Enum representing log levels.
pub type SpoutLibLogLevel = ffi::SpoutLibLogLevel;

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

// TODO use file!() and line!() instead of passing fn_name
/// Conversion helper for creating [CString]s from anything that implements [AsRef]<[str]>.
macro_rules! str_to_cstring {
    ($fn_name:expr, $str:expr) => {{
        match CString::new($str.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::FfiTypeInto {
                    ffi_type: FfiType::CString,
                    context: format!("{}: {e}", $fn_name),
                })
            }
        }
    }};
}

// TODO use file!() and line!() instead of passing fn_name
/// Conversion helper for creating [String]s from [CString]s.
macro_rules! cstring_to_string {
    ($fn_name:expr, $c_string:expr) => {{
        match $c_string.to_str() {
            Ok(v) => v.to_string(),
            Err(e) => {
                return Err(Error::FfiTypeFrom {
                    ffi_type: FfiType::CString,
                    context: format!("{}: {e}", $fn_name),
                })
            }
        }
    }};
}

/// Conversion helper for creating a [CStr] from a buffer.
macro_rules! buf_to_cstr {
    ($buf:expr) => {{
        match CStr::from_bytes_with_nul($buf.as_slice()) {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::FfiTypeInto {
                    ffi_type: FfiType::CStr,
                    context: format!("{} - {}: {e}", file!(), line!()),
                })
            }
        }
    }};
}

macro_rules! usize_to_c_int {
    ($usize:expr) => {{
        match i32::try_from($usize) {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::FfiTypeInto {
                    ffi_type: FfiType::CInt,
                    context: format!("read_memory_buffer: {e}"),
                })
            }
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
        width: u32,
        height: u32,
        invert: bool,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.SendFbo(fbo_id, width.into(), height.into(), invert))
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
        width: u32,
        height: u32,
        invert: bool,
        host_fbo: GLuint,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.SendTexture(
            texture_id,
            texture_target,
            width.into(),
            height.into(),
            invert,
            host_fbo,
        ))
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
        width: u32,
        height: u32,
        gl_format: GLenum,
        invert: bool,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let success =
            unsafe { lib.SendImage(pixels, width.into(), height.into(), gl_format, invert) };

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
    /// No safety guarantees are made about the `HANDLE`.
    pub fn get_handle(&mut self) -> Result<HANDLE> {
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
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// No safety guarantees are made about the `HANDLE`.
    pub fn get_sender_handle(&mut self) -> Result<HANDLE> {
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

    /// Disable frame counting specifically for this application;
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn disable_frame_count(&mut self) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.DisableFrameCount();

        Ok(())
    }

    /// Return frame count status.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn is_frame_count_enabled(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.IsFrameCountEnabled())
    }

    /// Sender frame rate control.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn hold_fps(&mut self, fps: i32) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.HoldFps(fps.into());

        Ok(())
    }

    /// Get the system refresh rate.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn get_refresh_rate(&mut self) -> Result<f64> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetRefreshRate())
    }

    /// Signal sync event.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// The [CString] should be copied on the Spout side and is safe to drop.
    pub fn set_frame_sync<T: AsRef<str>>(&mut self, sender_name: T) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        let name = match CString::new(sender_name.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::FfiTypeInto {
                    ffi_type: FfiType::CString,
                    context: format!("set_frame_sync: {e}"),
                })
            }
        };

        unsafe {
            lib.SetFrameSync(name.as_ptr());
        }

        Ok(())
    }

    /// Wait or test for a sync event.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// The [CString] should be copied on the Spout side and should be safe to drop.
    pub fn wait_frame_sync<T: AsRef<str>>(
        &mut self,
        sender_name: T,
        timeout: DWORD,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let name = match CString::new(sender_name.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::FfiTypeInto {
                    ffi_type: FfiType::CString,
                    context: format!("wait_frame_sync: {e}"),
                })
            }
        };

        let success = unsafe { lib.WaitFrameSync(name.as_ptr(), timeout) };

        Ok(success)
    }

    /// Write data.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// The sender name and data should be copied on the Spout side and should be safe to drop.
    pub fn write_memory_buffer<T: AsRef<str>>(&mut self, sender_name: T, data: T) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let name = str_to_cstring!("write_memory_buffer", sender_name);
        let data = str_to_cstring!("write_memory_buffer", data);
        let length = data.as_c_str().to_bytes_with_nul().len();

        let success =
            unsafe { lib.WriteMemoryBuffer(name.as_ptr(), data.as_ptr(), (length as i32).into()) };

        Ok(success)
    }

    /// Read data.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// No safety guarantees are made about the data returned from the memory buffer.
    pub fn read_memory_buffer<T: AsRef<str>>(
        &mut self,
        sender_name: T,
        max_length: usize,
    ) -> Result<(i32, String)> {
        let lib = unsafe { library!(self.library) };

        let name = str_to_cstring!("read_memory_buffer", sender_name);

        let mut buffer = vec![1; max_length - 1];
        buffer.push(0);
        let data = match CStr::from_bytes_with_nul(buffer.as_slice()) {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::FfiTypeInto {
                    ffi_type: FfiType::CStr,
                    context: format!("read_memory_buffer: {e}"),
                })
            }
        };

        let max_length: i32 = match max_length.try_into() {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::FfiTypeInto {
                    ffi_type: FfiType::CInt,
                    context: format!("read_memory_buffer: {e}"),
                })
            }
        };

        let result = unsafe {
            lib.ReadMemoryBuffer(name.as_ptr(), data.as_ptr().cast_mut(), max_length.into())
        };

        let data = cstring_to_string!("read_memory_buffer", data);

        Ok((result.0, data))
    }

    /// Create a shared memory buffer.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn create_memory_buffer<T: AsRef<str>>(&mut self, name: T, length: i32) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let name = str_to_cstring!("create_memory_buffer", name);

        let success = unsafe { lib.CreateMemoryBuffer(name.as_ptr(), length.into()) };

        Ok(success)
    }

    /// Delete a shared memory buffer.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn delete_memory_buffer(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.DeleteMemoryBuffer())
    }

    /// Get the number of bytes available for data transfer.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// The name should be copied by Spout and should be safe to drop.
    pub fn get_memory_buffer_size<T: AsRef<str>>(&mut self, name: T) -> Result<i32> {
        let lib = unsafe { library!(self.library) };

        let name = str_to_cstring!("get_memory_buffer_size", name);

        let size = unsafe { lib.GetMemoryBufferSize(name.as_ptr()) };

        Ok(size.0)
    }

    /// Open console for debugging.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn open_spout_console(&mut self) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.OpenSpoutConsole();

        Ok(())
    }

    pub fn close_spout_console(&mut self, warning: bool) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.CloseSpoutConsole(warning);

        Ok(())
    }

    pub fn enable_spout_log(&mut self) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.EnableSpoutLog();

        Ok(())
    }

    pub fn enable_spout_log_file<T: AsRef<str>>(
        &mut self,
        filename: T,
        append: bool,
    ) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        let filename = str_to_cstring!("enable_spout_log_file", filename);

        unsafe {
            lib.EnableSpoutLogFile(filename.as_ptr(), append);
        }

        Ok(())
    }

    pub fn get_spout_log(&mut self) -> Result<String> {
        let lib = unsafe { library!(self.library) };

        let log = lib.GetSpoutLog();

        Ok(log.to_string())
    }

    pub fn show_spout_logs(&mut self) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.ShowSpoutLogs();

        Ok(())
    }

    pub fn disable_spout_log(&mut self) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.DisableSpoutLog();

        Ok(())
    }

    pub fn set_spout_log_level(&mut self, level: SpoutLibLogLevel) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.SetSpoutLogLevel(level);

        Ok(())
    }

    pub fn spout_log<T: AsRef<str>>(&mut self, _format: T) -> Result<()> {
        Err(Error::Unbindable)
    }

    pub fn spout_log_verbose<T: AsRef<str>>(&mut self, _format: T) -> Result<()> {
        Err(Error::Unbindable)
    }

    pub fn spout_log_notice<T: AsRef<str>>(&mut self, _format: T) -> Result<()> {
        Err(Error::Unbindable)
    }

    pub fn spout_log_warning<T: AsRef<str>>(&mut self, _format: T) -> Result<()> {
        Err(Error::Unbindable)
    }

    pub fn spout_log_error<T: AsRef<str>>(&mut self, _format: T) -> Result<()> {
        Err(Error::Unbindable)
    }

    pub fn spout_log_fatal<T: AsRef<str>>(&mut self, _format: T) -> Result<()> {
        Err(Error::Unbindable)
    }

    /// MessageBox dialog with optional timeout.
    ///
    /// Used where a Windows MessageBox would interfere with the application GUI. The dialog closes iteself if a
    /// timeout is specified.
    ///
    /// # Important
    /// **How this actually works is not checkable from Rust!**
    ///
    /// Additionally, there is an overloaded method that is not bindable to Rust that takes more parameters.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    ///
    /// The message sent to Spout is assumed to be a valid [CString], and Spout is assumed to copy the message.
    pub fn spout_message_box<T: AsRef<str>>(
        &mut self,
        message: T,
        milliseconds: DWORD,
    ) -> Result<i32> {
        let lib = unsafe { library!(self.library) };

        let message = str_to_cstring!("spout_message_box", message);

        let result = unsafe { lib.SpoutMessageBox(message.as_ptr(), milliseconds) };

        Ok(result.0)
    }

    /// Read subkey DWORD value.
    ///
    /// # Important
    /// This method is not bindable to Rust.
    ///
    /// `key` is not actually a `DWORD`.
    pub fn read_dword_from_registry<T: AsRef<str>>(
        &mut self,
        _key: DWORD,
        _sub_key: T,
        _value_name: T,
        _value: DWORD,
    ) -> Result<bool> {
        Err(Error::Unbindable)
    }

    /// Write subkey DWORD value.
    ///
    /// # Important
    /// This method is not bindable to Rust.
    ///
    /// `key` is not actually a `DWORD`.
    pub fn write_dword_to_registry<T: AsRef<str>>(
        &mut self,
        _key: DWORD,
        _sub_key: T,
        _value_name: T,
        _value: DWORD,
    ) -> Result<bool> {
        Err(Error::Unbindable)
    }

    /// Read subkey character string.
    ///
    /// # Important
    /// This method is not bindable to Rust.
    ///
    /// `key` is not actually a `DWORD`.
    pub fn read_path_from_registry<T: AsRef<str>>(
        &mut self,
        _key: DWORD,
        _sub_key: T,
        _value_name: T,
        _file_path: T,
    ) -> Result<bool> {
        Err(Error::Unbindable)
    }

    /// Write subkey character string.
    ///
    /// # Important
    /// This method is not bindable to Rust.
    ///
    /// `key` is not actually a `DWORD`.
    pub fn write_path_to_registry<T: AsRef<str>>(
        &mut self,
        _key: DWORD,
        _sub_key: T,
        _value_name: T,
        _file_path: T,
    ) -> Result<bool> {
        Err(Error::Unbindable)
    }

    /// Remove subkey value name.
    ///
    /// # Important
    /// This method is not bindable to Rust.
    ///
    /// `key` is not actually a `DWORD`.
    pub fn remove_path_from_registry<T: AsRef<str>>(
        &mut self,
        _key: DWORD,
        _sub_key: T,
        _value_name: T,
    ) -> Result<bool> {
        Err(Error::Unbindable)
    }

    /// Delete a subkey and its values.
    ///
    /// It must be a subkey of the key that `key` identifies, but it cannot have subkeys. Note that key names are
    /// not case sensitive.
    ///
    /// # Important
    /// This method is not bindable to Rust.
    ///
    /// `key` is not actually a `DWORD`.
    pub fn remove_sub_key<T: AsRef<str>>(&mut self, _key: DWORD, _sub_key: T) -> Result<bool> {
        Err(Error::Unbindable)
    }

    /// Find subkey.
    ///
    /// # Important
    /// This method is not bindable to Rust.
    ///
    /// `key` is not actually a `DWORD`.
    pub fn find_sub_key<T: AsRef<str>>(&mut self, _key: DWORD, _sub_key: T) -> Result<bool> {
        Err(Error::Unbindable)
    }

    pub fn get_sdk_version(&mut self) -> Result<String> {
        let lib = unsafe { library!(self.library) };

        let version = lib.GetSDKversion();

        Ok(version.to_string())
    }

    pub fn is_laptop(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.IsLaptop())
    }

    pub fn start_timing(&mut self) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.StartTiming();

        Ok(())
    }

    pub fn end_timing(&mut self) -> Result<f64> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.EndTiming())
    }

    pub fn is_initialized(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.IsInitialized())
    }

    pub fn bind_shared_texture(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.BindSharedTexture())
    }

    pub fn unbind_shared_texture(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.UnBindSharedTexture())
    }

    pub fn get_shared_texture_id(&mut self) -> Result<GLuint> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetSharedTextureID())
    }

    pub fn get_sender_count(&mut self) -> Result<i32> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetSenderCount().0)
    }

    pub fn get_sender<T: AsRef<str>>(
        &mut self,
        index: i32,
        max_size: usize,
    ) -> Result<(bool, String)> {
        let lib = unsafe { library!(self.library) };

        let mut buffer = vec![1; max_size - 1];
        buffer.push(0);
        let sender_name = buf_to_cstr!(buffer);

        let max_size = usize_to_c_int!(max_size);

        let success = unsafe {
            lib.GetSender(
                index.into(),
                sender_name.as_ptr().cast_mut(),
                max_size.into(),
            )
        };

        let sender_name = cstring_to_string!("get_sender", sender_name);

        Ok((success, sender_name))
    }

    pub fn find_sender_name<T: AsRef<str>>(&mut self, sender_name: T) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let sender_name = str_to_cstring!("find_sender_name", sender_name);

        let found = unsafe { lib.FindSenderName(sender_name.as_ptr()) };

        Ok(found)
    }

    pub fn get_sender_info<T: AsRef<str>>(
        &mut self,
        sender_name: T,
        width: u32,
        height: u32,
        share_handle: HANDLE,
        format: DWORD,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let sender_name = str_to_cstring!("get_sender_info", sender_name);

        // TODO all these params need to be pinned

        // let success = unsafe {
        //     lib.GetSenderInfo(
        //         sender_name.as_ptr(),
        //         width.into(),
        //         height.into(),
        //         share_handle,
        //         format,
        //     )
        // };

        // Ok(success)

        todo!()
    }

    pub fn get_active_sender<T: AsRef<str>>(&mut self) -> Result<(bool, String)> {
        let lib = unsafe { library!(self.library) };

        let mut buffer = vec![];
        buffer.push(0);
        let sender_name = buf_to_cstr!(buffer);

        let success = unsafe { lib.GetActiveSender(sender_name.as_ptr().cast_mut()) };

        let sender_name = cstring_to_string!("get_active_sender", sender_name);

        Ok((success, sender_name))
    }

    pub fn set_active_sender<T: AsRef<str>>(&mut self, sender_name: T) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let sender_name = str_to_cstring!("set_active_sender", sender_name);

        let success = unsafe { lib.SetActiveSender(sender_name.as_ptr()) };

        Ok(success)
    }

    pub fn get_buffer_mode(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetBufferMode())
    }

    pub fn set_buffer_mode(&mut self, active: bool) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.SetBufferMode(active);

        Ok(())
    }

    pub fn get_buffers(&mut self) -> Result<i32> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetBuffers().0)
    }

    pub fn set_buffers(&mut self, buffers: i32) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.SetBuffers(buffers.into());

        Ok(())
    }

    pub fn get_max_senders(&mut self) -> Result<i32> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetMaxSenders().0)
    }

    pub fn set_max_senders(&mut self, max_senders: i32) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.SetMaxSenders(max_senders.into());

        Ok(())
    }

    pub fn create_sender<T: AsRef<str>>(
        &mut self,
        sender_name: T,
        width: u32,
        height: u32,
        format: DWORD,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let sender_name = str_to_cstring!("create_sender", sender_name);

        let success =
            unsafe { lib.CreateSender(sender_name.as_ptr(), width.into(), height.into(), format) };

        Ok(success)
    }

    pub fn update_sender<T: AsRef<str>>(
        &mut self,
        sender_name: T,
        width: u32,
        height: u32,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let sender_name = str_to_cstring!("update_sender", sender_name);

        let success =
            unsafe { lib.UpdateSender(sender_name.as_ptr(), width.into(), height.into()) };

        Ok(success)
    }

    /// Create receiver connection.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn create_receiver<T: AsRef<str>>(
        &mut self,
        sender_name: T,
        width: u32,
        height: u32,
        use_active: bool,
    ) -> Result<bool> {
        // TODO this method requires all params to be pinned
        todo!()
    }

    /// Check receiver connection.
    ///
    /// # Safety
    /// Guaranteed to have a valid pointer to `SPOUTLIBRARY` as long as the backing struct exists.
    pub fn check_receiver<T: AsRef<str>>(
        &mut self,
        sender_name: T,
        width: u32,
        height: u32,
        use_active: bool,
    ) -> Result<bool> {
        // TODO this method requires all params to be pinned
        todo!()
    }

    pub fn get_dx9(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetDX9())
    }

    pub fn set_dx9(&mut self, dx9: bool) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.SetDX9(dx9))
    }

    pub fn get_memory_share_mode(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetMemoryShareMode())
    }

    pub fn set_memory_share_mode(&mut self, mem: bool) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.SetMemoryShareMode(mem))
    }

    pub fn get_cpu_mode(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetCPUmode())
    }

    pub fn set_cpu_mode(&mut self, cpu: bool) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.SetCPUmode(cpu))
    }

    pub fn get_share_mode(&mut self) -> Result<i32> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetShareMode().0)
    }

    pub fn set_share_mode(&mut self, mode: i32) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.SetShareMode(mode.into());

        Ok(())
    }

    pub fn select_sender_panel(&mut self) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.SelectSenderPanel();

        Ok(())
    }

    // TODO host_path is an out param, maybe it's not needed?
    pub fn get_host_path<T: AsRef<str>>(
        &mut self,
        sender_name: T,
        host_path: T,
        max_chars: i32,
    ) -> Result<(bool, String)> {
        let lib = unsafe { library!(self.library) };

        let sender_name = str_to_cstring!("get_host_path", sender_name);
        let host_path = str_to_cstring!("get_host_path", host_path);

        let success = unsafe {
            lib.GetHostPath(
                sender_name.as_ptr(),
                // TODO cast_mut might not be right
                host_path.as_ptr().cast_mut(),
                max_chars.into(),
            )
        };

        let host_path = cstring_to_string!("get_host_path", host_path);

        Ok((success, host_path))
    }

    pub fn get_vertical_sync(&mut self) -> Result<i32> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetVerticalSync().0)
    }

    pub fn set_vertical_sync(&mut self, sync: bool) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.SetVerticalSync(sync))
    }

    pub fn get_spout_version(&mut self) -> Result<i32> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetSpoutVersion().0)
    }

    pub fn get_auto_share(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetAutoShare())
    }

    pub fn set_auto_share(&mut self, auto: bool) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.SetAutoShare(auto);

        Ok(())
    }

    pub fn is_gl_dx_ready(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.IsGLDXready())
    }

    pub fn get_num_adapters(&mut self) -> Result<i32> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetNumAdapters().0)
    }

    pub fn get_adapter_name<T: AsRef<str>>(
        &mut self,
        index: i32,
        max_chars: usize,
    ) -> Result<(bool, String)> {
        let lib = unsafe { library!(self.library) };

        let mut buffer = vec![1; max_chars - 1];
        buffer.push(0);
        let adapter_name = buf_to_cstr!(buffer);

        let max_chars = usize_to_c_int!(max_chars);

        let success = unsafe {
            lib.GetAdapterName(
                index.into(),
                adapter_name.as_ptr().cast_mut(),
                max_chars.into(),
            )
        };

        let adapter_name = cstring_to_string!("get_adapter_name", adapter_name);

        Ok((success, adapter_name))
    }

    pub fn get_adapter(&mut self) -> Result<i32> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.GetAdapter().0)
    }

    pub fn get_performance_preference<T: AsRef<str>>(
        &mut self,
        path: T,
    ) -> Result<DxgiGpuPreference> {
        let lib = unsafe { library!(self.library) };

        let path = str_to_cstring!("get_performance_preference", path);

        let val = unsafe { lib.GetPerformancePreference(path.as_ptr()) };

        DxgiGpuPreference::try_from(val.0)
    }

    pub fn set_performance_preference<T: AsRef<str>>(
        &mut self,
        preference: DxgiGpuPreference,
        path: T,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let preference = match TryInto::<i32>::try_into(preference) {
            Ok(v) => v.into(),
            // TODO rust analyzer doesn't like use e @ Err(_) for some reason
            Err(e) => return Err(e),
        };
        let path = str_to_cstring!("set_performance_preference", path);

        let success = unsafe { lib.SetPerformancePreference(preference, path.as_ptr()) };

        Ok(success)
    }

    pub fn get_preferred_adapter_name<T: AsRef<str>>(
        &mut self,
        preference: DxgiGpuPreference,
        max_chars: usize,
    ) -> Result<(bool, String)> {
        let lib = unsafe { library!(self.library) };

        let preference = match TryInto::<i32>::try_into(preference) {
            Ok(v) => v.into(),
            // TODO rust analyzer doesn't like use e @ Err(_) for some reason
            Err(e) => return Err(e),
        };

        let mut buffer = vec![1; max_chars - 1];
        buffer.push(0);
        let adapter_name = buf_to_cstr!(buffer);

        let max_chars = usize_to_c_int!(max_chars);

        let success = unsafe {
            lib.GetPreferredAdapterName(
                preference,
                adapter_name.as_ptr().cast_mut(),
                max_chars.into(),
            )
        };

        let adapter_name = cstring_to_string!("get_preferred_adapter_name", adapter_name);

        Ok((success, adapter_name))
    }

    pub fn set_preferred_adapter(&mut self, preference: DxgiGpuPreference) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let preference = match TryInto::<i32>::try_into(preference) {
            Ok(v) => v.into(),
            // TODO rust analyzer doesn't like use e @ Err(_) for some reason
            Err(e) => return Err(e),
        };

        Ok(lib.SetPreferredAdapter(preference))
    }

    pub fn is_preference_available(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.IsPreferenceAvailable())
    }

    pub fn is_application_path<T: AsRef<str>>(&mut self, path: T) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let path = str_to_cstring!("is_application_path", path);

        let success = unsafe { lib.IsApplicationPath(path.as_ptr()) };

        Ok(success)
    }

    pub fn create_opengl(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.CreateOpenGL())
    }

    pub fn close_opengl(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.CloseOpenGL())
    }

    pub fn copy_texture(
        &mut self,
        source_id: GLuint,
        source_target: GLuint,
        dest_id: GLuint,
        dest_target: GLuint,
        width: u32,
        height: u32,
        invert: bool,
        host_fbo: GLuint,
    ) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.CopyTexture(
            source_id,
            source_target,
            dest_id,
            dest_target,
            width.into(),
            height.into(),
            invert,
            host_fbo,
        ))
    }

    pub fn open_directx(&mut self) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        Ok(lib.OpenDirectX())
    }

    pub fn close_directx(&mut self) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.CloseDirectX();

        Ok(())
    }

    pub fn open_directx11(&mut self, device: *mut c_void) -> Result<bool> {
        let lib = unsafe { library!(self.library) };

        let success = unsafe { lib.OpenDirectX11(device) };

        Ok(success)
    }

    pub fn close_directx11(&mut self) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.CloseDirectX11();

        Ok(())
    }

    pub fn get_dx11_device(&mut self) -> Result<*mut c_void> {
        let lib = unsafe { library!(self.library) };

        let ptr = lib.GetDX11Device();
        if ptr.is_null() {
            return Err(Error::NullPtr);
        }

        Ok(ptr)
    }

    pub fn get_dx11_context(&mut self) -> Result<*mut c_void> {
        let lib = unsafe { library!(self.library) };

        let ptr = lib.GetDX11Context();
        if ptr.is_null() {
            return Err(Error::NullPtr);
        }

        Ok(ptr)
    }

    pub fn release(&mut self) -> Result<()> {
        let lib = unsafe { library!(self.library) };

        lib.Release();
        self.library = None;

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
