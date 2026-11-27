//! The overlay's command handoff: what its command hooks change without a lock, and what phase 8 takes back.
//!
//! The model's two tables of who holds what (the fighters' sets, `SetMembers`, and the article slots, `Members`) are small values
//! here. A command applies the model's own rule to a private copy and publishes it with one compare-and-swap, at command time
//! and in command order, as it did under the lock. What a set must be told later (a CATCH's grab spheres, a cleared rebound)
//! waits in that set's lane: as many grab commands as the set itself keeps (`MAX_PENDING`) and its newest rebound. A waiting
//! command is dead once its set was bound again or a later rebound came, and a dead cell is reused. So nothing queues without a
//! bound: every command changes a value or takes a cell its set has room for.
//!
//! A value's or a command's words sit in a buffer of the pool. A thread checks a buffer out, fills it while nobody else can
//! reach it, and publishes it with one compare-and-swap; whoever takes it back reads it and returns it. So a buffer is in a
//! cell, is one of the two current values, or is held by one thread between two of its own instructions, and a thread holds at
//! most one: `BUFFERS` always leaves one free. The pool keeps one bit a buffer; a checkout claims one of the free buffers by
//! count and then clears a set bit, and clearing it is the taking, so no buffer is handed out twice and nothing wraps or retires.
//! Nothing here locks, allocates after `new`, or waits: a compare-and-swap is retried only when another thread's finished.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::{AcqRel, Acquire, Relaxed}};

use super::hitbox::{FighterLife, SetMembers, Shape, FIGHTER_SETS, MAX_PENDING};
use super::hitbox_orbs::{Members, ARTICLES};

/// The words of one buffer: the fighters' bindings (three words a set and one for their order of use) are the largest value.
pub const WORDS: usize = 3 * FIGHTER_SETS + 1;
/// The threads one process can have (Eden `src/core/hle/kernel/k_resource_limit.cpp`: 800; Horizon gives an application fewer).
pub const THREADS: usize = 800;
/// The two current values, every cell's buffer (each lane's grabs, each fighter set's rebound) and one for every thread.
pub const BUFFERS: usize = 2 + ARTICLES * MAX_PENDING + FIGHTER_SETS * (MAX_PENDING + 1) + THREADS;

/// A word's low bits name a buffer (its index plus one, 0 for none); the rest is a generation, a version or a stamp.
const INDEX_BITS: u32 = 12;
const INDEX: u64 = (1 << INDEX_BITS) - 1;

/// Fixed buffers of `WORDS` atomic words and the free ones' bits.
pub struct Pool {
    words: Box<[AtomicU64]>,
    /// One bit a buffer, set while it is free.
    free: Box<[AtomicU64]>,
    /// The free buffers a checkout may still claim: never more than the set bits.
    available: AtomicU32,
}

impl Pool {
    /// `None` for no buffers, for more than a word can name, or when the allocation fails.
    pub fn new(capacity: usize) -> Option<Self> {
        if capacity == 0 || capacity > INDEX as usize { return None; }
        let (mut words, mut free) = (Vec::new(), Vec::new());
        words.try_reserve_exact(capacity * WORDS).ok()?;
        free.try_reserve_exact(capacity.div_ceil(64)).ok()?;
        words.resize_with(capacity * WORDS, || AtomicU64::new(0));
        for i in 0..capacity.div_ceil(64) {
            let count = (capacity - i * 64).min(64);
            let bits = if count == 64 { u64::MAX } else { (1u64 << count) - 1 };
            free.push(AtomicU64::new(bits));
        }
        Some(Self { words: words.into_boxed_slice(), free: free.into_boxed_slice(), available: AtomicU32::new(capacity as u32) })
    }

    pub fn capacity(&self) -> usize {
        self.words.len() / WORDS
    }

    /// The bytes `new` allocated.
    pub fn heap_bytes(&self) -> usize {
        std::mem::size_of_val(&*self.words) + std::mem::size_of_val(&*self.free)
    }

    /// Check a buffer out: `None` when every buffer is out.
    pub fn take(&self) -> Option<u32> {
        // Claim one of the free buffers first: then a set bit is this thread's to find, whatever the others take and return.
        self.available.fetch_update(AcqRel, Acquire, |free| free.checked_sub(1)).ok()?;
        loop {
            for (i, word) in self.free.iter().enumerate() {
                let mut bits = word.load(Acquire);
                while bits != 0 {
                    let bit = bits.trailing_zeros();
                    // Clearing the bit is the taking: only the thread that found it set has the buffer.
                    bits = word.fetch_and(!(1u64 << bit), AcqRel);
                    if bits & (1u64 << bit) != 0 { return Some(i as u32 * 64 + bit + 1); }
                }
            }
        }
    }

