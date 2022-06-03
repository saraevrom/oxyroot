use crate::rbase::consts::{kIsOnHeap, kIsReferenced};
use crate::rbytes::rbuffer::RBuffer;
use crate::rbytes::Unmarshaler;
use log::trace;

#[derive(Debug)]
pub struct Object {
    id: u32,
    bits: u32,
}

impl Object {
    fn test_bits(&self, bits: u32) -> bool {
        self.bits & bits != 0
    }
}

impl Default for Object {
    fn default() -> Self {
        Object {
            id: 0x0,
            bits: 0x3000000,
        }
    }
}

impl Unmarshaler for Object {
    fn unmarshal(&mut self, r: &mut RBuffer) -> anyhow::Result<()> {
        r.skip_version("")?;
        self.id = r.read_u32()?;
        self.bits = r.read_u32()?;

        trace!("obj = {:?}", self);

        self.bits = self.bits | kIsOnHeap;

        if self.test_bits(kIsReferenced) {
            r.read_u16()?;
        }

        Ok(())
    }
}
