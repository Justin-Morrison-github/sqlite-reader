mod cli;
use cli::handle_key;
use cli::{CliState, Mode};

use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write, stdout};
use std::path::PathBuf;
use std::vec;

use crossterm::{
    cursor,
    event::{self, Event},
    execute,
    style::{Attribute, SetAttribute},
    terminal::{self, ClearType},
    terminal::{disable_raw_mode, enable_raw_mode},
};

const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

#[derive(Debug)]
pub struct SqliteHeader {
    page_size: u16,
    // format_write_version: u8,
    // format_read_version: u8,
    // reserved_space: u8,
    // file_change_count: u32,
    size_in_pages: u32,
    // first_freelist_trunk_page_num: u32,
    // num_freelist_pages: u32,
    // schema_cookie: u32,
    // schema_format: u32,
    // suggested_cache_size: u32,
    // largest_b_tree_root_page_num: u32,
    // text_encoding: DBTextEncoding,
    // user_version: u32,
    // incremental_vacuum_enabled: bool,
    // application_id: u32,
    // valid_for_num: u32,
    // sqlite_version_num: u32,
    usable_page_size: u16,
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
    let _magic_bytes = read_magic(file)?;
    let page_size = read_page_size(file)?;

    let _format_write_version = read_u8(file)?;
    let _format_read_version = read_u8(file)?;

    let reserved_space = read_u8(file)?;
    let _max_payload_frac = read_max_payload_frac(file)?;
    let _min_payload_frac = read_min_payload_frac(file)?;
    let _leaf_payload_frac = read_leaf_payload_frac(file)?;

    let file_change_count = read_u32(file)?;
    let size_in_pages = read_u32(file)?;
    let _first_freelist_trunk_page_num = read_u32(file)?;
    let _num_freelist_pages = read_u32(file)?;
    let _schema_cookie = read_u32(file)?;

    let schema_format = read_u32(file)?;
    if schema_format > 4 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid schema_format ({})", schema_format),
        ));
    }

    let _suggested_cache_size = read_u32(file)?;

    let largest_b_tree_root_page_num = read_u32(file)?;

    let text_encoding_u32 = read_u32(file)?;
    let text_encoding = DBTextEncoding::try_from(text_encoding_u32)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let _user_version = read_u32(file)?;

    let incremental_vacuum_enabled_u32 = read_u32(file)?;
    let _incremental_vacuum_enabled = incremental_vacuum_enabled_u32 != 0;

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

    let _application_id = read_u32(file)?;

    // Account for reserved
    let mut reserved = [0u8; 20];
    file.read_exact(&mut reserved)?;
    if !reserved.iter().all(|&x| x == 0) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid reserved (must be 0)",
        ));
    }

    let _valid_for_num = read_u32(file)?;

    // file.seek(SeekFrom::Start(96))?;
    let sqlite_version_num = read_u32(file)?;
    if sqlite_version_num == 0 {
        println!("SQLite version number not set or invalid");
    }
    // else {
    //     let major = sqlite_version_num / 1_000_000;
    //     let minor = (sqlite_version_num / 1_000) % 1000;
    //     let patch = sqlite_version_num % 1000;
    //     // println!("SQLite version: {}.{}.{}", major, minor, patch);
    // }

    let usable_page_size = page_size - reserved_space as u16;
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
        page_size,
        // format_write_version,
        // format_read_version,
        // reserved_space,
        // file_change_count,
        size_in_pages,
        // first_freelist_trunk_page_num,
        // num_freelist_pages,
        // schema_cookie,
        // schema_format,
        // suggested_cache_size,
        // largest_b_tree_root_page_num,
        // text_encoding,
        // user_version,
        // incremental_vacuum_enabled,
        // application_id,
        // valid_for_num,
        // sqlite_version_num,
        usable_page_size,
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

fn parse_interior_index_cell(cell: &[u8]) -> Result<Cell<'_>, io::Error> {
    println!("Parsing Interior Index Cell");

    Ok(Cell::InteriorIndex {
        left_child_page: 0,
        key: vec![0, 1, 2],
    })
}

fn parse_interior_table_cell(cell: &[u8]) -> Result<Cell<'_>, io::Error> {
    println!("Parsing Interior Table Cell");
    Ok(Cell::InteriorTable {
        left_child_page: 0,
        key: 0,
    })
}