    /// Return a buffer its caller took out of a cell or checked out.
    pub fn give(&self, buffer: u32) {
        let index = buffer - 1;
        self.free[index as usize / 64].fetch_or(1u64 << (index % 64), AcqRel);
        self.available.fetch_add(1, AcqRel);
    }

    fn at(&self, buffer: u32) -> &[AtomicU64] {
        let first = (buffer as usize - 1) * WORDS;
        &self.words[first..first + WORDS]
    }

    /// Fill a buffer its caller holds.
    pub fn write(&self, buffer: u32, words: &[u64; WORDS]) {
        for (cell, word) in self.at(buffer).iter().zip(words) {
            cell.store(*word, Relaxed);
        }
    }

    /// A buffer's words: its holder's own, or a copy the caller checks against the word that named the buffer.
    pub fn read(&self, buffer: u32) -> [u64; WORDS] {
        let cells = self.at(buffer);
        std::array::from_fn(|i| cells[i].load(Relaxed))
    }
}

/// What a command hook leaves for its set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Record {
    /// A CATCH that passed its hook's gates, with the status it ran in: `life` is a fighter's stamp taken before the shape was
    /// read, `None` an article.
    Grab { key: usize, id: u32, life: Option<FighterLife>, shape: Shape, status: i32 },
    /// A fighter's rebound was cleared and its spheres go.
    Rebound { key: usize, life: FighterLife },
}

fn pair(low: f32, high: f32) -> u64 {
    u64::from(low.to_bits()) | (u64::from(high.to_bits()) << 32)
}

fn unpair(word: u64) -> [f32; 2] {
    [f32::from_bits(word as u32), f32::from_bits((word >> 32) as u32)]
}

impl Record {
    fn words(&self) -> [u64; WORDS] {
        let mut w = [0; WORDS];
        match *self {
            Record::Grab { key, id, life, shape, status } => {
                let b = shape.b.unwrap_or_default();
                w[0] = 1 | (u64::from(life.is_some()) << 8) | (u64::from(shape.b.is_some()) << 9) | (u64::from(status as u32) << 32);
                w[1] = key as u64;
                w[2] = life.map_or(u64::from(id), FighterLife::bits);
                w[3] = u64::from(shape.id as u32) | (u64::from(shape.size.to_bits()) << 32);
                w[4] = shape.joint;
                (w[5], w[6], w[7]) = (pair(shape.a[0], shape.a[1]), pair(shape.a[2], b[0]), pair(b[1], b[2]));
            }
            Record::Rebound { key, life } => (w[0], w[1], w[2]) = (2, key as u64, life.bits()),
        }
        w
    }

    fn read(w: &[u64; WORDS]) -> Option<Record> {
        let key = w[1] as usize;
        match w[0] & 0xff {
            1 => {
                let life = ((w[0] >> 8) & 1 != 0).then(|| FighterLife::from_bits(w[2]));
                let ([ax, ay], [az, bx], [by, bz]) = (unpair(w[5]), unpair(w[6]), unpair(w[7]));
                let shape = Shape {
                    id: w[3] as u32 as i32,
                    joint: w[4],
                    size: f32::from_bits((w[3] >> 32) as u32),
                    a: [ax, ay, az],
                    b: ((w[0] >> 9) & 1 != 0).then_some([bx, by, bz]),
                };
                Some(Record::Grab { key, id: w[2] as u32, life, shape, status: (w[0] >> 32) as u32 as i32 })
            }
            2 => Some(Record::Rebound { key, life: FighterLife::from_bits(w[2]) }),
            _ => None,
        }
    }
}

fn members_words(members: &Members) -> [u64; WORDS] {
    let mut w = [0; WORDS];
    for i in 0..ARTICLES {
        w[i] = members.ids[i].1 as u64;
        w[ARTICLES + i] = u64::from(members.ids[i].0) | (u64::from(members.attacking[i]) << 32);
        w[2 * ARTICLES + i] = members.ages[i];
        w[3 * ARTICLES + i] = members.touched[i];
    }
    w
}

fn members_read(w: &[u64; WORDS]) -> Members {
    let mut members = Members::default();
    for i in 0..ARTICLES {
        members.ids[i] = (w[ARTICLES + i] as u32, w[i] as usize);
        members.attacking[i] = (w[ARTICLES + i] >> 32) != 0;
        members.ages[i] = w[2 * ARTICLES + i];
        members.touched[i] = w[3 * ARTICLES + i];
    }
    members
}

