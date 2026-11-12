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
    fingerprint_extend(FINGERPRINT_BASIS, bytes)
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

/// Where the stock archive's bytes come from: the slice the game just decompressed, or (decision B3: another mod replaced
/// the archive) the vanilla file read through `arc:/` in chunks, so no copy of it is ever held.
pub trait Stock {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// FNV-1a of the whole archive.
    fn fingerprint(&mut self) -> Result<u64>;
    /// Fill `out` with the bytes at `offset` (the caller checked the range against `len`).
    fn read_at(&mut self, offset: usize, out: &mut [u8]) -> Result<()>;
}

impl Stock for &[u8] {
    fn len(&self) -> usize {
        <[u8]>::len(self)
    }
    fn fingerprint(&mut self) -> Result<u64> {
        Ok(fingerprint(self))
    }
    fn read_at(&mut self, offset: usize, out: &mut [u8]) -> Result<()> {
        out.copy_from_slice(self.get(offset..offset + out.len()).ok_or("archive copy outside stock")?);
        Ok(())
    }
}

/// Continue an FNV-1a hash over more bytes (`fingerprint` is `fingerprint_extend(FINGERPRINT_BASIS, bytes)`).
pub fn fingerprint_extend(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3))
}

/// The FNV-1a offset basis (the fingerprint of nothing).
pub const FINGERPRINT_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// Reconstruct the plugin's archive from `stock`. Never accepts a partial, corrupted or mismatched
/// recipe, a changed stock archive, or an output that does not match its fingerprint.
#[cfg(test)]
pub fn build(stock: &[u8], recipe: &[u8]) -> Result<Vec<u8>> {
    let size = header(recipe)?.output_len;
    if !(4..=MAX_ARCHIVE_BYTES).contains(&size) {
        return Err("invalid archive output size");
    }
    let mut output = Vec::with_capacity(size);
    build_into(&mut { stock }, recipe, &mut output)?;
    Ok(output)
}

/// As `build`, into a buffer that already has the room: the plugin makes its buffers once at boot (outside any hook) and
/// reuses them for every load, so building allocates nothing. The output never grows `output`: a recipe that needs more
/// than its capacity is refused. On an error `output` is left empty.
pub fn build_into(stock: &mut impl Stock, recipe: &[u8], output: &mut Vec<u8>) -> Result<()> {
    output.clear();
    let result = apply(stock, recipe, output);
    if result.is_err() {
        output.clear();
    }
    result
}

fn apply(stock: &mut impl Stock, recipe: &[u8], output: &mut Vec<u8>) -> Result<()> {
    let head = header(recipe)?;
    let mut recipe = &recipe[4 + 4 + 8 + 4 + 8..];
    if stock.len() != head.stock_len || stock.fingerprint()? != head.stock_fingerprint {
        return Err("stock layout archive mismatch");
    }
    let size = head.output_len;
    if !(4..=MAX_ARCHIVE_BYTES).contains(&size) {
        return Err("invalid archive output size");
    }
    if size > output.capacity() {
        return Err("archive output larger than the boot buffer");
    }
    while !recipe.is_empty() {
        let [op] = read(&mut recipe)?;
        let len = u32::from_le_bytes(read(&mut recipe)?) as usize;
        if len > size - output.len() {
            return Err("archive output exceeds declared size");
        }
        match op {
            0 => {
                let offset = u32::from_le_bytes(read(&mut recipe)?) as usize;
                if offset.checked_add(len).is_none_or(|end| end > stock.len()) {
                    return Err("archive copy outside stock");
                }
                let start = output.len();
                output.resize(start + len, 0);
                stock.read_at(offset, &mut output[start..])?;
            }
            2 => output.extend_from_slice(take(&mut recipe, len)?),
            _ => return Err("invalid archive recipe operation"),
        }
    }
    if output.len() != size || fingerprint(output) != head.output_fingerprint {
        return Err("archive reconstruction mismatch");
    }
    Ok(())
}

