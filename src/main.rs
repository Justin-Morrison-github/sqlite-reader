mod cli;
use clap::Parser;
use cli::Args;
use core::fmt;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;

const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

#[derive(Debug)]
pub struct SqliteHeader {
    // magic: [u8; 16],
    page_size: u16,
    // format_write_version: u8,
    // format_read_version: u8,
    // reseved_space: u8,
    // max_payload_frac: u8,
    // min_payload_frac: u8,
    // leaf_payload_frac: u8,
    file_change_count: u32,
    size_in_pages: u32,
    // first_freelist_trunk_page_num: u32,
    // num_freelist_pages: u32,
    // schema_cookie: u32,
    schema_format: u32,
    // suggested_cache_size: u32,
    // largest_b_tree_root_page_num: u32,
    text_encoding: DBTextEncoding,
    // user_version: u32,
    // incremental_vacuum_enabled: bool,
    // application_id: u32,
    // reserved: [u8; 20],
    // valid_for_num: u32,
    sqlite_version_num: u32,
}

impl fmt::Display for SqliteHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // let magic = String::from_utf8_lossy(&self.magic);

        writeln!(f, "SQLite Header {{")?;
        // writeln!(f, "  magic:             {:?}", magic)?;
        writeln!(f, "  page_size:               {}", self.page_size)?;
        // writeln!(
        //     f,
        //     "  write_version:           {:?}",
        //     self.format_write_version
        // )?;
        // writeln!(
        //     f,
        //     "  read_version:            {:?}",
        //     self.format_read_version
        // )?;
        // writeln!(f, "  reserved_space:          {}", self.reseved_space)?;
        // writeln!(f, "  max_payload_frac:        {}", self.max_payload_frac)?;
        // writeln!(f, "  min_payload_frac:        {}", self.min_payload_frac)?;
        // writeln!(f, "  leaf_payload_frac:       {}", self.leaf_payload_frac)?;
        writeln!(f, "  file_change_count:       {}", self.file_change_count)?;
        writeln!(f, "  size_in_pages:           {}", self.size_in_pages)?;
        // writeln!(
        //     f,
        //     "  freelist_trunk_page:     {}",
        //     self.first_freelist_trunk_page_num
        // )?;
        // writeln!(f, "  freelist_pages:          {}", self.num_freelist_pages)?;
        // writeln!(f, "  schema_cookie:           {}", self.schema_cookie)?;
        writeln!(f, "  schema_format:           {}", self.schema_format)?;
        // writeln!(
        //     f,
        //     "  default_cache_size:      {}",
        //     self.suggested_cache_size
        // )?;
        // writeln!(
        //     f,
        //     "  largest_root_page:       {}",
        //     self.largest_b_tree_root_page_num
        // )?;
        writeln!(f, "  text_encoding:           {:?}", self.text_encoding)?;
        // writeln!(f, "  user_version:            {}", self.user_version)?;
        // writeln!(
        //     f,
        //     "  incremental_vacuum:      {}",
        //     self.incremental_vacuum_enabled
        // )?;
        // writeln!(f, "  application_id:          {}", self.application_id)?;
        // writeln!(f, "  valid_for:               {}", self.valid_for_num)?;
        writeln!(f, "  sqlite_version:          {}", self.sqlite_version_num)?;
        write!(f, "}}")
    }
}

#[derive(Debug)]
#[repr(u8)]

enum DBTextEncoding {
    Utf8 = 1,
    Utf16Le = 2,
    Utf16Be = 3,
}
impl TryFrom<u32> for DBTextEncoding {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DBTextEncoding::Utf8),
            2 => Ok(DBTextEncoding::Utf16Le),
            3 => Ok(DBTextEncoding::Utf16Be),
            _ => Err("Invalid text encoding value"),
        }
    }
}

fn read_u8<R: Read>(reader: &mut R) -> std::io::Result<u8> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(u8::from_be_bytes(buf))
}

fn read_u16<R: Read>(reader: &mut R) -> std::io::Result<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

fn read_u32<R: Read>(reader: &mut R) -> std::io::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn is_valid_page_size(page_size: u16) -> bool {
    page_size >= 512 && page_size <= 32768 && page_size.is_power_of_two()
}

