#![warn(unused_crate_dependencies)]

use std::fs::File;
use std::io::Write;
use std::os::raw::c_void;

use retour::static_detour;
use sdk::core::{
    add_ticker_message, FString, UFunction,
    UObject, USFXGUI_MainMenu_RightComputer,
    USFXOnlineComponentUI_eventOnDisplayNotification_Parms,
};
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
static mut MESSAGE_SENT: bool = false;

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


    if name.contains("Function SFXGame.SFXOnlineComponentUI.OnDisplayNotification") {
        let params: *mut USFXOnlineComponentUI_eventOnDisplayNotification_Parms = params.cast();

        let params = params.as_ref().unwrap();
        let message = &params.Info;

        writeln!(
            MESSAGES.as_mut().unwrap(), 
            "Message: {}, Title: {}, Image: {}, Tracking ID: {}, Priority: {}, BWEntId: {}, offerId: {}, Type: {}", 
            message.Message, 
            message.Title,
             message.Image, 
             message.TrackingID, 
             message.Priority, 
             message.BWEntId, 
             message.offerId, 
             message.Type);

             let title = FString::from_string("This is a test title".to_string());
             let message = FString::from_string("This is a test message".to_string());
             let image = FString::from_string("".to_string());

             let params = USFXOnlineComponentUI_eventOnDisplayNotification_Parms{
                Info: sdk::core::FSFXOnlineMOTDInfo { Message: message, Title: title, Image: image, TrackingID: -1, Priority: 1, BWEntId: 0, offerId: 0, Type: 0 },
             };


             // Include custom message aswell
        ProcessEvent.call(object, func, &params as *const _ as *mut _, result);
    }

    // if name.contains("Function SFXGame.SFXGUI_MainMenu_RightComputer.MessageAboutToDisplay") {
    //     MESSAGES
    //         .as_mut()
    //         .unwrap()
    //         .write_all("WRITING TERMINAL MESSAGE".as_bytes());

    //     add_ticker_message(
    //         object as *mut USFXGUI_MainMenu_RightComputer,
    //         0,
    //         message,
    //         0,
    //         0,
    //     );
    // }

    ProcessEvent.call(object, func, params, result);
}
