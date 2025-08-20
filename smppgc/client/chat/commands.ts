type LocalCommand = [string, () => undefined];

declare const local_commands: Array<LocalCommand>;

export function localCmd(name: string, func: () => undefined) {
  local_commands.push([name, func]);
}

export function execLocalCmd(cmd: string): boolean {
  for (const lcmd of local_commands) {
    if (cmd == lcmd[0]) {
      lcmd[1]()
      return true;
    }
  }
  return false;
}