fn read_header(file: &mut File) -> std::io::Result<SqliteHeader> {
    let magic_bytes = read_magic(file)?;
    let page_size = read_page_size(file)?;

    let format_write_version = read_u8(file)?;
    let format_read_version = read_u8(file)?;

    let reseved_space = read_u8(file)?;
    let max_payload_frac = read_max_payload_frac(file)?;
    let min_payload_frac = read_min_payload_frac(file)?;
    let leaf_payload_frac = read_leaf_payload_frac(file)?;

    let file_change_count = read_u32(file)?;
    let size_in_pages = read_u32(file)?;
    let first_freelist_trunk_page_num = read_u32(file)?;
    let num_freelist_pages = read_u32(file)?;
    let schema_cookie = read_u32(file)?;

    let schema_format = read_u32(file)?;
    if schema_format > 4 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid schema_format ({})", schema_format),
        ));
    }

    let suggested_cache_size = read_u32(file)?;

    let largest_b_tree_root_page_num = read_u32(file)?;

    let text_encoding_u32 = read_u32(file)?;
    let text_encoding = DBTextEncoding::try_from(text_encoding_u32)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let user_version = read_u32(file)?;

    let incremental_vacuum_enabled_u32 = read_u32(file)?;
    let incremental_vacuum_enabled = incremental_vacuum_enabled_u32 != 0;

    // If the integer at offset 52 is zero then the integer at offset 64 must also be zero.
    if largest_b_tree_root_page_num == 0 && incremental_vacuum_enabled_u32 != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Invalid vacumm settings If the integer at offset 52 is zero then the integer at offset 64 must also be zero. (52:{}, 64:{})",
                largest_b_tree_root_page_num, incremental_vacuum_enabled_u32
            ),
        ));
    }

    let application_id = read_u32(file)?;

    // Account for reserved
    let mut reserved = [0u8; 20];
    file.read_exact(&mut reserved)?;
    if !reserved.iter().all(|&x| x == 0) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid reserved (must be 0)",
        ));
    }

    let valid_for_num = read_u32(file)?;

    // file.seek(SeekFrom::Start(96))?;
    let sqlite_version_num = read_u32(file)?;
    if sqlite_version_num == 0 {
        println!("SQLite version number not set or invalid");
    } else {
        let major = sqlite_version_num / 1_000_000;
        let minor = (sqlite_version_num / 1_000) % 1000;
        let patch = sqlite_version_num % 1000;
        println!("SQLite version: {}.{}.{}", major, minor, patch);
    }

    let usable_page_size = page_size - reseved_space as u16;
    if usable_page_size < 480 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Invalid usable size ({}) of database page",
                usable_page_size
            ),
        ));
    }

    Ok(SqliteHeader {
        // magic: magic_bytes,
        page_size,
        // format_write_version,
        // format_read_version,
        // reseved_space,
        // max_payload_frac,
        // min_payload_frac,
        // leaf_payload_frac,
        file_change_count,
        size_in_pages,
        // first_freelist_trunk_page_num,
        // num_freelist_pages,
        // schema_cookie,
        schema_format,
        // suggested_cache_size,
        // largest_b_tree_root_page_num,
        text_encoding,
        // user_version,
        // incremental_vacuum_enabled,
        // application_id,
        // valid_for_num,
        sqlite_version_num,
    })
}

fn read_max_payload_frac(file: &mut File) -> Result<u8, io::Error> {
    let max_payload_frac = read_u8(file)?;
    if max_payload_frac != 64 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid max_payload_frac ({} != 64", max_payload_frac),
        ));
    }
    Ok(max_payload_frac)
}

fn read_min_payload_frac(file: &mut File) -> Result<u8, io::Error> {
    let min_payload_frac = read_u8(file)?;
    if min_payload_frac != 32 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid min_payload_frac ({} != 32", min_payload_frac),
        ));
    }
    Ok(min_payload_frac)
}
fn read_leaf_payload_frac(file: &mut File) -> Result<u8, io::Error> {
    let leaf_payload_frac = read_u8(file)?;
    if leaf_payload_frac != 32 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid leaf_payload_frac ({} != 32", leaf_payload_frac),
        ));
    }

    Ok(leaf_payload_frac)
}