fn parse_leaf_index_cell(cell: &[u8], usable_size: u16) -> Result<Cell<'_>, io::Error> {
    let (payload_len, varint_size) = read_varint(&cell, 0);
    let offset = varint_size;
    let payload_len = payload_len as usize;

    let payload = cell
        .get(offset..offset + payload_len)
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "cell payload truncated"))?;

    let max_local = (usable_size as usize) - 35;

    let first_overflow_page = if payload_len > max_local {
        Some(read_u32_from_page(cell, offset + payload_len)?)
    } else {
        None
    };

    Ok(Cell::LeafIndex {
        payload,
        first_overflow_page,
    })
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
        // payload_varint_size,
        // rowid_varint_size,
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
        payload: &'a [u8],
        first_overflow_page: Option<u32>,
    },
    LeafTable {
        rowid: u64,
        payload: &'a [u8],
    },
}

#[derive(Debug)]
enum SerialType {
    Null,
    I8,
    I16,
    I24,
    I32,
    I48,
    I64,
    F64,
    Int0,
    Int1,
    Bytes(usize),
    Text(usize),
}

impl TryFrom<u64> for SerialType {
    type Error = io::Error;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SerialType::Null),
            1 => Ok(SerialType::I8),
            2 => Ok(SerialType::I16),
            3 => Ok(SerialType::I24),
            4 => Ok(SerialType::I32),
            5 => Ok(SerialType::I48),
            6 => Ok(SerialType::I64),
            7 => Ok(SerialType::F64),
            8 => Ok(SerialType::Int0),
            9 => Ok(SerialType::Int1),
            st if st >= 12 && st % 2 == 0 => {
                // BLOB
                let n = ((st - 12) / 2) as usize;
                Ok(SerialType::Bytes(n))
            }
            st if st >= 13 && st % 2 == 1 => {
                // TEXT
                let n = ((st - 13) / 2) as usize;
                Ok(SerialType::Text(n))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid serial type: {:?}", value),
            )),
        }
    }
}

#[derive(Debug)]
struct CellPayload {
    // serial_types: Vec<SerialType>,
    columns: Vec<SqliteValue>,
}

#[derive(Debug, Clone)]
enum SqliteValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}
impl SqliteValue {
    pub fn to_str(&self) -> String {
        match self {
            SqliteValue::Null => String::from("Null"),
            SqliteValue::Integer(i) => i.to_string(),
            SqliteValue::Real(r) => format!("{:e}", r),
            SqliteValue::Text(t) => t.to_string(),
            SqliteValue::Blob(items) => {
                format!("{:?}", items)
            }
        }
    }
}

fn decode_payload_values(
    payload: &[u8],
    serial_types: &[SerialType],
    header_length: usize,
) -> Result<Vec<SqliteValue>, io::Error> {
    let mut columns: Vec<SqliteValue> = Vec::new();

    let mut body_offset = header_length; // body starts after header
    for serial_type in serial_types {
        match serial_type {
            SerialType::Null => columns.push(SqliteValue::Null),
            SerialType::I8 => {
                columns.push(SqliteValue::Integer(payload[body_offset] as i8 as i64));
                body_offset += 1;
            }
            SerialType::I16 => {
                let val =
                    i16::from_be_bytes(payload[body_offset..body_offset + 2].try_into().unwrap())
                        as i64;
                columns.push(SqliteValue::Integer(val));
                body_offset += 2;
            }
            SerialType::I24 => {
                let val = ((payload[body_offset] as i32) << 16)
                    | ((payload[body_offset + 1] as i32) << 8)
                    | (payload[body_offset + 2] as i32);
                columns.push(SqliteValue::Integer(val as i64));
                body_offset += 3;
            }
            SerialType::I32 => {
                let val =
                    i32::from_be_bytes(payload[body_offset..body_offset + 4].try_into().unwrap())
                        as i64;
                columns.push(SqliteValue::Integer(val));
                body_offset += 4;
            }
            SerialType::I48 => {
                let val = ((payload[body_offset] as i64) << 40)
                    | ((payload[body_offset + 1] as i64) << 32)
                    | ((payload[body_offset + 2] as i64) << 24)
                    | ((payload[body_offset + 3] as i64) << 16)
                    | ((payload[body_offset + 4] as i64) << 8)
                    | (payload[body_offset + 5] as i64);
                columns.push(SqliteValue::Integer(val));
                body_offset += 6;
            }
            SerialType::I64 => {
                let val =
                    i64::from_be_bytes(payload[body_offset..body_offset + 8].try_into().unwrap());
                columns.push(SqliteValue::Integer(val));
                body_offset += 8;
            }
            SerialType::F64 => {
                let val =
                    f64::from_be_bytes(payload[body_offset..body_offset + 8].try_into().unwrap());
                columns.push(SqliteValue::Real(val));
                body_offset += 8;
            }
            SerialType::Int0 => columns.push(SqliteValue::Integer(0)),
            SerialType::Int1 => columns.push(SqliteValue::Integer(1)),
            SerialType::Bytes(len) => {
                let val = payload[body_offset..body_offset + *len].to_vec();
                columns.push(SqliteValue::Blob(val));
                body_offset += *len;
            }
            SerialType::Text(len) => {
                let val = std::str::from_utf8(&payload[body_offset..body_offset + *len])
                    .unwrap_or("<invalid utf8>")
                    .to_string();
                columns.push(SqliteValue::Text(val));
                body_offset += *len;
            }
        }
    }
    Ok(columns)
}

