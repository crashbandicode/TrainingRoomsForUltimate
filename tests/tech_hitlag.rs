#[path = "../src/model/tech_hitlag.rs"]
mod hitlag;
use hitlag::{frames, Parameters};
fn parameters(k: f32) -> Parameters {
    Parameters { maximum: 30.0, multiplier: 0.65, add: 6.0, threshold: 10,
        object_multiplier: k, percent_multiplier: 1.2 }
}
#[test]
fn core_generic_damage_matches_k1_and_k0925_tables() {
    // Independent operator-owned struct.pack f32 oracle, hitlag_f32.py.
    let unit = [6,7,7,8,9,9,10,11,11,12,13,13,14,15,15,16,17,17,18,19,19,20,20,21,22,22,23,24,24,25];
    let reduced = [6,7,7,8,9,9,10,10,10,11,12,12,13,14,14,15,16,16,17,18,18,19,19,20,21,21,21,22,22,23];
    for (i, (&a, &b)) in unit.iter().zip(&reduced).enumerate() {
        assert_eq!(frames((i+1) as f32, 1.0, parameters(1.0)), Some(a));
        assert_eq!(frames((i+1) as f32, 1.0, parameters(0.925)), Some(b));
    }
}
#[test]
fn captured_r1_replay_is_bit_exact_before_and_after_percent_bonus() {
    let base_damage = f32::from_bits(0x415a6666);
    assert_eq!(frames(base_damage, f32::from_bits(0x3f333333), parameters(1.0)), Some(10));
    let added = base_damage * 1.2f32;
    assert_eq!((20.0f32 + added).to_bits(), 0x4211851f);
}
#[test]
fn hitlag_floor_and_nan_clamp_follow_native_f32_order() {
    let mut p = parameters(0.925);
    p.maximum = 10.0;
    p.threshold = 10;
    assert_eq!(frames(30.0, 1.0, p), Some(9)); // 0/0 -> FMIN NaN -> FMAXNM zero
    assert_eq!(frames(1.0, 1.0, parameters(1.0)), Some(6));
    assert_eq!(frames(30.0, 1.0, parameters(1.0)), Some(25));
    assert_eq!(frames(f32::MAX, 1.0, parameters(1.0)), Some(30));
}
#[test]
fn hitlag_refuses_invalid_params_damage_and_results() {
    for d in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -1.0] {
        assert_eq!(frames(d, 1.0, parameters(1.0)), None);
    }
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -1.0] {
        for field in 0..4 {
            let mut p = parameters(1.0);
            match field { 0 => p.maximum=value, 1 => p.object_multiplier=value,
                2 => p.percent_multiplier=value, _ => {} }
            assert_eq!(frames(12.0, if field==3 {value} else {1.0}, p), None);
        }
    }
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut p=parameters(1.0); p.add=value;
        assert_eq!(frames(12.0, 1.0, p), None);
        p=parameters(1.0); p.multiplier=value;
        assert_eq!(frames(12.0, 1.0, p), None);
    }
    for threshold in [0, -1, i32::MIN] {
        let mut p=parameters(1.0); p.threshold=threshold;
        assert_eq!(frames(12.0, 1.0, p), None);
    }
    let mut p=parameters(1.0); p.add=-100.0;
    assert_eq!(frames(12.0, 1.0, p), None);
    p=parameters(1.0); p.maximum=2147483648.0; p.add=p.maximum;
    assert_eq!(frames(12.0, 1.0, p), None);
    p=parameters(f32::MAX);
    assert_eq!(frames(30.0, 1.0, p), None);
    assert_ne!(frames(30.0, 1.0, parameters(1.0)), frames(30.0, 1.0, parameters(0.925)));
}

#[test]
fn provider_binding_is_once_idempotent_and_conflict_stays_unavailable() {
    use std::cell::Cell;
    let calls=Cell::new(0);
    let binding=hitlag::Binding::new();
    assert!(binding.ready(|| { calls.set(calls.get()+1); Ok::<_,()>(()) }));
    assert!(binding.ready(|| -> Result<(),()> { panic!("must not rebind") }));
    assert_eq!(calls.get(),1);
    let conflict=hitlag::Binding::new();
    assert!(!conflict.ready(|| Err::<(),_>("different injected resolver")));
    assert!(!conflict.ready(|| -> Result<(),()> { panic!("must preserve conflict") }));
}
#[test]
fn resolver_is_live_guarded_owned_and_bound_off_hook() {
    let source=include_str!("../src/runtime/features/tech_hitlag_read.rs");
    let resolve=&source[source.find("pub fn resolve(").unwrap()..];
    assert_eq!(resolve.matches("n.hitlag_held()?").count(),2);
    assert!(resolve.contains("let params=parameters(n)"));
    assert!(source.contains("object_multiplier:n.hitlag_k(map,0x192e58d48b)"));
    assert!(source.contains("percent_multiplier:multiplier(n)?"));
    for witness in ["table != base+0x4f69b78", "pair.ops != base+ops", "pair.value != object.checked_add(field)?", "slot(n,table,0x10,base+0x3bd350)"] {
        assert!(source.contains(witness), "{witness}");
    }
    let cpu=include_str!("../src/runtime/features/cpu.rs");
    let service=&cpu[cpu.find("pub fn service_tech_init()").unwrap()..cpu.find("static CONST_ATTEMPTS").unwrap()];
    assert!(service.find("HITLAG_BINDING.ready").unwrap()<service.find("TECH_INIT.service").unwrap());
    assert!(service.contains("super::tech::bind_hitlag(super::tech_hitlag::resolve)"));
    assert!(!service.contains("parameters()") && !service.contains("live_multiplier("));
}
#[test]
fn launch_resolves_after_owned_hold_without_owning_mutex() {
    let source=include_str!("../src/runtime/features/tech_adapter.rs");
    let prepare=&source[source.find("unsafe fn prepare(").unwrap()..source.find("unsafe fn position(").unwrap()];
    assert!(!prepare.contains("hitlag("));
    let launch=&source[source.find("    fn launch(&mut self)").unwrap()..source.find("    fn restore(&mut self)").unwrap()];
    let resolve=launch.find("(self.hitlag)(accessor, owned.launch.hit_damage)?").unwrap();
    assert!(launch[..resolve].contains("drop(slot)"));
    let rest=&launch[resolve..];
    assert!(rest.find("OWNED.try_lock()").unwrap()<rest.find("owned.phase = Phase::Flying").unwrap());
    assert!(rest.find("held_context(accessor)? != context").unwrap()<rest.find("set_sleep(accessor, false)").unwrap());
    assert!(!launch.contains("owned.prepared.stop"));
    let place=&source[source.find("    fn place(&mut self").unwrap()..source.find("    fn launch(&mut self)").unwrap()];
    assert!(place.contains("hold(accessor, owned)"));
}

#[path = "common/mod.rs"]
mod common;
#[path = "../src/runtime/features/tech_hitlag_guards.rs"]
mod guards;
#[test]
fn hitlag_guards_match_owned_64_byte_windows() {
    let Some(owned)=common::owned_main("hitlag_guards_match_owned_64_byte_windows") else {return};
    assert_eq!(guards::HITLAG_GUARDS.len(),5);
    for &(name,offset,bytes) in guards::HITLAG_GUARDS {
        assert_eq!(bytes,owned.window(offset),"{name} at {offset:x}");
    }
}