fn read_page_size(file: &mut File) -> Result<u16, io::Error> {
    let page_size = read_u16(file)?;
    if !is_valid_page_size(page_size) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid page size",
        ));
    }
    Ok(page_size)
}

fn read_magic(file: &mut File) -> Result<[u8; 16], io::Error> {
    let mut magic_bytes = [0u8; 16];
    file.read_exact(&mut magic_bytes)?;
    if &magic_bytes != SQLITE_MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid SQLite magic",
        ));
    }
    Ok(magic_bytes)
}

fn read_page(file: &mut File, page_size: u16, page_num: u32) -> std::io::Result<Vec<u8>> {
    let mut buf = vec![0u8; page_size as usize];
    file.seek(SeekFrom::Start((page_num - 1) as u64 * page_size as u64))?;
    file.read_exact(&mut buf)?;
    Ok(buf)
}

fn parse_interior_index_cell(cell: &[u8], page_num: u32) -> Result<Cell<'_>, io::Error> {
    println!("Parsing Interior Index Cell {}", page_num);

    Ok(Cell::InteriorIndex {
        left_child_page: 0,
        key: vec![0, 1, 2],
    })
}

fn parse_interior_table_cell(cell: &[u8], page_num: u32) -> Result<Cell<'_>, io::Error> {
    println!("Parsing Interior Table Cell {}", page_num);
    Ok(Cell::InteriorTable {
        left_child_page: 0,
        key: 0,
    })
}

fn parse_leaf_index_cell(cell: &[u8], page_num: u32) -> Result<Cell<'_>, io::Error> {
    println!("Parsing Leaf Cell {}", page_num);
    Ok(Cell::LeafIndex { key: vec![0] })
}

// // cell may contain more bytes than needed
fn parse_leaf_table_cell(cell: &[u8]) -> Result<Cell<'_>, io::Error> {
    let (payload_size, payload_varint_size) = read_varint(&cell, 0);

    let (rowid, rowid_varint_size) = read_varint(&cell, payload_varint_size);
    let offset = payload_varint_size + rowid_varint_size;

    if payload_size as usize > cell.len() - offset {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "payload_size larger than remaining cell bytes",
        ));
    }

    let payload = cell
        .get(offset..offset + (payload_size as usize))
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "cell payload truncated"))?;

    Ok(Cell::LeafTable {
        payload_varint_size,
        rowid_varint_size,
        rowid,
        payload,
    })
}

fn read_u16_from_page(page: &[u8], offset: usize) -> Result<u16, io::Error> {
    let bytes = page.get(offset..offset + 2).ok_or(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "page too short",
    ))?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u32_from_page(page: &[u8], offset: usize) -> Result<u32, io::Error> {
    let bytes = page.get(offset..offset + 4).ok_or(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "page too short",
    ))?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u8_from_page(page: &[u8], offset: usize) -> Result<u8, io::Error> {
    Ok(page[offset])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum PageType {
    InteriorIndex = 0x02,
    InteriorTable = 0x05,
    LeafIndex = 0x0a,
    LeafTable = 0x0d,
}
use std::convert::TryFrom;
use std::vec;
#[derive(Debug)]
struct InvalidPageType(u8);

impl PageType {
    pub fn is_interior(&self) -> bool {
        matches!(self, PageType::InteriorIndex | PageType::InteriorTable)
    }
}

impl TryFrom<u8> for PageType {
    type Error = InvalidPageType;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x02 => Ok(PageType::InteriorIndex),
            0x05 => Ok(PageType::InteriorTable),
            0x0a => Ok(PageType::LeafIndex),
            0x0d => Ok(PageType::LeafTable),
            _ => Err(InvalidPageType(v)),
        }
    }
}

#[derive(Debug)]
pub struct PageHeader {
    page_num: u32,
    page_type: PageType,
    first_free_block: u16,
    num_cells_on_page: u16,
    start_cell_content: u16,
    frag_free_bytes: u8,
    rightmost_pointer: Option<u32>,
    size: usize,
}