fn parse_payload(payload: &[u8]) -> Result<CellPayload, io::Error> {
    let (header_length, header_len_bytes) = read_varint(payload, 0);
    let mut serial_types: Vec<SerialType> = Vec::new();

    let mut offset = header_len_bytes;
    while offset < header_length as usize {
        let (serial_type_u64, size) = read_varint(payload, offset);
        let serial_type = SerialType::try_from(serial_type_u64)?;
        serial_types.push(serial_type);
        offset += size;
    }
    let columns = decode_payload_values(&payload, &serial_types, header_length as usize)?;
    Ok(CellPayload {
        columns,
        // serial_types,
    })
}

#[derive(Debug, Clone)]
struct SchemaTable {
    pub _type: SqliteValue,
    pub name: SqliteValue,
    pub tbl_name: SqliteValue,
    pub rootpage: SqliteValue,
    pub sql: SqliteValue,
}

fn extract_schema_table(columns: &[SqliteValue]) -> Result<SchemaTable, io::Error> {
    Ok(SchemaTable {
        _type: columns[0].clone(),
        name: columns[1].clone(),
        tbl_name: columns[2].clone(),
        rootpage: columns[3].clone(),
        sql: columns[4].clone(),
    })
}

#[derive(Debug)]
pub struct Schema {
    _type: SqliteValue,
    name: SqliteValue,
    rootpage: SqliteValue,
    rowid: u64,
    columns: Vec<String>,
}

#[derive(Debug)]
pub struct MasterSchemaPage {
    map: HashMap<String, Schema>,
    // schema_table: Vec<SchemaTable>,
    // parsed_cells: Vec<Cell<'a>>,
    // parsed_payloads: Vec<CellPayload>,
    b_tree_pages: Vec<u32>,
}

fn parse_master_schema_page(
    page: &[u8],
    page_num: u32,
    usable_page_size: u16,
) -> Result<MasterSchemaPage, io::Error> {
    let mut master_schema_page = MasterSchemaPage {
        map: HashMap::new(),
        b_tree_pages: Vec::new(),
    };
    let offset: usize = 100;
    let page_header = parse_page_header(&page, page_num, offset)?;
    println!("master schema page_header={:?}", page_header);

    let base = offset + page_header.size;

    const CELL_SIZE: usize = 2;
    for i in 0..page_header.num_cells_on_page {
        let ptr_offset = base + (i as usize) * CELL_SIZE;
        let cell_pointer = read_u16_from_page(&page, ptr_offset).unwrap();
        let cell = &page[(cell_pointer as usize)..];

        match page_header.page_type {
            PageType::InteriorIndex => {
                println!("InteriorIndex");
                parse_interior_index_cell(cell)?;
                todo!()
            }
            PageType::InteriorTable => {
                println!("InteriorTable");
                parse_interior_table_cell(cell)?;

                todo!()
            }
            PageType::LeafIndex => {
                println!("LeafIndex");
                let parsed_cell = parse_leaf_index_cell(cell, usable_page_size)?;
                if let Cell::LeafIndex {
                    payload,
                    first_overflow_page,
                } = parsed_cell
                {
                    let parsed_payload = parse_payload(payload)?;
                    println!(
                        "\nParsed Leaf Index Cell {} at {}: {:?}",
                        i, cell_pointer, parsed_payload
                    );
                }
            }
            PageType::LeafTable => {
                println!("LeafTable");
                let parsed_cell = parse_leaf_table_cell(cell)?;
                if let Cell::LeafTable { rowid, payload } = parsed_cell {
                    let parsed_payload = parse_payload(&payload)?;
                    let schema_table = extract_schema_table(&parsed_payload.columns)?;
                    let columns = extract_table_columns(&parsed_payload.columns[4])?;

                    if let SqliteValue::Integer(root_page) = schema_table.rootpage {
                        master_schema_page.b_tree_pages.push(root_page as u32);
                    }

                    let schema = Schema {
                        _type: schema_table._type.clone(),
                        name: schema_table.name.clone(),
                        rootpage: schema_table.rootpage,
                        rowid,
                        columns,
                    };
                    if let SqliteValue::Text(name) = schema_table.name {
                        master_schema_page.map.insert(name, schema);
                    }
                }
            }
        };
    }

    Ok(master_schema_page)
}

