// Function entry main+650010, before its native prologue or any Rust call.
// Dispatcher1755e7c -> functor651640 tail BR preserves x19/x20/x21/LR.
// Native input is FE in x0; return x0 is forwarded without modification.
.section .text.native_service,"ax",%progbits
.p2align 2
.global training_mod_native_service_entry
.hidden training_mod_native_service_entry
.hidden training_mod_native_service_dispatch
.type training_mod_native_service_entry,%function
training_mod_native_service_entry:
    sub sp, sp, #0x20
    stp x19, x20, [sp]
    stp x21, x30, [sp, #0x10]
    mov x1, sp
    bl training_mod_native_service_dispatch
    ldr x30, [sp, #0x18]
    add sp, sp, #0x20
    ret
.size training_mod_native_service_entry, .-training_mod_native_service_entry

// rc.7 FLAG 6: x2(x0, x1) on the plugin's service stack whose 16-aligned top is x3; returns its x0. It writes only x29,
// x30 and SP. x19-x28 are the callee's to preserve (AAPCS64). x29 holds the caller's SP across the call and points at the
// saved x29/x30 pair, so the frame-record chain runs from the service stack back to the caller's. TPIDR_EL0 and
// TPIDRRO_EL0 (the thread's TLS) are untouched: the same thread runs, on another stack.
.global training_mod_native_service_on_stack
.hidden training_mod_native_service_on_stack
.type training_mod_native_service_on_stack,%function
training_mod_native_service_on_stack:
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    mov sp, x3
    blr x2
    mov sp, x29
    ldp x29, x30, [sp], #16
    ret
.size training_mod_native_service_on_stack, .-training_mod_native_service_on_stack