const FREE_BLOCK_OFFSET: usize = 1;
const NUM_CELLS_OFFSET: usize = 3;
const START_CELL_CONTENT_OFFSET: usize = 5;
const FRAG_FREE_BYTES_OFFSET: usize = 7;
const RIGHTMOST_POINTER_OFFSET: usize = 8;

fn parse_page_header(page: &[u8], page_num: u32, offset: usize) -> Result<PageHeader, io::Error> {
    let page_type_u8 = read_u8_from_page(&page, offset)?;
    let page_type = PageType::try_from(page_type_u8).map_err(|v| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid page type: {:?}", v),
        )
    })?;

    let first_free_block = read_u16_from_page(&page, offset + FREE_BLOCK_OFFSET)?;
    let num_cells_on_page = read_u16_from_page(&page, offset + NUM_CELLS_OFFSET)?;
    let start_cell_content = read_u16_from_page(&page, offset + START_CELL_CONTENT_OFFSET)?;
    let frag_free_bytes = read_u8_from_page(&page, offset + FRAG_FREE_BYTES_OFFSET)?;

    let size: usize = if page_type.is_interior() { 12 } else { 8 };

    let mut rightmost_pointer = None;
    if page_type.is_interior() {
        rightmost_pointer = Some(read_u32_from_page(
            &page,
            offset + RIGHTMOST_POINTER_OFFSET,
        )?);
    }

    Ok(PageHeader {
        page_num,
        page_type,
        first_free_block,
        num_cells_on_page,
        start_cell_content,
        frag_free_bytes,
        rightmost_pointer: rightmost_pointer,
        size,
    })
}

fn read_varint(buf: &[u8], offset: usize) -> (u64, usize) {
    let mut value: u64 = 0;
    let mut bytes_read = 0;

    for &byte in buf.iter().skip(offset).take(9) {
        bytes_read += 1;

        if bytes_read < 9 {
            value = (value << 7) | ((byte & 0x7F) as u64);
            if byte & 0x80 == 0 {
                // last byte
                return (value, bytes_read);
            }
        } else {
            // 9th byte uses all 8 bits
            value = (value << 8) | (byte as u64);
            return (value, bytes_read);
        }
    }

    // If we reach here, buf was too short
    (value, bytes_read)
}

#[derive(Debug)]
pub enum Cell<'a> {
    InteriorIndex {
        left_child_page: u32,
        key: Vec<u8>,
    },
    InteriorTable {
        left_child_page: u32,
        key: u64,
    },
    LeafIndex {
        key: Vec<u8>,
    },
    LeafTable {
        payload_varint_size: usize,
        rowid_varint_size: usize,
        rowid: u64,
        payload: &'a [u8],
    },
}

fn parse_page(page: &[u8], page_num: u32) -> Result<(), io::Error> {
    let offset: usize = if page_num == 1 { 100 } else { 0 };
    let page_header = parse_page_header(&page, page_num, offset)?;
    println!("page_header={:?}", page_header);

    let base = offset + page_header.size;

    const CELL_SIZE: usize = 2;
    for i in 0..page_header.num_cells_on_page {
        let ptr_offset = base + (i as usize) * CELL_SIZE;
        let cell_pointer = read_u16_from_page(&page, ptr_offset).unwrap();
        let cell = &page[(cell_pointer as usize)..];

        let parsed_cell = match page_header.page_type {
            PageType::InteriorIndex => parse_interior_index_cell(page, page_num)?,
            PageType::InteriorTable => parse_interior_table_cell(page, page_num)?,
            PageType::LeafIndex => parse_leaf_index_cell(page, page_num)?,
            PageType::LeafTable => parse_leaf_table_cell(cell)?,
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid Page Type -- Page {} type: {:?}",
                    page_num, page_header.page_type
                ),
            ))?,
        };
        println!("parsed_cell {} at {}: {:?}\n", i, cell_pointer, parsed_cell);
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let mut path = PathBuf::from(&args.file);

    if path.extension().is_none() {
        path.set_extension("db");
    }
    let mut file = File::open(path)?;

    let header = read_header(&mut file)?;
    println!("Valid SQLite \n{}", header);

    let page1 = read_page(&mut file, header.page_size, 1)?;
    // println!("{:?}", page1);

    parse_page(&page1, 1)?;
    // println!("{:?}", parsed);

    Ok(())
}
