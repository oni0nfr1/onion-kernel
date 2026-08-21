# ONFT v1 bitmap font format

## 1. Overview

ONFT (Onion Font) is a binary format for storing fixed-size monochrome bitmap
fonts. Version 1 is intentionally limited to the 128 characters in the basic
ASCII range.

An ONFT v1 file consists of a 40-byte header followed immediately by 128
fixed-size glyph bitmaps.

All multibyte integers are unsigned and encoded in little-endian byte order.
Offsets and sizes are byte counts measured from the beginning of the file.

## 2. Version 1 constraints

ONFT v1 has the following fixed constraints:

- The font contains exactly 128 glyphs.
- Glyph index `n` corresponds directly to ASCII code `n`, for
  `0x00 <= n <= 0x7f`.
- `first_codepoint` is 0 and `glyph_count` is 128.
- Every glyph has the same width, height, row stride, and storage size.
- Each pixel is represented by one bit: 0 is unset and 1 is set.
- Glyph rows are stored from top to bottom.
- Bytes within each row are stored from left to right.
- Within each byte, the most significant bit represents the leftmost pixel.
- Glyph data is uncompressed.
- No flags are defined.

Control characters do not have a special binary representation. A font
producer should normally store blank bitmaps for ASCII control characters,
but a parser must treat them like any other glyph.

## 3. File layout

```text
+-------------------------------+ 0x0000
| Header (40 bytes)             |
+-------------------------------+ 0x0028
| Glyph 0   (ASCII 0x00)        |
+-------------------------------+
| Glyph 1   (ASCII 0x01)        |
+-------------------------------+
| ...                           |
+-------------------------------+
| Glyph 127 (ASCII 0x7f)        |
+-------------------------------+ end of file
```

The file must not contain data before the glyph array or trailing data after
it in version 1.

## 4. Header

The header is exactly 40 bytes:

| Offset | Size | Type | Field | Required v1 value or meaning |
|---:|---:|---|---|---|
| `0x00` | 4 | byte array | `magic` | ASCII bytes `ONFT` (`4f 4e 46 54`) |
| `0x04` | 2 | `u16` | `version` | 1 |
| `0x06` | 2 | `u16` | `header_size` | 40 |
| `0x08` | 4 | `u32` | `flags` | 0 |
| `0x0c` | 2 | `u16` | `glyph_width` | Width of every glyph in pixels; nonzero |
| `0x0e` | 2 | `u16` | `glyph_height` | Height of every glyph in pixels; nonzero |
| `0x10` | 2 | `u16` | `row_stride` | Bytes occupied by one stored glyph row |
| `0x12` | 2 | `u16` | `reserved` | 0 |
| `0x14` | 4 | `u32` | `glyph_count` | 128 |
| `0x18` | 4 | `u32` | `first_codepoint` | 0 |
| `0x1c` | 4 | `u32` | `bytes_per_glyph` | Bytes occupied by one glyph record |
| `0x20` | 4 | `u32` | `glyph_data_offset` | 40 |
| `0x24` | 4 | `u32` | `glyph_data_size` | `128 * bytes_per_glyph` |

`header_size`, `glyph_data_offset`, and fields currently fixed to zero remain
in the header so that later format versions can be extended without changing
the meaning of the existing fields.

## 5. Glyph encoding

### 5.1 Row layout

The minimum number of bytes required for one row is:

```text
minimum_row_stride = ceil(glyph_width / 8)
                   = (glyph_width + 7) / 8
```

`row_stride` must be at least `minimum_row_stride`. A larger value adds
padding bytes to the end of every row.

For pixel coordinates `(x, y)`, where the origin is the top-left of a glyph:

```text
byte_offset = y * row_stride + x / 8
bit_mask    = 0x80 >> (x % 8)
pixel_set   = (glyph[byte_offset] & bit_mask) != 0
```

If the glyph width is not divisible by 8, unused low-order bits in the last
data byte of each row must be zero. Row padding bytes must also be zero.

### 5.2 Glyph record layout

The minimum size of a glyph record is:

```text
minimum_bytes_per_glyph = row_stride * glyph_height
```

`bytes_per_glyph` must be at least this value. Any additional bytes form
padding at the end of the glyph record and must be zero.

Glyph records are contiguous. The byte range for glyph index `i` is:

```text
start = glyph_data_offset + i * bytes_per_glyph
end   = start + bytes_per_glyph
```

All calculations performed by a parser must use checked arithmetic.

## 6. Character mapping

ONFT v1 uses an implicit contiguous mapping and contains no character map.

```text
glyph_index = ASCII code
```

A consumer should return no glyph for a character outside the ASCII range.
Policy such as replacing an unsupported character with the `?` glyph belongs
to the renderer or console, not to the ONFT parser.

## 7. File validation

A conforming v1 parser must reject a file when any of the following is true:

- The file is shorter than 40 bytes.
- `magic` is not `ONFT`.
- `version` is not 1.
- `header_size` is not 40.
- `flags` or `reserved` is nonzero.
- `glyph_width` or `glyph_height` is zero.
- `row_stride` is smaller than `(glyph_width + 7) / 8`.
- `bytes_per_glyph` is smaller than `row_stride * glyph_height`.
- `glyph_count` is not 128.
- `first_codepoint` is not 0.
- `glyph_data_offset` is not 40.
- `glyph_data_size` is not `128 * bytes_per_glyph`.
- The total file size is not `glyph_data_offset + glyph_data_size`.
- Any size, offset, or multiplication overflows the parser's integer type.

A strict parser may additionally reject nonzero unused row bits or padding
bytes. Producers must always write these bits and bytes as zero, even when a
consumer chooses to ignore them.

Parsers must decode individual integer fields from bytes. They must not cast
the file buffer directly to a Rust or C header structure because structure
padding, alignment, and host byte order are not part of the file format.

## 8. Example: 8x16 ASCII font

For an 8x16 font without row or glyph padding:

```text
glyph_width       = 8
glyph_height      = 16
row_stride        = 1
bytes_per_glyph   = 16
glyph_count       = 128
glyph_data_offset = 40
glyph_data_size   = 2048
file_size         = 2088
```

The complete header is:

```text
4f 4e 46 54  01 00  28 00  00 00 00 00
08 00  10 00  01 00  00 00  80 00 00 00
00 00 00 00  10 00 00 00  28 00 00 00
00 08 00 00
```

As an example, a row containing pixels `#......#` is encoded as `0x81`, and a
row containing `.######.` is encoded as `0x7e`.

## 9. Compatibility and future versions

Version 1 readers must reject unsupported versions, unknown flags, and
nonzero reserved fields rather than guessing their meaning. A future version
may define Unicode mapping tables, variable-size glyphs, metadata, compression,
or additional bitmap encodings. Such features are not valid in an ONFT v1
file.
