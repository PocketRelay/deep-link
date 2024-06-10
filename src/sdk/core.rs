static GAME_OBJECT_OFFSET: u32 = 0x01AB5634;

pub fn game_objects_ptr() -> *mut TArray<UObject> {
    GAME_OBJECT_OFFSET as *mut TArray<UObject>
}

#[repr(C)]
pub struct TArray<T> {
    pub data: *mut T,
    pub count: std::os::raw::c_int,
    pub max: std::os::raw::c_int,
    pub _phantom_0: ::std::marker::PhantomData<::std::cell::UnsafeCell<T>>,
}

impl<T> TArray<T> {
    fn get(&self, index: std::os::raw::c_int) -> Option<*mut T> {
        if index >= self.count {
            return None;
        }

        Some(unsafe { self.data.add(index as usize) })
    }
}

#[repr(C)]
pub struct UObjectVTable(std::os::raw::c_void);

#[repr(C, packed(4))]
pub struct UObject {
    pub vtable_: *const UObjectVTable,
    pub object_internal_integer: std::os::raw::c_int,
    pub object_flags: FQWord,
    pub hash_next: FPointer,
    pub hash_outer_next: FPointer,
    pub state_frame: FPointer,
    pub linker: *mut UObject,
    pub linker_index: FPointer,
    pub net_index: ::std::os::raw::c_int,
    pub outer: *mut UObject,
    pub name: FName,
    pub class: *mut UClass,
    pub object_archetype: *mut UObject,
}

#[repr(C)]
pub struct FQWord {
    pub a: ::std::os::raw::c_int,
    pub b: ::std::os::raw::c_int,
}

#[repr(C)]
pub struct FPointer {
    pub dummy: ::std::os::raw::c_int,
}

#[repr(C)]
pub struct FName {
    pub name_entry: *mut FNameEntry,
    pub name_index: std::os::raw::c_uint,
}

#[repr(C)]
pub struct FNameEntry {
    pub unknown_data00: [::std::os::raw::c_uchar; 8usize],
    pub name: [::std::os::raw::c_char; 16usize],
}

#[repr(C)]
pub struct UClass {
    pub _base: UState,
    pub unknown_data00: [::std::os::raw::c_uchar; 188usize],
}

#[repr(C)]
pub struct UState {
    pub _base: UStruct,
    pub unknown_data00: [::std::os::raw::c_uchar; 36usize],
}

#[repr(C)]
pub struct UStruct {
    pub _base: UField,
    pub unknown_data00: [::std::os::raw::c_uchar; 64usize],
}

#[repr(C, packed(4))]
pub struct UField {
    pub _base: UObject,
    pub super_field: *mut UField,
    pub next: *mut UField,
}

#[repr(C, packed(4))]
pub struct UFunction {
    pub _base: UStruct,
    pub func: *mut ::std::os::raw::c_void,
    pub function_flags: ::std::os::raw::c_ulong,
    pub i_native: ::std::os::raw::c_ushort,
    pub unknown_data00: [::std::os::raw::c_uchar; 8usize],
}
