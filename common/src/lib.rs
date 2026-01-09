#![no_std]

mod registers;
pub mod shared;
pub mod protocol;

pub use shared::SharedData;
pub use protocol::Hcp2Protocol;

use core::ffi::c_void;

#[no_mangle]
pub extern "C" fn hcp2_protocol_size() -> usize {
    core::mem::size_of::<Hcp2Protocol>()
}

#[no_mangle]
pub extern "C" fn hcp2_protocol_init(ptr: *mut c_void) {
    let proto = unsafe { &mut *(ptr as *mut Hcp2Protocol) };
    *proto = Hcp2Protocol::new();
}

#[no_mangle]
pub extern "C" fn hcp2_protocol_dispatch(
    ptr: *mut c_void,
    frame_ptr: *const u8,
    frame_len: usize,
    out_ptr: *mut u8,
    out_cap: usize,
    shared: *mut SharedData,
    millis: u32
) -> usize {
    let proto = unsafe { &mut *(ptr as *mut Hcp2Protocol) };
    let frame = unsafe { core::slice::from_raw_parts(frame_ptr, frame_len) };
    let out_buf = unsafe { core::slice::from_raw_parts_mut(out_ptr, out_cap) };
    let shared_data = unsafe { &mut *shared };
    
    proto.dispatch_frame(frame, out_buf, shared_data, millis)
}
