use std::{
    ffi::{CStr, CString},
    marker::PhantomData,
    os::raw::{c_char, c_int, c_uchar, c_uint, c_ulong, c_ushort, c_void},
};

/// Static memory address for the game objects
static GAME_OBJECT_OFFSET: u32 = 0x01AB5634;

/// Obtains a pointer to the [TArray] containing the game objects
pub fn game_objects_ptr() -> *mut TArray<UObject> {
    GAME_OBJECT_OFFSET as *mut TArray<UObject>
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
