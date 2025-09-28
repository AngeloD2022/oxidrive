// reverse engineered from HDPIM.dylib @ UnzipHandler::unzipFile

use lzma_sdk_sys::{
    Allocator, Byte, CLzma2Dec, ELzmaFinishMode, ELzmaStatus, Lzma2Dec_AllocateProbs,
    Lzma2Dec_DecodeToDic, Lzma2Dec_Init, SizeT, SZ_OK,
};
use std::mem::MaybeUninit;

pub struct HyperdriveLZMA2 {
    dec: CLzma2Dec,
    alloc: Allocator,
    current_prop: u8,
}

impl HyperdriveLZMA2 {
    pub fn new() -> Result<Self, i32> {
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

            Ok(Self {
                dec,
                alloc,
                current_prop: prop,
            })
        }
    }

    fn reallocate_probs(&mut self, prop: u8) -> Result<(), i32> {
        unsafe {
            let res = Lzma2Dec_AllocateProbs(&mut self.dec, prop, self.alloc.as_ref());
            if res != SZ_OK as i32 {
                return Err(res);
            }
        }
        self.current_prop = prop;
        Ok(())
    }

    pub fn decompress(&mut self, item: &[u8], lim_value: u64) -> Result<Vec<u8>, ()> {
        // if the prop value is different, reallocate our internal dictionary.
        let prop = item[0];
        if prop != self.current_prop {
            self.reallocate_probs(prop).unwrap();
        }

        const BLOCK_SIZE: u64 = 0x10000;
        let dict_limit = lim_value + BLOCK_SIZE;

        let mut dict = vec![0u8; dict_limit as usize];
        let mut result: Vec<u8> = Vec::new();

        self.dec.decoder.dic = dict.as_mut_ptr();
        self.dec.decoder.dicBufSize = dict_limit as SizeT;
        self.dec.decoder.dicPos = 0;

        unsafe { Lzma2Dec_Init(&mut self.dec) };

        unsafe {
            for chunk in item[1..].chunks(BLOCK_SIZE as usize) {
                let mut src_chunk_size = chunk.len();
                let mut status = ELzmaStatus::LZMA_STATUS_NOT_SPECIFIED;

                let dict_size_pre = self.dec.decoder.dicBufSize;

                let res = Lzma2Dec_DecodeToDic(
                    &mut self.dec,
                    dict_limit as SizeT,
                    chunk.as_ptr() as *const Byte,
                    &mut src_chunk_size,
                    ELzmaFinishMode::LZMA_FINISH_END,
                    &mut status,
                );

                if res != SZ_OK as i32 {
                    let dict_size_post = self.dec.decoder.dicBufSize;

                    if !((status != ELzmaStatus::LZMA_STATUS_MAYBE_FINISHED_WITHOUT_MARK
                        || dict_size_post == self.dec.decoder.buf.addr())
                        && (src_chunk_size > 0 || dict_size_pre != dict_size_post))
                    {
                        return Err(());
                    }
                }
            }
        }

        let decompressed = self.dec.decoder.dicPos;
        result.extend(&dict[..decompressed]);

        Ok(result)
    }
}

mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn hyperdrive_lzma2() {
        let file1 = fs::read("/Users/angelodeluca/Downloads/Install PHSP_26.10-en_US-macuniversal.app/Contents/Resources/products/PHSP/ext/AdobePhotoshop26-Core.pimx").unwrap();
        let file2 = fs::read("/Users/angelodeluca/Downloads/Install PHSP_26.10-en_US-macuniversal.app/Contents/Resources/products/PHSP/ext/1/Common Files/Adobe/HelpCfg/en_US/Photoshop_21.0.helpcfg").unwrap();

        let mut hd = HyperdriveLZMA2::new().unwrap();
        let fsize = 5_000_000;
        let result1 = hd.decompress(&file1, fsize).unwrap();
        let result2 = hd.decompress(&file2, fsize).unwrap();
        println!("{}", String::from_utf8(result1).unwrap());
        println!("{}", String::from_utf8(result2).unwrap());
    }
}
