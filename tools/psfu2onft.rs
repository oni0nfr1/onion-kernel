use std::{
    env, fs,
    path::Path,
    process::{self, Command},
};

const PSF2_MAGIC: [u8; 4] = [0x72, 0xb5, 0x4a, 0x86];
const PSF2_HEADER_SIZE: usize = 32;
const PSF2_HAS_UNICODE_TABLE: u32 = 1;

const ONFT_HEADER_SIZE: usize = 40;
const ONFT_GLYPH_COUNT: usize = 128;

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, String> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| "integer overflow while reading PSF2 header".to_owned())?;
    let bytes: [u8; 4] = data
        .get(offset..end)
        .ok_or_else(|| "truncated PSF2 header".to_owned())?
        .try_into()
        .expect("a four-byte slice converts to a four-byte array");
    Ok(u32::from_le_bytes(bytes))
}

fn read_input(path: &Path) -> Result<Vec<u8>, String> {
    let data =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    if !data.starts_with(&[0x1f, 0x8b]) {
        return Ok(data);
    }

    let output = Command::new("gzip")
        .arg("-dc")
        .arg("--")
        .arg(path)
        .output()
        .map_err(|error| format!("failed to run gzip: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gzip failed for {}: {stderr}", path.display()));
    }

    Ok(output.stdout)
}

struct Psf2Font<'a> {
    width: usize,
    height: usize,
    row_stride: usize,
    char_size: usize,
    glyphs: &'a [u8],
    ascii_map: [Option<usize>; ONFT_GLYPH_COUNT],
}

impl<'a> Psf2Font<'a> {
    fn parse(data: &'a [u8]) -> Result<Self, String> {
        if data.get(..4) != Some(&PSF2_MAGIC) {
            return Err("input is not a PSF2 font".to_owned());
        }

        let version = read_u32_le(data, 4)?;
        let header_size = usize::try_from(read_u32_le(data, 8)?)
            .map_err(|_| "PSF2 header size does not fit usize".to_owned())?;
        let flags = read_u32_le(data, 12)?;
        let glyph_count = usize::try_from(read_u32_le(data, 16)?)
            .map_err(|_| "PSF2 glyph count does not fit usize".to_owned())?;
        let char_size = usize::try_from(read_u32_le(data, 20)?)
            .map_err(|_| "PSF2 character size does not fit usize".to_owned())?;
        let height = usize::try_from(read_u32_le(data, 24)?)
            .map_err(|_| "PSF2 height does not fit usize".to_owned())?;
        let width = usize::try_from(read_u32_le(data, 28)?)
            .map_err(|_| "PSF2 width does not fit usize".to_owned())?;

        if version != 0 {
            return Err(format!("unsupported PSF2 version: {version}"));
        }
        if header_size < PSF2_HEADER_SIZE || header_size > data.len() {
            return Err(format!("invalid PSF2 header size: {header_size}"));
        }
        if flags & !PSF2_HAS_UNICODE_TABLE != 0 {
            return Err(format!("unsupported PSF2 flags: {flags:#x}"));
        }
        if glyph_count == 0 || width == 0 || height == 0 {
            return Err("PSF2 glyph count, width, and height must be nonzero".to_owned());
        }
        if width > u16::MAX as usize || height > u16::MAX as usize {
            return Err("PSF2 dimensions do not fit ONFT v1 fields".to_owned());
        }

        let row_stride = width
            .checked_add(7)
            .ok_or_else(|| "row-stride calculation overflow".to_owned())?
            / 8;
        let minimum_char_size = row_stride
            .checked_mul(height)
            .ok_or_else(|| "character-size calculation overflow".to_owned())?;
        if char_size < minimum_char_size {
            return Err(format!(
                "PSF2 character size {char_size} is smaller than {minimum_char_size}"
            ));
        }

        let glyph_data_size = glyph_count
            .checked_mul(char_size)
            .ok_or_else(|| "PSF2 glyph-data size overflow".to_owned())?;
        let glyph_data_end = header_size
            .checked_add(glyph_data_size)
            .ok_or_else(|| "PSF2 glyph-data offset overflow".to_owned())?;
        let glyphs = data
            .get(header_size..glyph_data_end)
            .ok_or_else(|| "truncated PSF2 glyph data".to_owned())?;

        let ascii_map = if flags & PSF2_HAS_UNICODE_TABLE != 0 {
            Self::parse_unicode_table(
                data.get(glyph_data_end..)
                    .ok_or_else(|| "missing PSF2 Unicode table".to_owned())?,
                glyph_count,
            )?
        } else {
            let mut map = [None; ONFT_GLYPH_COUNT];
            for (codepoint, entry) in map.iter_mut().enumerate().take(glyph_count) {
                *entry = Some(codepoint);
            }
            map
        };

        for codepoint in 0x20..=0x7e {
            if ascii_map[codepoint].is_none() {
                return Err(format!(
                    "printable ASCII codepoint U+{codepoint:04X} is missing from the PSF2 font"
                ));
            }
        }

        Ok(Self {
            width,
            height,
            row_stride,
            char_size,
            glyphs,
            ascii_map,
        })
    }

