//! Production-used admission/completion proof, independent of native pointers.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Scene {
    pub epoch: u64,
    pub wrapper: usize,
    pub core: usize,
    pub mode: u32,
    pub melee: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Resource {
    pub address: usize,
    pub state: u32,
    pub kind: u32,
    pub hash: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct View {
    pub scene: Scene,
    pub resource_owner: usize,
    pub task_owner: usize,
    pub task: usize,
    pub task_state: u32,
    pub task_fiber: usize,
    pub disabled: u8,
    pub stop_mask: u32,
    pub state: u32,
    pub active: i32,
    pub incoming: i32,
    pub outgoing: i32,
    pub slots: [usize; 3],
    pub resources: [Option<Resource>; 4],
    pub descriptor: [u8; 0xb0],
    pub waiting_ready: bool,
    pub ui_busy: bool,
    pub roster_busy: bool,
    pub item_ready: bool,
}

impl View {
    pub fn kind(self) -> u32 {
        u32::from_le_bytes([self.descriptor[0], self.descriptor[1], self.descriptor[2], self.descriptor[3]])
    }
    pub fn hash(self) -> u64 {
        let mut bytes = [0; 8];
        bytes.copy_from_slice(&self.descriptor[8..16]);
        u64::from_le_bytes(bytes)
    }
    pub fn native_next(self) -> usize {
        usize::from(self.resources[0].is_some_and(|r| (r.state | 2) == 3))
    }
}

/// Native allocation identities, never dereferenced by pure policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StageLease {
    pub object: usize,
    pub resource: Resource,
}

/// Read only at the observed state6 acknowledgment boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ActivationRead {
    pub native_ready: u8,
    pub ready_bits: u64,
    pub readiness_stop_mask: u32,
    pub target_kind: u32,
    pub target_hash: u64,
    pub target_flag_500: u8,
    pub request_kind: u32,
    pub request_hash: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Commit {
    pub scene: Scene,
    pub resource_owner: usize,
    pub task_owner: usize,
    pub task: usize,
    pub incoming: usize,
    pub outgoing: usize,
    pub target: u32,
    pub hash: u64,
    pub original: StageLease,
    pub activated: Option<StageLease>,
}

fn check(ok: bool, why: &'static str) -> Result<(), &'static str> {
    if ok { Ok(()) } else { Err(why) }
}

pub(crate) fn same_owner(v: View, c: Commit) -> bool {
    v.scene == c.scene && v.resource_owner == c.resource_owner
        && v.task_owner == c.task_owner && v.task == c.task
}

pub(crate) fn current_request(queued: Scene, current: Scene, original: usize, active: usize) -> bool {
    queued == current && original != 0 && original == active
}

/// The native scene driver (14a569c..56b4), independently repeated at
/// 24b9a94..9aac, explicitly acknowledges state6 only when native dc is ready.
/// It does not force readiness or enqueue another load. The manager then owns
/// states7..12, including observer acknowledgments and outgoing destruction.
pub(crate) fn acknowledge(v: View, c: Commit, a: ActivationRead) -> Result<Option<Commit>, &'static str> {
    check(same_owner(v,c), "activation owner changed")?;
    check(c.activated.is_none(), "activation already acknowledged")?;
    check(v.state==6 && v.active==c.outgoing as i32
        && v.incoming==c.incoming as i32 && v.outgoing==c.outgoing as i32,
        "activation phase or pair changed")?;
    check(v.task_state==0 && v.task_fiber==0, "activation task not finalized")?;
    check(v.slots[c.outgoing]==c.original.object
        && v.resources[c.outgoing]==Some(c.original.resource)
        && v.kind()==c.original.resource.kind && v.hash()==c.original.resource.hash,
        "activation original allocation changed")?;
    check(v.slots.iter().enumerate().all(|(i,p)|i==c.incoming || i==c.outgoing || *p==0)
        && v.resources.iter().enumerate().all(|(i,r)|i==c.incoming || i==c.outgoing || r.is_none()),
        "activation conflicting native slot")?;
    let resource=v.resources[c.incoming].ok_or("activation target resource absent")?;
    check(v.slots[c.incoming]!=0 && v.slots[c.incoming]!=c.original.object
        && resource.address!=c.original.resource.address && resource.state==1
        && resource.kind==c.target && resource.hash==c.hash
        && a.target_kind==c.target && a.target_hash==c.hash && a.target_flag_500==1
        && a.request_kind==c.target && a.request_hash==c.hash,
        "activation target not constructed or changed")?;
    check(a.native_ready<=1, "activation native readiness inconsistent")?;
    // Suspension and a stale per-frame readiness pair are waits, not lost
    // ownership. Check all immutable authority above before deferring again.
    if v.disabled!=0 || v.stop_mask!=0 || a.readiness_stop_mask!=0
        || v.ui_busy || v.roster_busy
        || (crate::engine::stages::lobby_stage_cycle::waiting(v.scene.mode, v.scene.melee) && !v.waiting_ready)
        || a.native_ready==0 || a.ready_bits!=3 {
        return Ok(None);
    }
    let mut result=c;
    result.activated=Some(StageLease { object:v.slots[c.incoming],resource });
    Ok(Some(result))
}

