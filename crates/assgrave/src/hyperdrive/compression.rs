
// reverse engineered from HDPIM.dylib @ UnzipHandler::unzipFile

use std::mem::MaybeUninit;
use lzma_sdk_sys::{Allocator, CLzma2Dec, Lzma2Dec_Allocate, Lzma2Dec_AllocateProbs, Lzma2Dec_Init, SZ_OK};


pub struct HyperdriveLZMA2 {
    dec: CLzma2Dec,
    alloc: Allocator,
}

impl HyperdriveLZMA2 {

    pub fn new() -> Result<Self, i32>{
        unsafe {
            let prop: u8 = 0x18;
            let alloc = Allocator::default();

            // property value comes from the first byte of each LZMA2-compressed file
            let mut dec: CLzma2Dec = MaybeUninit::zeroed().assume_init();

            // allocate internal buffers
            let res = Lzma2Dec_AllocateProbs(&mut dec, prop, alloc.as_ref());
            if res != SZ_OK as i32 {
                return Err(res);
            }

            Lzma2Dec_Init(&mut dec);
            Ok(Self { dec, alloc })
        }
    }
}
