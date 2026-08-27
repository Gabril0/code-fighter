const CRC_POLYNOMIAL: u32 = 0xEDB8_8320;

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (CRC_POLYNOMIAL & mask);
        }
    }
    !crc
}

struct Entry {
    name: String,
    body: Vec<u8>,
    offset: u32,
    crc: u32,
}

#[derive(Default)]
pub struct ZipBuilder {
    out: Vec<u8>,
    entries: Vec<Entry>,
}

impl ZipBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, name: impl Into<String>, body: impl Into<Vec<u8>>) {
        let name = name.into();
        let body = body.into();
        let crc = crc32(&body);
        let offset = self.out.len() as u32;

        self.out.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        self.out.extend_from_slice(&20_u16.to_le_bytes());
        self.out.extend_from_slice(&0_u16.to_le_bytes());
        self.out.extend_from_slice(&0_u16.to_le_bytes());
        self.out.extend_from_slice(&0_u16.to_le_bytes());
        self.out.extend_from_slice(&0_u16.to_le_bytes());
        self.out.extend_from_slice(&crc.to_le_bytes());
        self.out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        self.out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        self.out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        self.out.extend_from_slice(&0_u16.to_le_bytes());
        self.out.extend_from_slice(name.as_bytes());
        self.out.extend_from_slice(&body);

        self.entries.push(Entry { name, body, offset, crc });
    }

    pub fn finish(mut self) -> Vec<u8> {
        let directory_start = self.out.len() as u32;

        for entry in &self.entries {
            self.out.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
            self.out.extend_from_slice(&20_u16.to_le_bytes());
            self.out.extend_from_slice(&20_u16.to_le_bytes());
            self.out.extend_from_slice(&0_u16.to_le_bytes());
            self.out.extend_from_slice(&0_u16.to_le_bytes());
            self.out.extend_from_slice(&0_u16.to_le_bytes());
            self.out.extend_from_slice(&0_u16.to_le_bytes());
            self.out.extend_from_slice(&entry.crc.to_le_bytes());
            self.out.extend_from_slice(&(entry.body.len() as u32).to_le_bytes());
            self.out.extend_from_slice(&(entry.body.len() as u32).to_le_bytes());
            self.out.extend_from_slice(&(entry.name.len() as u16).to_le_bytes());
            self.out.extend_from_slice(&0_u16.to_le_bytes());
            self.out.extend_from_slice(&0_u16.to_le_bytes());
            self.out.extend_from_slice(&0_u16.to_le_bytes());
            self.out.extend_from_slice(&0_u16.to_le_bytes());
            self.out.extend_from_slice(&0_u32.to_le_bytes());
            self.out.extend_from_slice(&entry.offset.to_le_bytes());
            self.out.extend_from_slice(entry.name.as_bytes());
        }

        let directory_size = self.out.len() as u32 - directory_start;
        let count = self.entries.len() as u16;

        self.out.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
        self.out.extend_from_slice(&0_u16.to_le_bytes());
        self.out.extend_from_slice(&0_u16.to_le_bytes());
        self.out.extend_from_slice(&count.to_le_bytes());
        self.out.extend_from_slice(&count.to_le_bytes());
        self.out.extend_from_slice(&directory_size.to_le_bytes());
        self.out.extend_from_slice(&directory_start.to_le_bytes());
        self.out.extend_from_slice(&0_u16.to_le_bytes());

        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_the_reference_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn writes_a_readable_archive_header() {
        let mut zip = ZipBuilder::new();
        zip.add("a.txt", "hello");
        let bytes = zip.finish();
        assert_eq!(&bytes[0..4], &0x0403_4b50_u32.to_le_bytes());
        assert!(bytes.windows(5).any(|window| window == b"hello"));
        assert!(bytes.len() > 60);
    }
}
