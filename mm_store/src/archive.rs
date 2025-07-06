use mm_archive::traits::CompressionMethod;
use serde::{Serialize, Deserialize};
use zvariant::Type;
use crate::Checksum;


#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ThinArchiveEntry {
    offset: u64,
    compressed_file: Checksum,
    uncompressed_file: Checksum,
    compression_method: CompressionMethod,
    compression_level: i8
}