fn parse_page(
    page: &[u8],
    page_num: u32,
    usable_page_size: u16,
    app_state: &mut AppState,
) -> Result<(), io::Error> {
    println!("\nParsing Page {}", page_num);
    let offset: usize = if page_num == 1 { 100 } else { 0 };
    let page_header = parse_page_header(&page, page_num, offset)?;
    println!("{:?}", page_header);

    let base = offset + page_header.size;

    const CELL_SIZE: usize = 2;
    for i in 0..page_header.num_cells_on_page {
        let ptr_offset = base + (i as usize) * CELL_SIZE;
        let cell_pointer = read_u16_from_page(&page, ptr_offset).unwrap();
        let cell = &page[(cell_pointer as usize)..];

        match page_header.page_type {
            PageType::InteriorIndex => {
                println!("InteriorIndex");
                parse_interior_index_cell(cell)?;
                todo!()
            }
            PageType::InteriorTable => {
                println!("InteriorTable");
                parse_interior_table_cell(cell)?;
                todo!()
            }
            PageType::LeafIndex => {
                println!("LeafIndex");
                let parsed_cell: Cell<'_> = parse_leaf_index_cell(cell, usable_page_size)?;
                if let Cell::LeafIndex {
                    payload,
                    first_overflow_page,
                } = parsed_cell
                {
                    let parsed_payload = parse_payload(&payload)?;
                    // println!("LeafIndex : {:?}", parsed_payload);
                    println!(
                        "Parsed Leaf Index Cell {} at {}: {:?}",
                        i, cell_pointer, parsed_payload
                    );
                }
            }
            PageType::LeafTable => {
                println!("LeafTable");

                let parsed_cell = parse_leaf_table_cell(cell)?;
                if let Cell::LeafTable { rowid, payload } = parsed_cell {
                    let parsed_payload = parse_payload(&payload)?;
                    println!(" LeafTable  parsed_payload={:?}", parsed_payload);
                    app_state.rows.push(parsed_payload.columns);
                }
            }
        };
    }

    Ok(())
}
fn extract_table_columns(sql_text: &SqliteValue) -> Result<Vec<String>, io::Error> {
    let mut columns: Vec<String> = Vec::new();

    if let SqliteValue::Text(s) = sql_text {
        // Find the part inside the parentheses
        if let Some(start) = s.find('(') {
            if let Some(end) = s.rfind(')') {
                let cols = &s[start + 1..end]; // slice inside ()
                // Split by comma, handle each column definition
                for col_def in cols.split(',') {
                    let col_def = col_def.trim();
                    if col_def.is_empty() {
                        continue;
                    }
                    // Split by whitespace
                    let mut parts = col_def.split_whitespace();
                    if let Some(name) = parts.next() {
                        columns.push(name.to_string());
                    }
                }
            }
        }
    }

    Ok(columns)
}

fn draw_table(
    out: &mut impl std::io::Write,
    headers: &[String],
    rows: &[Vec<String>],
    app_state: &AppState,
) -> std::io::Result<()> {
    // compute column widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.len());
        }
    }

    // header
    write!(out, "  |")?; // left margin

    for (i, h) in headers.iter().enumerate() {
        write!(
            out,
            " {:width$} |",
            h.to_ascii_uppercase(),
            width = widths[i]
        )?;
    }
    writeln!(out)?;
    execute!(out, cursor::MoveTo(0, 1 as u16))?;

    // separator
    write!(out, "  |")?;
    for (i, w) in widths.iter().enumerate() {
        if i == 0 {}
        write!(out, " {} |", "-".repeat(*w))?;
    }
    writeln!(out)?;

    // rows
    for (i, row) in rows.iter().enumerate() {
        execute!(out, cursor::MoveTo(0, (i + 2) as u16))?;
        if i == app_state.cli_state.row_idx {
            write!(out, "> ")?;
        } else {
            write!(out, "  ")?;
        }
        write!(out, "|")?;
        for (j, cell) in row.iter().enumerate() {
            write!(out, " {:width$} ", cell, width = widths[j])?;
            write!(out, "|")?;
        }
        writeln!(out)?;
    }

    Ok(())
}

