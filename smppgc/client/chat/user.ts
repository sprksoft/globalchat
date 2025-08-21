export type LocalId = number;

export namespace LocalId {
  export function is_vaild(id: LocalId): boolean {
    if (id < 0) {
      return false;
    }
    return true;
  }
}

export enum Role {
  User = 0,
  Mod = 1,
  Admin = 2,
  Owner = 3,
}
export namespace Role {
  export function toString(role: Role): string {
    switch (role) {
      case Role.User:
        return "";
      case Role.Mod:
        return "mod";
      case Role.Admin:
        return "admin";
      case Role.Owner:
        return "owner";
    }
  }
}


export class User {
  username: string;
  role: Role;
  modBadge;

  constructor(name: string, modBadge = false, role: Role = Role.User) {
    this.username = name;
    this.modBadge = modBadge;
    this.role = role;
  }
}
export namespace User {
  export function system(): User {
    return new User("system", false, Role.User);
  }
  export function nonExisting(): User {
    return new User("Mr unknown", true, Role.Owner);
  }
}
