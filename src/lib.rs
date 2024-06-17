#![warn(unused_crate_dependencies)]

use std::fs::File;
use std::io::Write;
use std::os::raw::c_void;

use sdk::core::{FString, UFunction, UObject};
use sdk::sfxgame::{FSFXOnlineMOTDInfo, USFXOnlineComponentUI};
use serde::{Deserialize, Serialize};
use windows_sys::Win32::System::Console::{AllocConsole, FreeConsole};
use windows_sys::Win32::System::Memory::{
    VirtualAlloc, VirtualProtect, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE,
};
use windows_sys::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};

mod sdk;

type ProcessEvent =
    unsafe extern "thiscall" fn(*mut UObject, *mut UFunction, *mut c_void, *mut c_void);

const JMP_SIZE: usize = 5; // Size of a near jump instruction in x86

static mut ORIGINAL_BYTES: [u8; JMP_SIZE] = [0; JMP_SIZE];
static mut ORIGINAL_FUNCTION: Option<ProcessEvent> = None;

/// # Safety
pub unsafe fn process_event(
    this: *mut UObject,
    func: *mut UFunction,
    params: *mut c_void,
    result: *mut c_void,
) {
    // Call the original function
    (ORIGINAL_FUNCTION.unwrap())(this, func, params, result);
}

unsafe fn hook_function_address(target: *mut u8, hook: *const u8) {
    // Save original bytes
    std::ptr::copy_nonoverlapping(target, ORIGINAL_BYTES.as_mut_ptr(), JMP_SIZE);

    // Construct the jump instruction to the hook function
    let relative_offset = hook as isize - target as isize - JMP_SIZE as isize;
    let jmp_instruction = [
        0xE9,
        (relative_offset & 0xFF) as u8,
        ((relative_offset >> 8) & 0xFF) as u8,
        ((relative_offset >> 16) & 0xFF) as u8,
        ((relative_offset >> 24) & 0xFF) as u8,
    ];

    // Change memory permissions to writable
    let mut old_protect: u32 = 0;
    VirtualProtect(
        target as *mut _,
        JMP_SIZE,
        PAGE_EXECUTE_READWRITE,
        &mut old_protect,
    );

    // Write the jump instruction to the target function
    std::ptr::copy_nonoverlapping(jmp_instruction.as_ptr(), target, JMP_SIZE);

    // Restore memory permissions
    VirtualProtect(target as *mut _, JMP_SIZE, old_protect, &mut old_protect);

    // Calculate the address of the original function after the JMP instruction
    let trampoline_size = JMP_SIZE;
    let trampoline = VirtualAlloc(
        std::ptr::null_mut(),
        trampoline_size,
        MEM_COMMIT | MEM_RESERVE,
        PAGE_EXECUTE_READWRITE,
    );

    if trampoline.is_null() {
        panic!("Failed to allocate memory for trampoline");
    }

    std::ptr::copy_nonoverlapping(ORIGINAL_BYTES.as_ptr(), trampoline as *mut u8, JMP_SIZE);

    let jump_back_offset =
        (target.add(JMP_SIZE) as isize - trampoline as isize - JMP_SIZE as isize) as u32;
    let jump_back = [
        0xE9,
        (jump_back_offset & 0xFF) as u8,
        ((jump_back_offset >> 8) & 0xFF) as u8,
        ((jump_back_offset >> 16) & 0xFF) as u8,
        ((jump_back_offset >> 24) & 0xFF) as u8,
    ];

    std::ptr::copy_nonoverlapping(
        jump_back.as_ptr(),
        trampoline.add(JMP_SIZE) as *mut u8,
        jump_back.len(),
    );

    // Save the original function pointer, adjusted to skip the JMP instruction
    ORIGINAL_FUNCTION = Some(std::mem::transmute::<*mut c_void, ProcessEvent>(trampoline));
}

/// Windows DLL entrypoint for the plugin
#[no_mangle]
extern "stdcall" fn DllMain(_hmodule: isize, reason: u32, _: *mut ()) -> bool {
    if let DLL_PROCESS_ATTACH = reason {
        unsafe {
            AllocConsole();
        }

        unsafe { MESSAGES = Some(File::create("event-dump.txt").unwrap()) }

        unsafe {
            hook_function_address(
                0x00453120 as *const u8 as *mut u8,
                fake_process_event as *const u8,
            );
        }
    } else if let DLL_PROCESS_DETACH = reason {
        unsafe {
            FreeConsole();
        }
    }

    true
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

    process_event(object, func, params, result);
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
