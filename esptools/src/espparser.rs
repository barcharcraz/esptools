// SPDX-FileCopyrightText: Charles Barto
// SPDX-License-Identifier: GPL-2.0-only OR GPL-3.0-only
use std::io::Read;

use crate::common::*;
use crate::records::*;

pub trait ParseEsp: Sized {
    fn parse(input: &mut impl Read) -> Result<Self>;
}

trait EspReadExt: Read + Sized {
    fn parse<T: ParseEsp>(&mut self) -> Result<T> {
        <T as ParseEsp>::parse(self)
    }
}

impl ParseEsp for RecordHeader {
    fn parse(input: &mut impl Read) -> Result<Self> {
        let result = Self {
            typ: input.parse()?,
            data_size: u32::from_le_bytes(input.parse()?),
            flags: u32::from_le_bytes(input.parse()?),
            form_id: u32::from_le_bytes(input.parse()?),
            timestamp: u16::from_le_bytes(input.parse()?),
            vcs_info: u16::from_le_bytes(input.parse()?),
            internal_version: u16::from_le_bytes(input.parse()?)
        };
        input.skip(2)?;
        Ok(result)
    }
}

impl ParseEsp for GroupHeader {
    fn parse(input: &mut impl Read) -> Result<Self> {
        let result = Self {
            typ: input.parse()?,
            group_size: u32::from_le_bytes(input.parse()?),
            label: input.parse()?,
            group_type: u32::from_le_bytes(input.parse()?),
            timestamp: u16::from_le_bytes(input.parse()?),
            vcs_info: u16::from_le_bytes(input.parse()?)
        };
        input.skip(4)?; // unknown padding
        Ok(result)
    }
}
