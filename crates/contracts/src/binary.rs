//! Immutable V1 terrain-grid bulk transfer. Only native-registry bytes, never arbitrary UI paths.
use crate::{validation, ElevationGridV1};
use sha2::{Digest, Sha256};

const MAGIC: &[u8] = b"M2M-GRID-BIN-1\0";
const MAX_SAMPLES: usize = 1_000_000;

pub type DecodedGridPayload = (usize, usize, Vec<Option<f64>>, Vec<bool>);

fn read_u32(data: &[u8], position: &mut usize) -> Result<u32, String> {
    let end = position
        .checked_add(4)
        .ok_or("INCOMPLETE_GRID: offset overflow")?;
    let buf: [u8; 4] = data
        .get(*position..end)
        .ok_or("INCOMPLETE_GRID: header truncated")?
        .try_into()
        .map_err(|_| "INCOMPLETE_GRID: u32 malformed")?;
    *position = end;
    Ok(u32::from_le_bytes(buf))
}
pub fn decode_grid_binary(
    data: &[u8],
) -> Result<DecodedGridPayload, String> {
    if !data.starts_with(MAGIC) {
        return Err("INVALID_SOURCE: grid magic invalid".into());
    }
    let mut p = MAGIC.len();
    let width = read_u32(data, &mut p)? as usize;
    let height = read_u32(data, &mut p)? as usize;
    let count = read_u32(data, &mut p)? as usize;
    let actual = width
        .checked_mul(height)
        .ok_or("OUT_OF_MEMORY: shape overflow")?;
    if width < 2
        || height < 2
        || width > 4096
        || height > 4096
        || count != actual
        || count > MAX_SAMPLES
    {
        return Err("OUT_OF_MEMORY: invalid grid dimensions".into());
    }
    let mut samples = Vec::with_capacity(count);
    let mut mask = Vec::with_capacity(count);
    for _ in 0..count {
        let flag = *data.get(p).ok_or("INCOMPLETE_GRID: mask truncated")?;
        p += 1;
        match flag {
            0 => {
                mask.push(false);
                samples.push(None);
            }
            1 => {
                let end = p.checked_add(8).ok_or("INCOMPLETE_GRID: sample overflow")?;
                let bytes: [u8; 8] = data
                    .get(p..end)
                    .ok_or("INCOMPLETE_GRID: sample truncated")?
                    .try_into()
                    .map_err(|_| "INCOMPLETE_GRID: invalid f64")?;
                let z = f64::from_le_bytes(bytes);
                if !z.is_finite() {
                    return Err("NONFINITE_ELEVATION: invalid sample".into());
                }
                samples.push(Some(if z == 0.0 { 0.0 } else { z }));
                mask.push(true);
                p = end;
            }
            _ => return Err("INVALID_SOURCE: unsupported mask byte".into()),
        }
    }
    if p != data.len() {
        return Err("INVALID_SOURCE: trailing binary grid bytes".into());
    }
    Ok((width, height, samples, mask))
}
pub fn encode_grid_binary(grid: &ElevationGridV1) -> Result<Vec<u8>, String> {
    let total = grid.width.checked_mul(grid.height).ok_or("OUT_OF_MEMORY")?;
    if total > MAX_SAMPLES
        || grid.width < 2
        || grid.height < 2
        || grid.width > 4096
        || grid.height > 4096
    {
        return Err("OUT_OF_MEMORY: grid shape invalid".into());
    }
    let samples = grid
        .samples_meters
        .as_ref()
        .ok_or("INCOMPLETE_GRID: no samples")?;
    let masks = grid.valid_mask.as_ref().ok_or("INCOMPLETE_GRID: no mask")?;
    if samples.len() != total || masks.len() != total {
        return Err("INCOMPLETE_GRID: shape and samples disagree".into());
    }
    let mut result = Vec::with_capacity(MAGIC.len() + 12 + 9 * total);
    result.extend_from_slice(MAGIC);
    for n in [grid.width, grid.height, total] {
        let value: u32 = n.try_into().map_err(|_| "OUT_OF_MEMORY: u32 overflow")?;
        result.extend_from_slice(&value.to_le_bytes());
    }
    for (sample, valid) in samples.iter().zip(masks.iter()) {
        match (sample, valid) {
            (None, false) => result.push(0),
            (Some(z), true) if z.is_finite() => {
                result.push(1);
                result.extend_from_slice(&(if *z == 0.0 { 0.0 } else { *z }).to_le_bytes());
            }
            _ => return Err("INCOMPLETE_GRID: invalid mask/sample".into()),
        }
    }
    Ok(result)
}
/// The caller must obtain `bytes` from a permissioned native asset registry.
/// This validates both byte-level identity and canonical decoded grid identity.
pub fn hydrate_verified_grid(
    control: &ElevationGridV1,
    bytes: &[u8],
) -> Result<ElevationGridV1, String> {
    if control.storage_mode != "local-binary-token"
        || control.samples_meters.is_some()
        || control.valid_mask.is_some()
    {
        return Err("INVALID_SOURCE: expected unresolved token envelope".into());
    }
    let expected = control
        .grid_data_sha256
        .as_ref()
        .ok_or("INVALID_SOURCE: missing grid SHA")?;
    if control.grid_data_token.as_deref() != Some(format!("m2m-grid:{expected}").as_str()) {
        return Err("INVALID_SOURCE: grid token differs from digest".into());
    }
    if bytes.len() > 9_000_027 {
        return Err("OUT_OF_MEMORY: grid asset too large".into());
    }
    let actual = format!("{:x}", Sha256::digest(bytes));
    if &actual != expected {
        return Err("HASH_MISMATCH: byte-level grid SHA mismatch".into());
    }
    let (width, height, samples, mask) = decode_grid_binary(bytes)?;
    if width != control.width || height != control.height {
        return Err("INCOMPLETE_GRID: binary shape differs from token envelope".into());
    }
    let mut hydrated = control.clone();
    hydrated.storage_mode = "inline-fixture".into();
    hydrated.grid_data_sha256 = None;
    hydrated.grid_data_token = None;
    hydrated.samples_meters = Some(samples);
    hydrated.valid_mask = Some(mask);
    validation::validate_grid(&hydrated)?;
    Ok(hydrated)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_grid_transport_positive_negative_and_corrupt_vectors() {
        let input: ElevationGridV1 = serde_json::from_str(include_str!(
            "../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.json"
        ))
        .unwrap();
        let binary =
            include_bytes!("../../../tests/fixtures/contracts/v1/ElevationGridV1.valid.bin");
        let mut token = input.clone();
        token.storage_mode = "local-binary-token".into();
        token.samples_meters = None;
        token.valid_mask = None;
        token.grid_data_sha256 =
            Some("f5e61b6c143b3a3d7027eae439b37f4450f53eeaa8ec853de157486d4b69ae4e".into());
        token.grid_data_token = token
            .grid_data_sha256
            .as_ref()
            .map(|v| format!("m2m-grid:{v}"));
        assert!(validation::validate_grid(&token).is_err()); // Not geometry-ready.
        assert_eq!(encode_grid_binary(&input).unwrap().as_slice(), binary);
        let verified = hydrate_verified_grid(&token, binary).unwrap();
        assert_eq!(verified.samples_meters, input.samples_meters);
        let mut corrupt = binary.to_vec();
        *corrupt.last_mut().unwrap() ^= 0x01;
        assert!(hydrate_verified_grid(&token, &corrupt).is_err());
        let missing =
            include_bytes!("../../../tests/fixtures/contracts/v1/ElevationGridV1.missing.bin");
        assert_eq!(
            format!("{:x}", Sha256::digest(missing)),
            "a53e70baa44e52c0e1af3a81242c9cc78f55604748bd76c406a6a0edc05e8955"
        );
    }
}
