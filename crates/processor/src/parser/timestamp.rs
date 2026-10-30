/// 解析给定的时间戳字节数组为毫秒
///
/// 严格按照 Apple Music 会使用的时间戳格式来解析
///
/// # Returns
///
/// 如果时间戳解析失败，将返回 None，否则返回毫秒数
#[inline]
pub fn parse_timestamp(bytes: &[u8]) -> Option<u32> {
    let mut accum = 0u32;
    let mut current = 0u32;
    let mut has_fraction = false;
    let mut colons = 0;

    let mut iter = bytes.iter().copied();
    for b in iter.by_ref() {
        match b {
            b'0'..=b'9' => {
                current = current.checked_mul(10)?.checked_add(u32::from(b - b'0'))?;
            }
            b':' => {
                if colons >= 2 {
                    return None;
                }
                accum = accum.checked_add(current)?.checked_mul(60)?;
                current = 0;
                colons += 1;
            }
            b'.' => {
                accum = accum.checked_add(current)?;
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
                        fraction = fraction.checked_mul(10)?.checked_add(u32::from(b - b'0'))?;
                        digits += 1;
                    }
                }
                _ => return None,
            }
        }

        if digits == 1 {
            fraction = fraction.checked_mul(100)?;
        } else if digits == 2 {
            fraction = fraction.checked_mul(10)?;
        }
        fraction
    } else {
        accum = accum.checked_add(current)?;
        0
    };

    total_ms = total_ms.checked_add(accum.checked_mul(1000)?)?;

    Some(total_ms)
}

/// 解析十进制秒数为毫秒
///
/// 与 [`parse_timestamp`] 不同，允许前导 `-` 或 `+`，且只接受 `"0"`、`"0.046"`、
/// `"-0.271"` 这样的十进制秒数，不接受时钟形式。小数超过 3 位时截断；
/// 空串、缺少数字、多个小数点等非法输入返回 `None`。
///
/// 内部复用 [`parse_timestamp`]，因此数值范围也与其一致，
/// 即绝对值不超过 `u32::MAX` 毫秒。
#[inline]
pub fn parse_decimal_seconds(bytes: &[u8]) -> Option<i64> {
    let (negative, digits) = match *bytes.first()? {
        b'-' => (true, &bytes[1..]),
        b'+' => (false, &bytes[1..]),
        _ => (false, bytes),
    };

    // 时钟形式（含冒号）不属于十进制秒数；同时排除仅有符号或小数点的输入
    if digits.contains(&b':') || !digits.iter().any(u8::is_ascii_digit) {
        return None;
    }

    let ms = i64::from(parse_timestamp(digits)?);

    Some(if negative { -ms } else { ms })
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
        assert_eq!(parse_timestamp(b"4294967.295").unwrap(), u32::MAX);

        assert!(parse_timestamp(b"4294967.296").is_none());
        assert!(parse_timestamp(b"99999999999999999999").is_none());
        assert!(parse_timestamp(b"invalid").is_none());
    }

    #[test]
    fn test_parse_decimal_seconds() {
        assert_eq!(parse_decimal_seconds(b"0"), Some(0));
        assert_eq!(parse_decimal_seconds(b"0.040"), Some(40));
        assert_eq!(parse_decimal_seconds(b"0.046"), Some(46));
        assert_eq!(parse_decimal_seconds(b"0.5"), Some(500));
        assert_eq!(parse_decimal_seconds(b"1.152"), Some(1152));
        assert_eq!(parse_decimal_seconds(b"-0.271"), Some(-271));
        assert_eq!(parse_decimal_seconds(b"+1.5"), Some(1500));
        assert_eq!(parse_decimal_seconds(b"1.2345"), Some(1234));
        assert_eq!(parse_decimal_seconds(b"4294967.295"), Some(u32::MAX.into()));

        assert!(parse_decimal_seconds(b"4294967.296").is_none());
        assert!(parse_decimal_seconds(b"").is_none());
        assert!(parse_decimal_seconds(b"+").is_none());
        assert!(parse_decimal_seconds(b"-").is_none());
        assert!(parse_decimal_seconds(b".").is_none());
        assert!(parse_decimal_seconds(b"1:02").is_none());
        assert!(parse_decimal_seconds(b"1.2.3").is_none());
        assert!(parse_decimal_seconds(b"invalid").is_none());
    }
}