/// State0 alone is insufficient: exact retained indices, active descriptor,
/// outgoing release, task finalization and no cache/extra slot are required.
pub(crate) fn complete(v: View, c: Commit) -> bool {
    same_owner(v, c) && v.state == 0 && v.task_state == 0 && v.task_fiber == 0
        && v.active == c.incoming as i32 && v.incoming == c.incoming as i32
        && v.outgoing == c.outgoing as i32 && v.kind() == c.target && v.hash() == c.hash
        && c.activated.is_some_and(|a| v.slots[c.incoming]==a.object
            && v.resources[c.incoming]==Some(a.resource)
            && a.resource.state==1 && a.resource.kind==c.target && a.resource.hash==c.hash)
        && v.slots.iter().enumerate().all(|(i, p)| i == c.incoming || *p == 0)
        && v.resources.iter().enumerate().all(|(i, r)| i == c.incoming || r.is_none())
}

pub(crate) fn admit(v: View, settled: Option<Commit>) -> Result<usize, &'static str> {
    check((v.scene.mode == 0x04040000 && v.scene.melee == 0x20)
        || (crate::engine::stages::lobby_stage_cycle::waiting(v.scene.mode, v.scene.melee) && v.waiting_ready), "not owned practice scene")?;
    check(v.disabled == 0 && v.stop_mask == 0 && !v.ui_busy && !v.roster_busy, "practice suspended")?;
    check(v.state == 0 && v.task_state == 0 && v.task_fiber == 0, "native transition or task busy")?;
    check(v.item_ready, "native item gate closed")?;
    check((0..3).contains(&v.active), "invalid active slot")?;
    let active = v.active as usize;
    check(v.slots[active] != 0, "missing active stage")?;
    // Both state0 and state1 cache objects are ownership, never spare capacity.
    check(v.resources[3].is_none(), "pending match cache occupied")?;
    check(v.resources[active].is_some_and(|r| r.state == 1 && r.kind == v.kind() && r.hash == v.hash()), "active resource mismatch")?;
    check(v.slots.iter().enumerate().all(|(i, p)| i == active || *p == 0)
        && v.resources.iter().enumerate().all(|(i, r)| i == active || r.is_none()), "other native slot owned")?;
    check((v.incoming == -1 && v.outgoing == -1)
        || settled.is_some_and(|c| complete(v, c)), "ambiguous incoming/outgoing ownership")?;
    let next = v.native_next();
    check(next != active && v.slots[next] == 0 && v.resources[next].is_none(), "native selected slot not free")?;
    Ok(next)
}

pub(crate) fn plan(v: View, settled: Option<Commit>, target: u32, hash: u64) -> Result<(Commit, [u8; 0xb0]), &'static str> {
    let next = admit(v, settled)?;
    check(target != v.kind(), "stage already active")?;
    check(hash != 0 && hash <= 0xffffffffff, "invalid stage hash")?;
    let mut descriptor = v.descriptor;
    descriptor[..4].copy_from_slice(&target.to_le_bytes());
    descriptor[8..16].copy_from_slice(&hash.to_le_bytes());
    Ok((Commit { scene: v.scene, resource_owner: v.resource_owner,
        task_owner: v.task_owner, task: v.task, incoming: next, outgoing: v.active as usize,
        target, hash, original:StageLease { object:v.slots[v.active as usize],
            resource:v.resources[v.active as usize].ok_or("active resource absent")? }, activated:None }, descriptor))
}

