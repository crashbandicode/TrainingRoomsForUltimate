//! The layout archive recipe: the plugin's own `info_training` archive is the player's stock archive
//! (the bytes the game just decompressed, handed to the hook) plus the plugin's panes, so the repository
//! and the NRO hold only the changed bytes and ranges to copy, never a game archive (SPEC-core-07 §4.2
//! and question 2; SPEC-core-05 §4.2).
//!
//! The format is the earlier fork's `TMW1` waiting-room recipe (`waiting_archive.rs`, ours-new, carried
//! here) reduced to one source: the stock archive. Clean-room changes: one source instead of two, the
//! magic is `TRA1`, the bound is this archive's, and the builder's `fingerprint` is shared.
//!
//! Layout: `TRA1`, then `len: u32` and `fingerprint: u64` of the stock archive and of the output, then
//! operations until the end: `0, len: u32, offset: u32` copy `len` stock bytes from `offset`;
//! `2, len: u32, bytes...` insert literal bytes. (Tag 1 is unused: it was the donor of the fork's recipe.)
//! A recipe is applied only to the exact stock archive it was made for, and the output is verified.

use std::convert::TryInto;

type Result<T> = std::result::Result<T, &'static str>;

/// The bound on the output (the stock archive is about 1.3 MB; the plugin's additions are small).
pub const MAX_ARCHIVE_BYTES: usize = 9 * 1024 * 1024;

pub const MAGIC: [u8; 4] = *b"TRA1";

fn take<'a>(bytes: &mut &'a [u8], len: usize) -> Result<&'a [u8]> {
    let value = bytes.get(..len).ok_or("truncated archive recipe")?;
    *bytes = &bytes[len..];
    Ok(value)
}

fn read<const N: usize>(bytes: &mut &[u8]) -> Result<[u8; N]> {
    Ok(take(bytes, N)?.try_into().expect("fixed-width field"))
}

/// An identity check for versioned resources (FNV-1a, 64 bit), not a cryptographic authenticity check.
pub fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3))
}

/// What a recipe says about its inputs, read without applying it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub stock_len: usize,
    pub stock_fingerprint: u64,
    pub output_len: usize,
    pub output_fingerprint: u64,
}

pub fn header(mut recipe: &[u8]) -> Result<Header> {
    if read::<4>(&mut recipe)? != MAGIC {
        return Err("unsupported archive recipe");
    }
    let stock_len = u32::from_le_bytes(read(&mut recipe)?) as usize;
    let stock_fingerprint = u64::from_le_bytes(read(&mut recipe)?);
    let output_len = u32::from_le_bytes(read(&mut recipe)?) as usize;
    let output_fingerprint = u64::from_le_bytes(read(&mut recipe)?);
    Ok(Header { stock_len, stock_fingerprint, output_len, output_fingerprint })
}

/// Reconstruct the plugin's archive from `stock`. Never accepts a partial, corrupted or mismatched
/// recipe, a changed stock archive, or an output that does not match its fingerprint.
pub fn build(stock: &[u8], recipe: &[u8]) -> Result<Vec<u8>> {
    let head = header(recipe)?;
    let mut recipe = &recipe[4 + 4 + 8 + 4 + 8..];
    if stock.len() != head.stock_len || fingerprint(stock) != head.stock_fingerprint {
        return Err("stock layout archive mismatch");
    }
    let size = head.output_len;
    if !(4..=MAX_ARCHIVE_BYTES).contains(&size) {
        return Err("invalid archive output size");
    }
    let mut output = Vec::with_capacity(size);
    while !recipe.is_empty() {
        let [op] = read(&mut recipe)?;
        let len = u32::from_le_bytes(read(&mut recipe)?) as usize;
        if len > size - output.len() {
            return Err("archive output exceeds declared size");
        }
        let bytes = match op {
            0 => {
                let offset = u32::from_le_bytes(read(&mut recipe)?) as usize;
                stock.get(offset..).and_then(|bytes| bytes.get(..len)).ok_or("archive copy outside stock")?
            }
            2 => take(&mut recipe, len)?,
            _ => return Err("invalid archive recipe operation"),
        };
        output.extend_from_slice(bytes);
    }
    if output.len() != size || fingerprint(&output) != head.output_fingerprint {
        return Err("archive reconstruction mismatch");
    }
    Ok(output)
}