/// The sets' versions start past this, so a stamp of use is always later than a rank read back from the words.
const FIRST_SET_VERSION: u64 = FIGHTER_SETS as u64;

/// The rule reads only the order the sets were used in: each set's rank in it (ties by index, as the rule breaks them) and
/// whether it has a lifetime share the last word, four bits a set.
fn sets_words(members: &SetMembers) -> [u64; WORDS] {
    let mut w = [0; WORDS];
    for i in 0..FIGHTER_SETS {
        w[i] = members.keys[i] as u64;
        w[FIGHTER_SETS + i] = members.lives[i].map_or(0, FighterLife::bits);
        w[2 * FIGHTER_SETS + i] = members.bound[i];
        let rank = (0..FIGHTER_SETS).filter(|&j| (members.used[j], j) < (members.used[i], i)).count() as u64;
        w[3 * FIGHTER_SETS] |= (rank | (u64::from(members.lives[i].is_some()) << 3)) << (4 * i);
    }
    w
}

fn sets_read(w: &[u64; WORDS]) -> SetMembers {
    let mut members = SetMembers::NONE;
    for i in 0..FIGHTER_SETS {
        let bits = w[3 * FIGHTER_SETS] >> (4 * i);
        members.keys[i] = w[i] as usize;
        members.lives[i] = (bits & 8 != 0).then(|| FighterLife::from_bits(w[FIGHTER_SETS + i]));
        members.bound[i] = w[2 * FIGHTER_SETS + i];
        members.used[i] = bits & 7;
    }
    members
}

/// What became of a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Put {
    /// It changed its table, and what its set must be told waits for phase 8.
    Stored,
    /// Its lane holds `MAX_PENDING` live grab commands: the set's own queue would refuse it.
    Full,
    /// No buffer was free, which `BUFFERS` rules out.
    NoBuffer,
}

/// One set's waiting commands. A cell is 0 or a buffer and its command's stamp (its table's version).
struct Lane {
    /// Grab cells stamped below this are dead: a later rebound came.
    floor: AtomicU64,
    /// Raised after every take and every rise of the floor.
    changes: AtomicU64,
    rebound: AtomicU64,
    cells: [AtomicU64; MAX_PENDING],
}

impl Lane {
    fn new() -> Self {
        Lane { floor: AtomicU64::new(0), changes: AtomicU64::new(0), rebound: AtomicU64::new(0), cells: [const { AtomicU64::new(0) }; MAX_PENDING] }
    }

    /// Install a grab command of the set bound at `since` in a cell that is empty or dead (stamped before `since` or below the floor).
    fn put(&self, pool: &Pool, stamp: u64, since: u64, words: &[u64; WORDS]) -> Put {
        let Some(buffer) = pool.take() else { return Put::NoBuffer };
        pool.write(buffer, words);
        let word = u64::from(buffer) | (stamp << INDEX_BITS);
        loop {
            let changes = self.changes.load(Acquire);
            let floor = since.max(self.floor.load(Acquire));
            for cell in &self.cells {
                let mut old = cell.load(Acquire);
                while old == 0 || (old >> INDEX_BITS) < floor {
                    match cell.compare_exchange(old, word, AcqRel, Acquire) {
                        Ok(_) => {
                            if old != 0 { pool.give((old & INDEX) as u32); }
                            return Put::Stored;
                        }
                        Err(now) => old = now,
                    }
                }
            }
            // Every cell was live when it was looked at. Without a take or a rise of the floor since, those commands are all
            // still waiting (here or in the set phase 8 moved them to), and the set's queue is as full.
            if self.changes.load(Acquire) == changes {
                pool.give(buffer);
                return Put::Full;
            }
        }
    }

    /// The newest rebound wins, and every grab before it is dead.
    fn put_rebound(&self, pool: &Pool, stamp: u64, words: &[u64; WORDS]) -> Put {
        let Some(buffer) = pool.take() else { return Put::NoBuffer };
        pool.write(buffer, words);
        let word = u64::from(buffer) | (stamp << INDEX_BITS);
        let mut old = self.rebound.load(Acquire);
        loop {
            if (old >> INDEX_BITS) > stamp {
                pool.give(buffer);
                break;
            }
            match self.rebound.compare_exchange(old, word, AcqRel, Acquire) {
                Ok(_) => {
                    if old != 0 { pool.give((old & INDEX) as u32); }
                    break;
                }
                Err(now) => old = now,
            }
        }
        self.floor.fetch_max(stamp + 1, AcqRel);
        self.changes.fetch_add(1, AcqRel);
        Put::Stored
    }