/// Typed ingress is also the last admission barrier before native publication.
/// Only kind/hash change: music, hazards and all other descriptor fields survive.
pub(crate) fn plan_selection(v: View, settled: Option<Commit>, selection: crate::engine::stages::lobby_stages::Selection,
    hash: u64) -> Result<(Commit, [u8; 0xb0]), &'static str> {
    let target = crate::engine::stages::lobby_stages::native_stage(selection)?;
    plan(v, settled, target, hash)
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::stages::lobby_stages::{Form, Selection};

    fn alternate_forms_complete_then_require_a_new_menu_and_preserve_match_cache_ownership() {
        for (mode, melee) in [(0x04040000, 0x20), (0x02010000, 0x0d), (0x87010101, 0x0d)] {
            let mut v = waiting();
            v.scene.mode = mode;
            v.scene.melee = melee;
            if melee == 0x20 {
                v.active = 0;
                v.slots.swap(0, 2);
                v.resources.swap(0, 2);
            }
            let (first, _) = plan_selection(v, None,
                Selection { stage: 95, form: Form::Omega }, 0x1b9fc8fb46).unwrap();
            let (constructed, ready) = loaded(v, first);
            let acknowledged = acknowledge(constructed, first, ready).unwrap().unwrap();
            let mut done = constructed;
            done.state = 0;
            done.active = first.incoming as i32;
            done.slots[first.outgoing] = 0;
            done.resources[first.outgoing] = None;
            done.descriptor[..4].copy_from_slice(&96u32.to_le_bytes());
            done.descriptor[8..16].copy_from_slice(&0x1b9fc8fb46u64.to_le_bytes());
            assert!(complete(done, acknowledged));
            assert!(!current_request(first.scene, done.scene, first.original.object,
                done.slots[done.active as usize]));
            let selection = Selection { stage: 95, form: Form::Battlefield };
            let (second, _) = plan_selection(done, Some(acknowledged), selection, 0x1b9fc8fb46).unwrap();
            assert_eq!(second.target, 97); // Shared package, distinct native form.
            for state in [0, 1] {
                let mut match_preloading = done;
                match_preloading.resources[3] = Some(Resource { address: 88, state, kind: 44, hash: 99 });
                assert_eq!(plan_selection(match_preloading, Some(acknowledged), selection, 0x1b9fc8fb46),
                    Err("pending match cache occupied"));
                assert_eq!(match_preloading.resources[3].unwrap().state, state);
            }
        }
    }
    fn waiting() -> View {
        let mut descriptor = [0x5a; 0xb0];
        descriptor[..4].copy_from_slice(&311u32.to_le_bytes());
        descriptor[8..16].copy_from_slice(&0x165a48f367u64.to_le_bytes());
        View { scene: Scene { epoch: 7, wrapper: 8, core: 16, mode: 0x02010000, melee: 0xd },
            resource_owner: 24, task_owner: 32, task: 40, task_state: 0, task_fiber: 0,
            disabled: 0, stop_mask: 0, state: 0, active: 2, incoming: -1, outgoing: -1,
            slots: [0, 0, 48], resources: [None, None, Some(Resource { address: 56,
                state: 1, kind: 311, hash: 0x165a48f367 }), None], descriptor,
            waiting_ready: true, ui_busy: false, roster_busy: false, item_ready: true }
    }
    fn loaded(mut v:View,c:Commit)->(View,ActivationRead) {
        v.state=6; v.incoming=c.incoming as i32; v.outgoing=c.outgoing as i32;
        v.slots[c.incoming]=72;
        v.resources[c.incoming]=Some(Resource { address:80,state:1,kind:c.target,hash:c.hash });
        (v,ActivationRead { native_ready:1,ready_bits:3,readiness_stop_mask:0,
            target_kind:c.target,target_hash:c.hash,target_flag_500:1,
            request_kind:c.target,request_hash:c.hash })
    }
    fn observed_state6_requires_native_readiness_and_exact_allocation_lease_once() {
        let v=waiting();let(c,_)=plan(v,None,0,0x123456789a).unwrap();
        let (loaded,ready)=loaded(v,c);
        assert!(!complete(loaded,c));
        let mut not_ready=ready;not_ready.native_ready=0;not_ready.ready_bits=1;
        assert_eq!(acknowledge(loaded,c,not_ready),Ok(None));
        let ack=acknowledge(loaded,c,ready).unwrap().unwrap();
        assert_eq!(ack.original,c.original);
        assert_eq!(ack.activated.unwrap().object,72);
        assert_eq!(acknowledge(loaded,ack,ready),Err("activation already acknowledged"));
        let mut changed=loaded;changed.slots[2]+=8;
        assert_eq!(acknowledge(changed,c,ready),Err("activation original allocation changed"));
        changed=loaded;changed.resources[2].as_mut().unwrap().address+=8;
        assert_eq!(acknowledge(changed,c,ready),Err("activation original allocation changed"));
        changed=loaded;changed.resources[3]=Some(Resource { address:88,state:0,kind:44,hash:99 });
        assert_eq!(acknowledge(changed,c,ready),Err("activation conflicting native slot"));
        changed=loaded;changed.scene.epoch+=1;
        assert_eq!(acknowledge(changed,c,ready),Err("activation owner changed"));
        changed=loaded;changed.task_fiber=96;
        assert_eq!(acknowledge(changed,c,ready),Err("activation task not finalized"));
        let mut wrong=ready;wrong.target_hash+=1;
        assert_eq!(acknowledge(loaded,c,wrong),Err("activation target not constructed or changed"));
        wrong=ready;wrong.native_ready=2;
        assert_eq!(acknowledge(loaded,c,wrong),Err("activation native readiness inconsistent"));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("alternate_forms_complete_then_require_a_new_menu_and_preserve_match_cache_ownership", alternate_forms_complete_then_require_a_new_menu_and_preserve_match_cache_ownership as fn()),
            ("observed_state6_requires_native_readiness_and_exact_allocation_lease_once", observed_state6_requires_native_readiness_and_exact_allocation_lease_once as fn()),
        ]);
    }
    }
}
