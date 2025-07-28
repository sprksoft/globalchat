
export class User {
  username;
  role;
  modBadge;

  constructor(name, modBadge = false, role = 0) {
    this.username = name;
    this.modBadge = modBadge;
    this.role = role;
  }
}

