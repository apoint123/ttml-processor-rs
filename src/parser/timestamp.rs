/// 解析给定的时间戳字节数组为毫秒
///
/// 严格按照 Apple Music 会使用的时间戳格式来解析
///
/// # Returns
///
/// 如果时间戳解析失败，将返回 None，否则返回毫秒数
#[inline(never)]
pub fn parse_timestamp(bytes: &[u8]) -> Option<u32> {
    let mut accum = 0u32;
    let mut current = 0u32;
    let mut has_fraction = false;
    let mut colons = 0;

    let mut iter = bytes.iter().copied();
    for b in iter.by_ref() {
        match b {
            b'0'..=b'9' => {
                current = current * 10 + u32::from(b - b'0');
            }
            b':' => {
                if colons >= 2 {
                    return None;
                }
                accum = (accum + current) * 60;
                current = 0;
                colons += 1;
            }
            b'.' => {
                accum += current;
                has_fraction = true;
                break;
            }
            _ => return None,
        }
    }

    let mut total_ms = if has_fraction {
        let mut fraction = 0u32;
        let mut digits = 0;

        for b in iter {
            match b {
                b'0'..=b'9' => {
                    if digits < 3 {
                        fraction = fraction * 10 + u32::from(b - b'0');
                        digits += 1;
                    }
                }
                _ => return None,
            }
        }

        if digits == 1 {
            fraction *= 100;
        } else if digits == 2 {
            fraction *= 10;
        }
        fraction
    } else {
        accum += current;
        0
    };

    total_ms += accum * 1000;

    Some(total_ms)
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

        assert!(parse_timestamp(b"invalid").is_none());
    }
}
