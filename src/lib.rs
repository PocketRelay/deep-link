#![warn(unused_crate_dependencies)]

use std::fs::File;
use std::io::Write;
use std::os::raw::c_void;

use retour::static_detour;
use sdk::core::{FString, UFunction, UObject};
use sdk::sfxgame::{FSFXOnlineMOTDInfo, USFXOnlineComponentUI};
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
extern "stdcall" fn DllMain(_hmodule: isize, reason: u32, _: *mut ()) -> bool {
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

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "thiscall" fn fake_process_event(
    object: *mut UObject,
    func: *mut UFunction,
    params: *mut c_void,
    result: *mut c_void,
) {
    let mut name = func.read().as_object_ref().get_full_name();
    name.push('\n');

    // Log the processed event full function name by writing it to a file
    MESSAGES
        .as_mut()
        .unwrap()
        .write_all(name.as_bytes())
        .unwrap();

    // Hook existing display notification event code
    if name.contains("Function SFXGame.SFXOnlineComponentUI.OnDisplayNotification") {
        // Get mutable reference to type
        let this = object
            .cast::<USFXOnlineComponentUI>()
            .as_mut()
            .expect("USFXOnlineComponentUI class was null");

        // Message parameters
        let title = FString::from_str_with_null("Origin Confirmation Code\0");
        let message = FString::from_str_with_null(
            "You Origin confirmation code is <font color='#FFFF66'>AC198E</font>, enter this on the dashboard to set a new password\0");
        let image = FString::from_str_with_null("\0");

        // Include custom message aswell
        this.event_on_display_notification(FSFXOnlineMOTDInfo {
            message,
            title,
            image,
            tracking_id: -1,
            priority: 1,
            bw_ent_id: 0,
            offer_id: 0,
            ty: 0,
        });
    }

    ProcessEvent.call(object, func, params, result);
}