fn draw_menu(app_state: &AppState) -> io::Result<()> {
    let mut out = stdout();

    execute!(out, cursor::MoveTo(0, 0), terminal::Clear(ClearType::All))?;

    match app_state.cli_state.mode {
        Mode::TableSelect => {
            for table in app_state.items.iter() {
                execute!(out, cursor::MoveTo(0, table.index as u16))?;
                if table.index == app_state.cli_state.table_idx {
                    execute!(out, SetAttribute(Attribute::Reverse))?;
                    write!(out, "> {}", table.name)?;
                    execute!(out, SetAttribute(Attribute::Reset))?;
                } else {
                    write!(out, "  {}", table.name)?;
                }
            }

            write!(out, "\n{:?}", app_state)?;
        }
        Mode::RowSelect => {
            match &app_state.active_table {
                Some(table_columns) => {
                    let headers: Vec<String> = table_columns.clone();
                    let rows = app_state.get_active_rows();

                    draw_table(&mut out, &headers, &rows, &app_state)?;
                }
                None => todo!(),
            }

            write!(out, "\n{:?}", app_state)?;
        }
    }

    out.flush()?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct TableUISelect {
    name: String,
    index: usize,
    rowid: u64,
}

fn extract_table_name_rowid_map(
    master_schema_page: &MasterSchemaPage,
) -> Result<Vec<TableUISelect>, io::Error> {
    let mut items: Vec<TableUISelect> = Vec::new();
    for (index, (name, schema)) in master_schema_page.map.iter().enumerate() {
        items.insert(
            index,
            TableUISelect {
                name: name.to_string(),
                index,
                rowid: schema.rowid,
            },
        );
    }
    Ok(items)
}

fn extract_table_map(
    master_schema_page: &MasterSchemaPage,
) -> Result<HashMap<u64, Vec<String>>, io::Error> {
    let mut items: HashMap<u64, Vec<String>> = HashMap::new();
    for (_, schema) in &master_schema_page.map {
        items.insert(schema.rowid, schema.columns.clone());
    }
    Ok(items)
}

#[derive(Debug, Clone)]

pub struct AppState {
    cli_state: CliState,
    items: Vec<TableUISelect>,
    active_table: Option<Vec<String>>,
    map: HashMap<u64, Vec<String>>,
    rows: Vec<Vec<SqliteValue>>,
    row_strings: Vec<Vec<String>>,
}

impl AppState {
    pub fn get_active_rowid(&self) -> Option<u64> {
        self.items.get(self.cli_state.table_idx).map(|s| s.rowid)
    }

    pub fn get_active_table(&self) -> Option<Vec<String>> {
        self.get_active_rowid()
            .and_then(|rowid| self.map.get(&rowid).cloned())
    }

    pub fn get_active_rows(&self) -> Vec<Vec<String>> {
        self.rows
            .iter()
            .map(|row| row.iter().map(|v| v.to_str()).collect())
            .collect()
    }
}

fn main() -> io::Result<()> {
    let mut path = PathBuf::from("test.db");

    if path.extension().is_none() {
        path.set_extension("db");
    }
    let mut file = File::open(path)?;

    let header = read_header(&mut file)?;
    println!("{:?}", header);
    let master_schema_page = read_page(&mut file, header.page_size, 1)?;

    let master_schema_page =
        parse_master_schema_page(&master_schema_page, 1, header.usable_page_size)?;
    let items = extract_table_name_rowid_map(&master_schema_page)?;
    let num_tables = items.len();
    let mut app_state = AppState {
        items: items,
        map: extract_table_map(&master_schema_page)?,
        active_table: None,
        rows: Vec::new(),
        row_strings: Vec::new(),
        cli_state: CliState {
            table_idx: 0,
            row_idx: 0,
            num_tables,
            num_rows: 0,
            mode: Mode::TableSelect,
        },
    };

    // wait_for_key()?;

    println!("Valid b-tree pages {:?}", master_schema_page.b_tree_pages);

    for i in master_schema_page.b_tree_pages {
        let page = read_page(&mut file, header.page_size, i)?;
        parse_page(&page, i, header.usable_page_size, &mut app_state)?;
    }
    enable_raw_mode()?; // important
    execute!(stdout(), cursor::Hide, terminal::EnterAlternateScreen)?;

    loop {
        draw_menu(&app_state)?;

        if let Event::Key(event) = event::read()? {
            // if handle_key(&mut app_state, event.code) {
            //     break;
            // }
            if let Some(signal) = handle_key(&mut app_state.cli_state, event.code) {
                match signal {
                    cli::Signal::Exit => break,
                    cli::Signal::UpdateTable => {
                        app_state.active_table = app_state.get_active_table()
                    }
                    cli::Signal::UpdateRow => app_state.row_strings = app_state.get_active_rows(),
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), cursor::Show, terminal::LeaveAlternateScreen)?;

    Ok(())
}
