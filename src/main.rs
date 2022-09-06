use core_foundation::base::{kCFAllocatorDefault, mach_port_t, CFAllocatorRef, CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{
    CFDictionary, CFDictionaryRef, CFMutableDictionary, CFMutableDictionaryRef,
};
use core_foundation::string::CFString;
use libc::c_char;
use mach::{kern_return, port};
use native_dialog::{MessageDialog, MessageType};
use std::{mem, thread, time};

#[derive(Debug)]
pub enum PowerStatus {
    Plugged,
    Unplugged,
}

fn show_alert() {
    MessageDialog::new()
        .set_type(MessageType::Info)
        .set_title("Alerta alerta!")
        .set_text("You just unplugged your compooter. This is most likely an error. Please plug me back in as I have a horrible battery life and will shut down without a warning soon")
        .show_alert()
        .unwrap();
}

pub struct IOMasterPortInterface(mach_port_t);
pub type IOResult<T> = Result<T, String>; // change later maybe

impl IOMasterPortInterface {
    pub fn new() -> IOResult<IOMasterPortInterface> {
        let mut master_port: mach_port_t = port::MACH_PORT_NULL; // default MACH_PORT_NULL is 0

        unsafe {
            // TODO: Handle the possible error
            let _result = IOMasterPort(kIOMasterPortDefault, &mut master_port);
        }

        Ok(IOMasterPortInterface(master_port))
    }
}

fn main() {
    let one_sec = time::Duration::from_millis(1000);

    loop {
        thread::sleep(one_sec);

        let state = get_curr_state();

        dbg!(state);
    }
}

// todo: figure out which parts we can allocate and reuse
fn get_curr_state() -> PowerStatus {
    let interface = IOMasterPortInterface::new().expect("Could not create IOMasterPortInterface");

    unsafe {
        let match_dict = IOServiceMatching(b"IOPMPowerSource\0".as_ptr() as *const c_char);
        let mut iterator: IoIteratorT = mem::uninitialized();

        // TODO: Handle the possible error
        let _result = IOServiceGetMatchingServices(interface.0, match_dict, &mut iterator);

        let battery_obj = IOIteratorNext(iterator); // refering to some in-kernel object per id

        let mut props: CFMutableDictionaryRef = mem::uninitialized();

        let _registry_result =
            IORegistryEntryCreateCFProperties(battery_obj, &mut props, kCFAllocatorDefault, 0);

        let properties: CFDictionary<CFString, CFType> =
            CFMutableDictionary::wrap_under_create_rule(props).to_immutable();

        get_battery_external_source_state(&properties)
    }

    // TODO: deallocate mach port mach_port_deallocate(mach_task_self(), master_port);
}

/// https://developer.apple.com/documentation/kernel/iopmpowersource
/// ExternalConnected
/// Type: bool
/// IORegistry Key: kIOPMPSExternalConnectedKey
/// True if computer is drawing external power
pub fn get_battery_external_source_state(
    properties: &CFDictionary<CFString, CFType>,
) -> PowerStatus {
    let cfstr_external_source_connected = CFString::from_static_string("ExternalConnected");

    let is_charging: bool = properties
        .find(&cfstr_external_source_connected)
        .and_then(|state_cftype| state_cftype.downcast::<CFBoolean>())
        .map(Into::into)
        .expect(&format!(
            "Unable to find key {} in CFMutableDictionary::IOPMPowerSource",
            cfstr_external_source_connected.to_string()
        ));

    if is_charging {
        PowerStatus::Plugged
    } else {
        PowerStatus::Unplugged
    }
}

type IoObjectT = mach_port_t;
type IoIteratorT = IoObjectT;
pub type IoRegistryEntryT = IoObjectT;
pub type IOOptionBits = u32;

extern "C" {
    pub static kIOMasterPortDefault: mach_port_t;

    pub fn IOMasterPort(
        bootstrapPort: mach_port_t,
        masterPort: *mut mach_port_t,
    ) -> kern_return::kern_return_t;

    pub fn IOServiceMatching(name: *const c_char) -> CFMutableDictionaryRef;

    pub fn IOServiceGetMatchingServices(
        masterPort: mach_port_t,
        matching: CFDictionaryRef,
        existing: *mut IoIteratorT,
    ) -> kern_return::kern_return_t;

    pub fn IOIteratorNext(iterator: IoIteratorT) -> IoObjectT;

    pub fn IORegistryEntryCreateCFProperties(
        entry: IoRegistryEntryT,
        properties: *mut CFMutableDictionaryRef,
        allocator: CFAllocatorRef,
        options: IOOptionBits,
    ) -> kern_return::kern_return_t;
}
