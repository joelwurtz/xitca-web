#[cfg(any(feature = "compress-br", feature = "compress-gz", feature = "compress-de"))]
pub mod compress;
#[cfg(any(feature = "compress-br", feature = "compress-gz", feature = "compress-de"))]
pub mod decompress;