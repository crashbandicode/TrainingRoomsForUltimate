// Complete stock 13.0.5 windows. Each SHA-256 is stored as two big-endian halves.
pub(super) type Window = (usize, usize, u128, u128);

pub(super) const MAIN: &[Window] = &[
    // damage energy getter
    (
        0x6d8cc0,
        64,
        0x3b4f7acd5d016e1b017f5cd57cac2245,
        0x523af24ee694f9d1c2095b750a811dc6,
    ),
    // damage stop
    (
        0x20ad720,
        192,
        0x2b675cd6a03b68723cd2b427df529ea1,
        0x53dd46028f52f6d0305ce0ca114752d6,
    ),
    // passive floor tail
    (
        0x696950,
        256,
        0x1f973022812f1408fcb54ea2914a69d7,
        0x9b065e074d24464827a47f3dd82c891d,
    ),
    // ground is touch
    (
        0x4635b0,
        128,
        0x5d55dec562728561760695eca77de719,
        0xbfc93aa973a635dda97723444473ef82,
    ),
    // native length
    (
        0x22779b0,
        192,
        0x876009d2b5ad20030fbb3394e5ed630e,
        0x753db7e3897d61fb5eba7b5c6aa9883c,
    ),
];

pub(super) const COMMON: &[Window] = &[
    // passive speed rule
    (
        0xed2e0,
        320,
        0x61426c0559dda7741228ab8fdc0f276c,
        0xef2dd3435bb717952f64976b5fc5f485,
    ),
    // floor decision
    (
        0xfd630,
        512,
        0xbf0751db94f8b92fcd04de9532ab879e,
        0x1a57fa9986753c3b0622fdf5495927a3,
    ),
    // floor passive predicate
    (
        0xfd8a0,
        128,
        0x1bfd10bb86b94fd36b6e7cc7db098166,
        0xb416c15b89284f028ea3908e2d7c1d5d,
    ),
    // floor reflect
    (
        0xfb4d0,
        832,
        0x560295dd787cf8e077e54000919ab206,
        0xc33c16998ee326417d5e013efc458c0e,
    ),
];