/// Write a recipe that turns `stock` into `target` (the host builder and the tests). A greedy matcher:
/// 32-byte chunks of the stock archive are indexed every 16 bytes; at each target position the longest
/// run that matches a stock chunk becomes a copy, anything else a literal. Not used on the Switch.
#[cfg(test)]
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
    lib_unit_tests! {
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

    fn a_made_recipe_rebuilds_the_target_and_holds_only_the_changed_bytes() {
        let stock = stock();
        let target = target(&stock);
        let recipe = make(&stock, &target);
        assert_eq!(build(&stock, &recipe), Ok(target));
        assert!(recipe.len() < 200, "recipe is {} bytes", recipe.len());
    }

    fn a_changed_stock_archive_is_refused() {
        let stock = stock();
        let recipe = make(&stock, &target(&stock));
        let mut other = stock.clone();
        other[17] ^= 1;
        assert_eq!(build(&other, &recipe), Err("stock layout archive mismatch"));
        assert_eq!(build(&stock[..4000], &recipe), Err("stock layout archive mismatch"));
    }

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

    fn the_header_reads_without_applying() {
        let stock = stock();
        let target = target(&stock);
        let head = header(&make(&stock, &target)).unwrap();
        assert_eq!((head.stock_len, head.output_len), (stock.len(), target.len()));
        assert_eq!(head.output_fingerprint, fingerprint(&target));
    }

    /// R2-7 (decided 2026-11-02): the plugin builds its archive into buffers made once at boot, so a hand-out allocates
    /// nothing. This is the bound: the shipped recipe's output and stock sizes, and that `build_into` neither grows nor
    /// moves the buffer it is given.
    fn the_shipped_recipe_fits_the_boot_buffers_and_building_never_allocates() {
        let recipe: &[u8] = include_bytes!("../../assets/info_training.tra1");
        let head = header(recipe).unwrap();
        assert_eq!(head.stock_len, 451_604, "the stock Training layout archive of 13.0.5");
        assert!(head.output_len <= MAX_ARCHIVE_BYTES);
        // A synthetic stock of the right length would not match its fingerprint: use a small recipe of the same shape.
        let stock: Vec<u8> = (0..4096u32).map(|i| (i * 7 + i / 13) as u8).collect();
        let mut target = stock.clone();
        target.extend_from_slice(b"plugin panes");
        target[100] ^= 0xff;
        let small = make(&stock, &target);
        let mut output = Vec::with_capacity(header(&small).unwrap().output_len);
        let (pointer, capacity) = (output.as_ptr(), output.capacity());
        let mut source: &[u8] = &stock;
        build_into(&mut source, &small, &mut output).unwrap();
        assert_eq!(output, target);
        assert_eq!((output.as_ptr(), output.capacity()), (pointer, capacity), "no reallocation");
        // Built again into the same buffer (the next load): same result, still no reallocation.
        build_into(&mut source, &small, &mut output).unwrap();
        assert_eq!((output.as_ptr(), output.capacity()), (pointer, capacity));
        // A buffer too small is refused, never grown.
        let mut tiny: Vec<u8> = Vec::with_capacity(16);
        assert_eq!(build_into(&mut source, &small, &mut tiny), Err("archive output larger than the boot buffer"));
        assert!(tiny.is_empty() && tiny.capacity() < target.len());
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_made_recipe_rebuilds_the_target_and_holds_only_the_changed_bytes", a_made_recipe_rebuilds_the_target_and_holds_only_the_changed_bytes as fn()),
            ("a_changed_stock_archive_is_refused", a_changed_stock_archive_is_refused as fn()),
            ("a_partial_or_corrupted_recipe_is_never_accepted", a_partial_or_corrupted_recipe_is_never_accepted as fn()),
            ("the_header_reads_without_applying", the_header_reads_without_applying as fn()),
            ("the_shipped_recipe_fits_the_boot_buffers_and_building_never_allocates", the_shipped_recipe_fits_the_boot_buffers_and_building_never_allocates as fn()),
        ]);
    }
    }
}
