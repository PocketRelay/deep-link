#![warn(unused_crate_dependencies)]

use std::fs::File;
use std::io::Write;
use std::os::raw::c_void;

use retour::static_detour;
use sdk::core::{UFunction, UObject};
use windows_sys::Win32::System::Console::{AllocConsole, FreeConsole};
use windows_sys::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};

mod sdk;

type ProcessEventTy =
    unsafe extern "thiscall" fn(*mut UObject, *mut UFunction, *mut c_void, *mut c_void);

static_detour! {
  static ProcessEvent: unsafe extern "thiscall" fn(*mut UObject, *mut UFunction, *mut c_void, *mut c_void);
}

/// Windows DLL entrypoint for the plugin
#[no_mangle]
extern "stdcall" fn DllMain(hmodule: isize, reason: u32, _: *mut ()) -> bool {
    if let DLL_PROCESS_ATTACH = reason {
        unsafe {
            AllocConsole();
        }

        unsafe { MESSAGES = Some(File::create("event-dump.txt").unwrap()) }

        std::thread::spawn(hook_process_event);
    } else if let DLL_PROCESS_DETACH = reason {
        unsafe {
            FreeConsole();
        }
    }

    true
}

pub fn hook_process_event() {
    unsafe {
        ProcessEvent
            .initialize(
                std::mem::transmute::<u32, ProcessEventTy>(0x00453120),
                |object, func, params, result| fake_process_event(object, func, params, result),
            )
            .expect("Failed to create detour")
            .enable()
            .expect("Failed to enable detour")
    };
}

static mut MESSAGES: Option<File> = None;

/// Offline check that always returns TRUE
///
/// ## Safety
///
/// Doesn't perform any unsafe actions, just must be marked as unsafe to
/// be used as an extern fn
#[no_mangle]
pub unsafe extern "thiscall" fn fake_process_event(
    object: *mut UObject,
    func: *mut UFunction,
    params: *mut c_void,
    result: *mut c_void,
) {
    let mut name = func.read().as_object_ref().get_full_name();
    name.push('\n');

    MESSAGES.as_mut().unwrap().write_all(name.as_bytes());

    ProcessEvent.call(object, func, params, result);
}
