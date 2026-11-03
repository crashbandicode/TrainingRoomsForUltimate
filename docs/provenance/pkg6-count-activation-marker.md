# s6-11 count activation retains the completed install marker

Qualified r15 ab13a0f, NRO 0bbcfde5782d91d495dbea008b4495c332d901ec03d06b015e72fb448084cc50, failed the first count1→2 boundary. Raw archive `artifacts/trainingrooms-nro-archive/pkg6/s6-11/receipt.json` SHA bbb194ab8909e2e7ffb6981f13360291f2fdc50f81e9b8777f1ec2052c300d43 contains the complete run and actual screenshots. Count303 reserved generation1/epoch1/serial1 for entry77644931072. Completion308 observed canonical Samus state except changing=1; actual Samus body retained Lucario HUD. No publication followed. This is a retained failure, not acceptance.

Owned main13.0.5 ELF0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2, bounded `tmp-pkg6/s6-11/native-entry-state.asm`, establishes the distinction:

- Install653240 stages the descriptor;6533bc sets w22=1 and653438..444 writes that value to FE+5934. Nonactivating completion seals the fully loaded parked source.
- Native count155c950 calls656ee0 directly. Its ordinary body branch656f3c activates the body and656fb4..bc sets outer6. The count path does not call the full activation wrapper655f60.
- Full activation wrapper calls656ee0 at656134 or656520, then656530..534 clears FE+5934. This clear is outside656ee0. Loader650010 returns unless outer3; count has already set outer6.

Therefore EntryState::ready's changing0 requirement describes full active Reload completion, not direct count activation of a completed parked install. No native flag should be cleared by the plugin and no extra activation call is appropriate.

The correction introduces a separate count-promotion shape. Only a current completed source with a reserved PromotionToken uses it; its full native lifetime must still equal the sealed identity. The completed transaction's existing active_reload purpose selects the contract: parked installation may retain marker1 or reach0 after a native reset; active Reload requires0. Values above1 remain invalid. Every other canonical field, each body's owner/kind/phase/accessor/vtable, resource kind and all sealed lifetime identities retain their checks. The strict general active_identity, new active Reload completion, and unmanaged native-seat fallback remain unchanged.

The same purpose travels atomically with the existing Mii promotion receipt under the existing registry lock, so exact G consumers validate the same contract without taking STATE under G. Count completion, retained-seat admission, outgoing promoted reload admission and retained portrait consumers use the promotion shape. Fresh load completion never uses it. No new hook, flag write, native call, forced consumer release, or kind-only HUD publication is added.

Original host regression uses the actual s6-11 state, strict-reload negative control, both completion purposes, native reset marker0, and thirteen malformed/loading/parked shapes. Targeted execution and a new coherent native build/runtime remain required. This document does not close F9, cut-over, exact-Mii or lifetime acceptance.