use godot::{engine::global::Error, prelude::*};

use crate::RustySpout;

struct SpoutGdExtension;

#[gdextension]
unsafe impl ExtensionLibrary for SpoutGdExtension {}

#[derive(GodotClass)]
#[class(base = Object)]
struct SpoutGd {
    library: RustySpout,
}

#[godot_api]
impl ObjectVirtual for SpoutGd {
    fn init(_base: godot::obj::Base<Self::Base>) -> Self {
        Self::new()
    }
}

#[godot_api]
impl SpoutGd {
    #[func]
    fn get_spout(&mut self) -> Error {
        match self.library.get_spout() {
            Ok(_) => Error::OK,
            Err(e) => {
                godot_error!("{e}");
                Error::ERR_UNAVAILABLE
            }
        }
    }

    #[func]
    fn set_sender_name(&mut self, name: GodotString) -> Error {
        match self.library.set_sender_name(name.to_string()) {
            Ok(_) => Error::OK,
            Err(e) => {
                godot_error!("{e}");
                Error::ERR_CANT_ACQUIRE_RESOURCE
            }
        }
    }

    #[func]
    fn get_name(&mut self) -> Variant {
        match self.library.get_name() {
            Ok(v) => GodotString::from(v).to_variant(),
            Err(e) => {
                godot_error!("{e}");
                Error::ERR_CANT_ACQUIRE_RESOURCE.to_variant()
            }
        }
    }

    #[func]
    fn set_receiver_name(&mut self, sender_name: GodotString) -> Error {
        match self.library.set_receiver_name(sender_name.to_string()) {
            Ok(_) => Error::OK,
            Err(e) => {
                godot_error!("{e}");
                Error::ERR_CANT_CONNECT
            }
        }
    }

    #[func]
    fn read_memory_buffer(&mut self, buffer_name: GodotString, max_length: u32) -> Variant {
        let max_length = match usize::try_from(max_length) {
            Ok(v) => v,
            Err(e) => {
                godot_error!("{e}");
                return Error::ERR_INVALID_PARAMETER.to_variant();
            }
        };

        match self
            .library
            .read_memory_buffer(buffer_name.to_string(), max_length)
        {
            Ok((_bytes_read, data)) => {
                godot_print!("{data}");
                GodotString::from(data).to_variant()
            }
            Err(e) => {
                godot_error!("{e}");
                Error::ERR_INVALID_DATA.to_variant()
            }
        }
    }
}

impl SpoutGd {
    pub fn new() -> Self {
        Self {
            library: RustySpout::new(),
        }
    }
}
