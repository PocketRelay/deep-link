#![warn(unused_crate_dependencies)]

use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::os::raw::c_void;

use parking_lot::Mutex;
use retour::static_detour;
use sdk::core::{FString, UFunction, UObject};
use sdk::sfxgame::{FSFXOnlineMOTDInfo, USFXOnlineComponentUI};
use serde::{Deserialize, Serialize};
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

#[derive(Deserialize, Serialize)]
pub struct SystemMessage {
    title: String,
    message: String,
    image: String,
    ty: u8,
    tracking_id: i32,
    priority: i32,
}

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
        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        #[allow(non_camel_case_types)]
        struct Params {
            info: FSFXOnlineMOTDInfo,
        }

        let original_params = &params
            .cast::<Params>()
            .as_mut()
            .expect("OnDisplayNotification params were null")
            .info;

        // Log message call
        {
            let messages = MESSAGES.as_mut().unwrap();

            writeln!(messages, "MESSAGE: {:?}", original_params).unwrap();
        }

        let original_message = &original_params.message.to_string();

        // Handle system messages
        if let Some(message) = original_message.strip_prefix("[SYSTEM_TERMINAL]") {
            let value = serde_json::from_str::<SystemMessage>(
                // Stip all non JSON data from the end of the payload
                message.trim_end_matches(|value| value != '}'),
            );

            if let Ok(message) = value {
                // Get mutable reference to type
                let this = object
                    .cast::<USFXOnlineComponentUI>()
                    .as_mut()
                    .expect("USFXOnlineComponentUI class was null");

                // Send custom message instead
                this.event_on_display_notification(FSFXOnlineMOTDInfo {
                    title: FString::from_string(message.title),
                    message: FString::from_string(message.message),
                    image: FString::from_string(message.image),
                    tracking_id: message.tracking_id,
                    priority: message.priority,
                    bw_ent_id: 0,
                    offer_id: 0,
                    ty: message.ty,
                });

                return;
            }
        }
    }

    ProcessEvent.call(object, func, params, result);
}

// Enum SFXOnlineFoundation.SFXOnlineDefine.SFXOnlineConnection_MessageType
/*enum SFXOnlineConnection_MessageType
{
    SFXONLINE_MT_MESSAGEOFTHEDAY                       = 0,
    SFXONLINE_MT_DOWNLOAD_PROMPT                       = 1,
    SFXONLINE_MT_GAW_SUMMARY                           = 2,
    SFXONLINE_MT_GAW_STATUS_UPDATE                     = 3,
    SFXONLINE_MT_FRIEND_ACHIVEMENT                     = 4,
    SFXONLINE_MT_FRIEND_LEADERBOARD_RANK_CHANGE        = 5,
    SFXONLINE_MT_MESSAGEOFTHEDAY_TICKERONLY            = 6,
    SFXONLINE_MT_DISCONNECTED_TICKERONLY               = 7,
    SFXONLINE_MT_MP_PROMO                              = 8,
    SFXONLINE_MT_MAX                                   = 9
};*/
