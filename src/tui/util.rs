use std::fmt::Write as _;

/// Represents the unit of a value.
pub(crate) enum Unit {
    Bytes,
    Kilobytes,
    Megabytes,
    Gigabytes,
}

impl Unit {
    const KILOBYTE: u64 = 1000;
    const MEGABYTE: u64 = Self::KILOBYTE * 1000;
    const GIGABYTE: u64 = Self::MEGABYTE * 1000;

    pub fn human_readable(&self) -> &'static str {
        match self {
            Unit::Bytes => "B",
            Unit::Kilobytes => "kB",
            Unit::Megabytes => "MB",
            Unit::Gigabytes => "GB",
        }
    }
}

/// Converts the passed number of bytes to a human-readable representation using any suitable [Unit].
pub(crate) fn bytes_to_human_readable_units(bytes: impl Into<u64>) -> (f64, Unit) {
    let bytes = bytes.into();
    match bytes {
        0..Unit::KILOBYTE => (bytes as f64, Unit::Bytes),
        Unit::KILOBYTE..Unit::MEGABYTE => ((bytes as f64) / (Unit::KILOBYTE as f64), Unit::Kilobytes),
        Unit::MEGABYTE..Unit::GIGABYTE => ((bytes as f64) / (Unit::MEGABYTE as f64), Unit::Megabytes),
        Unit::GIGABYTE.. => ((bytes as f64) / (Unit::GIGABYTE as f64), Unit::Gigabytes),
    }
}

pub(crate) fn encode_hex(digest: impl AsRef<[u8]>) -> String {
    let mut s = String::with_capacity(digest.as_ref().len() * 2);
    for &b in digest.as_ref().iter() {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}