    /// The stamp of the oldest command waiting for the set bound at `since`, up to `limit`.
    fn oldest(&self, since: u64, limit: u64) -> Option<u64> {
        let floor = since.max(self.floor.load(Acquire));
        let rebound = Some(self.rebound.load(Acquire)).filter(|word| (word >> INDEX_BITS) >= since);
        let cells = self.cells.iter().map(|cell| cell.load(Acquire)).filter(|word| (word >> INDEX_BITS) >= floor);
        rebound.into_iter().chain(cells).filter(|word| *word != 0).map(|word| word >> INDEX_BITS).filter(|stamp| *stamp <= limit).min()
    }

    /// Take the command stamped `stamp` out: `None` when it is no longer there.
    fn take(&self, pool: &Pool, stamp: u64) -> Option<[u64; WORDS]> {
        for cell in std::iter::once(&self.rebound).chain(&self.cells) {
            let old = cell.load(Acquire);
            if old != 0 && (old >> INDEX_BITS) == stamp && cell.compare_exchange(old, 0, AcqRel, Relaxed).is_ok() {
                self.changes.fetch_add(1, AcqRel);
                let buffer = (old & INDEX) as u32;
                let words = pool.read(buffer);
                pool.give(buffer);
                return Some(words);
            }
        }
        None
    }
}

/// The pool, the two tables' current values and a lane for every fighter set and article slot.
pub struct Handoff {
    pool: Pool,
    /// The buffer that holds the fighters' bindings and their version, which every change raises.
    sets: AtomicU64,
    /// The same for the article slots.
    articles: AtomicU64,
    /// The fighter sets' lanes, then the article slots'.
    lanes: Box<[Lane]>,
}

impl std::fmt::Debug for Handoff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Handoff")
    }
}

impl Handoff {
    /// Everything is allocated here. `None` when an allocation fails.
    pub fn new() -> Option<Self> {
        let pool = Pool::new(BUFFERS)?;
        let mut lanes = Vec::new();
        lanes.try_reserve_exact(FIGHTER_SETS + ARTICLES).ok()?;
        lanes.resize_with(FIGHTER_SETS + ARTICLES, Lane::new);
        let (sets, articles) = (pool.take()?, pool.take()?);
        pool.write(sets, &sets_words(&SetMembers::NONE));
        pool.write(articles, &members_words(&Members::default()));
        Some(Self {
            pool,
            sets: AtomicU64::new(u64::from(sets) | (FIRST_SET_VERSION << INDEX_BITS)),
            articles: AtomicU64::new(u64::from(articles)),
            lanes: lanes.into_boxed_slice(),
        })
    }

    /// The bytes `new` allocated.
    pub fn heap_bytes(&self) -> usize {
        self.pool.heap_bytes() + std::mem::size_of_val(&*self.lanes)
    }

    /// A value's words and its version.
    fn words_of(&self, current: &AtomicU64) -> (u64, [u64; WORDS]) {
        loop {
            let word = current.load(Acquire);
            let words = self.pool.read((word & INDEX) as u32);
            // The buffer was the current one throughout, so nobody wrote to it.
            if current.load(Acquire) == word { return (word >> INDEX_BITS, words); }
        }
    }

    /// Apply `op` to a value's words and publish the result as the next version, which `op` is given: that version. `op` runs
    /// again when another change came first. `None` when no buffer was free.
    fn change_words(&self, current: &AtomicU64, op: &mut dyn FnMut(&mut [u64; WORDS], u64)) -> Option<u64> {
        loop {
            let word = current.load(Acquire);
            let old = (word & INDEX) as u32;
            let before = self.pool.read(old);
            if current.load(Acquire) != word { continue; }
            let version = (word >> INDEX_BITS) + 1;
            let mut after = before;
            op(&mut after, version);
            // A changed value goes into a buffer of its own; an unchanged one keeps the buffer and only takes the next version.
            let fresh = if after == before {
                None
            } else {
                let buffer = self.pool.take()?;
                self.pool.write(buffer, &after);
                Some(buffer)
            };
            if current.compare_exchange(word, u64::from(fresh.unwrap_or(old)) | (version << INDEX_BITS), AcqRel, Acquire).is_ok() {
                if fresh.is_some() { self.pool.give(old); }
                return Some(version);
            }
            // Another change came first. The checkout goes back even when it is the very buffer that was current before.
            if let Some(buffer) = fresh { self.pool.give(buffer); }
        }
    }