/// Write a recipe that turns `stock` into `target` (the host builder and the tests). A greedy matcher:
/// 32-byte chunks of the stock archive are indexed every 16 bytes; at each target position the longest
/// run that matches a stock chunk becomes a copy, anything else a literal. Not used on the Switch.
pub fn make(stock: &[u8], target: &[u8]) -> Vec<u8> {
    const CHUNK: usize = 32;
    const STRIDE: usize = 16;
    let mut index: std::collections::HashMap<&[u8], Vec<usize>> = std::collections::HashMap::new();
    let mut offset = 0;
    while offset + CHUNK <= stock.len() {
        let choices = index.entry(&stock[offset..offset + CHUNK]).or_default();
        // Early and recent positions per chunk cover common headers and zero runs.
        if choices.len() < 4 {
            choices.push(offset);
        } else {
            *choices.last_mut().expect("non-empty") = offset;
        }
        offset += STRIDE;
    }
    let mut recipe = MAGIC.to_vec();
    recipe.extend_from_slice(&(stock.len() as u32).to_le_bytes());
    recipe.extend_from_slice(&fingerprint(stock).to_le_bytes());
    recipe.extend_from_slice(&(target.len() as u32).to_le_bytes());
    recipe.extend_from_slice(&fingerprint(target).to_le_bytes());
    let mut literal: Vec<u8> = Vec::new();
    let flush = |recipe: &mut Vec<u8>, literal: &mut Vec<u8>| {
        if !literal.is_empty() {
            recipe.push(2);
            recipe.extend_from_slice(&(literal.len() as u32).to_le_bytes());
            recipe.extend_from_slice(literal);
            literal.clear();
        }
    };
    let mut pos = 0;
    while pos < target.len() {
        let mut best = (0usize, 0usize);
        if pos + CHUNK <= target.len() {
            if let Some(choices) = index.get(&target[pos..pos + CHUNK]) {
                for &at in choices {
                    let limit = (stock.len() - at).min(target.len() - pos);
                    let length = stock[at..at + limit].iter().zip(&target[pos..pos + limit]).take_while(|(a, b)| a == b).count();
                    if length > best.0 {
                        best = (length, at);
                    }
                }
            }
        }
        if best.0 >= CHUNK {
            flush(&mut recipe, &mut literal);
            recipe.push(0);
            recipe.extend_from_slice(&(best.0 as u32).to_le_bytes());
            recipe.extend_from_slice(&(best.1 as u32).to_le_bytes());
            pos += best.0;
        } else {
            literal.push(target[pos]);
            pos += 1;
        }
    }
    flush(&mut recipe, &mut literal);
    recipe
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stock() -> Vec<u8> {
        (0..4096u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect()
    }

    fn target(stock: &[u8]) -> Vec<u8> {
        let mut out = stock[..1000].to_vec();
        out.extend_from_slice(b"new panes inserted here");
        out.extend_from_slice(&stock[1000..3000]);
        out.extend_from_slice(&stock[3500..]);
        out
    }

    #[test]
    fn a_made_recipe_rebuilds_the_target_and_holds_only_the_changed_bytes() {
        let stock = stock();
        let target = target(&stock);
        let recipe = make(&stock, &target);
        assert_eq!(build(&stock, &recipe), Ok(target));
        assert!(recipe.len() < 200, "recipe is {} bytes", recipe.len());
    }

    #[test]
    fn a_changed_stock_archive_is_refused() {
        let stock = stock();
        let recipe = make(&stock, &target(&stock));
        let mut other = stock.clone();
        other[17] ^= 1;
        assert_eq!(build(&other, &recipe), Err("stock layout archive mismatch"));
        assert_eq!(build(&stock[..4000], &recipe), Err("stock layout archive mismatch"));
    }

    #[test]
    fn a_partial_or_corrupted_recipe_is_never_accepted() {
        let stock = stock();
        let recipe = make(&stock, &target(&stock));
        for end in 0..recipe.len() {
            assert!(build(&stock, &recipe[..end]).is_err(), "accepted a recipe cut at {end}");
        }
        for at in (0..recipe.len()).step_by(3) {
            let mut bad = recipe.clone();
            bad[at] ^= 0x80;
            assert!(build(&stock, &bad).is_err(), "accepted corruption at {at}");
        }
    }

    #[test]
    fn the_header_reads_without_applying() {
        let stock = stock();
        let target = target(&stock);
        let head = header(&make(&stock, &target)).unwrap();
        assert_eq!((head.stock_len, head.output_len), (stock.len(), target.len()));
        assert_eq!(head.output_fingerprint, fingerprint(&target));
    }
}