//! Explicit callbacks through unscanned model code and the production resolver function pointer.
//! Source mapping is asserted by hook_locks::tech_callbacks_match_production_binding_and_model_calls.
pub const EDGES: &[(&str,&str)] = &[
    ("cpu_common::change_status_hook","cpu_common::change_status_body"),
    ("tech::frame","tech_adapter::place"),
    ("tech::frame","tech_adapter::attempt"),
    ("tech::frame","tech_adapter::restore"),
    ("tech::frame","tech_adapter::launch"),
    ("tech_adapter::launch","tech_hitlag::resolve"),
];
