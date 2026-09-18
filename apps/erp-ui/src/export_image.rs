const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const MAX_PNG_BYTES: usize = 1024 * 1024;
const MAX_PNG_DIMENSION: u32 = 4096;
const MAX_PNG_PIXELS: u64 = 4 * 1024 * 1024;

/// Validates the PNG container used by XLSX image embedding.
///
/// This deliberately does not decompress IDAT data or prove that it decodes to
/// a complete pixel image. It validates the bounded PNG container, chunk CRCs,
/// required structure, and IHDR fields before the bytes enter the workbook.
pub(super) fn validate_png(bytes: &[u8]) -> Result<(u32, u32), String> {
    if bytes.len() > MAX_PNG_BYTES {
        return Err(format!("PNG exceeds the {MAX_PNG_BYTES}-byte limit"));
    }
    if !bytes.starts_with(PNG_SIGNATURE) {
        return Err("Invalid PNG signature".into());
    }

    let mut offset = PNG_SIGNATURE.len();
    let mut chunk_index = 0usize;
    let mut dimensions = None;
    let mut color_type = None;
    let mut saw_plte = false;
    let mut saw_idat = false;
    let mut idat_finished = false;
    let mut saw_iend = false;

    while offset < bytes.len() {
        let header_end = offset
            .checked_add(8)
            .ok_or_else(|| "PNG chunk header overflow".to_owned())?;
        if header_end > bytes.len() {
            return Err("Truncated PNG chunk header".into());
        }

        let length = u32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| "Invalid PNG chunk length".to_owned())?,
        ) as usize;
        let chunk_type: [u8; 4] = bytes[offset + 4..header_end]
            .try_into()
            .map_err(|_| "Invalid PNG chunk type".to_owned())?;
        if !chunk_type.iter().all(u8::is_ascii_alphabetic) || chunk_type[2].is_ascii_lowercase() {
            return Err("Invalid PNG chunk type".into());
        }

        let data_start = header_end;
        let data_end = data_start
            .checked_add(length)
            .ok_or_else(|| "PNG chunk length overflow".to_owned())?;
        let chunk_end = data_end
            .checked_add(4)
            .ok_or_else(|| "PNG chunk length overflow".to_owned())?;
        if chunk_end > bytes.len() {
            return Err("Truncated PNG chunk data".into());
        }

        let expected_crc = u32::from_be_bytes(
            bytes[data_end..chunk_end]
                .try_into()
                .map_err(|_| "Invalid PNG chunk CRC".to_owned())?,
        );
        let actual_crc = crc32(&bytes[offset + 4..data_end]);
        if expected_crc != actual_crc {
            return Err(format!(
                "PNG chunk {} has an invalid CRC",
                String::from_utf8_lossy(&chunk_type)
            ));
        }

        match &chunk_type {
            b"IHDR" => {
                if chunk_index != 0 || dimensions.is_some() || length != 13 {
                    return Err("PNG must start with one 13-byte IHDR chunk".into());
                }
                dimensions = Some(validate_ihdr(&bytes[data_start..data_end])?);
                color_type = Some(bytes[data_start + 9]);
            }
            b"IDAT" => {
                if dimensions.is_none() || saw_iend || idat_finished {
                    return Err("PNG IDAT chunks are out of order".into());
                }
                saw_idat = true;
            }
            b"IEND" => {
                if length != 0 || dimensions.is_none() || !saw_idat || saw_iend {
                    return Err("Invalid PNG IEND chunk".into());
                }
                saw_iend = true;
                if chunk_end != bytes.len() {
                    return Err("PNG IEND chunk must be last".into());
                }
            }
            b"acTL" | b"fcTL" | b"fdAT" => {
                return Err("Animated PNG images are not supported".into());
            }
            b"PLTE" => {
                if dimensions.is_none() || saw_idat || saw_iend || saw_plte {
                    return Err("PNG PLTE chunk is out of order".into());
                }
                if length == 0 || length > 768 || !length.is_multiple_of(3) {
                    return Err("PNG PLTE chunk has an invalid length".into());
                }
                if matches!(color_type, Some(0 | 4)) {
                    return Err("Grayscale PNG images cannot contain PLTE".into());
                }
                saw_plte = true;
            }
            _ if chunk_type[0].is_ascii_uppercase() => {
                return Err(format!(
                    "Unsupported critical PNG chunk {}",
                    String::from_utf8_lossy(&chunk_type)
                ));
            }
            _ => {
                if dimensions.is_none() || saw_iend {
                    return Err("PNG ancillary chunk is out of order".into());
                }
            }
        }

        if saw_idat && chunk_type != *b"IDAT" && chunk_type != *b"IEND" {
            idat_finished = true;
        }
        offset = chunk_end;
        chunk_index += 1;
    }

    if dimensions.is_none() {
        return Err("PNG is missing IHDR".into());
    }
    if !saw_idat {
        return Err("PNG is missing IDAT".into());
    }
    if color_type == Some(3) && !saw_plte {
        return Err("Indexed-color PNG is missing PLTE".into());
    }
    if !saw_iend {
        return Err("PNG is missing IEND".into());
    }
    dimensions.ok_or_else(|| "PNG is missing IHDR".into())
}

