use crate::util::{decode_hex, WriteSeek};
use flate2::{write::GzEncoder, Compression};
use std::{
    error::Error,
    fs::{self, File},
    io::{stdout, BufRead, BufReader, Read, Seek, Write},
    path::PathBuf,
};

// Size of ttbv1 header.
const HEADER_SIZE: u64 = 66u64;

#[derive(Debug)]
struct IndexEntry {
    path: PathBuf,
    hash: String,
    start: u64,
    length: u32,
}

impl ToString for IndexEntry {
    fn to_string(&self) -> String {
        format!(
            "{} {} {} {}",
            self.start,
            self.length,
            self.path.to_str().unwrap(),
            self.hash
        )
    }
}

pub struct BundleV1 {
    index: Vec<IndexEntry>,
    target: Box<dyn WriteSeek>,
    content_dir: PathBuf,

    index_start: u64,
    index_len: u32,
}

impl BundleV1 {
    pub fn make(target: Box<dyn WriteSeek>, content_dir: PathBuf) -> Result<(), Box<dyn Error>> {
        let mut bundle = BundleV1::new(target, content_dir)?;

        bundle.add_files()?;
        bundle.write_index()?;
        bundle.write_header()?;

        return Ok(());
    }

    fn new(target: Box<dyn WriteSeek>, content_dir: PathBuf) -> Result<BundleV1, Box<dyn Error>> {
        return Ok(BundleV1 {
            index: Vec::new(),
            target,
            content_dir: content_dir.to_owned(),
            index_start: 0,
            index_len: 0,
        });
    }

    fn add_files(&mut self) -> Result<u64, Box<dyn Error>> {
        let mut byte_count = HEADER_SIZE; // Size of header
        self.target
            .seek(std::io::SeekFrom::Start(byte_count.into()))?;

        let index_file = File::open(self.content_dir.join("INDEX")).unwrap();
        let reader = BufReader::new(index_file);

        let mut count = 0usize;

        for line in reader.lines() {
            count += 1;
            print!("\rBuilding V1 Bundle... {}", count);
            stdout().flush()?;

            let line = line?;
            let mut bits = line.split_whitespace();

            if let (Some(path), Some(hash)) = (bits.next(), bits.next()) {
                let path = path.to_owned();
                let hash = hash.to_owned();

                let mut file = fs::File::open(&self.content_dir.join(&path[1..])).unwrap();

                // Compress and write bytes
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                std::io::copy(&mut file, &mut encoder)?;
                let len = self.target.write(&encoder.finish()?)?;
                assert!(len < u32::MAX as usize);

                // Add to index
                self.index.push(IndexEntry {
                    start: byte_count,
                    length: len as u32,
                    path: PathBuf::from(path),
                    hash,
                });
                byte_count += len as u64;
            } else {
                panic!("malformed index line");
            }
        }

        println!("\rBuilding V1 Bundle... {}  Done.", count);
        return Ok(byte_count);
    }

    fn write_index(&mut self) -> Result<(), Box<dyn Error>> {
        // Generate a ttbv1 index and write it to the bundle.
        // This is NOT the same as the INDEX file (which is also included in this bundle)
        //
        // This index is a replacement for INDEX, containing everything in that file
        // in addition to the start and length of each file.
        // The original INDEX is still included in the bundle for consistency.

        // Get current position
        self.index_start = self.target.seek(std::io::SeekFrom::Current(0))?;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());

        for i in &self.index {
            let s = format!("{}\n", i.to_string());
            encoder.write_all(s.as_bytes())?;
        }

        let len = self.target.write(&encoder.finish()?)?;
        assert!(len < u32::MAX as usize);
        self.index_len = len as u32;

        return Ok(());
    }

    fn write_header(&mut self) -> Result<u64, Box<dyn Error>> {
        self.target.seek(std::io::SeekFrom::Start(0))?;

        // Parse bundle hash
        let mut index_file = File::open(self.content_dir.join("SHA256SUM")).unwrap();
        let mut text = String::new();
        index_file.read_to_string(&mut text)?;
        let digest = decode_hex(text.trim())?;

        let mut byte_count = 0u64;

        // 14 bytes: signature
        // Always "tectonicbundle", in any bundle version.
        //
        // This "magic sequence" lets us more easily distinguish between
        // random binary files and proper tectonic bundles.
        byte_count += self.target.write(b"tectonicbundle")? as u64;

        // 8 bytes: bundle version
        byte_count += self.target.write(&1u64.to_le_bytes())? as u64;

        // 8 + 4 = 12 bytes: location of index
        byte_count += self.target.write(&self.index_start.to_le_bytes())? as u64;
        byte_count += self.target.write(&self.index_len.to_le_bytes())? as u64;

        // 32 bytes: bundle hash
        // We include this in the header so we don't need to load the index to get the hash.
        byte_count += self.target.write(&digest)? as u64;

        // Make sure we wrote the expected number of bytes
        assert!(byte_count == HEADER_SIZE);

        return Ok(byte_count);
    }
}
