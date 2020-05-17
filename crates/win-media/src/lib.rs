// This creates sole purpose is to reduce the compile time by not doing the WinRT import in the
// actual crate that is using the API.

winrt::import!(
    dependencies
        "os"
    modules
        "windows.foundation"
        "windows.storage.streams"
        "windows.media.speechsynthesis"
        "windows.system"
);

pub use windows::*;
pub use winrt::{Error, Result, RuntimeType};