    fn parse_unicode_table(
        table: &[u8],
        glyph_count: usize,
    ) -> Result<[Option<usize>; ONFT_GLYPH_COUNT], String> {
        let mut map = [None; ONFT_GLYPH_COUNT];
        let mut cursor = 0;

        for glyph_index in 0..glyph_count {
            let relative_end = table
                .get(cursor..)
                .and_then(|remaining| remaining.iter().position(|byte| *byte == 0xff))
                .ok_or_else(|| format!("unterminated Unicode entry for glyph {glyph_index}"))?;
            let entry_end = cursor + relative_end;
            let entry = &table[cursor..entry_end];

            // Bytes following 0xfe describe multi-codepoint sequences. ONFT v1
            // cannot represent those, so only the direct mappings are used.
            let direct_end = entry
                .iter()
                .position(|byte| *byte == 0xfe)
                .unwrap_or(entry.len());
            let direct = std::str::from_utf8(&entry[..direct_end])
                .map_err(|error| format!("invalid UTF-8 for glyph {glyph_index}: {error}"))?;

            for character in direct.chars() {
                if character.is_ascii() {
                    let codepoint = character as usize;
                    map[codepoint].get_or_insert(glyph_index);
                }
            }

            cursor = entry_end + 1;
        }

        Ok(map)
    }

    fn glyph(&self, index: usize) -> Result<&[u8], String> {
        let start = index
            .checked_mul(self.char_size)
            .ok_or_else(|| "glyph offset overflow".to_owned())?;
        let end = start
            .checked_add(self.char_size)
            .ok_or_else(|| "glyph end offset overflow".to_owned())?;
        self.glyphs
            .get(start..end)
            .ok_or_else(|| format!("glyph index {index} is out of bounds"))
    }
}

fn convert(font: &Psf2Font<'_>) -> Result<Vec<u8>, String> {
    let bytes_per_glyph = font
        .row_stride
        .checked_mul(font.height)
        .ok_or_else(|| "ONFT glyph size overflow".to_owned())?;
    let glyph_data_size = ONFT_GLYPH_COUNT
        .checked_mul(bytes_per_glyph)
        .ok_or_else(|| "ONFT glyph-data size overflow".to_owned())?;
    let file_size = ONFT_HEADER_SIZE
        .checked_add(glyph_data_size)
        .ok_or_else(|| "ONFT file size overflow".to_owned())?;

    let row_stride_u16 = u16::try_from(font.row_stride)
        .map_err(|_| "ONFT row stride does not fit u16".to_owned())?;
    let bytes_per_glyph_u32 = u32::try_from(bytes_per_glyph)
        .map_err(|_| "ONFT glyph size does not fit u32".to_owned())?;
    let glyph_data_size_u32 = u32::try_from(glyph_data_size)
        .map_err(|_| "ONFT glyph-data size does not fit u32".to_owned())?;

    let mut output = Vec::with_capacity(file_size);
    output.extend_from_slice(b"ONFT");
    output.extend_from_slice(&1_u16.to_le_bytes());
    output.extend_from_slice(&(ONFT_HEADER_SIZE as u16).to_le_bytes());
    output.extend_from_slice(&0_u32.to_le_bytes());
    output.extend_from_slice(&(font.width as u16).to_le_bytes());
    output.extend_from_slice(&(font.height as u16).to_le_bytes());
    output.extend_from_slice(&row_stride_u16.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    output.extend_from_slice(&(ONFT_GLYPH_COUNT as u32).to_le_bytes());
    output.extend_from_slice(&0_u32.to_le_bytes());
    output.extend_from_slice(&bytes_per_glyph_u32.to_le_bytes());
    output.extend_from_slice(&(ONFT_HEADER_SIZE as u32).to_le_bytes());
    output.extend_from_slice(&glyph_data_size_u32.to_le_bytes());
    debug_assert_eq!(output.len(), ONFT_HEADER_SIZE);

    let unused_bits = font.row_stride * 8 - font.width;
    let last_byte_mask = if unused_bits == 0 {
        u8::MAX
    } else {
        u8::MAX << unused_bits
    };

    for codepoint in 0..ONFT_GLYPH_COUNT {
        let Some(glyph_index) = font.ascii_map[codepoint] else {
            output.resize(output.len() + bytes_per_glyph, 0);
            continue;
        };

        let source = font.glyph(glyph_index)?;
        for row in 0..font.height {
            let start = row * font.row_stride;
            let end = start + font.row_stride;
            output.extend_from_slice(&source[start..end]);
            if unused_bits != 0 {
                let last = output
                    .last_mut()
                    .expect("a nonempty glyph row has a final byte");
                *last &= last_byte_mask;
            }
        }
    }

    debug_assert_eq!(output.len(), file_size);
    Ok(output)
}

fn run() -> Result<(), String> {
    let mut arguments = env::args_os();
    let program = arguments.next().unwrap_or_else(|| "psfu2onft".into());
    let input = arguments.next();
    let output = arguments.next();

    if input.is_none() || output.is_none() || arguments.next().is_some() {
        return Err(format!(
            "usage: {} <input.psfu[.gz]> <output.onft>",
            Path::new(&program).display()
        ));
    }

    let input = input.expect("checked above");
    let output = output.expect("checked above");
    let input_path = Path::new(&input);
    let output_path = Path::new(&output);

    let input_data = read_input(input_path)?;
    let font = Psf2Font::parse(&input_data)?;
    let onft = convert(&font)?;

    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    fs::write(output_path, &onft)
        .map_err(|error| format!("failed to write {}: {error}", output_path.display()))?;

    println!(
        "converted {}x{} PSF2 font to {} ({} bytes)",
        font.width,
        font.height,
        output_path.display(),
        onft.len()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("psfu2onft: {error}");
        process::exit(1);
    }
}
