# Legacy capture controls

Two synthetic schema-1 TMRC files, built without the production serializer. Their lowercase-hex filenames
encode their replay IDs. Each track has three 44-byte records for fighter kind 0, facing right. Neither file
contains game output or a player/CPU role key. The first has only entry 1; the second stores entry 1 before
entry 0. The entries' buttons and left sticks differ so a guessed source cannot pass the export assertions.
