enum Perms {
    None = 0,
    Read = 1,
    Write = 2,
    Exec = 4,
    ReadWrite = 3,
}
declare function combinePerms(a: Perms, b: Perms): Perms;
declare function hasPerm(p: Perms, bit: Perms): boolean;
declare function defaultPerms(p?: Perms): Perms;
