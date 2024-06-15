use std::{
    ffi::{CStr, CString},
    marker::PhantomData,
    os::raw::{c_char, c_int, c_schar, c_uchar, c_uint, c_ulong, c_ushort, c_void},
    ptr::null_mut,
};

/// Static memory address for the game objects
static GAME_OBJECT_OFFSET: u32 = 0x01AB5634;

/// Obtains a pointer to the [TArray] containing the game objects
pub fn game_objects_ref() -> &'static TArray<UObject> {
    unsafe {
        (GAME_OBJECT_OFFSET as *const TArray<UObject>)
            .as_ref()
            .expect("Game objects pointer was null")
    }
}

/// Array type
#[repr(C)]
pub struct TArray<T> {
    /// Pointer to the data within the array
    data: *mut T,
    /// Number of items currently present
    count: c_int,
    /// Allocated capacity for underlying array memory
    capacity: c_int,
    /// Phantom type of the array generic type
    _type: PhantomData<::std::cell::UnsafeCell<T>>,
}

impl<T> TArray<T> {
    /// Gets a pointer to specific element by index
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len() {
            return None;
        }

        // Get a pointer to the data at the provided index
        let item = unsafe { self.data.add(index) };
        unsafe { item.as_ref() }
    }

    pub fn len(&self) -> usize {
        self.count as usize
    }

    pub fn capacity(&self) -> usize {
        self.capacity as usize
    }
}

impl<T> From<Vec<T>> for TArray<T> {
    fn from(value: Vec<T>) -> Self {
        let length = value.len() as c_int;
        let capacity = value.capacity() as c_int;
        let value = value.leak();

        let data = value.as_mut_ptr();

        Self {
            data,
            count: length,
            capacity,
            _type: PhantomData,
        }
    }
}

#[repr(C)]
pub struct FString(TArray<i16>);

impl FString {
    pub fn from_string(value: String) -> FString {
        let value = value
            .encode_utf16()
            .map(|value| value as i16)
            .collect::<Vec<_>>();
        FString(TArray::from(value))
    }
}

#[repr(C)]
pub struct UObjectVTable(c_void);

#[repr(C, packed(4))]
pub struct UObject {
    pub vtable_: *const UObjectVTable,
    pub object_internal_integer: c_int,
    pub object_flags: FQWord,
    pub hash_next: FPointer,
    pub hash_outer_next: FPointer,
    pub state_frame: FPointer,
    pub linker: *mut UObject,
    pub linker_index: FPointer,
    pub net_index: c_int,
    pub outer: *mut UObject,
    pub name: FName,
    pub class: *mut UClass,
    pub object_archetype: *mut UObject,
}

impl UObject {
    pub fn cast<T>(&self) -> *const T {
        self as *const UObject as *const T
    }

    /// Collects the full name of the object
    pub fn get_full_name(&self) -> String {
        match unsafe { (self.class.as_ref(), self.outer.as_ref()) } {
            (Some(class), Some(outer)) => {
                let class_name = class.get_name().to_str().expect("Class name invalid utf8");
                let outer_name = outer.get_name().to_str().expect("Class name invalid utf8");
                let this_name = self.get_name().to_str().expect("Class name invalid utf8");

                if let Some(outer) = unsafe { outer.outer.as_ref() } {
                    let outer_outer_name =
                        outer.get_name().to_str().expect("Class name invalid utf8");

                    format!(
                        "{} {}.{}.{}",
                        class_name, outer_outer_name, outer_name, this_name
                    )
                } else {
                    format!("{} {}.{}", class_name, outer_name, this_name)
                }
            }
            _ => "(null)".to_string(),
        }
    }

    pub fn get_name(&self) -> &CStr {
        self.name.get_name()
    }

    pub fn process_event(
        &self,
        function: *mut UFunction,
        params: *mut c_void,
        result: *mut c_void,
    ) {
        let fn_ptr = unsafe { self.vtable_.add(70) }.cast::<extern "C" fn(
            *mut UObject,
            *mut UFunction,
            *mut c_void,
            *mut c_void,
        )>();

        unsafe {
            (*fn_ptr)(
                self as *const UObject as *mut UObject,
                function,
                params,
                result,
            )
        }
    }
}

#[repr(C)]
pub struct FQWord {
    pub a: c_int,
    pub b: c_int,
}

#[repr(C)]
pub struct FPointer {
    pub dummy: c_int,
}

#[repr(C)]
pub struct FName {
    pub name_entry: *mut FNameEntry,
    pub name_index: c_uint,
}

