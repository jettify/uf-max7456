//! MAX7456/AT7456E register map

pub const READ: u8 = 0x80;
pub const END_STRING: u8 = 0xff;

pub const VM0: u8 = 0x00;
pub const VM1: u8 = 0x01;
pub const HOS: u8 = 0x02;
pub const VOS: u8 = 0x03;
pub const DMM: u8 = 0x04;
pub const DMAH: u8 = 0x05;
pub const DMAL: u8 = 0x06;
pub const DMDI: u8 = 0x07;
pub const CMM: u8 = 0x08;
pub const CMAH: u8 = 0x09;
pub const CMAL: u8 = 0x0a;
pub const CMDI: u8 = 0x0b;
pub const OSDM: u8 = 0x0c;

pub const RB0: u8 = 0x10;
pub const RB1: u8 = 0x11;
pub const RB2: u8 = 0x12;
pub const RB3: u8 = 0x13;
pub const RB4: u8 = 0x14;
pub const RB5: u8 = 0x15;
pub const RB6: u8 = 0x16;
pub const RB7: u8 = 0x17;
pub const RB8: u8 = 0x18;
pub const RB9: u8 = 0x19;
pub const RB10: u8 = 0x1a;
pub const RB11: u8 = 0x1b;
pub const RB12: u8 = 0x1c;
pub const RB13: u8 = 0x1d;
pub const RB14: u8 = 0x1e;
pub const RB15: u8 = 0x1f;

pub const OSDBL: u8 = 0x6c;
pub const STAT: u8 = 0xa0;

pub const VM0_RESET: u8 = 0x02;
pub const OSDM_DEFAULT: u8 = 0x1b;
pub const CMAL_CA8_BIT: u8 = 1 << 6;
