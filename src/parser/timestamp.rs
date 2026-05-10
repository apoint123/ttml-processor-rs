use crate::error::ParseErrorKind;

/// 解析给定的时间戳字节数组为毫秒
///
/// 严格按照 Apple Music 会使用的时间戳格式来解析
pub fn parse_timestamp(bytes: &[u8]) -> std::result::Result<u32, ParseErrorKind> {
    let mut parts = [0u32; 3];
    let mut part_idx = 0;
    let mut current = 0u32;
    let mut has_fraction = false;

    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'0'..=b'9' => {
                current = current * 10 + u32::from(b - b'0');
            }
            b':' => {
                if part_idx >= 2 {
                    return Err(ParseErrorKind::InvalidTimestamp(
                        String::from_utf8_lossy(bytes).into(),
                    ));
                }
                parts[part_idx] = current;
                part_idx += 1;
                current = 0;
            }
            b'.' => {
                parts[part_idx] = current;
                has_fraction = true;
                i += 1;
                break;
            }
            _ => {
                return Err(ParseErrorKind::InvalidTimestamp(
                    String::from_utf8_lossy(bytes).into(),
                ));
            }
        }
        i += 1;
    }

    let mut total_ms = 0;

    if has_fraction {
        let mut fraction = 0u32;
        let mut digits = 0;

        while i < bytes.len() {
            let b = bytes[i];
            match b {
                b'0'..=b'9' => {
                    if digits < 3 {
                        fraction = fraction * 10 + u32::from(b - b'0');
                        digits += 1;
                    }
                }
                _ => {
                    return Err(ParseErrorKind::InvalidTimestamp(
                        String::from_utf8_lossy(bytes).into(),
                    ));
                }
            }
            i += 1;
        }

        if digits == 1 {
            fraction *= 100;
        } else if digits == 2 {
            fraction *= 10;
        }
        total_ms += fraction;
    } else {
        parts[part_idx] = current;
    }

    total_ms += match part_idx {
        0 => parts[0] * 1000,
        1 => parts[0] * 60_000 + parts[1] * 1000,
        2 => parts[0] * 3_600_000 + parts[1] * 60_000 + parts[2] * 1000,
        _ => unreachable!(),
    };

    Ok(total_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        assert_eq!(parse_timestamp(b"1.152").unwrap(), 1152);
        assert_eq!(parse_timestamp(b"0.046").unwrap(), 46);
        assert_eq!(parse_timestamp(b"10.254").unwrap(), 10254);

        assert_eq!(parse_timestamp(b"3:36.120").unwrap(), 216_120);
        assert_eq!(parse_timestamp(b"1:00").unwrap(), 60000);

        assert_eq!(parse_timestamp(b"1:03:36.120").unwrap(), 3_816_120);

        assert!(matches!(
            parse_timestamp(b"invalid"),
            Err(ParseErrorKind::InvalidTimestamp(_))
        ));
    }
}