impl FName {
    /// Gets the name from the entry, name is stored
    /// in the name char
    pub fn get_name(&self) -> &CStr {
        unsafe {
            self.name_entry
                .as_ref()
                .expect("Name entry pointer was null")
                .get_name()
        }
    }
}

// Name entry
#[repr(C)]
pub struct FNameEntry {
    // Unknown block of data
    pub unknown_data00: [c_uchar; 8usize],
    // Name array data
    pub name: [c_char; 16usize],
}

impl FNameEntry {
    /// Gets the name from the entry, name is stored
    /// in the name char
    pub fn get_name(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.name.as_ptr()) }
    }
}

#[repr(C)]
pub struct UClass {
    pub _base: UState,
    pub unknown_data00: [c_uchar; 188usize],
}

impl UClass {
    pub fn get_name(&self) -> &CStr {
        self._base.get_name()
    }

    pub fn as_object_ref(&self) -> &UObject {
        self._base.as_object_ref()
    }
}

#[repr(C)]
pub struct UState {
    pub _base: UStruct,
    pub unknown_data00: [c_uchar; 36usize],
}

impl UState {
    pub fn get_name(&self) -> &CStr {
        self._base.get_name()
    }

    pub fn as_object_ref(&self) -> &UObject {
        self._base.as_object_ref()
    }
}

#[repr(C)]
pub struct UStruct {
    pub _base: UField,
    pub unknown_data00: [c_uchar; 64usize],
}

impl UStruct {
    pub fn get_name(&self) -> &CStr {
        self._base.get_name()
    }

    pub fn as_object_ref(&self) -> &UObject {
        self._base.as_object_ref()
    }
}

#[repr(C, packed(4))]
pub struct UField {
    pub _base: UObject,
    pub super_field: *mut UField,
    pub next: *mut UField,
}

impl UField {
    pub fn get_name(&self) -> &CStr {
        self._base.get_name()
    }

    pub fn as_object_ref(&self) -> &UObject {
        &self._base
    }
}

#[repr(C, packed(4))]
pub struct UFunction {
    pub _base: UStruct,
    pub func: *mut c_void,
    pub function_flags: c_ulong,
    pub i_native: c_ushort,
    pub unknown_data00: [c_uchar; 8usize],
}

impl UFunction {
    pub fn get_name(&self) -> &CStr {
        self._base.get_name()
    }

    pub fn as_object_ref(&self) -> &UObject {
        self._base.as_object_ref()
    }
}

// Class SFXGame.SFXGUI_MainMenu_RightComputer
// 0x0064 (0x00A0 - 0x003C)
#[repr(C, packed(4))]
pub struct USFXGUI_MainMenu_RightComputer {
    _base: UObject,
}

impl USFXGUI_MainMenu_RightComputer {
    pub fn as_object_ref(&self) -> &UObject {
        &self._base
    }
}

pub fn add_ticker_message(
    this: *mut USFXGUI_MainMenu_RightComputer,
    ty: c_uchar,
    message: FString,
    dlc_id: c_int,
    server_id: c_int,
) {
    let game_objects = game_objects_ref();
    let func_object: *const UFunction = game_objects
        .get(61401)
        .expect("Missing ticker message function")
        .cast();

    let params = USFXGUI_MainMenu_RightComputer_execAddTickerMessage_Parms {
        ty,
        message,
        dlc_id,
        server_id,
    };

    unsafe { this.read() }.as_object_ref().process_event(
        func_object as *mut UFunction,
        &params as *const _ as *mut c_void,
        null_mut(),
    )
}

struct USFXGUI_MainMenu_RightComputer_execAddTickerMessage_Parms {
    ty: c_uchar,      // 0x0000 (0x0001) [0x0000000000000080]              ( CPF_Parm )
    message: FString, // 0x0004 (0x000C) [0x0000000000400080]              ( CPF_Parm | CPF_NeedCtorLink )
    dlc_id: c_int,    // 0x0010 (0x0004) [0x0000000000000080]              ( CPF_Parm )
    server_id: c_int, // 0x0014 (0x0004) [0x0000000000000080]              ( CPF_Parm )
                      // class USFXGUI_MainMenu_Message_Text*            NewMessage;                                       		// 0x0018 (0x0004) [0x0000000000000000]
                      // class USFXGUI_MainMenu_Message_Text*            ExistingMessage;                                  		// 0x001C (0x0004) [0x0000000000000000]
}