    /// The fighters' bindings and their version.
    pub fn sets(&self) -> (u64, SetMembers) {
        let (version, words) = self.words_of(&self.sets);
        (version, sets_read(&words))
    }

    /// Change the fighters' bindings (`change_words`).
    pub fn change_sets<R>(&self, mut op: impl FnMut(&mut SetMembers, u64) -> R) -> Option<(u64, SetMembers, R)> {
        let mut out = None;
        let version = self.change_words(&self.sets, &mut |words, version| {
            let mut members = sets_read(words);
            let result = op(&mut members, version);
            *words = sets_words(&members);
            out = Some((members, result));
        })?;
        out.map(|(members, result)| (version, members, result))
    }

    /// The article slots and their version.
    pub fn articles(&self) -> (u64, Members) {
        let (version, words) = self.words_of(&self.articles);
        (version, members_read(&words))
    }

    /// Change the article slots (`change_words`).
    pub fn change_articles<R>(&self, mut op: impl FnMut(&mut Members, u64) -> R) -> Option<(u64, Members, R)> {
        let mut out = None;
        let version = self.change_words(&self.articles, &mut |words, version| {
            let mut members = members_read(words);
            let result = op(&mut members, version);
            *words = members_words(&members);
            out = Some((members, result));
        })?;
        out.map(|(members, result)| (version, members, result))
    }

    /// A fighter's CATCH: its set is found or taken over now, as under the lock, and the grab waits in the set's lane.
    pub fn catch_command(&self, key: usize, life: FighterLife, shape: Shape, status: i32) -> Put {
        let Some((version, members, (set, _))) = self.change_sets(|members, version| members.index_for(key, life, version)) else {
            return Put::NoBuffer;
        };
        self.lanes[set].put(&self.pool, version, members.bound[set], &Record::Grab { key, id: life.id, life: Some(life), shape, status }.words())
    }

    /// A fighter's rebound was cleared: when it has a set of this lifetime, the set is told to clear.
    pub fn rebound_command(&self, key: usize, life: FighterLife) -> Put {
        let Some((version, _, set)) = self.change_sets(|members, _| members.find_life(key, life)) else { return Put::NoBuffer };
        let Some(set) = set else { return Put::Stored };
        self.lanes[set].put_rebound(&self.pool, version, &Record::Rebound { key, life }.words())
    }

    /// An article's ATTACK, CATCH or wind command: it is tracked as attacking now, and a CATCH's grab waits in its slot's lane.
    pub fn article_command(&self, id: u32, key: usize, grab: Option<(Shape, i32)>) -> Put {
        let Some((version, members, slot)) = self.change_articles(|members, version| members.command(id, key, version)) else {
            return Put::NoBuffer;
        };
        let (Some(slot), Some((shape, status))) = (slot, grab) else { return Put::Stored };
        self.lanes[FIGHTER_SETS + slot].put(&self.pool, version, members.ages[slot], &Record::Grab { key, id, life: None, shape, status }.words())
    }

