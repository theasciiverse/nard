//! The C interface of bgsage's `cpp/include/bgbot/mobile.h`, by hand.

use std::ffi::{c_char, c_int};

#[repr(C)]
pub struct BgsageMobileEngine {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct BgsageMobileRequest {
    pub board: [c_int; 26],
    pub die1: c_int,
    pub die2: c_int,
    pub ply: c_int,
    pub cube_value: c_int,
    /// 0 centred, 1 player, 2 opponent.
    pub cube_owner: c_int,
    pub away1: c_int,
    pub away2: c_int,
    pub is_crawford: c_int,
    pub jacoby: c_int,
    pub beaver: c_int,
    pub budget_ms: c_int,
}

pub const OP_CHECKER: c_int = 0;
pub const OP_CUBE: c_int = 1;

unsafe extern "C" {
    pub fn bgsage_mobile_create(
        strategy_type: *const c_char,
        paths: *const *const c_char,
        hidden_sizes: *const c_int,
        count: c_int,
        bearoff_path: *const c_char,
    ) -> *mut BgsageMobileEngine;
    pub fn bgsage_mobile_destroy(engine: *mut BgsageMobileEngine);
    pub fn bgsage_mobile_analyze(
        engine: *mut BgsageMobileEngine,
        operation: c_int,
        request: *const BgsageMobileRequest,
    ) -> *mut c_char;
    pub fn bgsage_mobile_free(text: *mut c_char);
    pub fn bgsage_mobile_last_error() -> *const c_char;
}