fn validate_ihdr(data: &[u8]) -> Result<(u32, u32), String> {
    let width = u32::from_be_bytes(
        data[0..4]
            .try_into()
            .map_err(|_| "Invalid PNG width".to_owned())?,
    );
    let height = u32::from_be_bytes(
        data[4..8]
            .try_into()
            .map_err(|_| "Invalid PNG height".to_owned())?,
    );
    if width == 0 || height == 0 {
        return Err("PNG dimensions must be nonzero".into());
    }
    if width > MAX_PNG_DIMENSION || height > MAX_PNG_DIMENSION {
        return Err(format!(
            "PNG dimensions exceed {MAX_PNG_DIMENSION} pixels per side"
        ));
    }
    if u64::from(width) * u64::from(height) > MAX_PNG_PIXELS {
        return Err(format!("PNG exceeds the {MAX_PNG_PIXELS}-pixel limit"));
    }

    let bit_depth = data[8];
    let color_type = data[9];
    let valid_depth = match color_type {
        0 => matches!(bit_depth, 1 | 2 | 4 | 8 | 16),
        2 | 4 | 6 => matches!(bit_depth, 8 | 16),
        3 => matches!(bit_depth, 1 | 2 | 4 | 8),
        _ => false,
    };
    if !valid_depth {
        return Err("Invalid PNG color type and bit depth combination".into());
    }
    if data[10] != 0 || data[11] != 0 || !matches!(data[12], 0 | 1) {
        return Err("Unsupported PNG compression, filter, or interlace method".into());
    }
    Ok((width, height))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
pub(super) fn png_fixture(width: u32, height: u32) -> Vec<u8> {
    fn push_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(chunk_type);
        out.extend_from_slice(data);
        out.extend_from_slice(&crc32(&[chunk_type.as_slice(), data].concat()).to_be_bytes());
    }

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut out = PNG_SIGNATURE.to_vec();
    push_chunk(&mut out, b"IHDR", &ihdr);
    // A valid zlib stream for one transparent RGBA pixel. Container validation
    // intentionally does not require its decoded dimensions to match IHDR.
    push_chunk(
        &mut out,
        b"IDAT",
        &[
            0x78, 0x9c, 0x63, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x05, 0x00, 0x01,
        ],
    );
    push_chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_bounded_png_container() {
        let independent_sample = include_bytes!("../assets/export-sample.png");
        assert_eq!(validate_png(independent_sample), Ok((16, 16)));
    }

    #[test]
    fn rejects_truncation_and_bad_crc() {
        let mut truncated = png_fixture(1, 1);
        truncated.pop();
        assert!(validate_png(&truncated).is_err());

        let mut bad_crc = png_fixture(1, 1);
        bad_crc[29] ^= 1;
        assert!(validate_png(&bad_crc).is_err());
    }

    #[test]
    fn rejects_dimension_and_size_limits() {
        assert!(validate_png(&png_fixture(4097, 1)).is_err());
        assert!(validate_png(&png_fixture(4096, 1025)).is_err());

        let oversized = vec![0; MAX_PNG_BYTES + 1];
        assert!(validate_png(&oversized).is_err());
    }

    #[test]
    fn rejects_missing_chunks_unknown_critical_and_apng() {
        let mut missing_iend = png_fixture(1, 1);
        missing_iend.truncate(missing_iend.len() - 12);
        assert!(validate_png(&missing_iend).is_err());

        let mut unknown = png_fixture(1, 1);
        let iend = unknown.split_off(unknown.len() - 12);
        push_test_chunk(&mut unknown, b"ABCD", &[]);
        unknown.extend_from_slice(&iend);
        assert!(validate_png(&unknown).is_err());

        let mut apng = png_fixture(1, 1);
        let iend = apng.split_off(apng.len() - 12);
        push_test_chunk(&mut apng, b"acTL", &[0; 8]);
        apng.extend_from_slice(&iend);
        assert!(validate_png(&apng).is_err());
    }

    #[test]
    fn enforces_palette_structure() {
        let mut indexed = png_fixture(1, 1);
        indexed[25] = 3;
        rewrite_chunk_crc(&mut indexed, 8);
        assert!(validate_png(&indexed).is_err());

        let mut grayscale_with_palette = png_fixture(1, 1);
        grayscale_with_palette[25] = 0;
        rewrite_chunk_crc(&mut grayscale_with_palette, 8);
        let idat_offset = 8 + 12 + 13;
        let tail = grayscale_with_palette.split_off(idat_offset);
        push_test_chunk(&mut grayscale_with_palette, b"PLTE", &[0, 0, 0]);
        grayscale_with_palette.extend_from_slice(&tail);
        assert!(validate_png(&grayscale_with_palette).is_err());

        let mut duplicate_palette = png_fixture(1, 1);
        let tail = duplicate_palette.split_off(idat_offset);
        push_test_chunk(&mut duplicate_palette, b"PLTE", &[0, 0, 0]);
        push_test_chunk(&mut duplicate_palette, b"PLTE", &[1, 1, 1]);
        duplicate_palette.extend_from_slice(&tail);
        assert!(validate_png(&duplicate_palette).is_err());

        let mut invalid_palette = png_fixture(1, 1);
        let tail = invalid_palette.split_off(idat_offset);
        push_test_chunk(&mut invalid_palette, b"PLTE", &[0, 0]);
        invalid_palette.extend_from_slice(&tail);
        assert!(validate_png(&invalid_palette).is_err());

        let mut before_ihdr = PNG_SIGNATURE.to_vec();
        push_test_chunk(&mut before_ihdr, b"tEXt", b"key\0value");
        before_ihdr.extend_from_slice(&png_fixture(1, 1)[PNG_SIGNATURE.len()..]);
        assert!(validate_png(&before_ihdr).is_err());
    }

    fn push_test_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(chunk_type);
        out.extend_from_slice(data);
        out.extend_from_slice(&crc32(&[chunk_type.as_slice(), data].concat()).to_be_bytes());
    }

    fn rewrite_chunk_crc(bytes: &mut [u8], offset: usize) {
        let length = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        let data_end = offset + 8 + length;
        let crc = crc32(&bytes[offset + 4..data_end]);
        bytes[data_end..data_end + 4].copy_from_slice(&crc.to_be_bytes());
    }
}