    /// Phase 8: take the oldest command waiting in a lane (a fighter set's index, or `FIGHTER_SETS` plus an article slot's) for
    /// the set bound at `since`, up to the version `limit`.
    pub fn next_waiting(&self, lane: usize, since: u64, limit: u64) -> Option<Record> {
        let lane = &self.lanes[lane];
        loop {
            let stamp = lane.oldest(since, limit)?;
            if let Some(record) = lane.take(&self.pool, stamp).and_then(|words| Record::read(&words)) { return Some(record); }
        }
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use super::super::hitbox::FighterGenerations;
    use std::sync::{Arc, Mutex};

    impl Pool {
        fn free_count(&self) -> usize {
            self.available.load(Acquire) as usize
        }
    }

    fn shape(id: i32, extended: bool) -> Shape {
        Shape { id, joint: 7, size: 3.0, a: [1.0, 2.0, 3.0], b: extended.then_some([4.0, 5.0, 6.0]) }
    }

    fn new_refuses_no_nodes_and_more_than_the_index_holds() {
        assert!(Pool::new(0).is_none());
        assert!(Pool::new(0x1000).is_none());
        assert!(Pool::new(0xfff).is_some());
    }

    fn checkouts_from_several_threads_lose_and_duplicate_no_buffer() {
        let pool = Arc::new(Pool::new(8).unwrap());
        std::thread::scope(|scope| {
            for who in 0..4u64 {
                let pool = Arc::clone(&pool);
                scope.spawn(move || for n in 0..3000u64 {
                    let buffer = loop {
                        if let Some(buffer) = pool.take() { break buffer; }
                        std::thread::yield_now();
                    };
                    let tag = (who << 32) | n;
                    pool.write(buffer, &[tag; WORDS]);
                    assert_eq!(pool.read(buffer), [tag; WORDS]);
                    pool.give(buffer);
                });
            }
        });
        let buffers: Vec<_> = (0..pool.capacity()).map(|_| pool.take().unwrap()).collect();
        assert!(pool.take().is_none());
        for buffer in buffers { pool.give(buffer); }
        for capacity in [65usize, 128] {
            let pool = Pool::new(capacity).unwrap();
            for _ in 0..2 {
                let mut buffers: Vec<_> = (0..capacity).map(|_| pool.take().unwrap()).collect();
                assert!(pool.take().is_none());
                buffers.sort_unstable();
                assert_eq!(buffers, (1..=capacity as u32).collect::<Vec<_>>());
                for buffer in buffers { pool.give(buffer); }
            }
        }
    }

    fn values_and_records_survive_their_words() {
        let generations = FighterGenerations::new();
        let life = generations.stamp(0x10000).unwrap();
        let records = [
            Record::Grab { key: 16, id: life.id, life: Some(life), shape: shape(3, true), status: -17 },
            Record::Grab { key: 32, id: 3, life: None, shape: shape(-2, false), status: 7 },
            Record::Rebound { key: 16, life },
        ];
        for record in records {
            assert_eq!(Record::read(&record.words()), Some(record));
        }
        let members = Members { ids: [(1, 16), (2, 32), (0, 0), (4, 64)],
            ages: [3, 7, 0, 10], attacking: [true, false, false, true], touched: [5, 8, 0, 12] };
        assert_eq!(members_read(&members_words(&members)), members);
        let words = members_words(&members);
        assert_eq!(members_words(&members_read(&words)), words);
        let mut sets = SetMembers::NONE;
        sets.keys = [16, 32, 0, 64, 0, 96, 0, 0];
        sets.lives = [Some(life), None, None, Some(life), None, None, None, None];
        sets.bound = [9, 10, 0, 11, 0, 12, 0, 0];
        sets.used = [2, 2, 0, 5, 0, 1, 0, 0];
        let copy = sets_read(&sets_words(&sets));
        let words = sets_words(&sets);
        assert_eq!(sets_words(&sets_read(&words)), words);
        assert_eq!((copy.keys, copy.lives, copy.bound), (sets.keys, sets.lives, sets.bound));
        let mut a: Vec<_> = (0..FIGHTER_SETS).collect();
        let mut b = a.clone();
        a.sort_by_key(|&i| (sets.used[i], i));
        b.sort_by_key(|&i| (copy.used[i], i));
        assert_eq!(a, b);
    }

    fn the_pool_holds_every_cell_and_one_buffer_for_every_thread() {
        assert_eq!((ARTICLES, MAX_PENDING, FIGHTER_SETS, THREADS), (4, 24, 8, 800));
        assert_eq!(BUFFERS, 2 + 4 * 24 + 8 * 25 + 800);
        assert_eq!(BUFFERS, 2 + ARTICLES * MAX_PENDING + FIGHTER_SETS * (MAX_PENDING + 1) + THREADS);
        let link = Handoff::new().unwrap();
        let generations = FighterGenerations::new();
        assert_eq!(link.heap_bytes(), BUFFERS * WORDS * 8 + BUFFERS.div_ceil(64) * 8 + 12 * std::mem::size_of::<Lane>());
        assert_eq!(link.heap_bytes(), 222_336);
        for n in 1..=FIGHTER_SETS {
            let life = generations.stamp((n as u32) << 16).unwrap();
            let key = n * 16;
            for id in 0..MAX_PENDING {
                assert_eq!(link.catch_command(key, life, shape(id as i32, false), 10), Put::Stored);
            }
            assert_eq!(link.catch_command(key, life, shape(99, false), 10), Put::Full);
            assert_eq!(link.rebound_command(key, life), Put::Stored);
            for id in 0..MAX_PENDING {
                assert_eq!(link.catch_command(key, life, shape(id as i32, false), 10), Put::Stored);
            }
            assert_eq!(link.catch_command(key, life, shape(99, false), 10), Put::Full);
        }
        for id in 1..=ARTICLES as u32 {
            for n in 0..MAX_PENDING {
                assert_eq!(link.article_command(id, id as usize * 16, Some((shape(n as i32, false), 10))), Put::Stored);
            }
            assert_eq!(link.article_command(id, id as usize * 16, Some((shape(99, false), 10))), Put::Full);
        }
        let held: Vec<_> = (0..THREADS).map(|_| link.pool.take().unwrap()).collect();
        assert!(link.pool.take().is_none());
        for buffer in held { link.pool.give(buffer); }
    }

    fn a_full_lane_frees_a_cell_when_a_rebound_or_a_new_binding_comes() {
        let link = Handoff::new().unwrap();
        let generations = FighterGenerations::new();
        let life = generations.stamp(0x10000).unwrap();
        for id in 0..MAX_PENDING {
            assert_eq!(link.catch_command(16, life, shape(id as i32, false), 10), Put::Stored);
        }
        assert_eq!(link.catch_command(16, life, shape(99, false), 10), Put::Full);
        assert_eq!(link.rebound_command(16, life), Put::Stored);
        assert_eq!(link.catch_command(16, life, shape(42, false), 10), Put::Stored);
        assert_eq!(link.catch_command(16, life, shape(43, false), 10), Put::Stored);
        let (version, members) = link.sets();
        let slot = members.find_life(16, life).unwrap();
        assert_eq!(link.next_waiting(slot, members.bound[slot], version), Some(Record::Rebound { key: 16, life }));
        for id in [42, 43] {
            assert_eq!(link.next_waiting(slot, members.bound[slot], version),
                Some(Record::Grab { key: 16, id: life.id, life: Some(life), shape: shape(id, false), status: 10 }));
        }
        assert!(link.next_waiting(slot, members.bound[slot], version).is_none());
        for id in 0..MAX_PENDING {
            assert_eq!(link.catch_command(16, life, shape(id as i32, false), 10), Put::Stored);
        }
        for n in 2..=8 {
            let other = generations.stamp((n as u32) << 16).unwrap();
            assert_eq!(link.catch_command(n * 16, other, shape(n as i32, false), 10), Put::Stored);
        }
        let new_life = generations.stamp(9 << 16).unwrap();
        assert_eq!(link.catch_command(9 * 16, new_life, shape(100, false), 10), Put::Stored);
        let (version, members) = link.sets();
        let new_slot = members.find_life(9 * 16, new_life).unwrap();
        assert_eq!(new_slot, slot);
        assert_eq!(link.next_waiting(slot, members.bound[slot], version),
            Some(Record::Grab { key: 9 * 16, id: new_life.id, life: Some(new_life), shape: shape(100, false), status: 10 }));
        assert!(link.next_waiting(slot, members.bound[slot], version).is_none());
    }

    fn waiting_commands_from_several_threads_come_out_once_and_in_each_threads_order() {
        let link = Arc::new(Handoff::new().unwrap());
        let generations = FighterGenerations::new();
        let lives: Vec<_> = (1..=4).map(|n| generations.stamp((n as u32) << 16).unwrap()).collect();
        std::thread::scope(|scope| {
            for (who, &life) in lives.iter().enumerate() {
                let link = Arc::clone(&link);
                scope.spawn(move || for n in 0..3000 {
                    loop {
                        match link.catch_command((who + 1) * 16, life, shape(n, false), 10) {
                            Put::Stored => break,
                            Put::Full => std::thread::yield_now(),
                            Put::NoBuffer => panic!("buffer bound"),
                        }
                    }
                });
            }
            let mut next = [0; 4];
            while next.iter().sum::<i32>() < 12_000 {
                let (version, members) = link.sets();
                for slot in 0..FIGHTER_SETS {
                    while let Some(record) = link.next_waiting(slot, members.bound[slot], version) {
                        let Record::Grab { key, shape, .. } = record else { panic!("unexpected rebound") };
                        let who = key / 16 - 1;
                        assert_eq!(shape.id, next[who]);
                        next[who] += 1;
                    }
                }
                std::thread::yield_now();
            }
            assert_eq!(next, [3000; 4]);
        });
        assert_eq!(link.pool.free_count(), BUFFERS - 2);
    }

    fn a_change_that_loses_the_race_returns_its_buffer() {
        let link = Handoff::new().unwrap();
        let free = link.pool.free_count();
        let mut calls = 0;
        link.change_articles(|members, version| {
            calls += 1;
            if calls == 1 {
                link.change_articles(|members, version| members.track(2, 32, true, version)).unwrap();
            }
            members.track(1, 16, true, version)
        }).unwrap();
        assert_eq!(calls, 2);
        let articles = link.articles().1;
        assert!(articles.ids.contains(&(1, 16)) && articles.ids.contains(&(2, 32)));
        assert_eq!(link.pool.free_count(), free);

        let generations = FighterGenerations::new();
        let first = generations.stamp(0x10000).unwrap();
        let second = generations.stamp(0x20000).unwrap();
        let free = link.pool.free_count();
        let mut calls = 0;
        link.change_sets(|members, version| {
            calls += 1;
            if calls == 1 {
                link.change_sets(|members, version| members.index_for(32, second, version)).unwrap();
            }
            members.index_for(16, first, version)
        }).unwrap();
        assert_eq!(calls, 2);
        let sets = link.sets().1;
        assert!(sets.find_life(16, first).is_some() && sets.find_life(32, second).is_some());
        assert_eq!(link.pool.free_count(), free);
    }

    fn table_changes_from_several_threads_equal_one_order_of_them() {
        let link = Arc::new(Handoff::new().unwrap());
        let articles = Mutex::new(Vec::new());
        let sets = Mutex::new(Vec::new());
        let generations = FighterGenerations::new();
        let lives: Vec<_> = (1..=10).map(|n| generations.stamp((n as u32) << 16).unwrap()).collect();
        std::thread::scope(|scope| {
            for who in 0..4 {
                let (link, articles, sets, lives) = (Arc::clone(&link), &articles, &sets, &lives);
                scope.spawn(move || for n in 0..2000 {
                    let id = ((who * 2000 + n) % 6 + 1) as u32;
                    let key = id as usize * 16;
                    let (version, _, _) = link.change_articles(|m, v| m.track(id, key, true, v)).unwrap();
                    articles.lock().unwrap().push((version, id, key));
                    let fighter = (who * 2000 + n) % 10;
                    let life = lives[fighter];
                    let key = (fighter + 1) * 32;
                    let (version, _, _) = link.change_sets(|m, v| m.index_for(key, life, v)).unwrap();
                    sets.lock().unwrap().push((version, key, life));
                });
            }
        });
        let mut articles = articles.into_inner().unwrap();
        articles.sort_by_key(|entry| entry.0);
        assert!(articles.windows(2).all(|pair| pair[0].0 < pair[1].0));
        let mut expected = Members::default();
        for (version, id, key) in articles { expected.track(id, key, true, version); }
        assert_eq!(link.articles().1, expected);
        let mut changes = sets.into_inner().unwrap();
        changes.sort_by_key(|entry| entry.0);
        assert!(changes.windows(2).all(|pair| pair[0].0 < pair[1].0));
        let mut expected = SetMembers::NONE;
        for (version, key, life) in changes { expected.index_for(key, life, version); }
        let actual = link.sets().1;
        assert_eq!((actual.keys, actual.lives, actual.bound), (expected.keys, expected.lives, expected.bound));
        let mut a: Vec<_> = (0..FIGHTER_SETS).collect();
        let mut b = a.clone();
        a.sort_by_key(|&i| (actual.used[i], i));
        b.sort_by_key(|&i| (expected.used[i], i));
        assert_eq!(a, b);
        assert_eq!(link.pool.free_count(), BUFFERS - 2);
    }

    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("new_refuses_no_nodes_and_more_than_the_index_holds", new_refuses_no_nodes_and_more_than_the_index_holds as fn()),
            ("checkouts_from_several_threads_lose_and_duplicate_no_buffer", checkouts_from_several_threads_lose_and_duplicate_no_buffer as fn()),
            ("values_and_records_survive_their_words", values_and_records_survive_their_words as fn()),
            ("the_pool_holds_every_cell_and_one_buffer_for_every_thread", the_pool_holds_every_cell_and_one_buffer_for_every_thread as fn()),
            ("a_full_lane_frees_a_cell_when_a_rebound_or_a_new_binding_comes", a_full_lane_frees_a_cell_when_a_rebound_or_a_new_binding_comes as fn()),
            ("waiting_commands_from_several_threads_come_out_once_and_in_each_threads_order", waiting_commands_from_several_threads_come_out_once_and_in_each_threads_order as fn()),
            ("a_change_that_loses_the_race_returns_its_buffer", a_change_that_loses_the_race_returns_its_buffer as fn()),
            ("table_changes_from_several_threads_equal_one_order_of_them", table_changes_from_several_threads_equal_one_order_of_them as fn()),
        ]);
    }
    }
}
