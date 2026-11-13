pub fn parse_bits(s: &str, expected: usize, label: &str) -> Result<Vec<u8>, String> {
    let v: Vec<u8> = s
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .map(|b| match b {
            b'0' => Ok(0),
            b'1' => Ok(1),
            _ => Err(format!("{label} must contain only 0 and 1")),
        })
        .collect::<Result<_, _>>()?;
    if v.len() != expected {
        return Err(format!(
            "{label} length is {}, expected {expected}",
            v.len()
        ));
    }
    Ok(v)
}
pub fn parse_bit_frames(s: &str, expected: usize, label: &str) -> Result<Vec<Vec<u8>>, String> {
    let frames = s
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| parse_bits(line, expected, label))
        .collect::<Result<Vec<_>, _>>()?;
    if frames.is_empty() {
        return Err(format!("{label} file contains no non-empty frames"));
    }
    Ok(frames)
}
